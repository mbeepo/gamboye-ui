//! Adapted from https://github.com/parasyte/pixels/tree/39e84aacbe117347e7b8e7201c48184344aed9cc/examples/minimal-egui/src/main.rs

use std::collections::VecDeque;
use std::time;

use crate::comms::EmuMsgOut;
use crate::gui::Framework;
use comms::EmuMsgIn;
use error_iter::ErrorIter as _;
use pixels::{Error, Pixels, SurfaceTexture};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use gbc::{CpuStatus, Gbc, PpuStatus};

mod comms;
mod gui;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;
const WINDOW_WIDTH: u32 = WIDTH * 2;
const WINDOW_HEIGHT: u32 = HEIGHT * 2;

const FPS_LIMIT: u32 = 69;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let filename = std::env::args().nth(1).unwrap();
    let rom = std::fs::read(filename).unwrap();

    let event_loop = EventLoopBuilder::<EmuMsgOut>::with_user_event().build();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Beef Wellington")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture)?;
        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
        );

        (pixels, framework)
    };

    let (main_sender, mut emu_recv): (UnboundedSender<EmuMsgIn>, UnboundedReceiver<EmuMsgIn>) = tokio::sync::mpsc::unbounded_channel();
    let event_loop_proxy = event_loop.create_proxy();

    tokio::spawn(async move {
        let mbc = gbc::get_mbc(&rom);
        let mut emu = Gbc::new(mbc, false, true);
        emu.load_rom(&rom);
        
        loop {
            if let Ok(msg) = emu_recv.try_recv() {
                match msg {
                    EmuMsgIn::Sleep(until) => tokio::time::sleep_until(until.into()).await,
                }
            }

            match emu.step() {
                (Ok(CpuStatus::Run), ppu_status) => {
                    match ppu_status {
                        PpuStatus::VBlank => {
                            event_loop_proxy.send_event(EmuMsgOut::RequestRedraw(emu.cpu.ppu.fb.clone())).unwrap();
                        },
                        PpuStatus::Drawing => {},
                    }
                },
                (Ok(CpuStatus::Stop), _) => {
                    todo!("Die");
                },
                (Ok(CpuStatus::Break), _) => unimplemented!(),
                (Err(err), _) => {
                    eprintln!("{err}");
                }
            }
        }
    });

    let mut fps = 0;
    let mut fps_buf: VecDeque<u32> = vec![0; 10].into();
    let mut last_second = time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                framework.resize(size.width, size.height);
            }
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the flightless bird
                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render videog gaming
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context);

                    Ok(())
                });

                // Basic error handling
                if let Err(err) = render_result {
                    log_error("pixels.render", err);
                    *control_flow = ControlFlow::Exit;
                }
            },
            Event::UserEvent(msg) => {
                match msg {
                    EmuMsgOut::RequestRedraw(fb) => {
                        pixels.frame_mut().copy_from_slice(&fb);
                        window.request_redraw();

                        let elapsed = time::Instant::now().duration_since(last_second);

                        if elapsed.as_millis() >= 1000 {
                            fps_buf.pop_front();
                            fps_buf.push_back(fps);
                            let fps_average: u32 = fps_buf.iter().sum::<u32>() / fps_buf.iter().filter(|&&e| e > 0).count() as u32;

                            framework.gui.fps = fps;
                            framework.gui.fps_average = fps_average;
                            framework.gui.fps_min = Some(fps.min(framework.gui.fps_min.unwrap_or(u32::MAX)));
                            framework.gui.fps_max = fps.max(framework.gui.fps_max);

                            fps = 0;
                            last_second = time::Instant::now();
                        } else {
                            fps += 1;

                            if fps >= FPS_LIMIT {
                                let until = last_second + time::Duration::from_millis(1000);
                                main_sender.send(EmuMsgIn::Sleep(until)).unwrap();
                            }
                        }
                    },
                }
            },
            _ => (),
        }
    });

    Ok(())
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    eprintln!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        eprintln!("  Caused by: {source}");
    }
}
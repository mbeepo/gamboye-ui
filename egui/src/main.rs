//! https://github.com/parasyte/pixels/tree/39e84aacbe117347e7b8e7201c48184344aed9cc/examples/minimal-egui

use crate::comms::EmuMsgOut;
use crate::gui::Framework;
use error_iter::ErrorIter as _;
use log::{error, info};
use pixels::{Error, Pixels, SurfaceTexture};
use tokio::sync::mpsc::{Receiver, Sender};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use tokio::sync::mpsc::error::TryRecvError;

use gbc::{CpuStatus, Gbc, PpuStatus};

mod comms;
mod gui;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let filename = std::env::args().nth(1).unwrap();
    
    let rom = std::fs::read(filename).unwrap();

    let event_loop = EventLoop::default();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
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

    let (emu_sender, mut emu_receiver): (Sender<EmuMsgOut>, Receiver<EmuMsgOut>) = tokio::sync::mpsc::channel(16);
    let event_loop_proxy = event_loop.create_proxy();

    tokio::spawn(async move {
        let mbc = gbc::get_mbc(&rom);
        let mut emu = Gbc::new(mbc, false, true);
        emu.load_rom(&rom);

        println!("Emu running methinks");

        // TODO:
        //  - Messages with UI thread
        //  - Limit framerate
        loop {
            match emu.step() {
                (Ok(CpuStatus::Run), ppu_status) => {
                    match ppu_status {
                        PpuStatus::VBlank => {
                            emu_sender.send(EmuMsgOut::RequestRedraw).await.unwrap();
                            event_loop_proxy.send_event(()).unwrap();
                        },
                        PpuStatus::Drawing => {
                            for px in &emu.cpu.ppu.queue {
                                emu_sender.send(EmuMsgOut::FramebufferUpdate(*px)).await.unwrap();
                                event_loop_proxy.send_event(()).unwrap();
                            }

                            emu.cpu.ppu.queue.clear();
                        },
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
            Event::UserEvent(_) => {
                match emu_receiver.try_recv() {
                    Ok(msg) => {
                        match msg {
                            EmuMsgOut::FramebufferUpdate(px) => {
                                let fb = pixels.frame_mut();
                                let index = px.x as usize + px.y as usize * WIDTH as usize;

                                fb[index*4..index*4+4].copy_from_slice(&px.color.to_be_bytes());
                            },
                            EmuMsgOut::RequestRedraw => window.request_redraw(),
                        }
                    },
                    Err(TryRecvError::Empty) => {},
                    Err(TryRecvError::Disconnected) => eprintln!("The thread of prophecy has been severed"),
                }
            },
            _ => (),
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    eprintln!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        eprintln!("  Caused by: {source}");
    }
}
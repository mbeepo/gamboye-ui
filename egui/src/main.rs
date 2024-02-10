//! https://github.com/parasyte/pixels/tree/39e84aacbe117347e7b8e7201c48184344aed9cc/examples/minimal-egui

#![deny(clippy::all)]
#![forbid(unsafe_code)]

use crate::gui::Framework;
use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use gbc::{CpuStatus, Gbc,  PpuStatus};

mod gui;

const SCALE_FACTOR: u32 = 1;

const INTERNAL_WIDTH: u32 = 160;
const INTERNAL_HEIGHT: u32 = 144;
const WIDTH: u32 = INTERNAL_WIDTH * SCALE_FACTOR;
const HEIGHT: u32 = INTERNAL_HEIGHT * SCALE_FACTOR;

fn main() -> Result<(), Error> {
    env_logger::init();

    let filename = std::env::args().nth(1).unwrap();
    dbg!(&filename);

    let rom = std::fs::read(filename).unwrap();

    let mbc = gbc::get_mbc(&rom);
    let mut emu = Gbc::new(mbc, false, true);
    emu.load_rom(&rom);

    let event_loop = EventLoop::new();
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

            // Update internal state and request a redraw
            match emu.step() {
                (Ok(CpuStatus::Run), ppu_status) => {
                    match ppu_status {
                        PpuStatus::VBlank => {
                            emu.cpu.ppu.status = PpuStatus::Drawing;
                            window.request_redraw()
                        },
                        PpuStatus::Drawing => {},
                    }
                },
                (Ok(CpuStatus::Stop), _) => {
                    *control_flow = ControlFlow::Exit;
                    return
                },
                (Ok(CpuStatus::Break), _) => unimplemented!(),
                (Err(err), _) => {
                    error!("{err}");
                }
            }

            // Update internal state and request a redraw
            // world.update();
            // window.request_redraw();
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the world
                emu.draw(pixels.frame_mut());
                // world.draw(pixels.frame_mut());
                println!("drawing...");

                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
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
            }
            _ => (),
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}
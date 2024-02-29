use std::{sync::{atomic::Ordering, Arc}, time::{Duration, Instant}};

use egui::{pos2, vec2, Color32, ColorImage, Context, InnerResponse, Mesh, Rect, Shape, TextureOptions};
use gbc::{Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, gui::{EmuState, EmuWindow, InnerEmuState}};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;
const MAX_FRAMERATE: usize = 60;

pub struct Emu {
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
    state: Arc<InnerEmuState>,
}

#[derive(Clone, Debug)]
pub enum EmuStatus {
    Fresh,
    Running,
    Stopped,
    Break,
    LoadingRom,
}

impl Default for EmuStatus {
    fn default() -> Self {
        Self::Fresh
    }
}

#[derive(Clone, Copy, Debug, )]
pub enum EmuError {
    Uninitialized,
    What,
}

impl Emu {
    pub fn new(egui_ctx: Context, ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>, state: Arc<InnerEmuState>) -> Self {
        let inner = None;

        Self {
            inner,
            ui_channel,
            egui_ctx,
            state,
        }
    }

    pub fn init(&mut self, rom: &[u8]) {
        let mbc = gbc::get_mbc(rom);
        let mut emu = Gbc::new(mbc, false, true);
        emu.load_rom(rom);
        self.inner = Some(emu);
    }

    pub fn run(mut self) -> Result<(), EmuError> {
        if let Some(mut emu) = self.inner {
            tokio::spawn(async move {
                *self.state.status.lock() = EmuStatus::Running;

                loop {
                    match self.ui_channel.try_recv() {
                        Ok(msg) => {
                            match msg {
                                EmuMsgIn::Exit => return,
                                EmuMsgIn::Pause => {},
                                EmuMsgIn::Resume => *self.state.status.lock() = EmuStatus::Running,
                                EmuMsgIn::LoadRom => return, // this instance should be dropped and a new instance should replace it
                                _ => {}
                            }
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                        Err(mpsc::error::TryRecvError::Disconnected) => return,
                    }

                    match *self.state.status.lock() {
                        EmuStatus::Running => { 
                            let (cpu_status, ppu_status) = emu.step();

                            match ppu_status {
                                PpuStatus::VBlank => {
                                    // *self.state.fb.lock() = emu.cpu.ppu.fb.chunks(4).map(|bytes| Color32::from_rgb(bytes[0], bytes[1], bytes[2])).collect();
                                    *self.state.fb.lock() = emu.cpu.ppu.fb.chunks(4).flat_map(|bytes| [bytes[0], bytes[1], bytes[2]]).collect();
                                    self.state.fb_pending.store(true, Ordering::Relaxed);
                                    self.egui_ctx.request_repaint();
                                },
                                PpuStatus::Drawing => {}
                            }
                        }
                        _ => {}
                    }
                }
            });

            return Ok(())
        }

        Err(EmuError::Uninitialized)
    }
}

pub fn show(ctx: &Context, state: &mut EmuWindow) -> InnerResponse<()> {
    egui::CentralPanel::default().show(ctx, |ui| {
        if state.emu.atoms.fb_pending.load(Ordering::Relaxed) {
            state.emu.atoms.fb_pending.store(false, Ordering::Relaxed);
            
            let system_fb = state.emu.atoms.fb.lock().clone();
            if system_fb.len() != (WIDTH * HEIGHT * 3) {
                ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), WIDTH * HEIGHT * 3));
                return;
            }

            let new_display = ColorImage::from_rgb([WIDTH, HEIGHT], &system_fb);

            if new_display != state.emu.display {
                state.emu.texture = ctx.load_texture("emu_display", new_display, TextureOptions::NEAREST);
                state.emu.display_mesh = Mesh::with_texture(state.emu.texture.id());
            }

            if state.emu.rect_changed {
                state.emu.display_mesh.clear();
                state.emu.display_mesh.add_rect_with_uv(
                    state.emu.display_rect,
                    Rect::from_two_pos(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE
                );
                state.emu.rect_changed = false;
            }

            // let size = gui.display_rect.size();
            // let pos = gui.display_rect.min;
            // let scale = (size / vec2(WIDTH as f32, HEIGHT as f32)).min_elem();
            
            // for y in 0..HEIGHT{
            //     for x in 0..WIDTH {
            //         gui.display_mesh.add_colored_rect(
            //             Rect::from_min_max(
            //                 pos + vec2(x as f32 * scale, y as f32 * scale),
            //                 pos + vec2(x as f32 * scale + scale, y as f32 * scale + scale)
            //             ), system_fb[x + y * WIDTH]);
            //     }
            // }

            state.perf.frames += 1;
            
            let now = Instant::now();

            let Some(last_second) = state.perf.last_second else {
                state.perf.last_second = Some(now);
                return;
            };

            if now.duration_since(last_second).as_millis() >= 1000 {
                state.perf.last_second = Some(now);
                state.perf.fps_history.push_back(state.perf.frames);
                state.perf.frames = 0;
            } else if state.perf.frames >= MAX_FRAMERATE {
                dbg!(state.perf.frames, MAX_FRAMERATE, last_second, last_second.elapsed(), now);

                if let Some(ref emu_channel) = state.emu.sender {
                    let awaken = last_second + Duration::from_millis(1000);
                    state.emu.sleep_until = Some(awaken);
                    emu_channel.send(EmuMsgIn::Pause).unwrap();
                    state.emu.sleep_until = None;
                }
            }

            if let Some(ref awaken) = state.emu.sleep_until { 
                if now > *awaken { 
                    if let Some(ref emu_channel) = state.emu.sender {
                        emu_channel.send(EmuMsgIn::Resume).unwrap();
                    }
                }
            };
        }

        let display = Shape::Mesh(state.emu.display_mesh.clone());
        ui.painter().add(display);
    })
}
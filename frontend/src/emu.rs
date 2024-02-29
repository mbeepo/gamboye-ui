use std::{sync::{atomic::Ordering, Arc}, time::{Duration, Instant}};

use egui::{pos2, vec2, Color32, Context, InnerResponse, Mesh, Rect, Shape};
use gbc::{Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, gui::{EmuState, EmuWindow}};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;
const MAX_FRAMERATE: usize = 60;

pub struct Emu {
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
    state: Arc<EmuState>,
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
    pub fn new(egui_ctx: Context, ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>, state: Arc<EmuState>) -> Self {
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
                                    *self.state.fb.lock() = emu.cpu.ppu.fb.chunks(4).map(|bytes| Color32::from_rgb(bytes[0], bytes[1], bytes[2])).collect();
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

pub fn show(ctx: &Context, gui: &mut EmuWindow) -> InnerResponse<()> {
    egui::CentralPanel::default().show(ctx, |ui| {
        if gui.state.emu_state.fb_pending.load(Ordering::Relaxed) {
            gui.state.emu_state.fb_pending.store(false, Ordering::Relaxed);
            
            let system_fb = gui.state.emu_state.fb.lock().clone();
            if system_fb.len() != (WIDTH * HEIGHT) {
                ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), WIDTH * HEIGHT));
                return;
            }

            gui.display_mesh.clear();

            let size = gui.display_rect.size();
            let pos = gui.display_rect.min;
            let scale = (size / vec2(WIDTH as f32, HEIGHT as f32)).min_elem();
            
            for y in 0..HEIGHT{
                for x in 0..WIDTH {
                    gui.display_mesh.add_colored_rect(
                        Rect::from_min_max(
                            pos + vec2(x as f32 * scale, y as f32 * scale),
                            pos + vec2(x as f32 * scale + scale, y as f32 * scale + scale)
                        ), system_fb[x + y * WIDTH]);
                }
            }

            gui.frames += 1;
            
            let now = Instant::now();

            let Some(last_second) = gui.state.perf_state.last_second else {
                gui.state.perf_state.last_second = Some(now);
                return;
            };

            if now.duration_since(last_second).as_millis() >= 1000 {
                gui.state.perf_state.last_second = Some(now);
                gui.state.perf_state.fps_history.push_back(gui.frames);
                gui.frames = 0;
            } else if gui.frames >= MAX_FRAMERATE {
                dbg!(gui.frames, MAX_FRAMERATE, last_second, last_second.elapsed(), now);

                if let Some(ref emu_channel) = gui.emu_channel {
                    let awaken = last_second + Duration::from_millis(1000);
                    gui.sleep_until = Some(awaken);
                    emu_channel.send(EmuMsgIn::Pause).unwrap();
                    gui.sleep_until = None;
                }
            }

            if let Some(ref awaken) = gui.sleep_until { 
                if now > *awaken { 
                    if let Some(ref emu_channel) = gui.emu_channel {
                        emu_channel.send(EmuMsgIn::Resume).unwrap();
                    }
                }
            };
        }

        let display = Shape::Mesh(gui.display_mesh.clone());
        ui.painter().add(display);

        let mut trans_mesh = Mesh::with_texture(gui.texture.id());
        trans_mesh.add_rect_with_uv(
            Rect::from_min_size(pos2(64.0, 89.0), vec2(64.0, 64.0)),
            Rect::from_two_pos(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE
        );
        
        let trans = Shape::Mesh(trans_mesh);
        ui.painter().add(trans);
    })
}
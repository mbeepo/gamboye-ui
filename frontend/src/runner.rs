use std::{fmt::Display, sync::{atomic::Ordering, Arc}, time::{Duration, Instant}};

use egui::{pos2, vec2, Color32, ColorImage, Context, InnerResponse, Mesh, Pos2, Rect, Rounding, Shape, TextureOptions};
use gbc::{Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, gui::TopState, state::{EmuState, InnerEmuState}};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

pub struct Emu {
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
    state: Arc<InnerEmuState>,
}

#[derive(Clone, Copy, Debug)]
pub enum EmuStatus {
    Fresh,
    Running,
    Stopped,
    Break,
    LoadingRom,
}

impl Display for EmuStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
                                EmuMsgIn::Pause => {
                                    *self.state.status.lock() = EmuStatus::Stopped
                                },
                                EmuMsgIn::Resume => {
                                    *self.state.status.lock() = EmuStatus::Running
                                },
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
                                    *self.state.fb.lock() = emu.cpu.ppu.fb.clone();
                                    emu.cpu.ppu.debug_show(&emu.cpu.memory, [16, 8], &mut *self.state.vram.lock());
                                    self.state.fb_pending.store(true, Ordering::Relaxed);
                                    self.egui_ctx.request_repaint();
                                },
                                PpuStatus::Drawing => {}
                            }
                        },
                        _ => {}
                    }
                }
            });

            return Ok(())
        }

        Err(EmuError::Uninitialized)
    }
}
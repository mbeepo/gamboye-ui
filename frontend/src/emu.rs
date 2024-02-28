use std::sync::{atomic::Ordering, Arc};

use egui::{Color32, Context};
use gbc::{Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, gui::EmuState};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

pub struct Emu {
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
    state: Arc<EmuState>,
}

#[derive(Clone, Copy, Debug)]
pub enum EmuStatus {
    Fresh,
    Running,
    Stopped,
    Break,
}

impl Default for EmuStatus {
    fn default() -> Self {
        Self::Fresh
    }
}

#[derive(Clone, Copy, Debug)]
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

    pub fn run(self) -> Result<(), EmuError> {
        *self.state.status.lock() = EmuStatus::Running;

        if let Some(mut emu) = self.inner {
            let state = self.state.clone();

            loop {
                match *self.state.status.lock() {
                    EmuStatus::Running => { 
                        let (cpu_status, ppu_status) = emu.step();

                        match ppu_status {
                            PpuStatus::VBlank => {
                                *self.state.fb.lock() = emu.cpu.ppu.fb.chunks(4).map(|bytes| Color32::from_rgb(bytes[0], bytes[1], bytes[2])).collect();
                                // *self.state.fb.lock() = emu.cpu.ppu.fb.clone();
                                self.state.fb_pending.store(true, Ordering::Relaxed);
                                self.egui_ctx.request_repaint();
                            },
                            PpuStatus::Drawing => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        println!("[EMU] self.inner is None :(");

        Err(EmuError::Uninitialized)
    }
}
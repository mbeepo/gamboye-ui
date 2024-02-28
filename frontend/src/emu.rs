use std::{sync::{atomic::Ordering, Arc}, thread::sleep, time::Instant};

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

    pub fn run(mut self) -> Result<(), EmuError> {
        *self.state.status.lock() = EmuStatus::Running;
        let mut new_rom: Option<Vec<u8>> = None;

        if let Some(ref mut emu) = self.inner {
            let state = self.state.clone();

            loop {
                match self.ui_channel.try_recv() {
                    Ok(msg) => {
                        match msg {
                            EmuMsgIn::Exit => return Ok(()),
                            EmuMsgIn::Pause(duration) => {
                                *self.state.status.lock() = EmuStatus::Stopped;
                                let start = Instant::now();
                                println!("Too fast !! Going sleepo for {}us ({}us)", duration.as_micros(), start.elapsed().as_micros());
                                sleep(duration);
                                println!("Sleepo complete ({}us)", start.elapsed().as_micros());
                                *self.state.status.lock() = EmuStatus::Running;
                            },
                            EmuMsgIn::Resume => *self.state.status.lock() = EmuStatus::Running,
                            EmuMsgIn::LoadRom(rom) => {
                                *self.state.status.lock() = EmuStatus::LoadingRom;
                                new_rom = Some(rom);
                                break;
                            },
                            _ => {}
                        }
                    },
                    Err(mpsc::error::TryRecvError::Empty) => {},
                    Err(mpsc::error::TryRecvError::Disconnected) => return Ok(()),
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
        }

        let status = self.state.status.lock().clone();

        match status {
            EmuStatus::LoadingRom => {
                self.init(&new_rom.unwrap());
                return self.run();
            },
            _ => {}
        }

        Err(EmuError::Uninitialized)
    }
}
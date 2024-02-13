use std::sync::Arc;

use egui::{mutex::Mutex, Context};
use gbc::{CpuStatus, Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, gui::{EmuState, State}};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

pub struct Emu {
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
    state: Arc<Mutex<EmuState>>,
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
}

impl Emu {
    pub fn new(egui_ctx: Context, ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>, state: Arc<Mutex<EmuState>>) -> Self {
        let inner = None;

        Self {
            inner,
            ui_channel,
            egui_ctx,
            state,
        }
    }

    pub fn run(&mut self) -> Result<(), EmuError> {
        if let Some(emu) = self.inner {
            let state = self.state.clone();

            tokio::spawn(async move {
                loop {
                    match self.state.lock().status {
                        EmuStatus::Running => { 
                            let (cpu_status, ppu_status) =  emu.step();

                            match ppu_status {
                                PpuStatus::VBlank => {
                                    
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
     
        Err(EmuError::Uninitialized)
    }
}
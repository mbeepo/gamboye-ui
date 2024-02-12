use std::sync::Arc;

use egui::Context;
use gbc::Gbc;
use tokio::sync::{mpsc, Mutex};

use crate::{comms::EmuMsgIn, gui::State};

pub struct Emu {
    state: Arc<Mutex<State>>,
    inner: Option<Gbc>,
    ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>,
    egui_ctx: Context,
}

impl Emu {
    pub fn new(state: Arc<Mutex<State>>, egui_ctx: Context, ui_channel: mpsc::UnboundedReceiver<EmuMsgIn>) -> Self {
        let inner = None;

        Self {
            state,
            inner,
            ui_channel,
            egui_ctx
        }
    }
}
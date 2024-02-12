use std::sync::Arc;

use eframe::App;
use tokio::sync::{mpsc, Mutex};

use crate::{comms::EmuMsgIn, emu::Emu};

pub struct State {
}

pub struct EmuWindow {
    state: Arc<Mutex<State>>,
    emu_channel: mpsc::UnboundedSender<EmuMsgIn>,
    emu: Emu,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // From https://docs.rs/egui/0.26.1/egui/struct.Context.html:
        //     "Context is cheap to clone, and any clones refers to the same mutable data (Context uses refcounting internally)."
        let ctx = cc.egui_ctx.clone();
        let state = Arc::new(Mutex::new(State { }));
        let state_clone = state.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let emu = Emu::new(state_clone, ctx, emu_recv);

        Self {
            state,
            emu_channel: emu_send,
            emu,
        }
    }
}

impl App for EmuWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        
    }
}
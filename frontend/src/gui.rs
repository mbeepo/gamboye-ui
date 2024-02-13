use std::{sync::Arc, time::Instant};

use eframe::App;
use egui::{mutex::Mutex, Color32, ColorImage, ViewportBuilder};
use tokio::sync::mpsc;

use crate::{comms::{EmuMsgIn, EmuMsgOut}, emu::{self, Emu, EmuStatus}};

#[derive(Clone, Default)]
pub struct State {
    emu_state: Arc<Mutex<EmuState>>,
    perf_state: Arc<Mutex<PerfState>>,
}

#[derive(Clone, Debug, Default)]
pub struct EmuState {
    pub fb: Vec<Color32>,
    pub status: EmuStatus,
}

#[derive(Clone, Debug, Default)]
pub struct PerfState {
    pub open: bool,
    pub last_second: Option<Instant>,
    pub fps_history: Vec<usize>,
    pub min_fps: usize,
    pub max_fps: usize,
}

pub struct EmuWindow {
    emu_channel: mpsc::UnboundedSender<EmuMsgIn>,
    emu: Emu,
    state: State,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // From https://docs.rs/egui/0.26.1/egui/struct.Context.html:
        //     "Context is cheap to clone, and any clones refers to the same mutable data (Context uses refcounting internally)."
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let state = State::default();
        let emu = Emu::new(ctx, emu_recv, state.emu_state.clone());
        let state = Default::default();
        let emu_display = Arc::new(ColorImage::new([160, 144], Color32::BLACK));

        Self {
            emu_channel: emu_send,
            emu,
            state,
        }
    }
}

impl App for EmuWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.state.perf_state.lock().open, "Performance");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // TODO:
            //  Perhaps use `Shape::Callback`
            ui.painter().extend(shapes)
        });

        ctx.show_viewport_deferred(
            "performance".to_owned(),
            ViewportBuilder::default().with_title("Performance Stats"),
            |ctx, _viewport_class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("beepo is here");
                });
        });
    }
}
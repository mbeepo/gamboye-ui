use eframe::App;
use egui::{pos2, ColorImage, KeyboardShortcut, Modifiers, Pos2, TextureOptions};
use tokio::sync::mpsc;

use crate::{runner::Emu, state::{DebugState, EmuState, PerfState}};

pub mod emu;
pub mod perf;
pub mod debug;

pub const BASE_DISPLAY_POS: Pos2 = pos2(0.0, 0.0);
const MAX_FRAMERATE: usize = 60;

const PERF_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, egui::Key::P);
const DEBUG_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, egui::Key::D);

pub struct TopState {
    pub emu: EmuState,
    pub perf: PerfState,
    pub debug: DebugState,
}

impl TopState {
    pub fn new(cc: &eframe::CreationContext<'_>, rom: Vec<u8>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let emu_state = EmuState::new(&cc.egui_ctx, emu_send);
        let perf = Default::default();
        let debug = Default::default();
        
        *emu_state.atoms.fb.lock() = vec![Default::default(); crate::runner::WIDTH * crate::runner::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, emu_state.atoms.clone());
        emu.init(&rom);
        emu.run().unwrap();

        Self {
            emu: emu_state,
            perf,
            debug,
        }
    }
}

impl App for TopState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.perf.open, "Performance");

                if ui.checkbox(&mut self.debug.open, "Debug").changed() {
                    self.debug.vram = Some(ctx.load_texture(
                        "debug_vram",
                        ColorImage::from_rgb([128, 64], &self.emu.atoms.vram.lock().clone()),
                        TextureOptions::NEAREST,
                    ));
                }

                ()
            });
        });
            
        if self.perf.open {
            perf::show(ctx, &mut self.perf);
        }

        if self.debug.open {
            debug::show(ctx, &mut self.debug);
        }

        let res = emu::show(ctx, self);

        if res.response.rect != self.emu.display_rect {
            self.emu.display_rect = res.response.rect;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&PERF_SHORTCUT)) {
            self.perf.open = !self.perf.open;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&DEBUG_SHORTCUT)) {
            self.debug.open = !self.debug.open;
        }
    }
}
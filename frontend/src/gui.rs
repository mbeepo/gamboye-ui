use eframe::App;
use egui::{pos2, KeyboardShortcut, Modifiers, Pos2, ViewportId};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgOut, runner::{Emu, EmuStatus}, state::{DebugState, EmuState, PerfState}};

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
        let (ui_send, emu_recv) = mpsc::unbounded_channel();
        let (emu_send, ui_recv) = mpsc::unbounded_channel();
        let emu_state = EmuState::new(&cc.egui_ctx, ui_send, ui_recv);
        let perf = Default::default();
        
        *emu_state.atoms.fb.lock() = vec![Default::default(); crate::runner::WIDTH * crate::runner::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, emu_send, emu_state.atoms.clone());
        let debug = DebugState {
            stopped: *emu_state.atoms.status.lock() == EmuStatus::Stopped,
            ..Default::default()
        };
        
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
        if let Ok(msg) = self.emu.receiver.try_recv() {
            match msg {
                EmuMsgOut::State(state) => {
                    self.debug.emu_state = Some(state);
                }
            }
        }

        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.perf.open, "Performance");

                if ui.checkbox(&mut self.debug.open, "Debug").changed() {
                    if self.debug.open {
                        self.debug.vram = Some(debug::load_vram_texture(ctx, &*self.emu.atoms.vram.lock()));
                    }
                }

                ()
            });
        });
            
        if self.perf.open {
            perf::show(ctx, &mut self.perf);
        }

        if ctx.input_mut(|i| i.consume_shortcut(&DEBUG_SHORTCUT)) {
            self.debug.open = !self.debug.open;

            if self.debug.open {
                self.debug.vram = Some(debug::load_vram_texture(ctx, &*self.emu.atoms.vram.lock()));
            }
        }

        if self.debug.open {
            if let Some(ref sender) = self.emu.sender {
                debug::show(ctx, &mut self.debug, sender);
            }
        }

        let res = emu::show(ctx, self);

        if res.response.rect != self.emu.display_rect {
            self.emu.display_rect = res.response.rect;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&PERF_SHORTCUT)) {
            self.perf.open = !self.perf.open;
        }

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        });
    }
}
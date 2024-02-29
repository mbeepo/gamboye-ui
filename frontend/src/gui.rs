use std::{collections::VecDeque, io::{stdout, Write}, sync::{atomic::AtomicBool, Arc}, time::Instant};

use eframe::App;
use egui::{mutex::Mutex, pos2, vec2, Color32, ColorImage, KeyboardShortcut, Mesh, Modifiers, Pos2, Rect, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, Emu, EmuStatus}};

mod perf;

pub const BASE_DISPLAY_POS: Pos2 = pos2(0.0, 0.0);

const PERF_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::P);

#[derive(Clone, Default)]
pub struct State {
    pub emu_state: Arc<EmuState>,
    pub perf_state: PerfState,
}

#[derive(Default)]
pub struct EmuState {
    /// This should always be emu::WIDTH * emu::HEIGHT elements
    pub fb: Mutex<Vec<Color32>>,
    /// This should always be (emu::WIDTH * emu::HEIGHT * 4) elements
    // pub fb: Mutex<Vec<u8>>,
    pub status: Mutex<EmuStatus>,
    pub fb_pending: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct PerfState {
    pub open: bool,
    pub last_second: Option<Instant>,
    pub fps_history: VecDeque<usize>,
    pub min_fps: usize,
    pub max_fps: usize,
}

impl Default for PerfState {
    fn default() -> Self {
        Self {
            open: false,
            last_second: None,
            fps_history: VecDeque::with_capacity(perf::MAX_FPS_HISTORY),
            min_fps: usize::MAX,
            max_fps: 0,
        }
    }
}

pub struct EmuWindow {
    pub emu_channel: Option<mpsc::UnboundedSender<EmuMsgIn>>,
    pub state: State,
    pub display_mesh: Mesh,
    pub display_rect: Rect,
    pub frames: usize,
    pub display: ColorImage,
    pub texture: TextureHandle,
    pub sleep_until: Option<Instant>,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>, rom: Vec<u8>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let state = State::default();
        let display_mesh = Mesh::default();
        let display_rect = Rect::from_min_size(BASE_DISPLAY_POS, vec2(emu::WIDTH as f32, emu::HEIGHT as f32));
        
        *state.emu_state.fb.lock() = vec![Default::default(); emu::WIDTH * emu::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, state.emu_state.clone());
        emu.init(&rom);
        emu.run().unwrap();

        let display = ColorImage::new([emu::WIDTH, emu::HEIGHT], Color32::YELLOW);
        let texture = cc.egui_ctx.load_texture("emu_display", display.clone(), TextureOptions::NEAREST);
            
        Self {
            emu_channel: Some(emu_send),
            state,
            display_mesh,
            display_rect,
            frames: 0,
            display,
            texture,
            sleep_until: None,
        }
    }
}

impl App for EmuWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.state.perf_state.open, "Performance");
            });
        });

        if self.state.perf_state.open {
            perf::show(ctx, &mut self.state.perf_state);
        }

        let res = emu::show(ctx, self);

        self.display_rect = res.response.rect;

        let mut bep = stdout();
        bep.write_all(b".").unwrap();
        bep.flush().unwrap();

        if ctx.input_mut(|i| i.consume_shortcut(&PERF_SHORTCUT)) {
            self.state.perf_state.open = !self.state.perf_state.open;
        }
    }
}
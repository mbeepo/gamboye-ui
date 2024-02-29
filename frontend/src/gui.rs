use std::{collections::VecDeque, io::{stdout, Write}, sync::{atomic::AtomicBool, Arc}, time::Instant};

use eframe::App;
use egui::{mutex::Mutex, pos2, vec2, Color32, ColorImage, KeyboardShortcut, Mesh, Modifiers, Pos2, Rect, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, Emu, EmuStatus}};

mod perf;

pub const BASE_DISPLAY_POS: Pos2 = pos2(0.0, 0.0);

const PERF_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::P);

#[derive(Default)]
pub struct InnerEmuState {
    /// This should always be emu::WIDTH * emu::HEIGHT elements
    // pub fb: Mutex<Vec<Color32>>,
    /// This should always be (emu::WIDTH * emu::HEIGHT * 4) elements
    pub fb: Mutex<Vec<u8>>,
    pub status: Mutex<EmuStatus>,
    pub fb_pending: AtomicBool,
}

pub struct EmuState {
    pub atoms: Arc<InnerEmuState>,
    pub sleep_until: Option<Instant>,
    pub sender: Option<mpsc::UnboundedSender<EmuMsgIn>>,
    pub rect_changed: bool,
    pub display_mesh: Mesh,
    pub display_rect: Rect,
    pub display: ColorImage,
    pub texture: TextureHandle,
}

impl EmuState {
    fn new(ctx: &egui::Context, sender: mpsc::UnboundedSender<EmuMsgIn>) -> Self {
        let display_rect = Rect::from_min_size(BASE_DISPLAY_POS, vec2(emu::WIDTH as f32, emu::HEIGHT as f32));
        let display = ColorImage::new([emu::WIDTH, emu::HEIGHT], Color32::YELLOW);
        let texture = ctx.load_texture("emu_display", display.clone(), TextureOptions::NEAREST);
        let display_mesh = Mesh::with_texture(texture.id());

        Self {
            atoms: Default::default(),
            sleep_until: None,
            sender: Some(sender),
            rect_changed: false,
            display_mesh,
            display_rect,
            display,
            texture,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PerfState {
    pub open: bool,
    pub last_second: Option<Instant>,
    pub fps_history: VecDeque<usize>,
    pub min_fps: usize,
    pub max_fps: usize,
    pub frames: usize,
}

impl Default for PerfState {
    fn default() -> Self {
        Self {
            open: false,
            last_second: None,
            fps_history: VecDeque::with_capacity(perf::MAX_FPS_HISTORY),
            min_fps: usize::MAX,
            max_fps: 0,
            frames: 0,
        }
    }
}

pub struct EmuWindow {
    pub emu: EmuState,
    pub perf: PerfState,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>, rom: Vec<u8>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let emu_state = EmuState::new(&cc.egui_ctx, emu_send);
        let perf = Default::default();
        
        *emu_state.atoms.fb.lock() = vec![Default::default(); emu::WIDTH * emu::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, emu_state.atoms.clone());
        emu.init(&rom);
        emu.run().unwrap();

        Self {
            emu: emu_state,
            perf,
        }
    }
}

impl App for EmuWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.perf.open, "Performance");
            });
        });

        if self.perf.open {
            perf::show(ctx, &mut self.perf);
        }

        let res = emu::show(ctx, self);

        if res.response.rect != self.emu.display_rect {
            self.emu.rect_changed = true;
            self.emu.display_rect = res.response.rect;
        }

        let mut bep = stdout();
        bep.write_all(b".").unwrap();
        bep.flush().unwrap();

        if ctx.input_mut(|i| i.consume_shortcut(&PERF_SHORTCUT)) {
            self.perf.open = !self.perf.open;
        }
    }
}
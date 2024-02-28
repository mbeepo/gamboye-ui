use std::{collections::VecDeque, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::Instant};

use eframe::App;
use egui::{mutex::Mutex, pos2, vec2, Color32, KeyboardShortcut, Mesh, Modifiers, Pos2, Rect, Shape, ViewportId};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, Emu, EmuStatus}};

mod perf;

pub const BASE_DISPLAY_POS: Pos2 = pos2(0.0, 0.0);
const MAX_FRAMERATE: usize = 60;

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
    emu_channel: Option<mpsc::UnboundedSender<EmuMsgIn>>,
    state: State,
    display_mesh: Mesh,
    display_rect: Rect,
    frames: usize,
    perf_viewport: egui::ViewportId,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>, rom: Vec<u8>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let state = State::default();
        let display_mesh = Mesh::default();
        let display_rect = Rect::from_min_size(BASE_DISPLAY_POS, vec2(emu::WIDTH as f32, emu::HEIGHT as f32));
        let perf_viewport = ViewportId(egui::Id::new("performance"));
        
        *state.emu_state.fb.lock() = vec![Default::default(); emu::WIDTH * emu::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, state.emu_state.clone());
        emu.init(&rom);
        
        tokio::spawn(async move {
            emu.run().unwrap();
        });

        Self {
            emu_channel: Some(emu_send),
            state,
            display_mesh,
            display_rect,
            frames: 0,
            perf_viewport,
        }
    }
}

impl App for EmuWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.state.perf_state.open, "Performance");
            });
        });

        if self.state.perf_state.open {
            perf::show(ctx, &mut self.state.perf_state);
        }

        let res = egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.emu_state.fb_pending.load(Ordering::Relaxed) {
                self.state.emu_state.fb_pending.store(false, Ordering::Relaxed);
                
                let system_fb = self.state.emu_state.fb.lock().clone();
                if system_fb.len() != (emu::WIDTH * emu::HEIGHT) {
                    ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), emu::WIDTH * emu::HEIGHT));
                    return;
                }

                self.display_mesh.clear();

                let size = self.display_rect.size();
                let pos = self.display_rect.min;
                let scale = (size / vec2(emu::WIDTH as f32, emu::HEIGHT as f32)).min_elem();
                
                for y in 0..emu::HEIGHT{
                    for x in 0..emu::WIDTH {
                        self.display_mesh.add_colored_rect(
                            Rect::from_min_max(
                                pos + vec2(x as f32 * scale, y as f32 * scale),
                                pos + vec2(x as f32 * scale + scale, y as f32 * scale + scale)
                            ), system_fb[x + y * emu::WIDTH]);
                    }
                }

                self.frames += 1;
                
                let now = Instant::now();

                let Some(last_second) = self.state.perf_state.last_second else {
                    self.state.perf_state.last_second = Some(now);
                    return;
                };

                if now.duration_since(last_second).as_millis() >= 1000 {
                    self.state.perf_state.last_second = Some(now);
                    self.state.perf_state.fps_history.push_back(self.frames);
                    self.frames = 0;

                    ctx.request_repaint_of(self.perf_viewport);
                }
            }
                
            let display = Shape::Mesh(self.display_mesh.clone());
            ui.painter().add(display);
        });

        self.display_rect = res.response.rect;

        if ctx.input_mut(|i| i.consume_shortcut(&PERF_SHORTCUT)) {
            self.state.perf_state.open = !self.state.perf_state.open;
        }

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        });
    }
}
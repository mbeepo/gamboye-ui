use std::{collections::VecDeque, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::Instant};

use eframe::App;
use egui::{mutex::Mutex, pos2, Color32, Mesh, Pos2, Rect, Shape, Vec2, ViewportId};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, Emu, EmuStatus}};

mod perf;

const SCALE: usize = 4;

#[derive(Clone, Default)]
pub struct State {
    pub emu_state: Arc<EmuState>,
    pub perf_state: Arc<Mutex<PerfState>>,
}

#[derive(Default)]
pub struct EmuState {
    /// This should always be emu::WIDTH * emu::HEIGHT elements
    pub fb: Mutex<Vec<Color32>>,
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
    emu_channel: mpsc::UnboundedSender<EmuMsgIn>,
    state: State,
    display_mesh: Mesh,
    display_pos: Pos2,
    frames: usize,
    perf_viewport: egui::ViewportId,
}

impl EmuWindow {
    pub fn new(cc: &eframe::CreationContext<'_>, rom: Vec<u8>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let (emu_send, emu_recv) = mpsc::unbounded_channel();
        let state = State::default();
        let display_mesh = Mesh::default();
        let display_pos = pos2(0.0, 25.0);
        let perf_viewport = ViewportId(egui::Id::new("performance"));

        *state.emu_state.fb.lock() = vec![Default::default(); emu::WIDTH * emu::HEIGHT];

        let mut emu = Emu::new(ctx, emu_recv, state.emu_state.clone());
        emu.init(&rom);
        
        tokio::spawn(async move {
            emu.run().unwrap();
        });

        Self {
            emu_channel: emu_send,
            state,
            display_mesh,
            display_pos,
            frames: 0,
            perf_viewport,
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
            if self.state.emu_state.fb_pending.load(Ordering::Relaxed) {
                self.state.emu_state.fb_pending.store(false, Ordering::Relaxed);
                
                let system_fb = self.state.emu_state.fb.lock().clone();
                if system_fb.len() != (emu::WIDTH * emu::HEIGHT) {
                    ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), emu::WIDTH * emu::HEIGHT));
                    return;
                }

                self.display_mesh.clear();

                for y in 0..emu::HEIGHT {
                    for x in 0..emu::WIDTH {
                        self.display_mesh.add_colored_rect(
                            Rect::from_min_max(
                                self.display_pos + Vec2::new(x as f32, y as f32),
                                self.display_pos + Vec2::new((x + SCALE) as f32, (y + SCALE) as f32)
                            ), system_fb[x + y * emu::WIDTH]);
                    }
                }
                // ui.ctx().graphics_mut(|layer| {
                //     layer.entry(ui.layer_id()).add(Rect::from_min_max(pos2(125.0, 125.0), pos2(275.0, 275.0)), tringle);
                // });

                self.frames += 1;
                
                let now = Instant::now();
                let mut perf_state = self.state.perf_state.lock();

                let Some(last_second) = perf_state.last_second else {
                    perf_state.last_second = Some(now);
                    return;
                };

                if now.duration_since(last_second).as_millis() >= 1000 {
                    perf_state.last_second = Some(now);
                    perf_state.fps_history.push_back(self.frames);
                    self.frames = 0;

                    ctx.request_repaint_of(self.perf_viewport);
                }
            }
                
            let display = Shape::Mesh(self.display_mesh.clone());

            ui.painter().add(display);
        });

        let perf_state = self.state.perf_state.clone();

        if self.state.perf_state.lock().open {
            perf::update(ctx, self.perf_viewport, perf_state);
        }
    }
}
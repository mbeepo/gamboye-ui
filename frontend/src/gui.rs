use std::{collections::VecDeque, sync::{atomic::{AtomicBool, Ordering}, Arc}, time::Instant};

use eframe::{glow::HasContext, App};
use egui::{mutex::Mutex, panel::PanelState, pos2, vec2, Color32, Mesh, Pos2, Rect, Shape, Vec2, ViewportId};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, Emu, EmuStatus}};

pub const SCALE: usize = 4;
pub const MAX_FPS_HISTORY: usize = 10;
pub const BASE_DISPLAY_POS: Pos2 = pos2(0.0, 25.0);

#[derive(Clone, Default)]
pub struct State {
    pub emu_state: Arc<EmuState>,
    pub perf_state: Mutex<PerfState>,
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
            fps_history: VecDeque::with_capacity(MAX_FPS_HISTORY),
            min_fps: usize::MAX,
            max_fps: 0,
        }
    }
}

pub struct EmuWindow {
    emu_channel: mpsc::UnboundedSender<EmuMsgIn>,
    state: State,
    display_mesh: Mesh,
    // display_texture: TextureHandle,
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
        let display_pos = BASE_DISPLAY_POS;
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
        self.display_pos = BASE_DISPLAY_POS;

        egui::TopBottomPanel::top("main_menubar").show(ctx, |ui| {
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.state.perf_state.lock().open, "Performance");
            });
        });

        let perf_state = self.state.perf_state.clone();

        if self.state.perf_state.lock().open {
            egui::SidePanel::left("perf").show(ctx, |ui| {
                let (current, average, min, max) = {
                    let mut perf_state = perf_state.lock();
                    
                    if perf_state.fps_history.len() > 0 {
                        if perf_state.fps_history.len() > MAX_FPS_HISTORY {
                            perf_state.fps_history.pop_front();
                        }
        
                        let newest = *perf_state.fps_history.back().unwrap();
                        perf_state.max_fps = perf_state.max_fps.max(newest);
                        perf_state.min_fps = perf_state.min_fps.min(newest);
                        
                        let average: usize = perf_state.fps_history.iter().sum::<usize>() / perf_state.fps_history.len();
                        
                        (
                            format!("{newest}"),
                            format!("{average}"),
                            format!("{}", perf_state.min_fps),
                            format!("{}", perf_state.max_fps)
                        )
                    } else {
                        (
                            "N/A".to_owned(),
                            "N/A".to_owned(),
                            "N/A".to_owned(),
                            "N/A".to_owned()
                        )
                    }
                };
        
                ui.label(format!("FPS: {current}"));
                ui.label(format!("Avg. FPS: {average}"));
                ui.label(format!("Min: {min}"));
                ui.label(format!("Max: {max}"));

                let panel_size = vec2(115.0, 0.0);
                self.display_pos = BASE_DISPLAY_POS + panel_size;
                
                dbg!(crate::WINDOW_SIZE, panel_size);

                if ctx.screen_rect().size().x < crate::WINDOW_SIZE.x + panel_size.x {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(crate::WINDOW_SIZE + panel_size));
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.emu_state.fb_pending.load(Ordering::Relaxed) {
                self.state.emu_state.fb_pending.store(false, Ordering::Relaxed);
                
                let system_fb = self.state.emu_state.fb.lock().clone();
                if system_fb.len() != (emu::WIDTH * emu::HEIGHT) {
                    ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), emu::WIDTH * emu::HEIGHT));
                    return;
                }

                self.display_mesh.clear();

                let size = ctx.screen_rect().size();
                let scale = ((size - self.display_pos.to_vec2()) / (vec2(emu::WIDTH as f32, emu::HEIGHT as f32))).min_elem();

                dbg!(scale, self.display_pos);
                
                for y in 0..emu::HEIGHT{
                    for x in 0..emu::WIDTH {
                        self.display_mesh.add_colored_rect(
                            Rect::from_min_max(
                                self.display_pos + Vec2::new(x as f32 * scale, y as f32 * scale),
                                self.display_pos + Vec2::new(x as f32 * scale + scale, y as f32 * scale + scale)
                            ), system_fb[x + y * emu::WIDTH]);
                    }
                }

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

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                ctx.send_viewport_cmd_to(ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        });
    }
}
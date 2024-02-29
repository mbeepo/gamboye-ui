use std::{collections::VecDeque, sync::{atomic::AtomicBool, Arc}, time::Instant};

use egui::{mutex::Mutex, vec2, Color32, ColorImage, Mesh, Rect, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, emu::{self, EmuStatus}, gui::BASE_DISPLAY_POS};

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
    pub wait_until: Option<Instant>,
    pub sender: Option<mpsc::UnboundedSender<EmuMsgIn>>,
    pub display_mesh: Mesh,
    pub display_rect: Rect,
    pub display: ColorImage,
    pub texture: TextureHandle,
}

impl EmuState {
    pub fn new(ctx: &egui::Context, sender: mpsc::UnboundedSender<EmuMsgIn>) -> Self {
        let display_rect = Rect::from_min_size(BASE_DISPLAY_POS, vec2(emu::WIDTH as f32, emu::HEIGHT as f32));
        let display = ColorImage::new([emu::WIDTH, emu::HEIGHT], Color32::YELLOW);
        let texture = ctx.load_texture("emu_display", display.clone(), TextureOptions::NEAREST);
        let display_mesh = Mesh::with_texture(texture.id());

        Self {
            atoms: Default::default(),
            wait_until: None,
            sender: Some(sender),
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
            fps_history: VecDeque::with_capacity(crate::gui::perf::MAX_FPS_HISTORY),
            min_fps: usize::MAX,
            max_fps: 0,
            frames: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DebugState {
    pub open: bool,
    pub emu_status: EmuStatus,
}
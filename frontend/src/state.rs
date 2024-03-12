use std::{collections::VecDeque, sync::{atomic::AtomicBool, Arc}, time::Instant};

use egui::{mutex::Mutex, vec2, Color32, ColorImage, Mesh, Rect, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::{EmuMsgIn, EmuMsgOut}, gui::BASE_DISPLAY_POS, runner::{self, Breakpoints, EmuStatus}};

pub struct InnerEmuState {
    /// This should always be emu::WIDTH * emu::HEIGHT elements
    // pub fb: Mutex<Vec<Color32>>,
    /// This should always be (emu::WIDTH * emu::HEIGHT * 4) elements
    pub fb: Mutex<Vec<u8>>,
    pub vram: Mutex<Vec<u8>>,
    pub status: Mutex<EmuStatus>,
    pub fb_pending: AtomicBool,
}

impl Default for InnerEmuState {
    fn default() -> Self {
        Self {
            fb: Default::default(),
            vram: Mutex::new(vec![0; 128 * 192 * 3]),
            status: Default::default(),
            fb_pending: Default::default(),
        }
    }
}

pub struct EmuState {
    pub atoms: Arc<InnerEmuState>,
    pub wait_until: Option<Instant>,
    pub sender: Option<mpsc::UnboundedSender<EmuMsgIn>>,
    pub receiver: mpsc::UnboundedReceiver<EmuMsgOut>,
    pub display_mesh: Mesh,
    pub display_rect: Rect,
    pub display: ColorImage,
    pub texture: TextureHandle,
}

impl EmuState {
    pub fn new(ctx: &egui::Context, sender: mpsc::UnboundedSender<EmuMsgIn>, receiver: mpsc::UnboundedReceiver<EmuMsgOut>) -> Self {
        let display_rect = Rect::from_min_size(BASE_DISPLAY_POS, vec2(runner::WIDTH as f32, runner::HEIGHT as f32));
        let display = ColorImage::new([runner::WIDTH, runner::HEIGHT], Color32::YELLOW);
        let texture = ctx.load_texture("emu_display", display.clone(), TextureOptions::NEAREST);
        let display_mesh = Mesh::with_texture(texture.id());

        Self {
            atoms: Default::default(),
            wait_until: None,
            sender: Some(sender),
            receiver,
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

#[derive(Clone, Default)]
pub struct DebugState {
    pub open: bool,
    pub emu_status: EmuStatus,
    pub vram: Option<TextureHandle>,
    pub emu_state: Option<StateDump>,
    pub stopped: bool,
    pub breakpoints: Breakpoints,
}

#[derive(Clone, Debug)]
pub struct StateDump {
    pub instruction: gbc::Instruction,
    pub regs: gbc::Registers,
    pub io_regs: gbc::IoRegs,
    pub memory: Vec<u8>,
}
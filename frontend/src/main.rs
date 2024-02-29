#![allow(dead_code)]

use std::process::exit;

use eframe::egui;
use egui::{vec2, Vec2};
use gui::TopState;

mod comms;
mod runner;
mod gui;
mod state;

const WIDTH: f32 = runner::WIDTH as f32;
const HEIGHT: f32 = runner::HEIGHT as f32 + 25.0;
pub const WINDOW_SIZE: Vec2 = vec2(WIDTH, HEIGHT);

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Beef Wellington").with_inner_size(WINDOW_SIZE).with_min_inner_size(WINDOW_SIZE),
        vsync: false,
        ..Default::default()
    };

    let Some(filename) = std::env::args().nth(1) else {
        eprintln!("Usage: {} <rom path>", std::env::current_exe().unwrap().file_name().unwrap().to_str().unwrap());
        exit(1);
    };
    let Ok(rom) = std::fs::read(&filename) else {
        eprintln!("File not found: {filename}");
        exit(1);
    };

    eframe::run_native("gamboye", options, Box::new(|cc| Box::new(TopState::new(cc, rom))))
}
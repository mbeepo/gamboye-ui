#![allow(dead_code)]

use eframe::egui;
use gui::EmuWindow;

mod comms;
mod emu;
mod gui;

// egui wants these to be floats ¯\_(ツ)_/¯
const WIDTH: f32 = emu::WIDTH as f32;
const HEIGHT: f32 = emu::HEIGHT as f32 + 25.0;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Beef Wellington").with_inner_size([WIDTH, HEIGHT]).with_min_inner_size([WIDTH, HEIGHT]),
        vsync: false,
        ..Default::default()
    };

    let filename = std::env::args().nth(1).unwrap();
    let rom = std::fs::read(filename).unwrap();

    eframe::run_native("gamboye", options, Box::new(|cc| Box::new(EmuWindow::new(cc, rom))))
}
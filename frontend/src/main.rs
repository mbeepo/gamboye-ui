use eframe::egui;
use gui::EmuWindow;

mod comms;
mod emu;
mod gui;

// egui wants these to be floats ¯\_(ツ)_/¯
const WIDTH: f32 = 320.0;
const HEIGHT: f32 = 288.0;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIDTH, HEIGHT]),
        ..Default::default()
    };

    eframe::run_native("gamboye", options, Box::new(|cc| Box::new(EmuWindow::new(cc))))
}
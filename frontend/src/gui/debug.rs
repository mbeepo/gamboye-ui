use egui::{load::SizedTexture, Context};

use crate::state::DebugState;

pub fn show(ctx: &Context, state: &mut DebugState) {
    egui::SidePanel::left("debug").show(ctx, |ui| {
        ui.strong("Emu Status");
        ui.label(format!("{}", state.emu_status));
        
        if let Some(ref vram) = state.vram {
            ui.heading("VRAM");
            ui.image(SizedTexture::from_handle(vram));
        }
    });
}
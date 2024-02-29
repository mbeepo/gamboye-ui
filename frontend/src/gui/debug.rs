use egui::Context;

use crate::state::DebugState;

pub fn show(ctx: &Context, state: &mut DebugState) {
    egui::SidePanel::left("debug").show(ctx, |ui| {
        ui.strong("Emu Status");
        ui.label(format!("{}", state.emu_status));
    });
}
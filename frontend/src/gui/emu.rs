use std::sync::atomic::Ordering;

use egui::{vec2, ColorImage, Context, InnerResponse, TextureOptions};

use crate::runner::{HEIGHT, WIDTH};

use super::TopState;

pub fn show(ctx: &Context, state: &mut TopState) -> InnerResponse<()> {
    egui::CentralPanel::default().show(ctx, |ui| {
        if state.emu.atoms.fb_pending.load(Ordering::Relaxed) {
            state.emu.atoms.fb_pending.store(false, Ordering::Relaxed);
            
            let system_fb = state.emu.atoms.fb.lock().clone();
            if system_fb.len() != (WIDTH * HEIGHT * 3) {
                ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), WIDTH * HEIGHT * 3));
            }

            let new_display = ColorImage::from_rgb([WIDTH, HEIGHT], &system_fb);

            println!("Drawing fb");

            if new_display != state.emu.display {
                println!("Updating texture");
                state.emu.texture = ctx.load_texture("emu_display", new_display, TextureOptions::NEAREST);
            }

            crate::gui::perf::record_frame(state);
        }

        state.debug.emu_status = *state.emu.atoms.status.lock();

        ui.vertical_centered(|ui| {
            ui.add(egui::Image::from_texture(egui::load::SizedTexture::from_handle(&state.emu.texture)).maintain_aspect_ratio(true).fit_to_fraction(vec2(1.0, 1.0)));
        });
    })
}
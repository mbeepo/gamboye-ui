use std::sync::atomic::Ordering;

use egui::{vec2, ColorImage, Context, InnerResponse, TextureOptions};

use crate::{comms, runner::{HEIGHT, WIDTH}};

use super::TopState;

pub fn show(ctx: &Context, state: &mut TopState) -> InnerResponse<()> {
    const BUTTONS: [Keybind; 8] = [
        Keybind { key: egui::Key::W, button: gbc::Button::Up },
        Keybind { key: egui::Key::A, button: gbc::Button::Left },
        Keybind { key: egui::Key::S, button: gbc::Button::Down },
        Keybind { key: egui::Key::D, button: gbc::Button::Right },
        Keybind { key: egui::Key::J, button: gbc::Button::A },
        Keybind { key: egui::Key::K, button: gbc::Button::B },
        Keybind { key: egui::Key::Escape, button: gbc::Button::Start },
        Keybind { key: egui::Key::Enter, button: gbc::Button::Select },
    ];

    for button in BUTTONS {
        if ctx.input(|i| i.key_pressed(button.key)) {
            if let Some(ref sender) = state.emu.sender {
                sender.send(comms::EmuMsgIn::ButtonPressed(button.button)).unwrap();
            }
        } else if ctx.input(|i| i.key_released(button.key)) {
            if let Some(ref sender) = state.emu.sender {
                sender.send(comms::EmuMsgIn::ButtonReleased(button.button)).unwrap();
            }
        }
    };

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.emu.atoms.fb_pending.load(Ordering::Relaxed) {
            state.emu.atoms.fb_pending.store(false, Ordering::Relaxed);
            
            let system_fb = state.emu.atoms.fb.lock().clone();
            if system_fb.len() != (WIDTH * HEIGHT * 3) {
                ui.heading(format!("Emulator framebuffer is {} elements, not {}!", system_fb.len(), WIDTH * HEIGHT * 3));
            }

            let new_display = ColorImage::from_rgb([WIDTH, HEIGHT], &system_fb);
            state.emu.texture = ctx.load_texture("emu_display", new_display, TextureOptions::NEAREST);

            crate::gui::perf::record_frame(state);
            crate::gui::perf::ratelimit(state);
        }

        state.debug.emu_status = *state.emu.atoms.status.lock();

        ui.vertical_centered(|ui| {
            ui.add(egui::Image::from_texture(egui::load::SizedTexture::from_handle(&state.emu.texture)).maintain_aspect_ratio(true).fit_to_fraction(vec2(1.0, 1.0)));
        });
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Keybind {
    key: egui::Key,
    button: gbc::Button,
}
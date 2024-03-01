use egui::{load::SizedTexture, ColorImage, Context, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, runner::Breakpoint, state::DebugState};

pub fn show(ctx: &Context, state: &mut DebugState, sender: &mpsc::UnboundedSender<EmuMsgIn>) {
    egui::SidePanel::left("debug").show(ctx, |ui| {
        let text = if state.stopped {
            "Resume"
        } else {
            "Stop"
        };

        ui.horizontal(|ui| {
            if ui.button(text).clicked() {
                state.stopped = !state.stopped;

                if state.stopped {
                    sender.send(EmuMsgIn::Pause).unwrap();
                } else {
                    sender.send(EmuMsgIn::Resume).unwrap();
                }
            }

            if ui.button("Step").clicked() {
                sender.send(EmuMsgIn::Step(1)).unwrap();
            }
        });

        ui.menu_button("Breakpoints", |ui| {
            if ui.checkbox(&mut state.breakpoints.carry_flag, "Zero Flag").changed() {
                if state.breakpoints.carry_flag {
                    sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::Zero)).unwrap();
                } else {
                    sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::Zero)).unwrap();
                }
            }

            if ui.checkbox(&mut state.breakpoints.carry_flag, "Subtract Flag").changed() {
                if state.breakpoints.carry_flag {
                    sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::Subtract)).unwrap();
                } else {
                    sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::Subtract)).unwrap();
                }
            }

            if ui.checkbox(&mut state.breakpoints.carry_flag, "Half Carry Flag").changed() {
                if state.breakpoints.carry_flag {
                    sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::HalfCarry)).unwrap();
                } else {
                    sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::HalfCarry)).unwrap();
                }
            }

            if ui.checkbox(&mut state.breakpoints.carry_flag, "Carry Flag").changed() {
                if state.breakpoints.carry_flag {
                    sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::Carry)).unwrap();
                } else {
                    sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::Carry)).unwrap();
                }
            }
        });

        ui.strong("Last Instruction");
        ui.label(format!("{:?}", state.last_instruction.unwrap_or(gbc::Instruction::NOP)));

        ui.strong("Emu Status");
        ui.label(format!("{}", state.emu_status));
        
        if let Some(ref vram) = state.vram {
            ui.strong("VRAM");
            ui.image(SizedTexture::from_handle(vram));
        }

        ui.vertical_centered(|ui| {
            ui.strong("Registers");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("A");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.a).unwrap_or(0)));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("B");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.b).unwrap_or(0)));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("C");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.c).unwrap_or(0)));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("D");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.d).unwrap_or(0)));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("H");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.h).unwrap_or(0)));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("L");
                ui.separator();
                ui.label(format!("${:02X}", state.regs.map(|r| r.l).unwrap_or(0)));
            });
        });
    });
}

pub fn load_vram_texture(ctx: &Context, vram: &[u8]) -> TextureHandle {
    ctx.load_texture(
        "debug_vram",
        ColorImage::from_rgb([128, 64], vram),
        TextureOptions::NEAREST,
    )
}
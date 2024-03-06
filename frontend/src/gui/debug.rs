use egui::{load::SizedTexture, ColorImage, Context, TextureHandle, TextureOptions};
use tokio::sync::mpsc;

use crate::{comms::EmuMsgIn, runner::Breakpoint, state::DebugState};

pub fn show(ctx: &Context, state: &mut DebugState, sender: &mpsc::UnboundedSender<EmuMsgIn>) {
    egui::SidePanel::left("debug").show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
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
                if ui.checkbox(&mut state.breakpoints.zero_flag, "Zero Flag").changed() {
                    if state.breakpoints.zero_flag {
                        sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::Zero)).unwrap();
                    } else {
                        sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::Zero)).unwrap();
                    }
                }

                if ui.checkbox(&mut state.breakpoints.subtract_flag, "Subtract Flag").changed() {
                    if state.breakpoints.subtract_flag {
                        sender.send(EmuMsgIn::SetBreakpoint(Breakpoint::Subtract)).unwrap();
                    } else {
                        sender.send(EmuMsgIn::UnsetBreakpoint(Breakpoint::Subtract)).unwrap();
                    }
                }

                if ui.checkbox(&mut state.breakpoints.half_carry_flag, "Half Carry Flag").changed() {
                    if state.breakpoints.half_carry_flag {
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
            ui.label(format!("{:?}", state.emu_state.map(|s| s.instruction).unwrap_or(gbc::Instruction::NOP)));

            ui.strong("Emu Status");
            ui.label(format!("{}", state.emu_status));
            
            if let Some(ref vram) = state.vram {
                ui.strong("VRAM");
                ui.image(SizedTexture::from_handle(vram));
            }

            ui.vertical(|ui| {
                ui.strong("Registers");
                show_reg_hex(ui, "A", state.emu_state.map(|s| s.regs.a).unwrap_or(0));
                show_reg_hex(ui, "B", state.emu_state.map(|s| s.regs.b).unwrap_or(0));
                show_reg_hex(ui, "C", state.emu_state.map(|s| s.regs.c).unwrap_or(0));
                show_reg_hex(ui, "D", state.emu_state.map(|s| s.regs.d).unwrap_or(0));
                show_reg_hex(ui, "H", state.emu_state.map(|s| s.regs.h).unwrap_or(0));
                show_reg_hex(ui, "L", state.emu_state.map(|s| s.regs.l).unwrap_or(0));
                ui.add_space(5.0);
            });

            ui.vertical(|ui| {
                ui.strong("IO Registers");
                show_reg_bin(ui, "LCDC", state.emu_state.map(|s| s.io_regs.lcdc).unwrap_or(0));
            });
        });
    });
}

fn show_reg_hex(ui: &mut egui::Ui, name: &str, value: u8) {
    show_reg(ui, name, &format!("{value:#04X}"));
}

fn show_reg_bin(ui: &mut egui::Ui, name: &str, value: u8) {
    show_reg(ui, name, &format!("{value:#010b}"));
}

fn show_reg(ui: &mut egui::Ui, name: &str, text: &str) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(name);
        ui.separator();
        ui.label(text);
    });
}

pub fn load_vram_texture(ctx: &Context, vram: &[u8]) -> TextureHandle {
    ctx.load_texture(
        "debug_vram",
        ColorImage::from_rgb([128, 192], vram),
        TextureOptions::NEAREST,
    )
}
use egui::{epaint::text::Row, load::SizedTexture, ColorImage, Context, RichText, TextureHandle, TextureOptions};
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
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        breakpoint_toggle(ui, &mut state.breakpoints.a_reg, "A", Breakpoint::A, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.b_reg, "B", Breakpoint::B, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.c_reg, "C", Breakpoint::C, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.d_reg, "D", Breakpoint::D, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.h_reg, "H", Breakpoint::H, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.l_reg, "L", Breakpoint::L, sender);
                    });
                    ui.vertical(|ui| {
                        breakpoint_toggle(ui, &mut state.breakpoints.zero_flag, "Z", Breakpoint::Zero, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.subtract_flag, "N", Breakpoint::Subtract, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.half_carry_flag, "H", Breakpoint::HalfCarry, sender);
                        breakpoint_toggle(ui, &mut state.breakpoints.carry_flag, "C", Breakpoint::Carry, sender);
                    });
                });
                
                ui.separator();
                ui.strong("Memory Set");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut state.breakpoints.mem_write);
                    
                    let mut checked = false;
                    if ui.checkbox(&mut checked, "Set").changed() {
                        if let Ok(addr) = text.parse() {
                            let breakpoint = Breakpoint::MemoryWrite(addr);
                            if checked {
                                sender.send(EmuMsgIn::SetBreakpoint(breakpoint)).unwrap();
                            } else {
                                sender.send(EmuMsgIn::UnsetBreakpoint(breakpoint)).unwrap();
                            }
                        }
                    }
                });
            });

            ui.strong("Last Instruction");
            ui.label(format!("{:?}", state.emu_state.as_ref().map(|s| s.instruction).unwrap_or(gbc::Instruction::NOP)));

            ui.strong("Emu Status");
            ui.label(format!("{}", state.emu_status));
            
            if let Some(ref vram) = state.vram {
                ui.strong("VRAM");
                ui.image(SizedTexture::from_handle(vram));
            }

            ui.vertical(|ui| {
                ui.strong("Registers");
                show_reg_hex_word(ui, "PC", state.emu_state.as_ref().map(|s| s.regs.pc).unwrap_or(0));
                show_reg_hex(ui, "A", state.emu_state.as_ref().map(|s| s.regs.a).unwrap_or(0));
                show_reg_hex(ui, "B", state.emu_state.as_ref().map(|s| s.regs.b).unwrap_or(0));
                show_reg_hex(ui, "C", state.emu_state.as_ref().map(|s| s.regs.c).unwrap_or(0));
                show_reg_hex(ui, "D", state.emu_state.as_ref().map(|s| s.regs.d).unwrap_or(0));
                show_reg_hex(ui, "H", state.emu_state.as_ref().map(|s| s.regs.h).unwrap_or(0));
                show_reg_hex(ui, "L", state.emu_state.as_ref().map(|s| s.regs.l).unwrap_or(0));
            });

            ui.vertical(|ui| {
                ui.strong("IO Registers");
                show_reg_bin(ui, "LCDC", state.emu_state.as_ref().map(|s| s.io_regs.lcdc).unwrap_or(0));
                show_reg_bin(ui, "JOYP", state.emu_state.as_ref().map(|s| s.io_regs.joyp).unwrap_or(0));
                show_reg_dec(ui, "SCY", state.emu_state.as_ref().map(|s| s.io_regs.scx).unwrap_or(0));
                show_reg_dec(ui, "SCX", state.emu_state.as_ref().map(|s| s.io_regs.scy).unwrap_or(0));
                show_reg_bin(ui, "STAT", state.emu_state.as_ref().map(|s| s.io_regs.stat).unwrap_or(0));
                show_reg_dec(ui, "LYC", state.emu_state.as_ref().map(|s| s.io_regs.lyc).unwrap_or(0));
                show_reg_dec(ui, "LY", state.emu_state.as_ref().map(|s| s.io_regs.ly).unwrap_or(0));
            });
        });
    });

    egui::SidePanel::right("debug-memory").show(ctx, |ui| {
        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);

        egui::ScrollArea::vertical().show_rows(ui, row_height, (u16::MAX / 16).into(), |ui, row_range| {
            if let Some(memory) = state.emu_state.as_ref().map(|s| &s.memory) {
                for row in row_range {
                    let y = row * 16;

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("${y:04X}")).strong().monospace());

                        for x in 0..16 {
                            let current = memory[y as usize + x as usize];
                            ui.monospace(format!("{current:02X}"));
                        }
                    });
                }
            }
        });
    });
}

fn show_reg_dec(ui: &mut egui::Ui, name: &str, value: u8) {
    show_reg(ui, name, &value.to_string())
}

fn show_reg_hex(ui: &mut egui::Ui, name: &str, value: u8) {
    show_reg(ui, name, &format!("{value:#04X}"));
}

fn show_reg_hex_word(ui: &mut egui::Ui, name: &str, value: u16) {
    show_reg(ui, name, &format!("{value:#06X}"));
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

fn breakpoint_toggle(ui: &mut egui::Ui, value: &mut bool, text: &str, breakpoint: Breakpoint, sender: &mpsc::UnboundedSender<EmuMsgIn>) {
    if ui.checkbox(value, text).changed() {
        if *value {
            sender.send(EmuMsgIn::SetBreakpoint(breakpoint)).unwrap();
        } else {
            sender.send(EmuMsgIn::UnsetBreakpoint(breakpoint)).unwrap();
        }
    }
}

pub fn load_vram_texture(ctx: &Context, vram: &[u8]) -> TextureHandle {
    ctx.load_texture(
        "debug_vram",
        ColorImage::from_rgb([128, 192], vram),
        TextureOptions::NEAREST,
    )
}
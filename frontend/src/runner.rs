use std::{fmt::Display, sync::{atomic::Ordering, Arc}};

use egui::Context;
use gbc::{CpuEvent, CpuReg, CpuStatus, Gbc, PpuStatus};
use tokio::sync::mpsc;

use crate::{comms::{EmuMsgIn, EmuMsgOut}, state::{InnerEmuState, StateDump}};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmuStatus {
    Fresh,
    Running,
    Stopped,
    Break,
    LoadingRom,
    Stepping,
    FrameLimited,
}

impl Display for EmuStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for EmuStatus {
    fn default() -> Self {
        // Self::Fresh
        Self::Stopped
    }
}

#[derive(Clone, Copy, Debug, )]
pub enum EmuError {
    Uninitialized,
    What,
}

#[derive(Clone, Copy, Debug)]
pub enum Breakpoint {
    A, B,
    C, D,
    H, L,
    Zero,
    Subtract,
    HalfCarry,
    Carry,
    MemoryWrite(u16),
}

impl From<Breakpoint> for gbc::CpuEvent {
    fn from(value: Breakpoint) -> Self {
        match value {
            Breakpoint::A => Self::Reg(CpuReg::A),
            Breakpoint::B => Self::Reg(CpuReg::B),
            Breakpoint::C => Self::Reg(CpuReg::C),
            Breakpoint::D => Self::Reg(CpuReg::D),
            Breakpoint::H => Self::Reg(CpuReg::H),
            Breakpoint::L => Self::Reg(CpuReg::L),
            Breakpoint::Zero => Self::Flag(gbc::CpuFlag::Zero),
            Breakpoint::Subtract => Self::Flag(gbc::CpuFlag::Subtract),
            Breakpoint::HalfCarry => Self::Flag(gbc::CpuFlag::HalfCarry),
            Breakpoint::Carry => Self::Flag(gbc::CpuFlag::Carry),
            Breakpoint::MemoryWrite(addr) => Self::MemoryWrite(addr)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Breakpoints {
    pub zero_flag: bool,
    pub subtract_flag: bool,
    pub half_carry_flag: bool,
    pub carry_flag: bool,
    pub a_reg: bool,
    pub b_reg: bool,
    pub c_reg: bool,
    pub d_reg: bool,
    pub h_reg: bool,
    pub l_reg: bool,
    pub mem_write: String,
}

// impl Breakpoints {
//     pub fn set(&mut self, breakpoint: Breakpoint) {
//         match breakpoint {
//             Breakpoint::A => self.a_reg = true,
//             Breakpoint::B => self.b_reg = true,
//             Breakpoint::C => self.c_reg = true,
//             Breakpoint::D => self.d_reg = true,
//             Breakpoint::H => self.h_reg = true,
//             Breakpoint::L => self.l_reg = true,
//             Breakpoint::Zero => self.zero_flag = true,
//             Breakpoint::Subtract => self.subtract_flag = true,
//             Breakpoint::HalfCarry => self.half_carry_flag = true,
//             Breakpoint::Carry => self.carry_flag = true,
//         }
//     }

//     pub fn unset(&mut self, breakpoint: Breakpoint) {
//         match breakpoint {
//             Breakpoint::A => self.a_reg = false,
//             Breakpoint::B => self.b_reg = true,
//             Breakpoint::C => self.c_reg = true,
//             Breakpoint::D => self.d_reg = true,
//             Breakpoint::H => self.h_reg = true,
//             Breakpoint::L => self.l_reg = true,
//             Breakpoint::Zero => self.zero_flag = false,
//             Breakpoint::Subtract => self.subtract_flag = false,
//             Breakpoint::HalfCarry => self.half_carry_flag = false,
//             Breakpoint::Carry => self.carry_flag = false,
//             Breakpoint::MemoryWrite()
//         }
//     }
// }

pub struct Emu {
    inner: Option<Gbc>,
    receiver: mpsc::UnboundedReceiver<EmuMsgIn>,
    sender: mpsc::UnboundedSender<EmuMsgOut>,
    egui_ctx: Context,
    state: Arc<InnerEmuState>,
    steps_remaining: usize,
    breakpoints: Breakpoints,
}

impl Emu {
    pub fn new(egui_ctx: Context, receiver: mpsc::UnboundedReceiver<EmuMsgIn>, sender: mpsc::UnboundedSender<EmuMsgOut>, state: Arc<InnerEmuState>) -> Self {
        let inner = None;

        Self {
            inner,
            receiver,
            sender,
            egui_ctx,
            state,
            steps_remaining: 0,
            breakpoints: Default::default(),
        }
    }

    pub fn init(&mut self, rom: &[u8]) {
        let mbc = gbc::get_mbc(rom);
        let mut emu = Gbc::new(mbc, false, true);
        emu.load_rom(rom);
        self.inner = Some(emu);
    }

    pub fn run(mut self) -> Result<(), EmuError> {
        if let Some(mut emu) = self.inner {
            self.inner = None;

            tokio::spawn(async move {
                *self.state.status.lock() = EmuStatus::Running;

                loop {
                    match self.receiver.try_recv() {
                        Ok(msg) => {
                            use EmuMsgIn::*;

                            match msg {
                                Exit => return,
                                Pause => {
                                    *self.state.status.lock() = EmuStatus::Stopped
                                },
                                Resume => {
                                    *self.state.status.lock() = EmuStatus::Running
                                },
                                LoadRom => return, // this instance should be dropped and a new instance should replace it
                                Step(steps) => {
                                    self.steps_remaining = steps;
                                    *self.state.status.lock() = EmuStatus::Stepping;
                                },
                                SetBreakpoint(breakpoint) => {
                                    // self.breakpoints.set(breakpoint);
                                    emu.cpu.breakpoint_controls.set(breakpoint.into());
                                },
                                UnsetBreakpoint(breakpoint) => {
                                    // self.breakpoints.unset(breakpoint);
                                    emu.cpu.breakpoint_controls.unset(breakpoint.into());
                                },
                                FrameLimit => {
                                    if self.state.status.lock().clone() == EmuStatus::Running {
                                        *self.state.status.lock() = EmuStatus::FrameLimited;
                                    }
                                },
                                FrameUnlimit => {
                                    if self.state.status.lock().clone() == EmuStatus::FrameLimited {
                                        *self.state.status.lock() = EmuStatus::Running;
                                    }
                                },
                                ButtonPressed(button) => {
                                    *emu.cpu.host_input.get_mut(button) = true;
                                },
                                ButtonReleased(button) => {
                                    *emu.cpu.host_input.get_mut(button) = false;
                                }
                                _ => {}
                            }
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                        Err(mpsc::error::TryRecvError::Disconnected) => return,
                    }

                    let status = self.state.status.lock().clone();

                    match status {
                        EmuStatus::Running => {
                            let cpu_status = self.step(&mut emu);

                            match cpu_status {
                                Ok(CpuStatus::Break(instruction, _)) => {
                                    *self.state.status.lock() = EmuStatus::Break;
                                    self.dump_state(&emu, instruction).unwrap();

                                    println!("Breakpoint reached");
                                },
                                _ => {}
                            }
                        },
                        EmuStatus::Stepping => {
                            let cpu_status = self.step(&mut emu);
                            match cpu_status {
                                Ok(CpuStatus::Run(instruction)) => self.dump_state(&emu, instruction).unwrap(),
                                _ => {}
                            }

                            self.steps_remaining -= 1;
                            if self.steps_remaining == 0 {
                                *self.state.status.lock() = EmuStatus::Stopped;
                            }
                        }
                        _ => {}
                    }
                }
            });

            return Ok(())
        }

        Err(EmuError::Uninitialized)
    }

    fn step(&mut self, emu: &mut Gbc) -> Result<CpuStatus, gbc::CpuError> {
        let (cpu_status, ppu_status) = emu.step();

        match ppu_status {
            PpuStatus::EnterVBlank => {
                *self.state.fb.lock() = emu.cpu.ppu.fb.clone();
                emu.cpu.ppu.debug_show(&emu.cpu.memory, [16, 24], &mut *self.state.vram.lock());
                self.state.fb_pending.store(true, Ordering::Relaxed);
                self.egui_ctx.request_repaint();

                match cpu_status {
                    Ok(CpuStatus::Run(instruction)) => self.dump_state(emu, instruction).unwrap(),
                    _ => {}
                }
            },
            _ => {}
        }

        cpu_status
    }

    fn dump_state(&self, emu: &Gbc, instruction: gbc::Instruction) -> Result<(), mpsc::error::SendError<EmuMsgOut>> {
        let regs = emu.cpu.regs;
        let io_regs = emu.cpu.dump_io_regs();
        let memory = emu.cpu.memory.load_block(0, u16::MAX);

        let state = StateDump {
            instruction,
            regs,
            io_regs,
            memory,
        };
        
        self.sender.send(EmuMsgOut::State(state)) 
    }
}
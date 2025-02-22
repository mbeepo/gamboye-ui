use std::{fmt::Display, sync::{atomic::Ordering, Arc}};

use egui::Context;
use gbc::{memory::Memory, CpuEvent, CpuReg, CpuStatus, Gbc, Mmu, PpuStatus};
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
    Pc(u16),
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
            Breakpoint::MemoryWrite(addr) => Self::MemoryWrite(addr),
            Breakpoint::Pc(addr) => Self::Pc(addr),
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
    // String because straight from input
    pub mem_write: (String, bool),
    pub pc: (String, bool),
    
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
    inner: Option<Gbc<Mmu>>,
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
                // *self.state.status.lock() = EmuStatus::Break;
                // emu.cpu.breakpoint_controls.set(CpuEvent::LdBb);
                let mut buf: Option<EmuMsgIn> = None;
                let mut status = EmuStatus::Running;
                let mut old_status;
                *self.state.status.lock() = status;

                loop {
                    let msg = if let Some(msg) = buf {
                        buf = None;
                        Ok(msg)
                    } else {
                        self.receiver.try_recv()
                    };
                    
                    old_status = status;

                    match msg {
                        Ok(msg) => {
                            use EmuMsgIn::*;
                            
                            match msg {
                                Exit => return,
                                Pause => {
                                    status = EmuStatus::Stopped
                                },
                                Resume => {
                                    status = EmuStatus::Running
                                },
                                LoadRom => return, // this instance should be dropped and a new instance should replace it
                                Step(steps) => {
                                    self.steps_remaining = steps;
                                    status = EmuStatus::Stepping;
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
                                    status = EmuStatus::FrameLimited;
                                },
                                FrameUnlimit => {
                                    if status == EmuStatus::FrameLimited {
                                        status = EmuStatus::Running;
                                    }
                                },
                                ButtonPressed(button) => {
                                    emu.press_button(button);
                                },
                                ButtonReleased(button) => {
                                    emu.release_button(button);
                                }
                            }
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                        Err(mpsc::error::TryRecvError::Disconnected) => return,
                    }

                    match status {
                        EmuStatus::Running => {
                            let cpu_status = self.step(&mut emu);

                            match cpu_status {
                                Ok(CpuStatus::Break(_, _)) => {
                                    status = EmuStatus::Break;
                                    self.dump_state(&emu).unwrap();

                                    println!("Breakpoint reached");
                                },
                                _ => {}
                            }
                        },
                        EmuStatus::Stepping => {
                            let cpu_status = self.step(&mut emu);
                            match cpu_status {
                                Ok(CpuStatus::Run(_)) => {},
                                _ => {}
                            }

                            self.steps_remaining -= 1;
                            if self.steps_remaining == 0 {
                                status = EmuStatus::Stopped;
                            }
                        },
                        EmuStatus::Break
                        | EmuStatus::Stopped => {
                            buf = self.receiver.recv().await;
                        },
                        _ => {}
                    }
                    
                    if status != old_status {
                        *self.state.status.lock() = status;
                    }
                }
            });

            return Ok(())
        }

        Err(EmuError::Uninitialized)
    }

    fn step(&mut self, emu: &mut Gbc<Mmu>) -> Result<CpuStatus, gbc::CpuError> {
        let (cpu_status, draw_ready) = emu.step();

        if draw_ready {
            emu.set_drawn();
            *self.state.fb.lock() = emu.cpu.ppu.fb.clone();
            emu.cpu.ppu.debug_show(&emu.cpu.memory, [16, 24], &mut *self.state.vram.lock());
            self.state.fb_pending.store(true, Ordering::Relaxed);
            self.egui_ctx.request_repaint();
            self.dump_state(emu).unwrap();
        }

        // if let Some(serial) = emu.read_serial() {
        //     print!("{}", serial as char);
        // }
        
        match *self.state.status.lock() {
            EmuStatus::Break
            | EmuStatus::Stepping
            | EmuStatus::Stopped => self.dump_state(emu).unwrap(),
            _ => {}
        }
        
        cpu_status
    }

    fn dump_state(&self, emu: &Gbc<Mmu>) -> Result<(), mpsc::error::SendError<EmuMsgOut>> {
        let regs = emu.cpu.regs;
        let io_regs = emu.cpu.dump_io_regs();
        let memory = emu.cpu.memory.load_block(0, u16::MAX);
        let instruction_byte = emu.cpu.memory.load(emu.cpu.regs.pc).unwrap_or(0);
        let (instruction_byte, prefixed) = if instruction_byte == 0xCB {
            (emu.cpu.memory.load(emu.cpu.regs.pc + 1).unwrap_or(0), true)
        } else {
            (instruction_byte, false)
        };

        let next_instruction = gbc::Instruction::from_byte(prefixed, instruction_byte).unwrap_or(gbc::Instruction::NOP);

        let state = StateDump {
            next_instruction,
            regs,
            io_regs,
            memory,
        };
        
        self.sender.send(EmuMsgOut::State(state)) 
    }
}
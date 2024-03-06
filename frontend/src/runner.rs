use std::{fmt::Display, sync::{atomic::Ordering, Arc}};

use egui::Context;
use gbc::{CpuEvent, CpuStatus, Gbc, PpuStatus};
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
        Self::Fresh
    }
}

#[derive(Clone, Copy, Debug, )]
pub enum EmuError {
    Uninitialized,
    What,
}

#[derive(Clone, Copy, Debug)]
pub enum Breakpoint {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
}

impl From<Breakpoint> for gbc::CpuEvent {
    fn from(value: Breakpoint) -> Self {
        match value {
            Breakpoint::Zero => Self::Flag(gbc::CpuFlag::Zero),
            Breakpoint::Subtract => Self::Flag(gbc::CpuFlag::Subtract),
            Breakpoint::HalfCarry => Self::Flag(gbc::CpuFlag::HalfCarry),
            Breakpoint::Carry => Self::Flag(gbc::CpuFlag::Carry),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Breakpoints {
    pub zero_flag: bool,
    pub subtract_flag: bool,
    pub half_carry_flag: bool,
    pub carry_flag: bool,
}

impl Breakpoints {
    pub fn set(&mut self, breakpoint: Breakpoint) {
        match breakpoint {
            Breakpoint::Zero => self.zero_flag = true,
            Breakpoint::Subtract => self.subtract_flag = true,
            Breakpoint::HalfCarry => self.half_carry_flag = true,
            Breakpoint::Carry => self.carry_flag = true,
        }
    }

    pub fn unset(&mut self, breakpoint: Breakpoint) {
        match breakpoint {
            Breakpoint::Zero => self.zero_flag = false,
            Breakpoint::Subtract => self.subtract_flag = false,
            Breakpoint::HalfCarry => self.half_carry_flag = false,
            Breakpoint::Carry => self.carry_flag = false,
        }
    }
}

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
                // *self.state.status.lock() = EmuStatus::Stopped;

                loop {
                    match self.receiver.try_recv() {
                        Ok(msg) => {
                            dbg!(msg);

                            match msg {
                                EmuMsgIn::Exit => return,
                                EmuMsgIn::Pause => {
                                    *self.state.status.lock() = EmuStatus::Stopped
                                },
                                EmuMsgIn::Resume => {
                                    *self.state.status.lock() = EmuStatus::Running
                                },
                                EmuMsgIn::LoadRom => return, // this instance should be dropped and a new instance should replace it
                                EmuMsgIn::Step(steps) => {
                                    self.steps_remaining = steps;
                                    *self.state.status.lock() = EmuStatus::Stepping;
                                },
                                EmuMsgIn::SetBreakpoint(breakpoint) => {
                                    self.breakpoints.set(breakpoint);
                                    emu.cpu.breakpoint_controls.set(breakpoint.into());
                                },
                                EmuMsgIn::UnsetBreakpoint(breakpoint) => {
                                    self.breakpoints.unset(breakpoint);
                                    emu.cpu.breakpoint_controls.unset(breakpoint.into());
                                },
                                EmuMsgIn::FrameLimit => {
                                    if self.state.status.lock().clone() == EmuStatus::Running {
                                        *self.state.status.lock() = EmuStatus::FrameLimited;
                                    }
                                },
                                EmuMsgIn::FrameUnlimit => {
                                    if self.state.status.lock().clone() == EmuStatus::FrameLimited {
                                        *self.state.status.lock() = EmuStatus::Running;
                                    }
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
                                Ok(CpuStatus::Break(_)) => {
                                    *self.state.status.lock() = EmuStatus::Break;
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

        let state = StateDump {
            instruction,
            regs,
            io_regs,
        };
        
        self.sender.send(EmuMsgOut::State(state)) 
    }
}
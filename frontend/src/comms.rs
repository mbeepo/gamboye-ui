use crate::runner::Breakpoint;

#[derive(Clone, Copy, Debug)]
pub enum EmuMsgIn {
    LoadRom,
    Exit,
    Pause,
    Resume,
    Step(usize),
    SetBreakpoint(Breakpoint),
    UnsetBreakpoint(Breakpoint),
}

#[derive(Clone, Copy, Debug)]
pub enum EmuMsgOut {
    State {
        instruction: gbc::Instruction,
        regs: gbc::Registers,
    },
}
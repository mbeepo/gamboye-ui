use crate::{runner::Breakpoint, state::StateDump};

#[derive(Clone, Copy, Debug)]
pub enum EmuMsgIn {
    LoadRom,
    Exit,
    Pause,
    Resume,
    Step(usize),
    SetBreakpoint(Breakpoint),
    UnsetBreakpoint(Breakpoint),
    FrameLimit,
    FrameUnlimit,
}

#[derive(Clone, Copy, Debug)]
pub enum EmuMsgOut {
    State(StateDump),
}
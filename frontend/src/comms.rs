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
    ButtonPressed(gbc::Button),
    ButtonReleased(gbc::Button),
}

#[derive(Clone, Debug)]
pub enum EmuMsgOut {
    State(StateDump),
}
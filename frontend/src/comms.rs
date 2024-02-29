use std::time::Duration;

#[derive(Clone, Debug)]
pub enum EmuMsgIn {
    LoadRom,
    Exit,
    Pause,
    Resume,
}
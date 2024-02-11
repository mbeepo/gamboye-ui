use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum EmuMsgOut {
    RequestRedraw(Vec<u8>),
}

#[derive(Debug)]
pub enum EmuMsgIn {
    Sleep(Instant)
}
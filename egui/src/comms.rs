#[derive(Debug)]
pub enum EmuMsgOut {
    RequestRedraw(Vec<u8>),
}
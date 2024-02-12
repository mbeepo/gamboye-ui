#[derive(Clone, Debug)]
pub enum EmuMsgIn {
    LoadRom(Vec<u8>),
    Exit,
    Pause,
    Resume,
}
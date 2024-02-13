use egui::Color32;

#[derive(Clone, Debug)]
pub enum EmuMsgIn {
    LoadRom(Vec<u8>),
    Exit,
    Pause,
    Resume,
}

#[derive(Clone, Debug)]
pub enum EmuMsgOut {
    UpdateFb(Vec<Color32>),
}
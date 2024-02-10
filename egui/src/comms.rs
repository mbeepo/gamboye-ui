#[derive(Clone, Copy, Debug)]
pub enum EmuMsgOut {
    FramebufferUpdate(gbc::Pixel),
    RequestRedraw,
}

#[derive(Clone, Copy, Debug)]
pub enum Events {
    
}
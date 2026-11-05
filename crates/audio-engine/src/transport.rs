//! Estados do transporte (ver diagrama na arquitetura, seção 5.5).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TransportState {
    #[default]
    Stopped = 0,
    Playing = 1,
    Paused = 2,
    CountIn = 3,
    Recording = 4,
}

impl TransportState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Playing,
            2 => Self::Paused,
            3 => Self::CountIn,
            4 => Self::Recording,
            _ => Self::Stopped,
        }
    }

    /// A timeline avança neste estado?
    pub fn is_rolling(self) -> bool {
        matches!(self, Self::Playing | Self::Recording)
    }
}

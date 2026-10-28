#[derive(Default)]
pub struct Settings {
    pub scale: u32,
    pub fg_color: [u8; 3],
    pub bg_color: [u8; 3],
}

impl Settings {
    pub fn load() -> Self {
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        Ok(())
    }
}
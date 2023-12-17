#[derive(Debug, Clone, Default)]
pub struct GraphicsSettings {
    pub bloom: BloomSettings,
}

#[derive(Debug, Clone)]
pub struct BloomSettings {
    pub blend_factor: f32,
    pub activated: bool,
}
impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            blend_factor: 0.1,
            activated: true,
        }
    }
}

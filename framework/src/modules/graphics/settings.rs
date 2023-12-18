use super::elements::color::Color;

#[derive(Debug, Clone)]
pub struct GraphicsSettings {
    pub bloom: BloomSettings,
    pub clear_color: Color,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            bloom: Default::default(),
            clear_color: Color {
                r: 0.4,
                g: 0.4,
                b: 0.6,
                a: 1.0,
            },
        }
    }
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

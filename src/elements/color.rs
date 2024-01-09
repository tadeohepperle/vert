use glam::{Vec3, Vec4};

/// An SRGB color.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    /// Red component of the color
    pub r: f32,
    /// Green component of the color
    pub g: f32,
    /// Blue component of the color
    pub b: f32,
    /// Alpha component of the color
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
}

impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0);
    pub const LIGHTGREY: Color = Color::new(0.7, 0.7, 0.75);
    pub const GREY: Color = Color::new(0.4, 0.4, 0.5);
    pub const RED: Color = Color::new(1.0, 0.0, 0.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b, a: 1.0 }
    }

    /// creates colors from rgb and maps them into srgb space
    ///
    /// srgb_color = ((rgb_color / 255 + 0.055) / 1.055) ^ 2.4
    pub fn u8(r: u8, g: u8, b: u8) -> Self {
        /// srgb_color = ((rgb_color / 255 + 0.055) / 1.055) ^ 2.4
        #[inline]
        pub fn color_map_to_srgb(u: u8) -> f32 {
            ((u as f32 / 255.0 + 0.055) / 1.055).powf(2.4)
        }

        Color {
            r: color_map_to_srgb(r),
            g: color_map_to_srgb(g),
            b: color_map_to_srgb(b),
            a: 1.0,
        }
    }

    pub const fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

impl From<Color> for wgpu::Color {
    fn from(value: Color) -> Self {
        wgpu::Color {
            r: value.r as f64,
            g: value.g as f64,
            b: value.b as f64,
            a: value.a as f64,
        }
    }
}

impl From<Vec3> for Color {
    fn from(value: Vec3) -> Self {
        Color {
            r: value.x,
            g: value.y,
            b: value.z,
            a: 1.0,
        }
    }
}

impl From<[f32; 3]> for Color {
    fn from(value: [f32; 3]) -> Self {
        Color {
            r: value[0],
            g: value[1],
            b: value[2],
            a: 1.0,
        }
    }
}

impl From<Vec4> for Color {
    fn from(value: Vec4) -> Self {
        Color {
            r: value.x,
            g: value.y,
            b: value.z,
            a: value.w,
        }
    }
}

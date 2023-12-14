use glam::{Vec3, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
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

impl Color {
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0);
    pub const RED: Color = Color::new(1.0, 0.0, 0.0);
    pub const GREEN: Color = Color::new(0.0, 1.0, 0.0);
    pub const BLUE: Color = Color::new(0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0);
    pub const YELLOW: Color = Color::new(1.0, 1.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b, a: 1.0 }
    }

    pub const fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
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

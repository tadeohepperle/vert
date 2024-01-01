#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSizeValues {
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl ScreenSizeValues {
    /// width / height
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// the stuff that gets sent to the shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct ScreenSizeRaw {
    width: f32,
    height: f32,
    aspect: f32,
}

impl ToRaw for ScreenSizeValues {
    type Raw = ScreenSizeRaw;

    fn to_raw(&self) -> Self::Raw {
        ScreenSizeRaw {
            width: self.width as f32,
            height: self.height as f32,
            aspect: self.aspect(),
        }
    }
}

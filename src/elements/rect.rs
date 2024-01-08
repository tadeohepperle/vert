use glam::{dvec2, vec2, DVec2, Vec2};

///  min_x, min_y form the top left corner.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const UNIT: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    pub const ZERO: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(min_x: f32, min_y: f32, width: f32, height: f32) -> Self {
        Self {
            min_x,
            min_y,
            width,
            height,
        }
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.min_x
            && pos.y >= self.min_y
            && pos.x <= self.min_x + self.width
            && pos.y <= self.min_y + self.height
    }

    pub fn d_size(&self) -> DVec2 {
        dvec2(self.width as f64, self.height as f64)
    }

    pub fn size(&self) -> Vec2 {
        vec2(self.width, self.height)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Aabb {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Aabb {
    pub const fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.min_x && pos.y >= self.min_y && pos.x <= self.max_x && pos.y <= self.max_y
    }

    pub fn unit() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        }
    }
}

impl Into<Aabb> for Rect {
    fn into(self) -> Aabb {
        Aabb {
            min_x: self.min_x,
            min_y: self.min_y,
            max_x: self.min_x + self.width,
            max_y: self.min_y + self.height,
        }
    }
}

impl Into<Rect> for Aabb {
    fn into(self) -> Rect {
        Rect {
            min_x: self.min_x,
            min_y: self.min_y,
            width: self.max_x - self.min_x,
            height: self.max_y - self.min_y,
        }
    }
}

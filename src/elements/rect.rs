use std::ops::{Add, Div, Mul};

use super::lerp::Lerp;
use glam::{dvec2, vec2, DVec2, Vec2};

///  min_x, min_y form the top left corner.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Lerp)]
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
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Lerp)]
pub struct Aabb {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Add<Vec2> for Aabb {
    type Output = Aabb;

    fn add(self, rhs: Vec2) -> Self::Output {
        Aabb {
            min_x: self.min_x + rhs.x,
            min_y: self.min_y + rhs.y,
            max_x: self.max_x + rhs.x,
            max_y: self.max_y + rhs.y,
        }
    }
}

impl Mul<f32> for Aabb {
    type Output = Aabb;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.min_x *= rhs;
        self.min_y *= rhs;
        self.max_x *= rhs;
        self.max_y *= rhs;
        self
    }
}

impl Div<f32> for Aabb {
    type Output = Aabb;

    fn div(mut self, rhs: f32) -> Self::Output {
        self.min_x /= rhs;
        self.min_y /= rhs;
        self.max_x /= rhs;
        self.max_y /= rhs;
        self
    }
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

    /// scales the Aabb around its center.
    ///
    /// Scaling with a factor of 2 results in an Aabb twice as large.
    ///
    /// Scaling with a factor of 0.5 creates a smaller Aabb, useful for zooming in at icon uv coords.
    pub fn scale(mut self, factor: f32) -> Self {
        let center_x = (self.max_x + self.min_x) * 0.5;
        let center_y = (self.max_y + self.min_y) * 0.5;
        self.min_x = center_x + (self.min_x - center_x) * factor;
        self.max_x = center_x + (self.max_x - center_x) * factor;
        self.min_y = center_y + (self.min_y - center_y) * factor;
        self.max_y = center_y + (self.max_y - center_y) * factor;
        self
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.min_x && pos.y >= self.min_y && pos.x <= self.max_x && pos.y <= self.max_y
    }

    pub const UNIT: Aabb = Aabb {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 1.0,
    };
}

impl From<Rect> for Aabb {
    fn from(val: Rect) -> Self {
        Aabb {
            min_x: val.min_x,
            min_y: val.min_y,
            max_x: val.min_x + val.width,
            max_y: val.min_y + val.height,
        }
    }
}

impl From<Aabb> for Rect {
    fn from(val: Aabb) -> Self {
        Rect {
            min_x: val.min_x,
            min_y: val.min_y,
            width: val.max_x - val.min_x,
            height: val.max_y - val.min_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Aabb;
    #[test]
    fn scale_aabb() {
        let aabb = Aabb::UNIT.scale(0.5);
        dbg!(aabb);
    }
}

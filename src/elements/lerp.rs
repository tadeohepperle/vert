use glam::{DVec2, DVec3, Quat, Vec2, Vec3};
pub use vert_macros::Lerp;

pub trait Lerp {
    fn lerp(&self, other: &Self, factor: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        *self + (*other - *self) * factor
    }
}

impl Lerp for f64 {
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        *self + (*other - *self) * factor as f64
    }
}

pub struct Lerped<T: Lerp> {
    pub current: T,
    pub target: T,
}

impl<T: Lerp + Clone> Lerped<T> {
    pub fn lerp(&mut self, delta_secs_x_speed: f32) {
        self.current = self.current.lerp(&self.target, delta_secs_x_speed);
    }

    pub fn new(value: T) -> Self {
        Lerped {
            current: value.clone(),
            target: value,
        }
    }

    pub fn set_target(&mut self, value: T) {
        self.target = value;
    }
}

impl Lerp for Vec2 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Vec2::lerp(*self, *other, factor)
    }
}

impl Lerp for DVec2 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        DVec2::lerp(*self, *other, factor as f64)
    }
}

impl Lerp for Vec3 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Vec3::lerp(*self, *other, factor)
    }
}

impl Lerp for DVec3 {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        DVec3::lerp(*self, *other, factor as f64)
    }
}

impl Lerp for Quat {
    #[inline(always)]
    fn lerp(&self, other: &Self, factor: f32) -> Self {
        Quat::lerp(*self, *other, factor)
    }
}

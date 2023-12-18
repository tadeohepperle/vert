use std::sync::{Arc, OnceLock};

use glam::{vec3, Mat4, Vec2, Vec3};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout};
use winit::dpi::PhysicalSize;

use crate::modules::graphics::{
    elements::buffer::{ToRaw, UniformBuffer},
    graphics_context::GraphicsContext,
    shader::bind_group::StaticBindGroup,
};

pub struct Camera {
    uniform: UniformBuffer<CameraValues>,
}

impl Camera {
    /// Warning: Does not write to the GPU uniform buffer!
    pub fn set_cam_transform(&mut self, transform: CamTransform) {
        self.uniform.value.transform = transform;
    }

    pub fn new_default(ctx: &GraphicsContext) -> Self {
        let camera_data = CamTransform::new(vec3(-5.0, 1.0, 0.0), 0.0, 0.0);
        let size = ctx.size();
        let projection = Projection::new(size.width, size.height, 0.8, 0.1, 5000.0);
        Camera::new(camera_data, projection, &ctx.device)
    }

    pub fn new(transform: CamTransform, projection: Projection, device: &wgpu::Device) -> Camera {
        let uniform = UniformBuffer::new(
            CameraValues {
                transform,
                projection,
            },
            device,
        );

        // initialize static bind group for the camera:
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CameraBindGroupLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CameraBindGroup"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.buffer().as_entire_binding(),
            }],
        });

        _CAMERA_BIND_GROUP
            .set((bind_group, layout))
            .expect("_CAMERA_BIND_GROUP cannot be set");

        Camera { uniform }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.uniform.value.projection.resize(height, width);
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_raw_and_buffer(queue);
    }

    pub fn transform(&self) -> &CamTransform {
        &self.uniform.value.transform
    }

    pub fn transform_mut(&mut self) -> &mut CamTransform {
        &mut self.uniform.value.transform
    }

    // todo!() fn camera_plane_point(
    //     camera: &Camera,
    //     camera_transform: &GlobalTransform,
    //     cursor_pos: Vec2,
    // ) -> Vec3 {
    //     let ray = camera
    //         .viewport_to_world(camera_transform, cursor_pos)
    //         .unwrap();
    //     let dist_to_plane = ray.intersect_plane(Vec3::ZERO, Vec3::Y).unwrap_or(10.0);
    //     ray.get_point(dist_to_plane)
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct CamTransform {
    pub pos: Vec3,
    pub pitch: f32,
    pub yaw: f32,
}

impl CamTransform {
    pub fn new(pos: Vec3, pitch: f32, yaw: f32) -> Self {
        CamTransform { pos, pitch, yaw }
    }

    pub fn position(&self) -> Vec3 {
        self.pos
    }

    pub fn calc_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Mat4::look_to_rh(
            self.pos,
            vec3(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vec3::Y,
        )
    }

    pub fn forward(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let forward = vec3(yaw_cos, 0.0, yaw_sin).normalize();
        forward
    }

    pub fn right(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let right = vec3(-yaw_sin, 0.0, yaw_cos).normalize();
        right
    }
}

pub struct Projection {
    /// width / height
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Projection {
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, height: u32, width: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

pub struct CameraValues {
    pub transform: CamTransform,
    pub projection: Projection,
}

impl ToRaw for CameraValues {
    type Raw = CameraRaw;

    fn to_raw(&self) -> Self::Raw {
        CameraRaw::new(&self.transform, &self.projection)
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraRaw {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraRaw {
    fn new(camera: &CamTransform, projection: &Projection) -> Self {
        let mut new = CameraRaw {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        new.update_view_proj(camera, projection);
        new
    }

    fn update_view_proj(&mut self, camera: &CamTransform, projection: &Projection) {
        // homogenous position:
        self.view_position = camera.position().extend(1.0).into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d();
    }
}

static _CAMERA_BIND_GROUP: OnceLock<(BindGroup, BindGroupLayout)> = OnceLock::new();

impl StaticBindGroup for Camera {
    fn bind_group_layout() -> &'static wgpu::BindGroupLayout {
        &_CAMERA_BIND_GROUP
            .get()
            .expect("_CAMERA_BIND_GROUP not set")
            .1
    }

    fn bind_group() -> &'static wgpu::BindGroup {
        &_CAMERA_BIND_GROUP
            .get()
            .expect("_CAMERA_BIND_GROUP not set")
            .0
    }
}

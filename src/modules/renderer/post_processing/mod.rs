use crate::{
    app::{ModuleId, UntypedHandle},
    Handle, Module,
};
use wgpu::{CommandEncoder, ShaderModuleDescriptor};

pub mod tone_mapping;
pub use tone_mapping::{AcesToneMapping, ToneMappingSettings};

pub mod bloom;
pub use bloom::{Bloom, BloomSettings};

pub trait PostProcessingEffect {
    fn apply<'e>(
        &'e mut self,
        encoder: &'e mut CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    );
}

pub(super) struct PostProcessingEffectHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    /// A type punned fn apply<'e>(&'e mut self, encoder: &'e mut CommandEncoder, input_texture: &wgpu::BindGroup, output_texture: &wgpu::TextureView, );
    apply_fn: fn(
        *const (),
        encoder: *const (),
        input_texture: *const (),
        output_texture: *const (),
    ) -> (),
}

impl std::fmt::Debug for PostProcessingEffectHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostProcessingEffectHandle")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl PostProcessingEffectHandle {
    pub fn new<R: PostProcessingEffect + Module>(handle: Handle<R>) -> Self {
        return PostProcessingEffectHandle {
            module_id: ModuleId::of::<R>(),
            handle: handle.untyped(),
            apply_fn: apply::<R>,
        };

        fn apply<R: PostProcessingEffect>(
            obj: *const (),
            encoder: *const (),
            input_texture: *const (),
            output_texture: *const (),
        ) {
            unsafe {
                <R as PostProcessingEffect>::apply(
                    &mut *(obj as *mut R),
                    &mut *(encoder as *mut wgpu::CommandEncoder),
                    &*(input_texture as *const wgpu::BindGroup),
                    &*(output_texture as *const wgpu::TextureView),
                );
            }
        }
    }

    pub fn apply<'e>(
        &'e self,
        encoder: &'e mut CommandEncoder,
        input_texture: &wgpu::BindGroup,
        output_texture: &wgpu::TextureView,
    ) {
        let obj_ptr = self.handle.ptr();
        let encoder_ptr = encoder as *const CommandEncoder as *const ();
        let input_texture_ptr = input_texture as *const wgpu::BindGroup as *const ();
        let output_texture_ptr = output_texture as *const wgpu::TextureView as *const ();
        (self.apply_fn)(obj_ptr, encoder_ptr, input_texture_ptr, output_texture_ptr);
    }
}

/// Shader for a single triangle that covers the entire screen.
pub struct ScreenVertexShader(wgpu::ShaderModule);

impl ScreenVertexShader {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Screen Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen.vert.wgsl").into()),
        });
        ScreenVertexShader(module)
    }

    pub fn vertex_state(&self) -> wgpu::VertexState<'_> {
        wgpu::VertexState {
            module: &self.0,
            entry_point: "vs_main",
            buffers: &[],
        }
    }
}

pub trait SdrSurfaceRenderer {
    fn render<'e>(&'e self, encoder: &'e mut CommandEncoder, view: &wgpu::TextureView);
}

pub(super) struct SdrSurfaceRendererHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    /// A type punned fn render<'e>(&'e mut self, encoder: &'e mut CommandEncoder, view: &wgpu::TextureView);
    render_fn: fn(*const (), encoder: *const (), view: *const ()) -> (),
}

impl std::fmt::Debug for SdrSurfaceRendererHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdrSurfaceRendererHandle")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl SdrSurfaceRendererHandle {
    pub fn new<R: SdrSurfaceRenderer + Module>(handle: Handle<R>) -> Self {
        return SdrSurfaceRendererHandle {
            module_id: ModuleId::of::<R>(),
            handle: handle.untyped(),
            render_fn: render::<R>,
        };

        fn render<R: SdrSurfaceRenderer>(obj: *const (), encoder: *const (), view: *const ()) {
            unsafe {
                <R as SdrSurfaceRenderer>::render(
                    &*(obj as *const R),
                    &mut *(encoder as *mut wgpu::CommandEncoder),
                    &*(view as *const wgpu::TextureView),
                );
            }
        }
    }

    pub fn render<'e>(&'e self, encoder: &'e mut CommandEncoder, view: &wgpu::TextureView) {
        let obj_ptr = self.handle.ptr();
        let encoder_ptr = encoder as *const CommandEncoder as *const ();
        let view_ptr = view as *const wgpu::TextureView as *const ();
        (self.render_fn)(obj_ptr, encoder_ptr, view_ptr);
    }
}

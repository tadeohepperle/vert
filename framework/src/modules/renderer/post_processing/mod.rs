use crate::{
    app::{ModuleId, UntypedHandle},
    modules::GraphicsContext,
    Dependencies, Handle, Module,
};
use wgpu::{CommandEncoder, ShaderModuleDescriptor};

use super::Renderer;

pub mod tone_mapping;
pub use tone_mapping::{AcesToneMapping, ToneMappingSettings};

pub mod bloom;

#[derive(Debug, Dependencies)]
pub struct PostProcessingDefaultDeps {
    renderer: Handle<Renderer>,
    ctx: Handle<GraphicsContext>,
}

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
    /// A type punned  fn apply<'e>(&'e mut self, encoder: &'e mut CommandEncoder, input_texture: &wgpu::BindGroup, output_texture: &wgpu::TextureView, );
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
                    std::mem::transmute(obj),
                    std::mem::transmute(encoder),
                    std::mem::transmute(input_texture),
                    std::mem::transmute(output_texture),
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

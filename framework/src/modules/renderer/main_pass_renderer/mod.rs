use std::fmt::Debug;

use crate::{
    app::{ModuleId, UntypedHandle},
    Handle, Module,
};

pub trait MainPassRenderer {
    /// The renderpass here is expected to be 4xMSAA and has HDR_COLOR_FORMAT as its format.
    fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>);
}

pub(super) struct MainPassRendererHandle {
    module_id: ModuleId,
    handle: UntypedHandle,
    /// A type punned fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>);
    render_fn: fn(*const (), render_pass: *const ()) -> (),
}

impl Debug for MainPassRendererHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainPassRendererHandle")
            .field("module_id", &self.module_id)
            .finish()
    }
}

impl MainPassRendererHandle {
    pub fn new<R: MainPassRenderer + Module>(handle: Handle<R>) -> Self {
        return MainPassRendererHandle {
            module_id: ModuleId::of::<R>(),
            handle: handle.untyped(),
            render_fn: render::<R>,
        };

        fn render<R: MainPassRenderer>(obj: *const (), render_pass: *const ()) {
            unsafe {
                <R as MainPassRenderer>::render(
                    std::mem::transmute(obj),
                    std::mem::transmute(render_pass),
                );
            }
        }
    }

    pub fn render<'encoder>(&self, render_pass: &mut wgpu::RenderPass<'encoder>) {
        let obj_ptr = self.handle.ptr();
        let render_pass_ptr = render_pass as *const wgpu::RenderPass<'encoder> as *const ();
        (self.render_fn)(obj_ptr, render_pass_ptr);
    }
}

use crate::utils::{ResourceList, ResourceBuffer, ResourceManager, DHObject};
use dashi::Context;
use std::sync::{Arc, Mutex};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightDesc {
    pub position: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub range: f32,
    pub direction: [f32; 3],
    pub _pad: u32,
}

pub struct BindlessLights {
    pub lights: Arc<Mutex<ResourceList<ResourceBuffer>>>,
    cpu: Vec<LightDesc>,
}

impl BindlessLights {
    pub fn new() -> Self {
        Self { lights: Arc::new(Mutex::new(ResourceList::default())), cpu: Vec::new() }
    }

    pub fn add_light(&mut self, ctx: &mut Context, res: &mut ResourceManager, light: LightDesc) -> u32 {
        let dh = DHObject::new(ctx, &mut res.allocator, light).unwrap();
        let mut list = self.lights.lock().unwrap();
        list.push(ResourceBuffer::from(dh));
        self.cpu.push(light);
        (list.len() - 1) as u32
    }

    pub fn update_light(&mut self, ctx: &mut Context, index: usize, light: LightDesc) {
        if index >= self.cpu.len() {
            return;
        }
        self.cpu[index] = light;
        let list = self.lights.lock().unwrap();
        if index >= list.entries.len() {
            return;
        }
        let handle = list.entries[index];
        let buf = list.pool.get_ref(handle).unwrap();
        let slice = ctx.map_buffer_mut(buf.handle).unwrap();
        let bytes = bytemuck::bytes_of(&light);
        let offset = buf.offset as usize;
        slice[offset..offset + bytes.len()].copy_from_slice(bytes);
        ctx.unmap_buffer(buf.handle).unwrap();
    }

    pub fn upload_all(&self, ctx: &mut Context) {
        let list = self.lights.lock().unwrap();
        for (i, light) in self.cpu.iter().enumerate() {
            if i >= list.entries.len() {
                break;
            }
            let handle = list.entries[i];
            let buf = list.pool.get_ref(handle).unwrap();
            let slice = ctx.map_buffer_mut(buf.handle).unwrap();
            let bytes = bytemuck::bytes_of(light);
            let offset = buf.offset as usize;
            slice[offset..offset + bytes.len()].copy_from_slice(bytes);
            ctx.unmap_buffer(buf.handle).unwrap();
        }
    }

    /// Register the internal buffer array with the [`ResourceManager`].
    ///
    /// The shader in the tests expects the storage buffer to be named
    /// `Lights`, so we register under that key.  This mirrors the interface
    /// block name used in GLSL and ensures the pipeline builder can locate the
    /// resource when reflecting descriptor bindings.
    pub fn register(&self, res: &mut ResourceManager) {
        // Descriptor reflection for unsized arrays does not preserve the
        // variable name, so the pipeline builder ends up looking for an empty
        // string key. Register under an empty name to satisfy that lookup.
        res.register_buffer_array("", self.lights.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::pipeline_builder::PipelineBuilder;
    use crate::utils::*;
    use dashi::builders::RenderPassBuilder as DPRenderPassBuilder;
    use dashi::{AttachmentDescription, ContextInfo, Viewport};
    use inline_spirv::inline_spirv;
    use serial_test::serial;

    fn make_ctx() -> Context {
        Context::headless(&ContextInfo::default()).unwrap()
    }

    #[test]
    #[serial]
    fn dynamic_light_allocation() {
        let mut ctx = make_ctx();
        let rp = DPRenderPassBuilder::new("rp", Viewport::default())
            .add_subpass(&[AttachmentDescription::default()], None, &[])
            .build(&mut ctx)
            .unwrap();

        let vert = inline_spirv!(
            r"#version 450
            layout(location=0) in vec2 pos;
            void main(){ gl_Position = vec4(pos,0,1); }
            ",
            vert
        ).to_vec();

        let frag = inline_spirv!(
            r"#version 450
            struct Light { vec3 pos; float intensity; vec3 color; float range; vec3 dir; uint pad; };
            layout(set=0,binding=0) buffer Lights { Light lights[]; };
            layout(set=0,binding=1) uniform Count { uint count; };
            layout(location=0) out vec4 o;
            void main(){
                vec3 c = vec3(0.0);
                for(uint i=0u;i<count;i++) { c += lights[i].color * lights[i].intensity; }
                o = vec4(c / float(count), 1.0);
            }
            ",
            frag
        ).to_vec();

        let mut pso = PipelineBuilder::new(&mut ctx, "light_test")
            .vertex_shader(&vert)
            .fragment_shader(&frag)
            .render_pass(rp, 0)
            .build();

        let mut lights = BindlessLights::new();
        let mut res = ResourceManager::new(&mut ctx, 1024 * 1024).unwrap();
        for _ in 0..1000 {
            let ld = LightDesc {
                position: [0.0; 3],
                intensity: 1.0,
                color: [1.0, 1.0, 1.0],
                range: 1.0,
                direction: [0.0; 3],
                _pad: 0,
            };
            lights.add_light(&mut ctx, &mut res, ld);
        }
        let count = lights.lights.lock().unwrap().len() as u32;
        lights.register(&mut res);
        // For unsized arrays the descriptor name is empty when reflected, so we
        // register the accompanying uniform under an empty key as well.
        res.register_variable("", &mut ctx, count);

        let group = pso.create_bind_group(0, &res);
        assert!(group.bind_group.valid());
        assert_eq!(count, 1000);
        res.destroy(&mut ctx);
        ctx.destroy();
    }
}

use crate::utils::{ResourceList, ResourceBuffer, ResourceManager, DHObject};
use dashi::Context;
use std::sync::{Arc, Mutex};

#[repr(C)]
#[derive(Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

    /// Remove the light at `index` from the internal list.
    pub fn remove_light(&mut self, index: usize) {
        let mut list = self.lights.lock().unwrap();
        if let Some(handle) = list.entries.get(index).copied() {
            list.release(handle);
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

        let group = pso.create_bind_group(0, &res).unwrap();
        assert!(group.bind_group.valid());
        assert_eq!(count, 1000);
        res.destroy(&mut ctx);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn update_and_remove_light() {
        let mut ctx = make_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();

        let mut lights = BindlessLights::new();
        let ld = LightDesc { position: [1.0, 2.0, 3.0], intensity: 1.0, color: [4.0, 5.0, 6.0], _pad: 0, ..Default::default() };
        lights.add_light(&mut ctx, &mut res, ld);

        // Update the light
        let new_ld = LightDesc { position: [0.0, 0.0, 0.0], intensity: 2.0, color: [1.0, 1.0, 1.0], _pad: 0, ..Default::default() };

        lights.update_light(&mut ctx, 0, new_ld);
        
        let llights = lights.lights.lock().unwrap();
        let handle = llights.entries[0];
        let buf = llights.get_ref(handle);
        let read_back: LightDesc = {
            let slice = ctx.map_buffer::<u8>(buf.handle).unwrap();
            let data = &slice[buf.offset as usize..][..std::mem::size_of::<LightDesc>()];
            let val = *bytemuck::from_bytes::<LightDesc>(data);
            ctx.unmap_buffer(buf.handle).unwrap();
            val
        };
        assert_eq!(read_back.intensity, 2.0);
        
        drop(llights);

        // Remove the light
        lights.remove_light(0);
        assert_eq!(lights.lights.lock().unwrap().len(), 0);

        res.destroy(&mut ctx);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn upload_all_writes_cpu_values() {
        let mut ctx = make_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
        let mut lights = BindlessLights::new();
        let ld = LightDesc {
            position: [1.0, 1.0, 1.0],
            intensity: 5.0,
            color: [2.0, 3.0, 4.0],
            range: 1.0,
            direction: [0.0; 3],
            _pad: 0,
        };
        lights.add_light(&mut ctx, &mut res, ld);

        {
            let ll = lights.lights.lock().unwrap();
            let handle = ll.entries[0];
            let buf = ll.get_ref(handle);
            let mut slice = ctx.map_buffer_mut(buf.handle).unwrap();
            let range = buf.offset as usize..buf.offset as usize + std::mem::size_of::<LightDesc>();
            for b in &mut slice[range] {
                *b = 0;
            }
            ctx.unmap_buffer(buf.handle).unwrap();
        }

        lights.upload_all(&mut ctx);

        let ll = lights.lights.lock().unwrap();
        let handle = ll.entries[0];
        let buf = ll.get_ref(handle);
        let read_back: LightDesc = {
            let slice = ctx.map_buffer::<u8>(buf.handle).unwrap();
            let data = &slice[buf.offset as usize..][..std::mem::size_of::<LightDesc>()];
            let val = *bytemuck::from_bytes::<LightDesc>(data);
            ctx.unmap_buffer(buf.handle).unwrap();
            val
        };
        assert_eq!(read_back.intensity, ld.intensity);
        assert_eq!(read_back.position, ld.position);

        res.destroy(&mut ctx);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn remove_lights_various_indices() {
        let mut ctx = make_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
        let mut lights = BindlessLights::new();
        let ld = LightDesc { intensity: 1.0, ..Default::default() };
        for _ in 0..3 {
            lights.add_light(&mut ctx, &mut res, ld);
        }
        assert_eq!(lights.lights.lock().unwrap().len(), 3);

        lights.remove_light(1);
        assert_eq!(lights.lights.lock().unwrap().len(), 2);
        lights.remove_light(0);
        assert_eq!(lights.lights.lock().unwrap().len(), 1);
        lights.remove_light(5);
        assert_eq!(lights.lights.lock().unwrap().len(), 1);
        lights.remove_light(0);
        assert_eq!(lights.lights.lock().unwrap().len(), 0);

        res.destroy(&mut ctx);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn update_nonexistent_index() {
        let mut ctx = make_ctx();
        let mut res = ResourceManager::new(&mut ctx, 1024).unwrap();
        let mut lights = BindlessLights::new();
        let ld = LightDesc { intensity: 1.0, ..Default::default() };
        lights.add_light(&mut ctx, &mut res, ld);

        let new_ld = LightDesc { intensity: 3.0, ..Default::default() };
        lights.update_light(&mut ctx, 5, new_ld);

        let ll = lights.lights.lock().unwrap();
        let handle = ll.entries[0];
        let buf = ll.get_ref(handle);
        let read_back: LightDesc = {
            let slice = ctx.map_buffer::<u8>(buf.handle).unwrap();
            let data = &slice[buf.offset as usize..][..std::mem::size_of::<LightDesc>()];
            let val = *bytemuck::from_bytes::<LightDesc>(data);
            ctx.unmap_buffer(buf.handle).unwrap();
            val
        };
        assert_eq!(read_back.intensity, ld.intensity);

        res.destroy(&mut ctx);
        ctx.destroy();
    }
}

// Render pass utilities and core modules
pub mod material;
pub mod utils;
pub mod renderer;
pub mod gltf;
pub mod animation;
pub use utils::*;
pub use material::*;
use dashi::utils::*;
use dashi::*;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;

use crate::{
    Attachment, AttachmentDescription, Format, Handle, ImageInfo, ImageViewInfo, LoadOp,
    RenderPassInfo, SampleCount, StoreOp, SubpassDependency, SubpassDescription, Viewport,
};

pub struct NamedAttachment {
    pub name: String,
    pub format: Format,
}

pub struct NamedSubpass {
    pub name: String,
    pub color_attachments: Vec<String>,
    pub depth_stencil_attachment: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Default)]
pub struct RenderPassBuilder {
    attachments: IndexMap<String, NamedAttachment>,
    subpasses: Vec<NamedSubpass>,
    viewport: Viewport,
    debug_name: &'static str,
    extent: [u32; 2],
}

pub struct AllRenderAttachments {
    pub attachments: Vec<RenderAttachment>,
}

#[derive(Clone)]
pub struct RenderAttachment {
    pub name: String,
    pub attachment: Attachment,
}

#[derive(Clone)]
pub struct RenderTarget {
    pub name: String,
    pub colors: Vec<RenderAttachment>,
    pub depth: Option<RenderAttachment>,
}

type RenderTargets = Vec<RenderTarget>;

#[derive(Debug, Deserialize)]
pub struct YamlAttachment {
    pub name: String,
    pub format: Format,
}

#[derive(Debug, Deserialize)]
pub struct YamlSubpass {
    pub name: String,
    pub color_attachments: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct YamlRenderPass {
    pub debug_name: Option<String>,
    pub extent: Option<[u32; 2]>,
    pub attachments: Vec<YamlAttachment>,
    pub subpasses: Vec<YamlSubpass>,
    pub viewport: Option<Viewport>,
}

impl RenderTarget {
    pub fn get_color_views(&self) -> Vec<Handle<ImageView>> {
        self.colors.iter().map(|a| a.attachment.img).collect()
    }

    pub fn get_depth_view(&self) -> Option<Handle<ImageView>> {
        self.depth.as_ref().map(|a| a.attachment.img)
    }
}

impl RenderPassBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn debug_name(mut self, name: &'static str) -> Self {
        self.debug_name = name;
        self
    }

    pub fn extent(mut self, extent: [u32; 2]) -> Self {
        self.extent = extent;
        self
    }

    pub fn color_attachment(mut self, name: impl Into<String>, format: Format) -> Self {
        let name_str = name.into();
        self.attachments.insert(
            name_str.clone(),
            NamedAttachment {
                name: name_str.clone(),
                format,
            },
        );
        self
    }

    pub fn depth_attachment(mut self, name: impl Into<String>, format: Format) -> Self {
        let name_str = name.into();
        self.attachments.insert(
            name_str.clone(),
            NamedAttachment {
                name: name_str.clone(),
                format,
            },
        );
        self
    }

    pub fn subpass<C, Dep>(
        mut self,
        name: impl Into<String>,
        color_attachments: C,
        depends_on: Dep,
    ) -> Self
    where
        C: IntoIterator,
        C::Item: ToString,
        Dep: IntoIterator,
        Dep::Item: ToString,
    {
        let color_attachments = color_attachments
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let depends_on = depends_on.into_iter().map(|d| d.to_string()).collect();
        self.subpasses.push(NamedSubpass {
            name: name.into(),
            color_attachments,
            depth_stencil_attachment: None,
            depends_on,
        });
        self
    }
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    pub fn build_with_images(
        self,
        ctx: &mut Context,
    ) -> Result<(Handle<RenderPass>, RenderTargets, AllRenderAttachments), GPUError> {
        let mut name_to_subpass_index = IndexMap::new();
        for (i, sp) in self.subpasses.iter().enumerate() {
            name_to_subpass_index.insert(sp.name.clone(), i as u32);
        }

        let mut color_refs_storage = Vec::new();
        let mut depth_refs_storage = Vec::new();
        let mut deps_storage = Vec::new();
        let mut subpass_descs = Vec::new();

        for sub in &self.subpasses {
            let color_refs = sub
                .color_attachments
                .iter()
                .map(|name| AttachmentDescription {
                    format: self.attachments.get(name).unwrap().format,
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            color_refs_storage.push(color_refs);

            let depth_ref =
                sub.depth_stencil_attachment
                    .as_ref()
                    .map(|name| AttachmentDescription {
                        format: self.attachments.get(name).unwrap().format,
                        ..Default::default()
                    });
            depth_refs_storage.push(depth_ref);

            let deps = sub
                .depends_on
                .iter()
                .map(|dep| SubpassDependency {
                    subpass_id: *name_to_subpass_index.get(dep).unwrap(),
                    attachment_id: 0,
                    depth_id: 0,
                })
                .collect::<Vec<_>>();
            deps_storage.push(deps);
        }

        for i in 0..self.subpasses.len() {
            subpass_descs.push(SubpassDescription {
                color_attachments: &color_refs_storage[i],
                depth_stencil_attachment: depth_refs_storage[i].as_ref(),
                subpass_dependencies: &deps_storage[i],
            });
        }

        let rp_info = RenderPassInfo {
            subpasses: &subpass_descs,
            viewport: self.viewport,
            debug_name: self.debug_name,
        };

        let rp_handle = ctx.make_render_pass(&rp_info)?;

        let mut name_to_render_attachment = IndexMap::new();
        for (name, att) in &self.attachments {
            let image = ctx.make_image(&ImageInfo {
                debug_name: &att.name,
                dim: [self.extent[0], self.extent[1], 1],
                format: att.format,
                mip_levels: 1,
                layers: 1,
                initial_data: None,
            })?;

            let view = ctx.make_image_view(&ImageViewInfo {
                debug_name: &att.name,
                img: image,
                layer: 0,
                mip_level: 0,
                aspect: if att.format == Format::D24S8 {
                    AspectMask::Depth
                } else {
                    Default::default()
                }
            })?;

            let clear = match att.format {
                Format::R8Sint => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::R8Uint => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::RGB8 => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::BGRA8 => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::BGRA8Unorm => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::RGBA8 => dashi::ClearValue::IntColor([0, 0, 0, 1]),
                Format::RGBA32F => dashi::ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                Format::D24S8 => dashi::ClearValue::DepthStencil {
                    depth: 0.0,
                    stencil: 0,
                },
            };
            let attachment = Attachment { img: view, clear };

            name_to_render_attachment.insert(
                name.clone(),
                RenderAttachment {
                    name: name.clone(),
                    attachment,
                },
            );
        }

        let mut targets: Vec<RenderTarget> = Vec::new();
        // Find the first depth attachment (if any)
        let global_depth = self
            .attachments
            .keys()
            .find(|k| {
                self.subpasses
                    .iter()
                    .any(|sp| sp.depth_stencil_attachment.as_ref() == Some(*k))
            })
            .or_else(|| {
                self.attachments
                    .keys()
                    .find(|k| matches!(self.attachments.get(*k).unwrap().format, Format::D24S8))
            });

        for sp in &self.subpasses {
            let colors = sp
                .color_attachments
                .iter()
                .map(|name| name_to_render_attachment.get(name).unwrap().clone())
                .collect();

            let depth = global_depth.and_then(|name| name_to_render_attachment.get(name).cloned());

            targets.push(RenderTarget {
                name: sp.name.clone(),
                colors,
                depth,
            });
        }
        let mut all_colors = Vec::new();
        let mut depth_attachment = None;

        for (name, att) in &name_to_render_attachment {
            if self
                .subpasses
                .iter()
                .any(|sp| sp.depth_stencil_attachment.as_ref() == Some(name))
            {
                depth_attachment = Some(att.clone());
            } else {
                all_colors.push(att.clone());
            }
        }

        let mut ordered_names = Vec::new();
        for sub in &self.subpasses {
            for name in &sub.color_attachments {
                if !ordered_names.contains(name) {
                    ordered_names.push(name.clone());
                }
            }
            if let Some(name) = &sub.depth_stencil_attachment {
                if !ordered_names.contains(name) {
                    ordered_names.push(name.clone());
                }
            }
        }
        let all = AllRenderAttachments {
            attachments: ordered_names
                .iter()
                .map(|k| name_to_render_attachment.get(k).unwrap().clone())
                .collect(),
        };
        Ok((rp_handle, targets, all))
    }

    pub fn from_yaml_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let config: YamlRenderPass = serde_yaml::from_reader(file)?;
        return Ok(RenderPassBuilder::from_yaml(config));
    }

    pub fn from_yaml(config: YamlRenderPass) -> Self {
        let mut builder = RenderPassBuilder::new();

        if let Some(name) = config.debug_name {
            builder = builder.debug_name(Box::leak(name.into_boxed_str()));
        }

        if let Some(vp) = config.viewport {
            builder = builder.viewport(vp);
        }

        if let Some(extent) = config.extent {
            builder = builder.extent(extent);
        }

        for att in config.attachments {
            builder = builder.color_attachment(att.name, att.format);
        }

        for sp in config.subpasses {
            builder = builder.subpass(
                sp.name,
                sp.color_attachments,
                sp.depends_on,
            );
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashi::gpu;
    use serial_test::serial;

    fn init_ctx() -> gpu::Context {
        gpu::Context::headless(&Default::default()).unwrap()
    }

    #[test]
    #[serial]
    fn render_pass_builder_constructs() {
        let builder = RenderPassBuilder::new()
            .debug_name("unit_test")
            .extent([1280, 1024])
            .color_attachment("color", Format::RGBA8)
            .subpass("main", &["color"], &[] as &[&str]);

        assert_eq!(builder.subpasses.len(), 1);
        assert!(builder.attachments.contains_key("color"));
    }

    #[test]
    #[serial]
    fn build_render_pass_with_ctx() {
        let mut ctx = init_ctx();
        let builder = RenderPassBuilder::new()
            .debug_name("ctx_test")
            .extent([800, 600])
            .color_attachment("color", Format::RGBA8)
            .depth_attachment("depth", Format::D24S8)
            .subpass("main", ["color"], &[] as &[&str]);

        let (_, targets, all) = builder.build_with_images(&mut ctx).unwrap();
        assert!(!targets.is_empty());
        assert_eq!(targets[0].colors.len(), 1);
        assert!(targets[0].depth.is_some());

        ctx.destroy();
    }

    #[test]
    #[serial]
    fn build_from_yaml_config() {
        let yaml = r#"
            debug_name: example_test
            extent: [640, 480]
            attachments:
              - name: color
                format: RGBA8
              - name: depth
                format: D24S8
            subpasses:
              - name: main
                color_attachments: [color]
                depth_stencil_attachment: depth
                depends_on: []
        "#;

        let parsed: YamlRenderPass = serde_yaml::from_str(yaml).unwrap();
        let builder = RenderPassBuilder::from_yaml(parsed);

        assert_eq!(builder.subpasses.len(), 1);
        assert!(builder.attachments.contains_key("color"));
        assert!(builder.attachments.contains_key("depth"));
        assert_eq!(builder.extent, [640, 480]);
    }
}

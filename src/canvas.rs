//! Abstractions for constructing offscreen render targets.
//!
//! A [`Canvas`] bundles a [`RenderPass`] and its attachments so that it can be
//! inserted into a [`crate::RenderGraph`]. [`crate::CanvasBuilder`] is a small
//! wrapper around [`crate::RenderPassBuilder`] that simplifies the creation of
//! these passes. The
//! resulting `Canvas` exposes its attachments for pipeline creation and can be
//! connected to other graph nodes.

use crate::render_pass::{RenderAttachment, RenderPassBuilder, RenderTarget};
use dashi::utils::*;
use dashi::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod io;
pub use io::*;

#[derive(Clone)]
pub struct Canvas {
    render_pass: Handle<RenderPass>,
    target: RenderTarget,
    attachments: IndexMap<String, RenderAttachment>,
    extent: [u32; 2],
}

/// Helper to reference a specific canvas attachment when creating a pipeline.
pub struct CanvasOutput<'a> {
    pub(crate) canvas: &'a Canvas,
    pub name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttachmentDesc {
    pub name: String,
    pub format: Format,
    #[serde(default)]
    pub clear_color: Option<[f32; 4]>,
    #[serde(default)]
    pub clear_depth: Option<f32>,
    #[serde(default)]
    pub clear_stencil: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasDesc {
    pub extent: [u32; 2],
    pub attachments: Vec<AttachmentDesc>,
}

impl From<&Canvas> for CanvasDesc {
    fn from(c: &Canvas) -> Self {
        let mut attachments = Vec::new();
        for att in c.target.colors.iter() {
            let clear_color = match att.attachment.clear {
                ClearValue::Color(c) => Some(c),
                ClearValue::IntColor(_) => None,
                _ => None,
            };
            attachments.push(AttachmentDesc {
                name: att.name.clone(),
                format: att.format,
                clear_color,
                clear_depth: None,
                clear_stencil: None,
            });
        }
        if let Some(depth) = &c.target.depth {
            let (clear_depth, clear_stencil) = match depth.attachment.clear {
                ClearValue::DepthStencil { depth, stencil } => (Some(depth), Some(stencil)),
                _ => (None, None),
            };
            attachments.push(AttachmentDesc {
                name: depth.name.clone(),
                format: depth.format,
                clear_color: None,
                clear_depth,
                clear_stencil,
            });
        }
        Self {
            extent: c.extent,
            attachments,
        }
    }
}

impl Canvas {
    pub fn render_pass(&self) -> Handle<RenderPass> {
        self.render_pass
    }

    pub fn target(&self) -> &RenderTarget {
        &self.target
    }

    pub fn target_mut(&mut self) -> &mut RenderTarget {
        &mut self.target
    }

    pub fn view(&self, name: &str) -> Option<Handle<ImageView>> {
        self.attachments.get(name).map(|a| a.attachment.img)
    }

    pub fn format(&self, name: &str) -> Option<Format> {
        self.attachments.get(name).map(|a| a.format)
    }

    /// Convenience method to reference an attachment for pipeline creation.
    pub fn output<'a>(&'a self, name: &'a str) -> CanvasOutput<'a> {
        CanvasOutput { canvas: self, name }
    }

    pub fn extent(&self) -> [u32; 2] {
        self.extent
    }

    pub fn from_desc(ctx: &mut Context, desc: &CanvasDesc) -> Result<Self, GPUError> {
        let mut builder = CanvasBuilder::new().extent(desc.extent);
        for att in &desc.attachments {
            if matches!(att.format, Format::D24S8) {
                if att.clear_depth.is_some() || att.clear_stencil.is_some() {
                    builder = builder.depth_attachment_with_clear(
                        att.name.clone(),
                        att.format,
                        att.clear_depth.unwrap_or(1.0),
                        att.clear_stencil.unwrap_or(0),
                    );
                } else {
                    builder = builder.depth_attachment(att.name.clone(), att.format);
                }
            } else if let Some(color) = att.clear_color {
                builder = builder.color_attachment_with_clear(att.name.clone(), att.format, color);
            } else {
                builder = builder.color_attachment(att.name.clone(), att.format);
            }
        }
        builder.build(ctx)
    }
}

#[derive(Default)]
pub struct CanvasBuilder {
    builder: RenderPassBuilder,
    color_names: Vec<String>,
    color_clears: IndexMap<String, [f32; 4]>,
    depth_clear: Option<(String, f32, u32)>,
    extent: [u32; 2],
}

impl CanvasBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn debug_name(mut self, name: &'static str) -> Self {
        self.builder = self.builder.debug_name(name);
        self
    }

    pub fn extent(mut self, extent: [u32; 2]) -> Self {
        self.builder = self.builder.extent(extent);
        self.extent = extent;
        self
    }

    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.builder = self.builder.viewport(viewport);
        self
    }

    pub fn color_attachment(mut self, name: impl Into<String>, format: Format) -> Self {
        let n = name.into();
        self.builder = self.builder.color_attachment(n.clone(), format);
        self.color_names.push(n);
        self
    }

    pub fn color_attachment_with_clear(
        mut self,
        name: impl Into<String>,
        format: Format,
        clear: [f32; 4],
    ) -> Self {
        let n = name.into();
        self.builder = self.builder.color_attachment(n.clone(), format);
        self.color_names.push(n.clone());
        self.color_clears.insert(n, clear);
        self
    }

    pub fn depth_attachment(mut self, name: impl Into<String>, format: Format) -> Self {
        let n = name.into();
        self.builder = self.builder.depth_attachment(n.clone(), format);
        self.depth_clear = Some((n, 1.0, 0));
        self
    }

    pub fn depth_attachment_with_clear(
        mut self,
        name: impl Into<String>,
        format: Format,
        depth: f32,
        stencil: u32,
    ) -> Self {
        let n = name.into();
        self.builder = self.builder.depth_attachment(n.clone(), format);
        self.depth_clear = Some((n, depth, stencil));
        self
    }

    pub fn build(mut self, ctx: &mut Context) -> Result<Canvas, GPUError> {
        let sub_colors = self.color_names.clone();
        self.builder = self.builder.subpass("main", sub_colors, &[] as &[&str]);
        let (rp, mut targets, all) = self.builder.build_with_images(ctx)?;
        let mut target = targets.remove(0);
        for att in &mut target.colors {
            if let Some(clear) = self.color_clears.get(&att.name) {
                att.attachment.clear = ClearValue::Color(*clear);
            }
        }
        if let Some((ref name, depth, stencil)) = self.depth_clear {
            if let Some(depth_att) = target.depth.as_mut().filter(|a| &a.name == name) {
                depth_att.attachment.clear = ClearValue::DepthStencil { depth, stencil };
            }
        }
        let mut attachments = IndexMap::new();
        for att in all.attachments {
            attachments.insert(att.name.clone(), att);
        }
        Ok(Canvas {
            render_pass: rp,
            target,
            attachments,
            extent: self.extent,
        })
    }
}

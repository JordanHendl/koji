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
            attachments.push(AttachmentDesc {
                name: att.name.clone(),
                format: att.format,
            });
        }
        if let Some(depth) = &c.target.depth {
            attachments.push(AttachmentDesc {
                name: depth.name.clone(),
                format: depth.format,
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
                builder = builder.depth_attachment(att.name.clone(), att.format);
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

    pub fn depth_attachment(mut self, name: impl Into<String>, format: Format) -> Self {
        let n = name.into();
        self.builder = self.builder.depth_attachment(n.clone(), format);
        self
    }

    pub fn build(mut self, ctx: &mut Context) -> Result<Canvas, GPUError> {
        let sub_colors = self.color_names.clone();
        self.builder = self.builder.subpass("main", sub_colors, &[] as &[&str]);
        let (rp, mut targets, all) = self.builder.build_with_images(ctx)?;
        let target = targets.remove(0);
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

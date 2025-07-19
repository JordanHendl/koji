use crate::render_pass::{RenderAttachment, RenderPassBuilder, RenderTarget};
use dashi::utils::*;
use dashi::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub struct Canvas {
    render_pass: Handle<RenderPass>,
    target: RenderTarget,
    attachments: IndexMap<String, RenderAttachment>,
}

/// Helper to reference a specific canvas attachment when creating a pipeline.
pub struct CanvasOutput<'a> {
    pub(crate) canvas: &'a Canvas,
    pub name: &'a str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasDesc {
    pub attachments: Vec<String>,
}

impl From<&Canvas> for CanvasDesc {
    fn from(c: &Canvas) -> Self {
        Self {
            attachments: c.attachments.keys().cloned().collect(),
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
}

#[derive(Default)]
pub struct CanvasBuilder {
    builder: RenderPassBuilder,
    color_names: Vec<String>,
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
        })
    }
}

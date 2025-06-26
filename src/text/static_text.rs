use crate::renderer::StaticMesh;
use crate::text::{TextRenderer2D, TextRenderable};
use crate::utils::ResourceManager;
use dashi::*;
use dashi::utils::Handle;

/// Parameters for constructing [`StaticText`].
pub struct StaticTextCreateInfo<'a> {
    /// The string contents to render
    pub text: &'a str,
    /// Font scale in pixels
    pub scale: f32,
    /// Position of the lower-left corner
    pub pos: [f32; 2],
    /// Resource key for the uploaded texture
    pub key: &'a str,
}

/// Immutable text mesh with pre-generated geometry and glyph texture.
pub struct StaticText {
    /// Quad mesh uploaded to the GPU
    pub mesh: StaticMesh,
    /// Resource manager key for the glyph texture
    pub texture_key: String,
    /// Dimensions of the generated texture
    pub dim: [u32; 2],
}

impl TextRenderable for StaticText {
    fn vertex_buffer(&self) -> Handle<Buffer> {
        self.mesh.vertex_buffer.expect("text vertex buffer")
    }

    fn index_buffer(&self) -> Option<Handle<Buffer>> {
        self.mesh.index_buffer
    }

    fn index_count(&self) -> usize {
        self.mesh.index_count
    }
}

impl StaticText {
    /// Create a new `StaticText` object. This uploads a texture for `text`
    /// and creates a quad mesh covering its bounds.
    pub fn new(
        ctx: &mut Context,
        res: &mut ResourceManager,
        renderer: &TextRenderer2D,
        info: StaticTextCreateInfo<'_>,
    ) -> Result<Self, GPUError> {
        let dim =
            renderer.upload_text_texture(ctx, res, info.key, info.text, info.scale);
        let mut mesh = renderer.make_quad(dim, info.pos);
        mesh.upload(ctx)?;
        Ok(Self {
            mesh,
            texture_key: info.key.into(),
            dim,
        })
    }
}

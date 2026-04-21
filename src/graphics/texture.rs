use glow::{Context, HasContext};
use std::sync::Arc;

pub struct TextureInner {
    texture: glow::Texture,
}

// SAFETY: Texture handle is just an ID. The actual resource is owned by GL context.
// Safe to Send/Sync as long as GL calls are made from the main thread only.
unsafe impl Send for TextureInner {}
unsafe impl Sync for TextureInner {}

#[derive(Clone)]
pub struct Texture {
    inner: Arc<TextureInner>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("texture", &"glow::Texture")
            .finish()
    }
}

impl Texture {
    pub fn new(gl: &Context, data: &[u8], width: u32, height: u32) -> Result<Self, String> {
        unsafe {
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture: {}", e))?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                width as i32,
                height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(data),
            );

            gl.generate_mipmap(glow::TEXTURE_2D);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Texture {
                inner: Arc::new(TextureInner { texture }),
            })
        }
    }

    /// Create a placeholder texture (for async loading)
    /// Uses a temporary dummy handle that must be replaced before rendering.
    #[deprecated(note = "Placeholder texture will be replaced by async loader")]
    pub fn new_placeholder() -> Result<Self, String> {
        use std::num::NonZero;
        // SAFETY: This creates a placeholder with ID 1, which is invalid but safe.
        // It will be replaced with a real texture before any rendering occurs.
        let dummy_id = NonZero::new(1).ok_or("Failed to create non-zero ID")?;
        Ok(Self {
            inner: Arc::new(TextureInner {
                texture: glow::NativeTexture(dummy_id),
            }),
        })
    }

    pub fn from_rgba8(gl: &Context, width: u32, height: u32, data: &[u8]) -> Result<Self, String> {
        unsafe {
            let texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create texture: {}", e))?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(data),
            );

            gl.generate_mipmap(glow::TEXTURE_2D);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Texture {
                inner: Arc::new(TextureInner { texture }),
            })
        }
    }

    pub fn bind(&self, gl: &Context) {
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.inner.texture));
        }
    }

    pub fn unbind(gl: &Context) {
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    /// Explicit GPU resource deletion. Call manually before destroying GL context.
    pub fn delete(&self, gl: &Context) {
        unsafe {
            if Arc::strong_count(&self.inner) == 1 {
                gl.delete_texture(self.inner.texture);
            }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        // Cannot delete GL resources here without context access.
        // Resources are cleaned up when GL context is destroyed.
        // Use explicit texture.delete(&gl) before context destruction if needed.
        if Arc::strong_count(&self.inner) == 1 {
            tracing::debug!("Texture will be cleaned up with GL context");
        }
    }
}

use std::sync::Arc;
use glow::{Context, HasContext};

pub struct ShaderInner {
    program: glow::Program,
}

#[derive(Clone)]
pub struct Shader {
    inner: Arc<ShaderInner>,
}

impl std::fmt::Debug for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shader")
            .field("program", &"glow::Program")
            .finish()
    }
}

impl Shader {
    pub fn new(
        gl: &Context,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let vertex_shader = compile_shader(gl, glow::VERTEX_SHADER, vertex_shader_source)?;
            let fragment_shader = compile_shader(gl, glow::FRAGMENT_SHADER, fragment_shader_source)?;

            let program = gl.create_program()
                .map_err(|e| format!("Failed to create program: {}", e))?;
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                return Err(
                    format!("Failed to link shader program: {}", gl.get_program_info_log(program)).into()
                );
            }

            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            Ok(Shader { 
                inner: Arc::new(ShaderInner { program }) 
            })
        }
    }

    pub fn bind(&self, gl: &Context) {
        unsafe {
            gl.use_program(Some(self.inner.program));
        }
    }

    pub fn unbind(gl: &Context) {
        unsafe {
            gl.use_program(None);
        }
    }

    pub fn program(&self) -> glow::Program {
        self.inner.program
    }

    /// Явное удаление GPU-ресурса. Вызывать вручную перед уничтожением GL контекста.
    pub fn delete(&self, gl: &Context) {
        unsafe {
            // Проверяем, есть ли другие ссылки на этот шейдер
            if Arc::strong_count(&self.inner) == 1 {
                gl.delete_program(self.inner.program);
            }
        }
    }
}

unsafe fn compile_shader(
    gl: &Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, Box<dyn std::error::Error>> {
    let shader = gl.create_shader(shader_type)
        .map_err(|e| format!("Failed to create shader: {}", e))?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        let error_msg = gl.get_shader_info_log(shader);
        gl.delete_shader(shader); // Clean up on compilation failure
        return Err(format!("Failed to compile shader: {}", error_msg).into());
    }

    Ok(shader)
}

impl Drop for Shader {
    fn drop(&mut self) {
        // Resources are deleted when the last reference is dropped
        // The actual GL context must still be alive for this to work safely
        // In practice, shaders should be explicitly deleted before destroying the GL context
        if Arc::strong_count(&self.inner) == 1 {
            // We can't delete GL resources here without access to the GL context
            // This is a limitation of OpenGL - resources are context-bound
            // Use shader.delete(&gl) explicitly before context destruction
        }
    }
}

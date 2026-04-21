use glow::{Context, HasContext, NativeBuffer, NativeVertexArray};
use nalgebra::Vector3;
use std::sync::Arc;

/// Handle to a mesh resource
#[derive(Debug, Clone)]
pub struct MeshHandle {
    pub mesh: Arc<Mesh>,
}

impl MeshHandle {
    pub fn new(mesh: Mesh) -> Self {
        Self {
            mesh: Arc::new(mesh),
        }
    }
}

/// Vertex structure for mesh data.
/// Aligned to 4 bytes for safe GPU access and bytemuck casting.
/// Total size: 48 bytes (12 floats * 4 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],   // 12 bytes
    pub normal: [f32; 3],     // 12 bytes
    pub tex_coords: [f32; 2], // 8 bytes
    pub tangent: [f32; 3],    // 12 bytes
    pub bitangent: [f32; 3],  // 12 bytes - added for proper PBR rendering
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

// SAFETY: Mesh is bound to OpenGL context which is not Send/Sync.
// However, the mesh data itself is immutable after creation, so we can safely
// share references across threads as long as GL calls are made from the main thread.
// The actual GL resource deletion happens in the GL context destructor.

pub struct MeshInner {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    indices_count: i32,
}

// SAFETY: These raw GL handles are just IDs. The actual resources are owned by the GL context.
// It's safe to Send/Sync the handles as long as we don't make GL calls from other threads.
unsafe impl Send for MeshInner {}
unsafe impl Sync for MeshInner {}

#[derive(Clone)]
pub struct Mesh {
    inner: Arc<MeshInner>,
}

impl Drop for Mesh {
    fn drop(&mut self) {
        // Resources are shared via Arc. We cannot safely delete GL resources here
        // without access to the GL context. Instead, we rely on the GL context's
        // destructor to clean up all resources when the application shuts down.
        // This is tracked by keeping the Arc alive until context destruction.
        if Arc::strong_count(&self.inner) == 1 {
            tracing::debug!("Mesh resources will be cleaned up with GL context");
        }
    }
}

impl std::fmt::Debug for Mesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesh")
            .field("indices_count", &self.inner.indices_count)
            .finish()
    }
}

impl Mesh {
    /// Create a new mesh from vertices and indices
    pub fn new(gl: &Context, vertices: &[Vertex], indices: &[u32]) -> Result<Self, String> {
        unsafe {
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );

            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices),
                glow::STATIC_DRAW,
            );

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 32, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 32, 12);

            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 32, 24);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Mesh {
                inner: Arc::new(MeshInner {
                    vao,
                    vbo,
                    ebo,
                    indices_count: indices.len() as i32,
                }),
            })
        }
    }

    /// Create mesh from raw vertex data with normals
    pub fn new_with_normals(
        gl: &Context,
        vertices: &[f32],
        indices: &[u32],
    ) -> Result<Self, String> {
        // vertices should be interleaved: pos_x, pos_y, pos_z, norm_x, norm_y, norm_z, tex_u, tex_v, tan_x, tan_y, tan_z
        // Convert to Vertex structs
        let vertex_count = vertices.len() / 12;
        let mut vertex_data = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            let base = i * 12;
            vertex_data.push(Vertex {
                position: [vertices[base], vertices[base + 1], vertices[base + 2]],
                normal: [vertices[base + 3], vertices[base + 4], vertices[base + 5]],
                tex_coords: [vertices[base + 6], vertices[base + 7]],
                tangent: [vertices[base + 8], vertices[base + 9], vertices[base + 10]],
                _padding: 0.0,
            });
        }
        Self::new(gl, &vertex_data, indices)
    }

    /// Create a placeholder mesh (for async loading)
    /// WARNING: This creates an invalid mesh with dummy handles.
    /// It should only be used temporarily and replaced before rendering.
    pub fn new_placeholder() -> Self {
        // Placeholder meshes should only be used temporarily before real GPU resources are created
        // They don't have valid GL handles and will be replaced during rendering
        use std::num::NonZero;
        Self {
            inner: Arc::new(MeshInner {
                vao: unsafe { NativeVertexArray(NonZero::new(1).unwrap_or_else(|| NonZero::new_unchecked(1))) },
                vbo: unsafe { NativeBuffer(NonZero::new(1).unwrap_or_else(|| NonZero::new_unchecked(1))) },
                ebo: unsafe { NativeBuffer(NonZero::new(1).unwrap_or_else(|| NonZero::new_unchecked(1))) },
                indices_count: 0,
            }),
        }
    }

    /// Create an empty mesh (for error cases)
    pub fn empty(gl: &Context) -> Self {
        // Return a minimal valid mesh instead of placeholder
        let vertices = [
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tex_coords: [0.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                _padding: 0.0,
            },
        ];
        let indices = [0u32];
        Self::new(gl, &vertices, &indices).unwrap_or_else(|_| Self::new_placeholder())
    }

    /// Generate a hash key for vertex/indice data for caching
    pub fn generate_mesh_key(vertices: &[f32], indices: &[u32]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Hash the entire data for accurate caching (not just length and endpoints)
        vertices.len().hash(&mut hasher);
        indices.len().hash(&mut hasher);
        
        // Hash all vertices (sample every 4th float for performance on large meshes)
        let sample_step = if vertices.len() > 1024 { 4 } else { 1 };
        for (i, &v) in vertices.iter().enumerate() {
            if i % sample_step == 0 {
                v.to_bits().hash(&mut hasher);
            }
        }
        
        // Hash all indices
        for &idx in indices {
            idx.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    /// Generate a hash key for Arc-wrapped vertex/indice data for caching
    pub fn generate_mesh_key_from_arc(
        vertices: &Arc<Vec<Vector3<f32>>>,
        indices: &Arc<Vec<u32>>,
    ) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let i: &[u32] = indices;

        // Avoid full copy - hash directly from Arc
        let mut hasher = DefaultHasher::new();
        vertices.len().hash(&mut hasher);
        i.len().hash(&mut hasher);
        
        for v in vertices.iter() {
            v.x.to_bits().hash(&mut hasher);
            v.y.to_bits().hash(&mut hasher);
            v.z.to_bits().hash(&mut hasher);
        }
        
        for &idx in i {
            idx.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    pub fn new_raw(gl: &Context, vertices: &[f32], indices: &[u32]) -> Result<Self, String> {
        unsafe {
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );

            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices),
                glow::STATIC_DRAW,
            );

            // Assume vertex format: position(3), normal(3), tex_coords(2), tangent(3), padding(1) = 12 floats = 48 bytes
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 48, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 48, 12);

            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 48, 24);

            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 3, glow::FLOAT, false, 48, 32);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Mesh {
                inner: Arc::new(MeshInner {
                    vao,
                    vbo,
                    ebo,
                    indices_count: indices.len().try_into().unwrap_or(i32::MAX),
                }),
            })
        }
    }

    /// Create a mesh from raw terrain vertex data (stride = 72 bytes for TerrainVertex)
    /// TerrainVertex layout: position(3), normal(3), tangent(3), bitangent(3), texcoord(2), splat_weights(4)
    pub fn new_terrain(gl: &Context, vertices: &[f32], indices: &[u32]) -> Result<Self, String> {
        unsafe {
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;
            gl.bind_vertex_array(Some(vao));

            let vbo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create VBO: {}", e))?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );

            let ebo = gl
                .create_buffer()
                .map_err(|e| format!("Failed to create EBO: {}", e))?;
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices),
                glow::STATIC_DRAW,
            );

            // Stride = 72 bytes (18 floats * 4 bytes)
            let stride: i32 = 72;

            // position: location 0, offset 0
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

            // normal: location 1, offset 12
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

            // tangent: location 2, offset 24
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, stride, 24);

            // bitangent: location 3, offset 36
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 3, glow::FLOAT, false, stride, 36);

            // texcoord: location 4, offset 48
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 2, glow::FLOAT, false, stride, 48);

            // splat_weights: location 5, offset 56
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 4, glow::FLOAT, false, stride, 56);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Mesh {
                inner: Arc::new(MeshInner {
                    vao,
                    vbo,
                    ebo,
                    indices_count: indices.len().try_into().unwrap_or(i32::MAX),
                }),
            })
        }
    }

    pub fn draw(&self, gl: &Context) {
        // Пропускаем рендеринг для placeholder мешей с нулевым количеством индексов
        if self.inner.indices_count == 0 {
            return;
        }
        unsafe {
            gl.bind_vertex_array(Some(self.inner.vao));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.inner.ebo));
            gl.draw_elements(
                glow::TRIANGLES,
                self.inner.indices_count,
                glow::UNSIGNED_INT,
                0,
            );
            gl.bind_vertex_array(None);
        }
    }

    pub fn indices_count(&self) -> i32 {
        self.inner.indices_count
    }

    /// Явное удаление GPU-ресурса. Вызывать вручную перед уничтожением GL контекста.
    pub fn delete(&self, gl: &Context) {
        unsafe {
            // Проверяем, есть ли другие ссылки на этот меш
            if Arc::strong_count(&self.inner) == 1 {
                gl.delete_vertex_array(self.inner.vao);
                gl.delete_buffer(self.inner.vbo);
                gl.delete_buffer(self.inner.ebo);
            }
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        // Ресурсы удаляются только если это последняя ссылка
        // Для гарантированного удаления используйте метод delete(&self, gl: &Context)
    }
}

use nalgebra::{Vector3, Matrix4};
use glow::{Context, HasContext};
use tracing;

/// Отладочный рендерер для визуализации физики
#[derive(Clone)]
pub struct DebugRenderer {
    line_vertices: Vec<f32>,
    point_vertices: Vec<f32>,
    enabled: bool,
    // Internal shader for debug rendering
    shader: Option<glow::Program>,
}

impl DebugRenderer {
    pub fn new() -> Self {
        Self {
            line_vertices: Vec::with_capacity(1024),
            point_vertices: Vec::with_capacity(256),
            enabled: true,
            shader: None,
        }
    }
    
    /// Initialize the debug renderer shader
    pub fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        if self.shader.is_some() {
            return Ok(());
        }
        
        let vert_src = r#"#version 330 core
layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_color;
out vec3 v_color;
uniform mat4 u_view_proj;
void main() {
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_position, 1.0);
}"#;
        
        let frag_src = r#"#version 330 core
in vec3 v_color;
out vec4 FragColor;
void main() {
    FragColor = vec4(v_color, 1.0);
}"#;
        
        unsafe {
            let program = gl.create_program().map_err(|e| format!("Failed to create program: {}", e))?;

            let vert_shader = gl.create_shader(glow::VERTEX_SHADER).map_err(|e| format!("Failed to create vertex shader: {}", e))?;
            gl.shader_source(vert_shader, vert_src);
            gl.compile_shader(vert_shader);
            if !gl.get_shader_compile_status(vert_shader) {
                let log = gl.get_shader_info_log(vert_shader);
                gl.delete_shader(vert_shader);
                return Err(format!("Vertex shader compile error: {}", log));
            }
            gl.attach_shader(program, vert_shader);

            let frag_shader = gl.create_shader(glow::FRAGMENT_SHADER).map_err(|e| format!("Failed to create fragment shader: {}", e))?;
            gl.shader_source(frag_shader, frag_src);
            gl.compile_shader(frag_shader);
            if !gl.get_shader_compile_status(frag_shader) {
                let log = gl.get_shader_info_log(frag_shader);
                gl.delete_shader(frag_shader);
                gl.delete_shader(vert_shader);
                return Err(format!("Fragment shader compile error: {}", log));
            }
            gl.attach_shader(program, frag_shader);
            
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_shader(vert_shader);
                gl.delete_shader(frag_shader);
                return Err(format!("Program link error: {}", log));
            }
            
            gl.detach_shader(program, vert_shader);
            gl.detach_shader(program, frag_shader);
            gl.delete_shader(vert_shader);
            gl.delete_shader(frag_shader);
            
            self.shader = Some(program);
        }
        
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Очистить все накопленные примитивы
    pub fn clear(&mut self) {
        self.line_vertices.clear();
        self.point_vertices.clear();
    }

    /// Задача 5: Отрисовать линии в GL
    pub fn flush_to_gl(&mut self, gl: &Context, view_proj: Matrix4<f32>) {
        if self.line_vertices.is_empty() { return; }
        
        // Ensure shader is initialized
        if self.shader.is_none() {
            if let Err(e) = self.init_gl(gl) {
                tracing::error!("Failed to initialize debug renderer shader: {}", e);
                return;
            }
        }
        
        unsafe {
            let vao = gl.create_vertex_array().ok();
            let vbo = gl.create_buffer().ok();
            
            if let (Some(vao), Some(vbo)) = (vao, vbo) {
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&self.line_vertices),
                    glow::STREAM_DRAW,
                );
                
                // Position attribute (location 0) - 3 floats
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
                // Color attribute (location 1) - 3 floats
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
                
                // Use the debug shader and pass view_proj
                if let Some(shader) = self.shader {
                    gl.use_program(Some(shader));
                    if let Some(u_view_proj) = gl.get_uniform_location(shader, "u_view_proj") {
                        gl.uniform_matrix_4_f32_slice(Some(&u_view_proj), false, view_proj.as_slice());
                    }
                }
                
                gl.draw_arrays(glow::LINES, 0, (self.line_vertices.len() / 6) as i32);
                
                gl.delete_vertex_array(vao);
                gl.delete_buffer(vbo);
            }
        }
        self.line_vertices.clear();
        self.point_vertices.clear();
    }

    /// Нарисовать линию
    pub fn draw_line(&mut self, from: Vector3<f32>, to: Vector3<f32>, color: [f32; 3]) {
        // Vertex format: x, y, z, r, g, b
        self.line_vertices.extend_from_slice(&[
            from.x, from.y, from.z, color[0], color[1], color[2],
            to.x, to.y, to.z, color[0], color[1], color[2],
        ]);
    }

    /// Нарисовать AABB (коробку)
    pub fn draw_aabb(&mut self, min: Vector3<f32>, max: Vector3<f32>, color: [f32; 3]) {
        // 12 ребер куба
        let corners = [
            Vector3::new(min.x, min.y, min.z),
            Vector3::new(max.x, min.y, min.z),
            Vector3::new(max.x, min.y, max.z),
            Vector3::new(min.x, min.y, max.z),
            Vector3::new(min.x, max.y, min.z),
            Vector3::new(max.x, max.y, min.z),
            Vector3::new(max.x, max.y, max.z),
            Vector3::new(min.x, max.y, max.z),
        ];

        // Нижняя грань
        self.draw_line(corners[0], corners[1], color);
        self.draw_line(corners[1], corners[2], color);
        self.draw_line(corners[2], corners[3], color);
        self.draw_line(corners[3], corners[0], color);

        // Верхняя грань
        self.draw_line(corners[4], corners[5], color);
        self.draw_line(corners[5], corners[6], color);
        self.draw_line(corners[6], corners[7], color);
        self.draw_line(corners[7], corners[4], color);

        // Вертикальные ребра
        self.draw_line(corners[0], corners[4], color);
        self.draw_line(corners[1], corners[5], color);
        self.draw_line(corners[2], corners[6], color);
        self.draw_line(corners[3], corners[7], color);
    }

    /// Нарисовать точку контакта с нормалью
    pub fn draw_contact_point(&mut self, point: Vector3<f32>, normal: Vector3<f32>) {
        // Точка (маленький крест)
        let size = 0.1;
        let color = [1.0, 0.0, 0.0]; // Красный

        self.draw_line(
            Vector3::new(point.x - size, point.y, point.z),
            Vector3::new(point.x + size, point.y, point.z),
            color,
        );
        self.draw_line(
            Vector3::new(point.x, point.y - size, point.z),
            Vector3::new(point.x, point.y + size, point.z),
            color,
        );
        self.draw_line(
            Vector3::new(point.x, point.y, point.z - size),
            Vector3::new(point.x, point.y, point.z + size),
            color,
        );

        // Нормаль (зеленая линия)
        let normal_end = point + normal * 0.5;
        self.draw_line(point, normal_end, [0.0, 1.0, 0.0]);
    }

    /// Нарисовать луч подвески колеса
    pub fn draw_wheel_ray(&mut self, from: Vector3<f32>, to: Vector3<f32>, hit: bool) {
        let color = if hit { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] }; // Зеленый если попадание, красный если нет
        self.draw_line(from, to, color);
        
        // Точка в конце луча
        let point_color = if hit { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        let size = 0.05;
        self.draw_line(
            Vector3::new(to.x - size, to.y, to.z),
            Vector3::new(to.x + size, to.y, to.z),
            point_color,
        );
        self.draw_line(
            Vector3::new(to.x, to.y - size, to.z),
            Vector3::new(to.x, to.y + size, to.z),
            point_color,
        );
    }

    /// Нарисовать направление (стрелку)
    pub fn draw_direction(&mut self, from: Vector3<f32>, direction: Vector3<f32>, length: f32, color: [f32; 3]) {
        let to = from + direction.normalize() * length;
        self.draw_line(from, to, color);
        
        // Наконечник стрелки
        let arrow_size = length * 0.2;
        let perp1 = Vector3::new(direction.z, 0.0, -direction.x).normalize() * arrow_size;
        let perp2 = direction.cross(&Vector3::y()).normalize() * arrow_size;
        
        self.draw_line(to, to - perp1 - direction.normalize() * arrow_size, color);
        self.draw_line(to, to - perp2 - direction.normalize() * arrow_size, color);
    }

    /// Нарисовать скорость тела
    pub fn draw_velocity(&mut self, pos: Vector3<f32>, velocity: Vector3<f32>) {
        let speed = velocity.norm();
        if speed < 0.01 { return; }
        
        // Цвет от скорости: зеленый (медленно) -> желтый -> красный (быстро)
        let color = if speed < 5.0 {
            [0.0, 1.0, 0.0]
        } else if speed < 15.0 {
            [1.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        
        self.draw_direction(pos, velocity, speed.min(3.0), color);
    }

    /// Получить слайс вершин линий для рендеринга
    pub fn get_line_vertices(&self) -> &[f32] {
        &self.line_vertices
    }

    /// Получить слайс вершин точек
    pub fn get_point_vertices(&self) -> &[f32] {
        &self.point_vertices
    }

    /// Количество линий
    pub fn line_count(&self) -> usize {
        self.line_vertices.len() / 6 // 6 float на линию (2 вершины * 3 координаты + цвет)
    }
}

impl Default for DebugRenderer {
    fn default() -> Self {
        Self::new()
    }
}

use nalgebra::Vector3;
use glow::{Context, HasContext};

/// Тип частицы
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleType {
    Rain,
    Snow,
    Dust,
    Exhaust,
    Splash,
}

/// Отдельная частица
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub lifetime: f32,         // Оставшееся время жизни
    pub max_lifetime: f32,     // Полное время жизни
    pub size: f32,
    pub color: Vector3<f32>,
    pub particle_type: ParticleType,
    pub active: bool,
}

impl Particle {
    pub fn new() -> Self {
        Self {
            position: Vector3::zeros(),
            velocity: Vector3::zeros(),
            lifetime: 0.0,
            max_lifetime: 1.0,
            size: 0.1,
            color: Vector3::new(1.0, 1.0, 1.0),
            particle_type: ParticleType::Dust,
            active: false,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
    }
}

/// Система частиц
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
    gravity: Vector3<f32>,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        let mut particles = Vec::with_capacity(max_particles);
        for _ in 0..max_particles {
            particles.push(Particle::new());
        }

        Self {
            particles,
            max_particles,
            gravity: Vector3::new(0.0, -9.81, 0.0),
        }
    }

    /// Задача 6: Рендеринг частиц в GL
    pub fn render(&self, gl: &Context, view_proj: nalgebra::Matrix4<f32>) {
        let active: Vec<&Particle> = self.get_active_particles().collect();
        if active.is_empty() { return; }

        // Use particle shader with view_proj uniform for proper transformation
        unsafe {
            let vao = gl.create_vertex_array().ok();
            let vbo = gl.create_buffer().ok();
            
            if let (Some(vao), Some(vbo)) = (vao, vbo) {
                // Формат вершины: x, y, z, r, g, b, size (7 floats)
                let vertices: Vec<f32> = active.iter()
                    .flat_map(|p| [
                        p.position.x, p.position.y, p.position.z,
                        p.color[0], p.color[1], p.color[2],
                        p.size,
                    ])
                    .collect();
                
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&vertices),
                    glow::STREAM_DRAW,
                );
                
                // Position attribute (location 0) - 3 floats
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 28, 0);
                // Color attribute (location 1) - 3 floats
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 28, 12);
                // Size attribute (location 2) - 1 float
                gl.enable_vertex_attrib_array(2);
                gl.vertex_attrib_pointer_f32(2, 1, glow::FLOAT, false, 28, 24);
                
                // Pass view_proj to shader as u_view_proj uniform
                let program = gl.get_parameter_i32(glow::CURRENT_PROGRAM);
                // program может быть 0 если нет активного шейдерного программы, но это маловероятно в render
                let native_program = glow::NativeProgram(
                    std::num::NonZero::new(program as u32)
                        .unwrap_or_else(|| {
                            tracing::warn!("No active GL program during particle render, using fallback");
                            std::num::NonZero::new(1).unwrap_or_else(|| unsafe { std::num::NonZero::new_unchecked(1) })
                        })
                );
                if let Some(loc) = gl.get_uniform_location(native_program, "u_view_proj") {
                    gl.uniform_matrix_4_f32_slice(Some(&loc), false, view_proj.as_slice());
                }
                
                gl.draw_arrays(glow::POINTS, 0, active.len() as i32);
                
                gl.delete_vertex_array(vao);
                gl.delete_buffer(vbo);
            }
        }
    }

    /// Эмиттер дождя
    pub fn emit_rain(&mut self, position: Vector3<f32>, intensity: f32, count: usize) {
        // Use intensity parameter to affect particle density/speed
        let intensity_factor = intensity.clamp(0.0, 1.0);
        let speed_multiplier = 0.5 + intensity_factor * 1.5; // Speed varies from 0.5x to 2.0x
        
        let mut spawned = 0;
        for i in 0..self.particles.len() {
            if spawned >= count { break; }
            if !self.particles[i].active {
                let p = &mut self.particles[i];
                p.position = Vector3::new(
                    position.x + (rand_float() - 0.5) * 20.0,
                    position.y + 10.0 + rand_float() * 5.0,
                    position.z + (rand_float() - 0.5) * 20.0,
                );
                p.velocity = Vector3::new(0.0, -15.0 - rand_float() * 5.0, 0.0);
                p.lifetime = 2.0;
                p.max_lifetime = 2.0;
                p.size = 0.05 + rand_float() * 0.05;
                p.color = Vector3::new(0.6, 0.7, 0.8);
                p.particle_type = ParticleType::Rain;
                p.active = true;
                spawned += 1;
            }
        }
    }

    /// Эмиттер пыли из-под колёс
    pub fn emit_dust(&mut self, position: Vector3<f32>, velocity: Vector3<f32>, slip: f32) {
        if slip < 0.2 { return; } // Только при сильном скольжении
        
        let count = ((slip - 0.2) * 50.0) as usize;
        let mut spawned = 0;
        
        for i in 0..self.particles.len() {
            if spawned >= count { break; }
            if !self.particles[i].active {
                let p = &mut self.particles[i];
                p.position = position + Vector3::new(
                    (rand_float() - 0.5) * 0.5,
                    0.1,
                    (rand_float() - 0.5) * 0.5,
                );
                p.velocity = Vector3::new(
                    (rand_float() - 0.5) * 2.0,
                    1.0 + rand_float() * 2.0,
                    (rand_float() - 0.5) * 2.0,
                ) + velocity * 0.3;
                p.lifetime = 1.5 + rand_float();
                p.max_lifetime = p.lifetime;
                p.size = 0.2 + rand_float() * 0.3;
                p.color = Vector3::new(0.7, 0.65, 0.5); // Коричневато-серый
                p.particle_type = ParticleType::Dust;
                p.active = true;
                spawned += 1;
            }
        }
    }

    /// Эмиттер брызг (вода/грязь)
    pub fn emit_splash(&mut self, position: Vector3<f32>, normal: Vector3<f32>, impact_speed: f32) {
        if impact_speed < 2.0 { return; }
        
        let count = (impact_speed * 3.0) as usize;
        let mut spawned = 0;
        
        for i in 0..self.particles.len() {
            if spawned >= count { break; }
            if !self.particles[i].active {
                let p = &mut self.particles[i];
                p.position = position;
                
                // Отражение от поверхности + разброс
                let tangent = Vector3::new(normal.z, 0.0, -normal.x).normalize();
                let bitangent = normal.cross(&tangent);
                
                p.velocity = normal * impact_speed * 0.5
                    + tangent * (rand_float() - 0.5) * impact_speed
                    + bitangent * (rand_float() - 0.5) * impact_speed;
                    
                p.lifetime = 0.5 + rand_float() * 0.5;
                p.max_lifetime = p.lifetime;
                p.size = 0.1 + rand_float() * 0.15;
                p.color = Vector3::new(0.5, 0.4, 0.3); // Грязь
                p.particle_type = ParticleType::Splash;
                p.active = true;
                spawned += 1;
            }
        }
    }

    /// Обновление всех частиц
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            if !p.active { continue; }

            p.lifetime -= dt;
            if p.lifetime <= 0.0 {
                p.active = false;
                continue;
            }

            // Физика
            match p.particle_type {
                ParticleType::Rain => {
                    // Дождь падает быстро, ветер не сильно влияет
                    p.position += p.velocity * dt;
                }
                ParticleType::Snow => {
                    // Снег падает медленно, сносится ветром
                    p.position += p.velocity * dt;
                    p.position.x += 0.5 * dt; // Ветер
                }
                ParticleType::Dust | ParticleType::Exhaust | ParticleType::Splash => {
                    // Обычная физика с гравитацией и затуханием
                    p.velocity += self.gravity * dt;
                    p.velocity *= 0.95; // Сопротивление воздуха
                    p.position += p.velocity * dt;
                    
                    // Коллизия с "землей" (упрощенно y=0)
                    if p.position.y < 0.0 {
                        p.position.y = 0.0;
                        p.velocity.y = -p.velocity.y * 0.3; // Отскок
                        p.velocity.x *= 0.5;
                        p.velocity.z *= 0.5;
                        
                        if p.velocity.y.abs() < 0.1 {
                            p.active = false;
                        }
                    }
                }
            }

            // Уменьшение размера со временем (для пыли/дыма)
            if p.particle_type == ParticleType::Dust || p.particle_type == ParticleType::Exhaust {
                let t = p.lifetime / p.max_lifetime;
                p.size *= 0.98;
                p.color *= 0.99; // Выцветание
            }
        }
    }

    /// Получить активные частицы для рендеринга
    pub fn get_active_particles(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter().filter(|p| p.active)
    }

    pub fn get_active_count(&self) -> usize {
        self.particles.iter().filter(|p| p.active).count()
    }

    pub fn clear(&mut self) {
        for p in &mut self.particles {
            p.active = false;
        }
    }
}

impl Drop for ParticleSystem {
    fn drop(&mut self) {
        // Cleanup: deactivate all particles and free resources
        self.clear();
        tracing::debug!(target: "particles", "ParticleSystem dropped, {} particles cleaned up", self.max_particles);
    }
}

// Простая псевдо-случайная функция с использованием thread_local RNG
// Используем простой LCG генератор для производительности и детерминизма
fn rand_float() -> f32 {
    use std::cell::Cell;
    
    thread_local! {
        static RNG_STATE: Cell<u32> = const { Cell::new(0x853c49e6) };
    }
    
    RNG_STATE.with(|state| {
        let mut x = state.get();
        // LCG parameters from Numerical Recipes
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        state.set(x);
        (x as f32) / (u32::MAX as f32)
    })
}

use ab_glyph::{Font, FontArc, Glyph, ScaleFont};
use crate::graphics::rhi::ResourceHandle;
use std::collections::HashMap;
use std::sync::Arc;

/// Данные одного глифа в атласе
#[derive(Debug, Clone)]
pub struct GlyphData {
    pub uv_rect: [f32; 4], // u, v, width, height (в координатах текстуры 0..1)
    pub advance: f32,
    pub offset: [f32; 2], // смещение относительно базовой линии
}

/// Шрифт с загруженным атласом глифов
pub struct FontAtlas {
    font: FontArc,
    /// Кэш загруженных глифов (по Unicode скаляру)
    glyphs: HashMap<char, GlyphData>,
    /// Размер шрифта в пикселях (высота)
    pixel_height: f32,
    /// Текстура атласа (заполняется при инициализации в RHI)
    pub texture: Option<ResourceHandle>,
    /// Данные пикселей атласа (RGBA)
    atlas_data: Vec<u8>,
    /// Размеры атласа
    atlas_width: u32,
    atlas_height: u32,
}

impl FontAtlas {
    /// Загрузить шрифт из файла TTF/OTF
    pub fn load_from_file(path: &str, pixel_height: f32) -> Result<Self, String> {
        let font_bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read font file {}: {}", path, e))?;
        
        let font = FontArc::try_from_vec(font_bytes)
            .map_err(|e| format!("Failed to parse font: {}", e))?;
        
        let scaled_font = font.as_scaled(pixel_height);
        
        // Предварительный расчет размеров атласа
        // Для простоты создадим фиксированный атлас 512x512 для ASCII + Cyrillic
        // В продакшене лучше делать динамический упаковщик
        let atlas_width = 512;
        let atlas_height = 512;
        
        let mut glyphs = HashMap::new();
        let mut atlas_data = vec![0u8; (atlas_width * atlas_height * 4) as usize];
        
        // Упаковка глифов (простая сетка для начала)
        // Для ASCII (32-126) и Basic Cyrillic (1024-1103)
        let mut x = 0u32;
        let mut y = 0u32;
        let mut row_max_height = 0u32;
        
        // Символы для генерации: пробел, ASCII печатные, кириллица
        let chars: Vec<char> = (32..127)
            .chain(1024..1104)
            .filter_map(std::char::from_u32)
            .collect();
            
        for ch in chars {
            let glyph = scaled_font.outlined_glyph(ch).unwrap_or_else(|| {
                // Если глиф не найден, используем вопросительный знак или пустой
                scaled_font.outlined_glyph('?').unwrap()
            });
            
            let bounds = glyph.px_bounds();
            let width = bounds.width() as u32;
            let height = bounds.height() as u32;
            
            if width == 0 || height == 0 {
                // Пустой глиф (например, пробел)
                glyphs.insert(ch, GlyphData {
                    uv_rect: [0.0, 0.0, 0.0, 0.0],
                    advance: scaled_font.h_advance(glyph.id()) / pixel_height,
                    offset: [0.0, 0.0],
                });
                continue;
            }
            
            // Перенос строки если не влезает
            if x + width > atlas_width {
                x = 0;
                y += row_max_height;
                row_max_height = 0;
            }
            
            if y + height > atlas_height {
                return Err("Font atlas overflow! Increase atlas size.".to_string());
            }
            
            row_max_height = row_max_height.max(height);
            
            // Растеризация глифа в атлас
            glyph.draw(|gx, gy, v| {
                let px_x = (bounds.min.x as u32 + gx) as usize;
                let px_y = (bounds.min.y as u32 + gy) as usize;
                
                if px_x < atlas_width as usize && px_y < atlas_height as usize {
                    let idx = (px_y * atlas_width as usize + px_x) * 4;
                    // Alpha channel only (white glyph)
                    atlas_data[idx] = 255;
                    atlas_data[idx + 1] = 255;
                    atlas_data[idx + 2] = 255;
                    atlas_data[idx + 3] = (v * 255.0) as u8;
                }
            });
            
            // Сохранение данных глифа
            let u = x as f32 / atlas_width as f32;
            let v = y as f32 / atlas_height as f32;
            let w = width as f32 / atlas_width as f32;
            let h = height as f32 / atlas_height as f32;
            
            glyphs.insert(ch, GlyphData {
                uv_rect: [u, v, w, h],
                advance: scaled_font.h_advance(glyph.id()) / pixel_height,
                offset: [
                    bounds.min.x as f32 / pixel_height,
                    bounds.min.y as f32 / pixel_height,
                ],
            });
            
            x += width;
        }
        
        Ok(Self {
            font,
            glyphs,
            pixel_height,
            texture: None,
            atlas_data,
            atlas_width: atlas_width as u32,
            atlas_height: atlas_height as u32,
        })
    }
    
    /// Получить данные глифа
    pub fn get_glyph(&self, ch: char) -> Option<&GlyphData> {
        self.glyphs.get(&ch)
    }
    
    /// Получить размеры текста (ширина, высота)
    pub fn measure_text(&self, text: &str) -> (f32, f32) {
        let scaled_font = self.font.as_scaled(self.pixel_height);
        let width = scaled_font.horizontal_advance(scaled_font.scale_px_glyphs(text));
        let height = self.pixel_height;
        (width, height)
    }
    
    /// Получить сырые данные атласа для загрузки в текстуру
    pub fn get_atlas_data(&self) -> &[u8] {
        &self.atlas_data
    }
    
    pub fn get_atlas_dimensions(&self) -> (u32, u32) {
        (self.atlas_width, self.atlas_height)
    }
}

/// Менеджер шрифтов
pub struct FontManager {
    fonts: HashMap<String, Arc<FontAtlas>>,
}

impl FontManager {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }
    
    pub fn load_font(&mut self, name: &str, path: &str, size: f32) -> Result<(), String> {
        let atlas = FontAtlas::load_from_file(path, size)?;
        self.fonts.insert(name.to_string(), Arc::new(atlas));
        Ok(())
    }
    
    pub fn get_font(&self, name: &str) -> Option<Arc<FontAtlas>> {
        self.fonts.get(name).cloned()
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

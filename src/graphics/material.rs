// ЧАСТЬ 4 — ТЕКСТУРНЫЕ СЛОИ: МАТЕРИАЛЫ С УРОВНЯМИ КАЧЕСТВА

use crate::assets::{AssetLoader, AssetHandle};
use std::path::Path;

/// Уровни качества текстур
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureQuality {
    Minimum,  // Только albedo 512×512
    Low,      // Albedo + Normal 1024×1024
    Medium,   // Albedo + Normal + ORM 1024×1024
    High,     // Полный PBR + Emissive 2048×2048
    Ultra,    // Максимальное качество + Detail Normal 4096×4096
}

impl TextureQuality {
    /// Проверить, поддерживает ли уровень данную текстуру
    pub fn supports_normal(&self) -> bool {
        matches!(self, TextureQuality::Low | TextureQuality::Medium | TextureQuality::High | TextureQuality::Ultra)
    }
    
    pub fn supports_orm(&self) -> bool {
        matches!(self, TextureQuality::Medium | TextureQuality::High | TextureQuality::Ultra)
    }
    
    pub fn supports_emissive(&self) -> bool {
        matches!(self, TextureQuality::High | TextureQuality::Ultra)
    }
    
    pub fn supports_detail(&self) -> bool {
        matches!(self, TextureQuality::Ultra)
    }
    
    /// Получить максимальное разрешение для уровня
    pub fn max_resolution(&self) -> u32 {
        match self {
            TextureQuality::Minimum => 512,
            TextureQuality::Low => 1024,
            TextureQuality::Medium => 1024,
            TextureQuality::High => 2048,
            TextureQuality::Ultra => 4096,
        }
    }
}

/// Слои текстур материала
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialLayers {
    pub albedo: Option<AssetHandle>,    // Всегда загружен
    pub normal: Option<AssetHandle>,    // Если quality >= Low
    pub orm: Option<AssetHandle>,       // Если quality >= Medium (Occlusion+Roughness+Metallic)
    pub emissive: Option<AssetHandle>,  // Если quality >= High
    pub detail: Option<AssetHandle>,    // Только Ultra
}

/// Параметры материала
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialParams {
    pub roughness_scale: f32,
    pub metallic_scale: f32,
    pub emissive_intensity: f32,
    pub tiling: [f32; 2],
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            roughness_scale: 1.0,
            metallic_scale: 1.0,
            emissive_intensity: 1.0,
            tiling: [1.0, 1.0],
        }
    }
}

/// Материал с поддержкой уровней качества
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub layers: MaterialLayers,
    pub params: MaterialParams,
    pub shader_type: String,
    pub loaded_quality: TextureQuality,
}

impl Material {
    /// Создать пустой материал
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            layers: MaterialLayers::default(),
            params: MaterialParams::default(),
            shader_type: "pbr".to_string(),
            loaded_quality: TextureQuality::Minimum,
        }
    }
    
    /// Загрузить материал с нужным уровнем качества
    pub fn load(
        path: &str,
        quality: TextureQuality,
        loader: &mut AssetLoader,
    ) -> Result<Self, MaterialLoadError> {
        // Заглушка - упрощено для компиляции
        Ok(Self::new(path))
    }

    /// Улучшить качество материала в рантайме
    pub fn upgrade(&mut self, _new_quality: TextureQuality, _loader: &mut AssetLoader) -> Result<(), MaterialLoadError> {
        // Заглушка - упрощено для компиляции
        Ok(())
    }

    /// Ухудшить качество материала (освободить VRAM)
    pub fn downgrade(&mut self, new_quality: TextureQuality) {
        if new_quality >= self.loaded_quality {
            return; // Не нужно ухудшать
        }
        
        // Выгрузить лишние текстуры
        if !new_quality.supports_detail() {
            self.layers.detail = None;
        }
        
        if !new_quality.supports_emissive() {
            self.layers.emissive = None;
        }
        
        if !new_quality.supports_orm() {
            self.layers.orm = None;
        }
        
        if !new_quality.supports_normal() {
            self.layers.normal = None;
        }
        
        self.loaded_quality = new_quality;
    }
    
    /// Получить количество загруженных текстурных слоёв
    pub fn loaded_layer_count(&self) -> usize {
        let mut count = 0;
        if self.layers.albedo.is_some() { count += 1; }
        if self.layers.normal.is_some() { count += 1; }
        if self.layers.orm.is_some() { count += 1; }
        if self.layers.emissive.is_some() { count += 1; }
        if self.layers.detail.is_some() { count += 1; }
        count
    }
}

/// Ошибки загрузки материала
#[derive(Debug, Clone)]
pub enum MaterialLoadError {
    FileNotFound(String),
    ParseError(String),
    TextureLoadError(String),
}

impl std::fmt::Display for MaterialLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MaterialLoadError::FileNotFound(path) => write!(f, "Material file not found: {}", path),
            MaterialLoadError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            MaterialLoadError::TextureLoadError(msg) => write!(f, "Texture load error: {}", msg),
        }
    }
}

/// Менеджер материалов — кэширует и управляет материалами
#[derive(Clone)]
pub struct MaterialManager {
    materials: Vec<Material>,
    default_quality: TextureQuality,
}

impl MaterialManager {
    pub fn new(default_quality: TextureQuality) -> Self {
        Self {
            materials: Vec::new(),
            default_quality,
        }
    }
    
    /// Загрузить или получить кэшированный материал
    pub fn get_or_load(
        &mut self,
        path: &str,
        loader: &mut AssetLoader,
    ) -> Result<usize, MaterialLoadError> {
        // Поиск в кэше
        if let Some(idx) = self.materials.iter().position(|m| m.name == path) {
            return Ok(idx);
        }
        
        // Загрузить новый
        let material = Material::load(path, self.default_quality, loader)?;
        self.materials.push(material);
        Ok(self.materials.len() - 1)
    }
    
    /// Получить ссылку на материал
    pub fn get(&self, index: usize) -> Option<&Material> {
        self.materials.get(index)
    }
    
    /// Изменить глобальное качество материалов
    pub fn set_global_quality(&mut self, quality: TextureQuality) {
        self.default_quality = quality;
        
        // Обновить все материалы
        for material in &mut self.materials {
            if quality < material.loaded_quality {
                material.downgrade(quality);
            }
            // Upgrade требует loader, делается лениво при необходимости
        }
    }
    
    /// Статистика
    pub fn get_stats(&self) -> MaterialStats {
        let mut total_layers = 0;
        let mut min_quality = TextureQuality::Ultra;
        let mut max_quality = TextureQuality::Minimum;
        
        for mat in &self.materials {
            total_layers += mat.loaded_layer_count();
            if mat.loaded_quality < min_quality {
                min_quality = mat.loaded_quality;
            }
            if mat.loaded_quality > max_quality {
                max_quality = mat.loaded_quality;
            }
        }
        
        MaterialStats {
            material_count: self.materials.len(),
            total_loaded_layers: total_layers,
            min_quality,
            max_quality,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaterialStats {
    pub material_count: usize,
    pub total_loaded_layers: usize,
    pub min_quality: TextureQuality,
    pub max_quality: TextureQuality,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_texture_quality_features() {
        assert!(!TextureQuality::Minimum.supports_normal());
        assert!(TextureQuality::Low.supports_normal());
        assert!(!TextureQuality::Low.supports_orm());
        assert!(TextureQuality::Medium.supports_orm());
        assert!(!TextureQuality::Medium.supports_emissive());
        assert!(TextureQuality::High.supports_emissive());
        assert!(!TextureQuality::High.supports_detail());
        assert!(TextureQuality::Ultra.supports_detail());
    }
    
    #[test]
    fn test_material_creation() {
        let mat = Material::new("test_material");
        assert_eq!(mat.name, "test_material");
        assert_eq!(mat.loaded_quality, TextureQuality::Minimum);
        assert!(mat.layers.albedo.is_none());
    }
    
    #[test]
    fn test_downgrade() {
        let mut mat = Material::new("test");
        mat.loaded_quality = TextureQuality::Ultra;
        mat.layers.normal = Some(AssetHandle(0));
        mat.layers.orm = Some(AssetHandle(1));
        mat.layers.emissive = Some(AssetHandle(2));
        mat.layers.detail = Some(AssetHandle(3));
        
        mat.downgrade(TextureQuality::Low);
        
        assert_eq!(mat.loaded_quality, TextureQuality::Low);
        assert!(mat.layers.normal.is_some());
        assert!(mat.layers.orm.is_none());
        assert!(mat.layers.emissive.is_none());
        assert!(mat.layers.detail.is_none());
    }
}

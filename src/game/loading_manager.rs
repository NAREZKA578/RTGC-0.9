//! Loading Manager - Проверка и отслеживание загрузки всех ресурсов
//! Фиксирует какие файлы загрузились и используются

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

/// Статус загрузки ресурса
#[derive(Debug, Clone, PartialEq)]
pub enum LoadStatus {
    /// Ресурс ещё не загружен
    Pending,
    /// Ресурс загружается
    Loading,
    /// Ресурс успешно загружен
    Loaded,
    /// Ошибка загрузки
    Failed(String),
    /// Ресурс не найден
    NotFound,
}

/// Информация о загружаемом ресурсе
#[derive(Debug, Clone)]
pub struct LoadableResource {
    /// Путь к файлу
    pub path: String,
    /// Тип ресурса
    pub resource_type: ResourceType,
    /// Статус загрузки
    pub status: LoadStatus,
    /// Время начала загрузки
    pub load_start_time: Option<Instant>,
    /// Время завершения загрузки
    pub load_end_time: Option<Instant>,
    /// Размер файла в байтах
    pub file_size_bytes: Option<u64>,
    /// Приоритет загрузки (0 - highest)
    pub priority: u8,
    /// Количество ссылок на ресурс
    pub ref_count: usize,
}

/// Тип ресурса
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Mesh,
    Texture,
    Shader,
    Audio,
    Config,
    Font,
    Script,
    Other,
}

/// Состояние загрузчика
#[derive(Debug, Clone, PartialEq)]
pub enum LoadingState {
    /// Загрузка ещё не началась
    Idle,
    /// Проверка наличия файлов
    Checking,
    /// Загрузка ресурсов
    Loading,
    /// Загрузка завершена
    Complete,
    /// Ошибка загрузки
    Failed,
}

/// Прогресс загрузки
#[derive(Debug, Clone)]
pub struct LoadingProgress {
    /// Общее количество ресурсов
    pub total_resources: usize,
    /// Количество загруженных ресурсов
    pub loaded_resources: usize,
    /// Количество проваленных загрузок
    pub failed_resources: usize,
    /// Текущий этап загрузки
    pub current_stage: String,
    /// Прогресс в процентах (0.0 - 1.0)
    pub progress: f32,
    /// Расчётное время до завершения (сек)
    pub eta_seconds: Option<f32>,
}

/// Этап загрузки для детального прогресс-бара
#[derive(Debug, Clone, PartialEq)]
pub enum LoadingStage {
    InitWorld,
    GeneratingSettlements,
    LoadingTerrain { chunks_done: u32, chunks_total: u32 },
    LoadingVehicle(String),
    GeneratingMissions,
    LoadingAudio,
    BuildingLOD,
    SpawningPhysics,
    UploadingGPU,
    Finalizing,
    Done,
}

/// Состояние загрузки с поддержкой этапов
#[derive(Debug, Clone)]
pub struct LoadingStateDetailed {
    pub stage: LoadingStage,
    pub progress: f32,         // 0.0 .. 1.0
    pub status_text: String,   // "Загрузка Новосибирска..."
    pub error: Option<String>, // если что-то упало
}

/// Менеджер загрузки - проверяет и отслеживает все ресурсы
#[derive()]
pub struct LoadingManager {
    /// Список всех ресурсов
    resources: HashMap<String, LoadableResource>,
    /// Очередь загрузки (отсортированная по приоритету)
    load_queue: Vec<String>,
    /// Текущее состояние
    state: LoadingState,
    /// Детальное состояние загрузки
    detailed_state: LoadingStateDetailed,
    /// Время начала загрузки
    start_time: Option<Instant>,
    /// Время завершения загрузки
    end_time: Option<Instant>,
    /// Корневая директория ассетов
    asset_root: PathBuf,
    /// Статистика загрузки
    stats: LoadingStats,
    /// Callback для обновления прогресса
    progress_callback: Option<Box<dyn Fn(LoadingProgress) + Send + Sync>>,
}

impl Clone for LoadingManager {
    fn clone(&self) -> Self {
        Self {
            resources: self.resources.clone(),
            load_queue: self.load_queue.clone(),
            state: self.state.clone(),
            detailed_state: self.detailed_state.clone(),
            start_time: self.start_time,
            end_time: self.end_time,
            asset_root: self.asset_root.clone(),
            stats: self.stats.clone(),
            progress_callback: None,
        }
    }
}

/// Статистика загрузки
#[derive(Debug, Clone, Default)]
pub struct LoadingStats {
    /// Общее количество проверенных файлов
    pub total_files_checked: usize,
    /// Количество загруженных файлов
    pub total_files_loaded: usize,
    /// Количество проваленных загрузок
    pub total_files_failed: usize,
    /// Общее время загрузки (мс)
    pub total_load_time_ms: u64,
    /// Среднее время загрузки одного ресурса (мс)
    pub avg_load_time_ms: f32,
    /// Максимальное время загрузки (мс)
    pub max_load_time_ms: u64,
    /// Минимальное время загрузки (мс)
    pub min_load_time_ms: u64,
    /// Общий размер загруженных данных (байты)
    pub total_data_size_bytes: u64,
    /// Количество потокобезопасных загрузок
    pub async_loads: usize,
}

impl LoadingManager {
    /// Создать новый менеджер загрузки
    pub fn new(asset_root: &str) -> Self {
        Self {
            resources: HashMap::new(),
            load_queue: Vec::new(),
            state: LoadingState::Idle,
            detailed_state: LoadingStateDetailed {
                stage: LoadingStage::InitWorld,
                progress: 0.0,
                status_text: "Инициализация...".to_string(),
                error: None,
            },
            start_time: None,
            end_time: None,
            asset_root: PathBuf::from(asset_root),
            stats: LoadingStats::default(),
            progress_callback: None,
        }
    }

    /// Установить callback для обновления прогресса
    pub fn set_progress_callback<F>(&mut self, callback: F)
    where
        F: Fn(LoadingProgress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
    }

    /// Добавить ресурс в список загрузки
    pub fn add_resource(&mut self, path: &str, resource_type: ResourceType, priority: u8) {
        let resource = LoadableResource {
            path: path.to_string(),
            resource_type: resource_type.clone(),
            status: LoadStatus::Pending,
            load_start_time: None,
            load_end_time: None,
            file_size_bytes: None,
            priority,
            ref_count: 1,
        };

        self.resources.insert(path.to_string(), resource);
        self.load_queue.push(path.to_string());

        // Сортировка очереди по приоритету
        self.load_queue.sort_by(|a, b| {
            let priority_a = self.resources.get(a).map(|r| r.priority).unwrap_or(255);
            let priority_b = self.resources.get(b).map(|r| r.priority).unwrap_or(255);
            priority_a.cmp(&priority_b)
        });

        debug!(
            "Added resource: {} (type: {:?}, priority: {})",
            path, resource_type, priority
        );
    }

    /// Получить детализированное состояние загрузки
    pub fn get_detailed_state(&self) -> &LoadingStateDetailed {
        &self.detailed_state
    }

    /// Установить текущий этап загрузки
    pub fn set_stage(&mut self, stage: LoadingStage, progress: f32, status_text: String) {
        self.detailed_state.stage = stage;
        self.detailed_state.progress = progress;
        self.detailed_state.status_text = status_text;

        // Вызываем callback если есть
        if let Some(ref callback) = self.progress_callback {
            let loading_progress = self.get_progress();
            callback(loading_progress);
        }
    }

    /// Обновить прогресс текущего этапа
    pub fn update_stage_progress(&mut self, progress: f32) {
        self.detailed_state.progress = progress.clamp(0.0, 1.0);
    }

    /// Установить ошибку загрузки
    pub fn set_error(&mut self, error: String) {
        self.detailed_state.error = Some(error);
        self.state = LoadingState::Failed;
    }

    /// Проверить наличие всех файлов
    pub fn check_all_files(&mut self) -> LoadingProgress {
        info!("Checking all files...");
        self.state = LoadingState::Checking;

        let mut checked = 0;
        let mut not_found = 0;

        for (path, resource) in &mut self.resources {
            let full_path = self.asset_root.join(path);

            if full_path.exists() {
                // Получение размера файла
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    resource.file_size_bytes = Some(metadata.len());
                    resource.status = LoadStatus::Pending;
                } else {
                    resource.status = LoadStatus::Pending;
                }
                checked += 1;
            } else {
                resource.status = LoadStatus::NotFound;
                not_found += 1;
                warn!("File not found: {}", path);
            }
        }

        self.stats.total_files_checked = checked;
        self.stats.total_files_failed = not_found;

        self.get_progress()
    }

    /// Загрузить все ресурсы
    pub fn load_all(&mut self) -> LoadingProgress {
        info!("Starting loading all resources...");
        self.state = LoadingState::Loading;
        self.start_time = Some(Instant::now());

        let mut loaded = 0;
        let mut failed = 0;

        // Копируем очередь и callback, чтобы избежать проблем с заимствованием
        let queue: Vec<String> = self.load_queue.drain(..).collect();
        let callback = self.progress_callback.take();

        // Загрузка ресурсов по очереди
        for path in &queue {
            // Сначала копируем всю необходимую информацию
            let (resource_type_opt, status) = {
                if let Some(resource) = self.resources.get(path) {
                    (
                        Some(resource.resource_type.clone()),
                        resource.status.clone(),
                    )
                } else {
                    (None, LoadStatus::NotFound)
                }
            };

            if status == LoadStatus::NotFound {
                continue;
            }

            let resource_type = match resource_type_opt {
                Some(rt) => rt,
                None => continue,
            };

            // Обновляем статус загрузки
            if let Some(resource) = self.resources.get_mut(path) {
                resource.status = LoadStatus::Loading;
                resource.load_start_time = Some(Instant::now());
            }

            // Загрузка ресурса (вне borrow)
            let full_path = self.asset_root.join(path);
            let load_result = self.load_resource_internal(path, &full_path, &resource_type);

            let load_time = Instant::now();
            let size_result = load_result.and_then(|_| {
                std::fs::metadata(&full_path)
                    .map(|m| m.len())
                    .map_err(|e| e.to_string())
            });

            // Обновляем результат загрузки
            if let Some(resource) = self.resources.get_mut(path) {
                resource.load_end_time = Some(load_time);

                match size_result {
                    Ok(size) => {
                        resource.status = LoadStatus::Loaded;
                        resource.file_size_bytes = Some(size);
                        loaded += 1;
                        self.stats.total_files_loaded += 1;
                        self.stats.total_data_size_bytes += size;

                        // Обновление статистики времени
                        if let Some(start) = resource.load_start_time {
                            let duration = load_time.duration_since(start).as_millis() as u64;
                            self.stats.total_load_time_ms += duration;
                            if duration > self.stats.max_load_time_ms {
                                self.stats.max_load_time_ms = duration;
                            }
                            if self.stats.min_load_time_ms == 0
                                || duration < self.stats.min_load_time_ms
                            {
                                self.stats.min_load_time_ms = duration;
                            }
                        }

                        debug!("Loaded: {} ({:.2} KB)", path, size as f32 / 1024.0);
                    }
                    Err(err) => {
                        resource.status = LoadStatus::Failed(err.clone());
                        failed += 1;
                        self.stats.total_files_failed += 1;
                        error!("Failed to load {}: {}", path, err);
                    }
                }
            }

            // Вызов callback для обновления прогресса (вне borrow)
            if let Some(ref cb) = callback {
                let progress = self.get_progress();
                cb(progress);
            }
        }

        // Восстанавливаем callback
        self.progress_callback = callback;

        self.state = LoadingState::Complete;
        self.end_time = Some(Instant::now());

        // Вычисление среднего времени загрузки
        if self.stats.total_files_loaded > 0 {
            self.stats.avg_load_time_ms =
                self.stats.total_load_time_ms as f32 / self.stats.total_files_loaded as f32;
        }

        info!(
            "Loading complete: {} loaded, {} failed, {:.2} MB total",
            loaded,
            failed,
            self.stats.total_data_size_bytes as f32 / (1024.0 * 1024.0)
        );

        self.get_progress()
    }

    /// Внутренняя загрузка ресурса
    fn load_resource_internal(
        &self,
        path: &str,
        full_path: &Path,
        resource_type: &ResourceType,
    ) -> Result<u64, String> {
        // Проверка существования файла
        if !full_path.exists() {
            return Err("File does not exist".to_string());
        }

        // Получение размера файла
        let metadata =
            std::fs::metadata(full_path).map_err(|e| format!("Failed to read metadata: {}", e))?;

        let size = metadata.len();

        // Имитация загрузки (в реальности здесь будет загрузка конкретного типа)
        // Используем resource_type только для предупреждений
        match resource_type {
            ResourceType::Mesh => {
                // Проверка формата меша
                if !path.ends_with(".obj") && !path.ends_with(".fbx") {
                    warn!("Unknown mesh format: {}", path);
                }
            }
            ResourceType::Texture => {
                // Проверка формата текстуры
                if !path.ends_with(".png") && !path.ends_with(".jpg") && !path.ends_with(".dds") {
                    warn!("Unknown texture format: {}", path);
                }
            }
            ResourceType::Shader => {
                // Проверка формата шейдера
                if !path.ends_with(".glsl") && !path.ends_with(".vert") && !path.ends_with(".frag")
                {
                    warn!("Unknown shader format: {}", path);
                }
            }
            _ => {}
        }

        Ok(size)
    }

    /// Получить текущий прогресс загрузки
    pub fn get_progress(&self) -> LoadingProgress {
        let total = self.resources.len();
        let loaded = self
            .resources
            .values()
            .filter(|r| r.status == LoadStatus::Loaded)
            .count();
        let failed = self
            .resources
            .values()
            .filter(|r| matches!(r.status, LoadStatus::Failed(_) | LoadStatus::NotFound))
            .count();

        let progress = if total > 0 {
            loaded as f32 / total as f32
        } else {
            0.0
        };

        // Расчёт ETA
        let eta_seconds = if let Some(start) = self.start_time {
            if progress > 0.0 && progress < 1.0 {
                let elapsed = start.elapsed().as_secs_f32();
                let total_estimated = elapsed / progress;
                Some(total_estimated - elapsed)
            } else {
                None
            }
        } else {
            None
        };

        let current_stage = match self.state {
            LoadingState::Idle => "Ожидание".to_string(),
            LoadingState::Checking => "Проверка файлов".to_string(),
            LoadingState::Loading => format!("Загрузка: {}/{}", loaded, total),
            LoadingState::Complete => "Завершено".to_string(),
            LoadingState::Failed => "Ошибка".to_string(),
        };

        LoadingProgress {
            total_resources: total,
            loaded_resources: loaded,
            failed_resources: failed,
            current_stage,
            progress,
            eta_seconds,
        }
    }

    /// Получить состояние загрузки
    pub fn state(&self) -> LoadingState {
        self.state.clone()
    }

    /// Получить статистику загрузки
    pub fn stats(&self) -> &LoadingStats {
        &self.stats
    }

    /// Получить список всех ресурсов
    pub fn get_all_resources(&self) -> &HashMap<String, LoadableResource> {
        &self.resources
    }

    /// Получить только загруженные ресурсы
    pub fn get_loaded_resources(&self) -> Vec<&LoadableResource> {
        self.resources
            .values()
            .filter(|r| r.status == LoadStatus::Loaded)
            .collect()
    }

    /// Получить ресурсы с ошибкой загрузки
    pub fn get_failed_resources(&self) -> Vec<&LoadableResource> {
        self.resources
            .values()
            .filter(|r| matches!(r.status, LoadStatus::Failed(_) | LoadStatus::NotFound))
            .collect()
    }

    /// Проверить, загружен ли конкретный ресурс
    pub fn is_resource_loaded(&self, path: &str) -> bool {
        self.resources
            .get(path)
            .map(|r| r.status == LoadStatus::Loaded)
            .unwrap_or(false)
    }

    /// Увеличить счётчик ссылок на ресурс
    pub fn add_ref(&mut self, path: &str) {
        if let Some(resource) = self.resources.get_mut(path) {
            resource.ref_count += 1;
        }
    }

    /// Уменьшить счётчик ссылок на ресурс
    pub fn release_ref(&mut self, path: &str) {
        if let Some(resource) = self.resources.get_mut(path) {
            if resource.ref_count > 0 {
                resource.ref_count -= 1;
            }
        }
    }

    /// Получить ресурсы, которые не используются (ref_count == 0)
    pub fn get_unused_resources(&self) -> Vec<&LoadableResource> {
        self.resources
            .values()
            .filter(|r| r.ref_count == 0 && r.status == LoadStatus::Loaded)
            .collect()
    }

    /// Выгрузить неиспользуемые ресурсы
    pub fn unload_unused_resources(&mut self) -> usize {
        // Сначала собираем пути неиспользуемых ресурсов
        let unused_paths: Vec<String> = self
            .resources
            .iter()
            .filter(|(_, r)| r.ref_count == 0 && r.status == LoadStatus::Loaded)
            .map(|(path, _)| path.clone())
            .collect();

        let count = unused_paths.len();

        // Теперь выгружаем по путям
        for path in unused_paths {
            if let Some(resource) = self.resources.get_mut(&path) {
                info!("Unloading unused resource: {}", resource.path);
                resource.status = LoadStatus::Pending;
                resource.ref_count = 0;
            }
        }

        count
    }

    /// Сбросить состояние менеджера
    pub fn reset(&mut self) {
        self.resources.clear();
        self.load_queue.clear();
        self.state = LoadingState::Idle;
        self.start_time = None;
        self.end_time = None;
        self.stats = LoadingStats::default();
    }
}

/// Глобальный экземпляр LoadingManager (для доступа из любого места)
use once_cell::sync::Lazy;

static GLOBAL_LOADING_MANAGER: Lazy<Arc<Mutex<LoadingManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(LoadingManager::new("assets"))));

/// Получить глобальный менеджер загрузки
pub fn get_global_loading_manager() -> Arc<Mutex<LoadingManager>> {
    GLOBAL_LOADING_MANAGER.clone()
}

/// Быстрое добавление ресурса в глобальный менеджер
pub fn register_resource(path: &str, resource_type: ResourceType, priority: u8) {
    if let Ok(mut manager) = GLOBAL_LOADING_MANAGER.lock() {
        manager.add_resource(path, resource_type, priority);
    }
}

/// Быстрая проверка загрузки ресурса
pub fn is_resource_loaded(path: &str) -> bool {
    if let Ok(manager) = GLOBAL_LOADING_MANAGER.lock() {
        manager.is_resource_loaded(path)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_manager_creation() {
        let manager = LoadingManager::new("assets");
        assert_eq!(manager.state(), LoadingState::Idle);
    }

    #[test]
    fn test_add_resource() {
        let mut manager = LoadingManager::new("assets");
        manager.add_resource("test.obj", ResourceType::Mesh, 1);
        assert_eq!(manager.resources.len(), 1);
    }

    #[test]
    fn test_loading_progress() {
        let mut manager = LoadingManager::new("assets");
        manager.add_resource("test.obj", ResourceType::Mesh, 1);

        let progress = manager.get_progress();
        assert_eq!(progress.total_resources, 1);
        assert_eq!(progress.loaded_resources, 0);
        assert_eq!(progress.progress, 0.0);
    }
}

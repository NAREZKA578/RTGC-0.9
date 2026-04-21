//! Core - Центральный управляющий модуль приложения
//!
//! Этот файл является точкой входа и координации всех подсистем движка.
//! Он импортирует Engine из модуля engine и предоставляет удобный интерфейс
//! для запуска и управления приложением.

// Импорт основного движка
pub use crate::engine::Engine;

// Ре-экспорт ключевых типов для удобного доступа
pub use crate::engine::EngineState;
pub use crate::engine::MenuState;
pub use crate::engine::PauseReason;

// Ре-экспорт менеджеров
pub use crate::engine::GameLoopManager;
pub use crate::engine::InputManagerWrapper;
pub use crate::engine::PhysicsManager;
pub use crate::engine::RenderManager;
pub use crate::engine::VehicleManager;
pub use crate::engine::WorldManager;

// Ре-экспорт подсистем
pub use crate::engine::EngineSubsystems;
pub use crate::engine::GraphicsSubsystem;
pub use crate::engine::PhysicsSubsystem;
pub use crate::engine::UISubsystem;
pub use crate::engine::WorldSubsystem;

/// Тип результата для операций ядра
pub type CoreResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Центральная функция для создания и запуска движка
///
/// # Пример использования
/// ```rust
/// fn main() -> CoreResult<()> {
///     core::run()?;
///     Ok(())
/// }
/// ```
pub fn run() -> CoreResult<()> {
    // Инициализация логгера с проверкой на повторную инициализацию
    init_logger_safe();

    // Создание движка
    let mut engine = Engine::new()?;

    // Запуск игрового цикла
    engine.run()?;

    Ok(())
}

/// Безопасная инициализация логгера с защитой от повторной инициализации
fn init_logger_safe() {
    use std::fs::OpenOptions;
    use std::io::BufWriter;
    use std::sync::Once;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        // Open log file for writing
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("rtgc.log");

        if let Ok(file) = file {
            // Use file writer
            let writer = BufWriter::new(file);
            let file_layer = fmt::layer()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(writer));

            tracing_subscriber::registry()
                .with(filter)
                .with(file_layer)
                .init();
        } else {
            // Fallback to console only
            fmt()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_level(true)
                .with_timer(fmt::time::SystemTime::default())
                .with_env_filter(filter)
                .init();
        }
    });
}

/// Создание нового экземпляра движка
///
/// # Возвращает
/// * `CoreResult<Engine>` - Успешно созданный движок или ошибка
pub fn create_engine() -> CoreResult<Engine> {
    Engine::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_exports() {
        // Проверяем, что все ключевые типы доступны
        let _ = std::any::type_name::<Engine>();
        let _ = std::any::type_name::<EngineState>();
        let _ = std::any::type_name::<PhysicsManager>();
        let _ = std::any::type_name::<WorldManager>();
        let _ = std::any::type_name::<VehicleManager>();
    }
}

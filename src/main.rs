// RTGC-0.8 Main Entry Point - Simple engine runner
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rtgc::core_api;
use tracing::{error, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("========================================");
    eprintln!("RTGC Starting... v0.8.0");
    eprintln!("========================================");

    // Set up panic hook for debugging crashes
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("===========================================");
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!(
                "  at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        eprintln!("===========================================");
        eprintln!("Press Enter to exit...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
    }));

    // Запуск движка через центральный модуль core
    match core_api::run() {
        Ok(()) => {
            info!("Engine shutdown successfully");
            Ok(())
        }
        Err(e) => {
            error!("Engine failed with error: {}", e);
            eprintln!("Fatal error: {}", e);
            eprintln!("Press Enter to exit...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            Err(e)
        }
    }
}

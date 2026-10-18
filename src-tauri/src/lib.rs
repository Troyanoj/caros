mod gps;
mod tiles_server;

use std::io::Write;
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_path = std::env::temp_dir().join("caros_debug.log");
    let _log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[CarOS] ERROR abriendo archivo de log {:?}: {}", log_path, e);
            std::process::exit(1);
        }
    };

    println!("[CarOS] === INICIANDO APLICACIÓN ===");
    println!("[CarOS] Log file: {:?}", log_path);

    std::panic::set_hook(Box::new(move |panic_info| {
        let msg = format!("[PANIC] {}\n", panic_info);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .and_then(|mut f| f.write_all(msg.as_bytes()));
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            gps::get_gps_position,
            gps::scan_gps_devices,
            tiles_server::get_tiles_server_status,
            tiles_server::get_backend_status
        ])
        .setup(|app| {
            println!("[CarOS] Setup callback ejecutándo...");

            let _window = app.get_webview_window("main");
            println!(
                "[CarOS] Ventana principal disponible: {:?}",
                _window.is_some()
            );

            tauri::async_runtime::spawn(async move {
                println!("[tiles_server] Iniciando en background...");
                match tiles_server::start_tiles_server().await {
                    Ok(url) => {
                        println!("[tiles_server] Servidor iniciado en {}", url);
                    }
                    Err(e) => {
                        println!("[tiles_server] ERROR crítico: {}", e);
                        println!("[tiles_server] La app continuará sin servidor de tiles");
                        println!("[tiles_server] Inspeccione %TEMP%\\caros_debug.log para detalles");
                    }
                }
            });

            println!("[CarOS] Setup completado, continuando...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

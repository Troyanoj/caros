mod gps;
mod tiles_server;

use std::io::Write;
use tauri::Manager;

fn log_to_file(msg: &str) {
    let log_path = std::env::temp_dir().join("caros_debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(f, "{}", msg);
        let _ = f.flush();
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_path = std::env::temp_dir().join("caros_debug.log");

    let log_file_result = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path);

    match log_file_result {
        Ok(log_file) => {
            let mut writer = std::io::BufWriter::new(log_file);

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let _ = writeln!(writer, "\n=== CAROS STARTUP: {} ===", timestamp);
            let _ = writeln!(writer, "[CarOS] Log file: {:?}", log_path);
            let _ = writer.flush();

            println!("[CarOS] === INICIANDO APLICACIÓN ===");
            println!("[CarOS] Log file: {:?}", log_path);

            let log_path_clone = log_path.clone();
            std::panic::set_hook(Box::new(move |panic_info| {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path_clone)
                    .and_then(|mut f| {
                        writeln!(f, "[PANIC CRITICO] {}", panic_info)?;
                        writeln!(f, "Backtrace: {:?}", std::backtrace::Backtrace::force_capture())?;
                        f.flush()
                    });
            }));

            drop(writer);
        }
        Err(e) => {
            eprintln!("[CarOS] ERROR creando log file {:?}: {}", log_path, e);
            eprintln!("[CarOS] La app continuará sin logging en archivo");

            std::panic::set_hook(Box::new(|panic_info| {
                eprintln!("[PANIC] {}", panic_info);
            }));
        }
    }

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
            println!("[CarOS] Setup callback INICIANDO");
            log_to_file("[CarOS] Setup callback INICIANDO");

            let window_result = app.get_webview_window("main");
            println!(
                "[CarOS] Ventana principal disponible: {:?}",
                window_result.is_some()
            );
            log_to_file(&format!(
                "[CarOS] Ventana principal disponible: {:?}",
                window_result.is_some()
            ));

            tauri::async_runtime::spawn(async move {
                println!("[tiles_server] Spawning background task...");
                log_to_file("[tiles_server] Spawning background task...");
                match tiles_server::start_tiles_server().await {
                    Ok(url) => {
                        println!("[tiles_server] Servidor iniciado en {}", url);
                        log_to_file(&format!("[tiles_server] Server started: {}", url));
                    }
                    Err(e) => {
                        eprintln!("[tiles_server] ERROR critico: {}", e);
                        log_to_file(&format!("[tiles_server] ERROR critico: {}", e));
                    }
                }
            });

            let _ = std::io::stderr().flush();
            println!("[CarOS] Setup completado, continuando...");
            log_to_file("[CarOS] Setup completado");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

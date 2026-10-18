mod gps;
mod tiles_server;

use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("[CarOS] === INICIANDO APLICACIÓN ===");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            gps::get_gps_position,
            gps::scan_gps_devices,
            tiles_server::get_tiles_server_status
        ])
        .setup(|app| {
            println!("[CarOS] Setup callback ejecutándo...");

            let _window = app.get_webview_window("main");
            println!("[CarOS] Ventana principal disponible: {:?}", _window.is_some());

            tauri::async_runtime::spawn(async move {
                println!("[tiles_server] Iniciando en background...");
                match tiles_server::start_tiles_server().await {
                    Ok(url) => {
                        println!("[tiles_server] ✅ Servidor iniciado en {}", url);
                    }
                    Err(e) => {
                        eprintln!("[tiles_server] ❌ ERROR crítico: {}", e);
                        println!("[tiles_server] La app continuará sin servidor de tiles");
                    }
                }
            });

            println!("[CarOS] Setup completado, continuando...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

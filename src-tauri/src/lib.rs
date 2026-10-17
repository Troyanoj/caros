mod gps;
mod tiles_server;

use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            gps::get_gps_position,
            gps::scan_gps_devices,
            tiles_server::get_tiles_server_status
        ])
        .setup(|app| {
            println!("[CarOS] Iniciando aplicación...");

            let window = app.get_webview_window("main");
            if let Some(win) = window {
                println!("[CarOS] Ventana principal creada: {:?}", win.label());
            }

            println!("[CarOS] Iniciando servidor de tiles...");
            tauri::async_runtime::spawn(async move {
                match tiles_server::start_tiles_server().await {
                    Ok(url) => {
                        println!("[CarOS] Servidor de tiles iniciado correctamente en {}", url);
                    }
                    Err(e) => {
                        println!("[CarOS] ERROR: No se pudo iniciar el servidor de tiles: {}", e);
                        println!("[CarOS] La aplicación continuará sin servidor de tiles.");
                        println!("[CarOS] Verifique que el directorio de tiles existe y tiene un archivo .mbtiles");
                    }
                }
            });

            println!("[CarOS] Aplicación iniciada correctamente");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

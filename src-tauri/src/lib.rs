// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod gps;
mod tiles_server;

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
        .setup(|_app| {
            tauri::async_runtime::spawn(async move {
                match tiles_server::start_tiles_server().await {
                    Ok(url) => println!("Tiles server started at {}", url),
                    Err(e) => eprintln!("Failed to start tiles server: {}", e),
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

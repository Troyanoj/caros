use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use lru::LruCache;
use rusqlite::Connection;
use serde_json::json;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TilesState {
    pub db_path: PathBuf,
    pub tiles_dir: PathBuf,
    pub cache: Mutex<LruCache<String, Vec<u8>>>,
    pub mbtiles_found: bool,
}

pub fn get_tiles_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(app_data).join("caros").join("tiles")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("caros")
            .join("tiles")
    }
}

pub fn find_mbtiles_file(tiles_dir: &PathBuf) -> Option<PathBuf> {
    println!("[tiles_server] Buscando archivos MBTiles en: {:?}", tiles_dir);

    if !tiles_dir.exists() {
        println!("[tiles_server] WARNING: El directorio de tiles no existe: {:?}", tiles_dir);
        return None;
    }

    match std::fs::read_dir(tiles_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "mbtiles") {
                    println!("[tiles_server] Archivo MBTiles encontrado: {:?}", path);
                    return Some(path);
                }
            }
            println!("[tiles_server] WARNING: No se encontró archivo .mbtiles en {:?}", tiles_dir);
            None
        }
        Err(e) => {
            println!("[tiles_server] ERROR al leer directorio de tiles: {}", e);
            None
        }
    }
}

pub async fn start_tiles_server() -> Result<String, String> {
    println!("[tiles_server] Iniciando servidor de tiles...");

    let tiles_dir = get_tiles_dir();
    println!("[tiles_server] Directorio de tiles: {:?}", tiles_dir);

    if let Err(e) = std::fs::create_dir_all(&tiles_dir) {
        println!(
            "[tiles_server] ERROR creando directorio de tiles {:?}: {}",
            tiles_dir, e
        );
    } else {
        println!("[tiles_server] Directorio de tiles listo: {:?}", tiles_dir);
    }

    let mbtiles_path = find_mbtiles_file(&tiles_dir);
    let mbtiles_found = mbtiles_path.is_some();

    if !mbtiles_found {
        println!("[tiles_server] WARNING: No se encontró archivo MBTiles");
        println!(
            "[tiles_server] El servidor continuará funcionando pero no podrá servir tiles"
        );
        println!(
            "[tiles_server] Copie un archivo .mbtiles al directorio: {:?}",
            tiles_dir
        );
    }

    let cache = LruCache::new(NonZeroUsize::new(500).unwrap());
    let state = Arc::new(TilesState {
        db_path: mbtiles_path.unwrap_or_default(),
        tiles_dir,
        cache: Mutex::new(cache),
        mbtiles_found,
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/tiles/{z}/{x}/{y}.pbf", get(get_tile))
        .route("/metadata", get(get_metadata))
        .with_state(state.clone());

    let addr = "127.0.0.1:8765";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Error binding to {}: {}", addr, e))?;

    println!("[tiles_server] Servidor escuchando en http://{}", addr);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            println!("[tiles_server] ERROR: Servidor de tiles detenido: {}", e);
        }
    });

    Ok(format!("http://{}", addr))
}

async fn health_check(State(state): State<Arc<TilesState>>) -> impl IntoResponse {
    let status = if state.mbtiles_found { "ok" } else { "error" };
    let response = json!({
        "status": status,
        "mbtiles_found": state.mbtiles_found,
        "tiles_dir": state.tiles_dir.to_string_lossy(),
        "mbtiles_path": if state.mbtiles_found {
            state.db_path.to_string_lossy().to_string()
        } else {
            "".to_string()
        }
    });

    match status {
        "ok" => (StatusCode::OK, Json(response)),
        _ => (StatusCode::SERVICE_UNAVAILABLE, Json(response)),
    }
}

async fn get_tile(
    Path((z, x, y)): Path<(u32, u32, u32)>,
    State(state): State<Arc<TilesState>>,
) -> Response {
    if !state.mbtiles_found {
        println!(
            "[tiles_server] Tile solicitado pero no hay MBTiles: z={}, x={}, y={}",
            z, x, y
        );
        return (
            StatusCode::NOT_FOUND,
            "MBTiles file not found. Please place a .mbtiles file in the tiles directory.",
        )
            .into_response();
    }

    let cache_key = format!("{}/{}/{}", z, x, y);

    {
        let mut cache = state.cache.lock().await;
        if let Some(tile_data) = cache.get(&cache_key) {
            return (
                StatusCode::OK,
                [("content-type", "application/vnd.mapbox-vector-tile")],
                tile_data.clone(),
            )
                .into_response();
        }
    }

    let tile_data = {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] Error abriendo DB: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response();
            }
        };

        let mut stmt = match conn.prepare(
            "SELECT tile_data FROM tiles WHERE zoom_level=? AND tile_column=? AND tile_row=?",
        ) {
            Ok(s) => s,
            Err(e) => {
                println!("[tiles_server] Error preparando SQL: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("SQL error: {}", e))
                    .into_response();
            }
        };

        match stmt.query_row([z as i64, x as i64, y as i64], |row| {
            row.get::<_, Vec<u8>>(0)
        }) {
            Ok(data) => data,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return StatusCode::NO_CONTENT.into_response();
            }
            Err(e) => {
                println!("[tiles_server] Error consultando tile: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {}", e)).into_response();
            }
        }
    };

    {
        let mut cache = state.cache.lock().await;
        cache.put(cache_key, tile_data.clone());
    }

    (
        StatusCode::OK,
        [("content-type", "application/vnd.mapbox-vector-tile")],
        tile_data,
    )
        .into_response()
}

async fn get_metadata(State(state): State<Arc<TilesState>>) -> Response {
    if !state.mbtiles_found {
        let response = json!({
            "error": "MBTiles file not found",
            "tiles_dir": state.tiles_dir.to_string_lossy()
        });
        return (
            StatusCode::NOT_FOUND,
            [("content-type", "application/json")],
            serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
        )
            .into_response();
    }

    let metadata = {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] Error abriendo DB para metadata: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response();
            }
        };

        let mut stmt = match conn.prepare("SELECT name, value FROM metadata") {
            Ok(s) => s,
            Err(e) => {
                println!("[tiles_server] Error preparando metadata SQL: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("SQL error: {}", e))
                    .into_response();
            }
        };

        let mut result = std::collections::HashMap::new();
        let mut rows = match stmt.query([]) {
            Ok(r) => r,
            Err(e) => {
                println!("[tiles_server] Error en query metadata: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {}", e))
                    .into_response();
            }
        };

        while let Ok(Some(row)) = rows.next() {
            if let (Ok(name), Ok(value)) = (row.get::<_, String>(0), row.get::<_, String>(1)) {
                result.insert(name, value);
            }
        }

        result
    };

    let json = serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".to_string());

    (
        StatusCode::OK,
        [("content-type", "application/json")],
        json,
    )
        .into_response()
}

#[tauri::command]
pub fn get_tiles_server_status() -> Result<String, String> {
    let tiles_dir = get_tiles_dir();

    println!("[tiles_server] Estado del servidor solicitado");
    println!("[tiles_server] Directorio de tiles: {:?}", tiles_dir);

    if let Some(mbtiles_path) = find_mbtiles_file(&tiles_dir) {
        println!("[tiles_server] Estado: MBTiles encontrado en {:?}", mbtiles_path);
        Ok(format!("MBTiles found: {:?}", mbtiles_path))
    } else {
        println!("[tiles_server] Estado: No se encontró MBTiles en {:?}", tiles_dir);
        Err(format!("No MBTiles file found in {:?}", tiles_dir))
    }
}

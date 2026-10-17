use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use lru::LruCache;
use rusqlite::Connection;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TilesState {
    pub db_path: PathBuf,
    pub cache: Mutex<LruCache<String, Vec<u8>>>,
}

pub fn get_tiles_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(app_data).join("caros").join("tiles")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local").join("share").join("caros").join("tiles")
    }
}

pub fn find_mbtiles_file(tiles_dir: &PathBuf) -> Option<PathBuf> {
    if tiles_dir.exists() {
        for entry in std::fs::read_dir(tiles_dir).ok()? {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "mbtiles") {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub async fn start_tiles_server() -> Result<String, String> {
    let tiles_dir = get_tiles_dir();
    std::fs::create_dir_all(&tiles_dir).map_err(|e| format!("Error creating tiles dir: {}", e))?;

    let mbtiles_path = find_mbtiles_file(&tiles_dir)
        .ok_or_else(|| format!("No MBTiles file found in {:?}", tiles_dir))?;

    let cache = LruCache::new(NonZeroUsize::new(500).unwrap());
    let state = Arc::new(TilesState {
        db_path: mbtiles_path.clone(),
        cache: Mutex::new(cache),
    });

    let app = Router::new()
        .route("/tiles/{z}/{x}/{y}.pbf", get(get_tile))
        .route("/metadata", get(get_metadata))
        .with_state(state);

    let addr = "127.0.0.1:8765";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Error binding to {}: {}", addr, e))?;

    println!("Tiles server listening on http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(format!("http://{}", addr))
}

async fn get_tile(
    Path((z, x, y)): Path<(u32, u32, u32)>,
    State(state): State<Arc<TilesState>>,
) -> Response {
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
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response();
            }
        };

        let mut stmt = match conn.prepare("SELECT tile_data FROM tiles WHERE zoom_level=? AND tile_column=? AND tile_row=?") {
            Ok(s) => s,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("SQL error: {}", e)).into_response();
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

async fn get_metadata(
    State(state): State<Arc<TilesState>>,
) -> Response {
    let metadata = {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response();
            }
        };

        let mut stmt = match conn.prepare("SELECT name, value FROM metadata") {
            Ok(s) => s,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("SQL error: {}", e)).into_response();
            }
        };

        let mut result = std::collections::HashMap::new();
        let mut rows = match stmt.query([]) {
            Ok(r) => r,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {}", e)).into_response();
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
    if let Some(mbtiles_path) = find_mbtiles_file(&tiles_dir) {
        Ok(format!("MBTiles found: {:?}", mbtiles_path))
    } else {
        Err(format!("No MBTiles file found in {:?}", tiles_dir))
    }
}

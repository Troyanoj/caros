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
    println!("[tiles_server] === INICIANDO SERVIDOR ===");

    let tiles_dir = get_tiles_dir();
    println!("[tiles_server] Directorio de tiles: {:?}", tiles_dir);

    std::fs::create_dir_all(&tiles_dir).map_err(|e| {
        println!("[tiles_server] ERROR creando directorio: {}", e);
        format!("Error creando directorio: {}", e)
    })?;
    println!("[tiles_server] Directorio verificado/creado");

    let mbtiles_path = find_mbtiles_file(&tiles_dir).ok_or_else(|| {
        let msg = format!("No se encontró archivo .mbtiles en {:?}", tiles_dir);
        println!("[tiles_server] ERROR: {}", msg);
        msg
    })?;

    println!("[tiles_server] Archivo MBTiles encontrado: {:?}", mbtiles_path);
    println!("[tiles_server] Intentando abrir conexión SQLite para validación...");

    let (_tables, _tile_count) = {
        let conn = match Connection::open(&mbtiles_path) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] ERROR abriendo SQLite: {}", e);
                println!("[tiles_server] Detalles del error: {:?}", e);
                return Err(format!("Error abriendo base de datos: {}", e));
            }
        };
        println!("[tiles_server] Conexión SQLite exitosa");

        println!("[tiles_server] Verificando esquema de base de datos...");
        let mut stmt = match conn.prepare("SELECT name FROM sqlite_master WHERE type='table'") {
            Ok(s) => s,
            Err(e) => {
                println!("[tiles_server] ERROR consultando tablas: {}", e);
                return Err(format!("Error consultando esquema: {}", e));
            }
        };

        let tables: Vec<String> = match stmt.query_map([], |row| row.get(0)) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                println!("[tiles_server] ERROR mapeando tablas: {}", e);
                return Err(format!("Error mapeando tablas: {}", e));
            }
        };

        println!("[tiles_server] Tablas encontradas: {:?}", tables);

        if !tables.contains(&"tiles".to_string()) {
            println!("[tiles_server] ERROR CRITICO: La tabla 'tiles' no existe");
            return Err("El archivo MBTiles no tiene la tabla 'tiles'".to_string());
        }

        if !tables.contains(&"metadata".to_string()) {
            println!("[tiles_server] WARNING: La tabla 'metadata' no existe");
        }

        let tile_count: i64 = match conn.query_row("SELECT COUNT(*) FROM tiles", [], |row| row.get(0)) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] ERROR contando tiles: {}", e);
                0
            }
        };

        println!("[tiles_server] Total de tiles en la base de datos: {}", tile_count);

        if tile_count == 0 {
            println!("[tiles_server] WARNING: El archivo MBTiles no tiene tiles");
        }

        drop(stmt);
        drop(conn);

        (tables, tile_count)
    };

    println!("[tiles_server] Configurando cache LRU...");
    let cache = LruCache::new(NonZeroUsize::new(500).unwrap());
    let state = Arc::new(TilesState {
        db_path: mbtiles_path.clone(),
        tiles_dir,
        cache: Mutex::new(cache),
        mbtiles_found: true,
    });

    println!("[tiles_server] Creando router Axum...");
    let app = Router::new()
        .route("/ping", get(ping))
        .route("/health", get(health_check))
        .route("/debug-db", get(debug_db))
        .route("/tiles/{z}/{x}/{y}.pbf", get(get_tile))
        .route("/metadata", get(get_metadata))
        .with_state(state);

    let addr = "127.0.0.1:8765";
    println!("[tiles_server] Intentando bind en {}...", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| {
            println!("[tiles_server] ERROR binding to {}: {}", addr, e);
            format!("Error binding to {}: {}", addr, e)
        })?;

    println!("[tiles_server] Servidor escuchando en http://{}", addr);

    tokio::spawn(async move {
        println!("[tiles_server] Spawned server task");
        if let Err(e) = axum::serve(listener, app).await {
            println!("[tiles_server] ERROR: Servidor de tiles detenido: {}", e);
        }
    });

    Ok(format!("http://{}", addr))
}

async fn ping() -> &'static str {
    println!("[tiles_server] PING recibido - servidor está vivo");
    "PONG - Servidor de tiles CarOS OK"
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

async fn debug_db(State(state): State<Arc<TilesState>>) -> impl IntoResponse {
    println!("[tiles_server] /debug-db solicidado");

    if !state.mbtiles_found {
        let response = json!({
            "error": "MBTiles file not found",
            "tiles_dir": state.tiles_dir.to_string_lossy()
        });
        return (StatusCode::NOT_FOUND, Json(response)).into_response();
    }

    let result = {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] ERROR abriendo DB para debug: {}", e);
                let response = json!({ "error": format!("DB error: {}", e) });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        let tables: Vec<String> = match conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name") {
            Ok(mut stmt) => match stmt.query_map([], |row| row.get(0)) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    println!("[tiles_server] ERROR lista tablas: {}", e);
                    let response = json!({ "error": format!("Tables error: {}", e) });
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            },
            Err(e) => {
                println!("[tiles_server] ERROR preparando tabla: {}", e);
                let response = json!({ "error": format!("Tables prepare error: {}", e) });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
            }
        };

        let tile_count: i64 = match conn.query_row("SELECT COUNT(*) FROM tiles", [], |row| row.get(0)) {
            Ok(c) => c,
            Err(e) => {
                println!("[tiles_server] ERROR contando tiles: {}", e);
                0
            }
        };

        let mut metadata = std::collections::HashMap::new();
        if tables.contains(&"metadata".to_string()) {
            if let Ok(mut stmt) = conn.prepare("SELECT name, value FROM metadata") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }) {
                    for r in rows.filter_map(|x| x.ok()) {
                        metadata.insert(r.0, r.1);
                    }
                }
            }
        }

        let mut sample_tiles: Vec<serde_json::Value> = Vec::new();
        if tables.contains(&"tiles".to_string()) {
            if let Ok(mut stmt) = conn.prepare("SELECT zoom_level, tile_column, tile_row, LENGTH(tile_data) FROM tiles LIMIT 10") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok(json!({
                        "zoom": row.get::<_, i64>(0)?,
                        "x": row.get::<_, i64>(1)?,
                        "y": row.get::<_, i64>(2)?,
                        "size": row.get::<_, i64>(3)?
                    }))
                }) {
                    for r in rows.filter_map(|x| x.ok()) {
                        sample_tiles.push(r);
                    }
                }
            }
        }

        drop(conn);

        json!({
            "tables": tables,
            "tile_count": tile_count,
            "metadata": metadata,
            "sample_tiles": sample_tiles
        })
    };

    (StatusCode::OK, Json(result)).into_response()
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

#[derive(serde::Serialize)]
pub struct BackendStatus {
    pub tiles_dir: String,
    pub tiles_dir_exists: bool,
    pub mbtiles_found: bool,
    pub mbtiles_path: String,
    pub mbtiles_exists: bool,
    pub appdata: String,
    pub port: String,
    pub all_mbtiles_files: Vec<String>,
}

#[tauri::command]
pub fn get_backend_status() -> BackendStatus {
    println!("[tiles_server] get_backend_status solicitado");

    let tiles_dir = get_tiles_dir();
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "UNKNOWN".to_string());

    let tiles_dir_exists = tiles_dir.exists();
    println!("[tiles_server] Directorio de tiles existe: {}", tiles_dir_exists);
    println!("[tiles_server] Ruta del directorio: {:?}", tiles_dir);

    let (mbtiles_found, mbtiles_path, all_mbtiles_files) = if tiles_dir_exists {
        match std::fs::read_dir(&tiles_dir) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let p = e.path();
                        if p.extension().map_or(false, |ext| ext == "mbtiles") {
                            Some(p.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                let found = !files.is_empty();
                let first_path = files.first().cloned().unwrap_or_default();
                println!("[tiles_server] Archivos MBTiles encontrados: {:?}", files);
                (found, first_path, files)
            }
            Err(e) => {
                println!("[tiles_server] Error leyendo directorio: {}", e);
                (false, "".to_string(), vec![])
            }
        }
    } else {
        println!("[tiles_server] El directorio de tiles NO existe");
        (false, "".to_string(), vec![])
    };

    let status = BackendStatus {
        tiles_dir: tiles_dir.to_string_lossy().to_string(),
        tiles_dir_exists,
        mbtiles_found,
        mbtiles_path,
        mbtiles_exists: mbtiles_found,
        appdata,
        port: "127.0.0.1:8765".to_string(),
        all_mbtiles_files,
    };

    println!(
        "[tiles_server] Status: tiles_dir_exists={}, mbtiles_found={}, appdata={}",
        tiles_dir_exists, mbtiles_found, status.appdata
    );

    status
}

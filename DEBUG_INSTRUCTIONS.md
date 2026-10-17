# CarOS · Instrucciones de Debug (FASE 5.1)

Este documento ayuda a diagnosticar problemas en CarOS, particularmente el **Error 500** que ocurre cuando la aplicación no encuentra los archivos de mapas.

## 1. Acceder a DevTools de la Aplicación

La aplicación CarOS es una aplicación Tauri, lo que significa que usa un motor web incrustado (Webview). Puedes acceder a las DevTools de la siguiente manera:

### Método 1: Teclado
1. Enciende la Surface Go 3 y abre CarOS
2. Presiona **F12** o **Ctrl+Shift+I** (puede requerir una tecla física si usas teclado Bluetooth)
3. Se abrirá la ventana de herramientas de desarrollo

### Método 2: Click derecho (si hay teclado conectado)
1. Haz click derecho en cualquier parte de la aplicación
2. Selecciona "Inspeccionar elemento"

## 2. Revisar la Consola del Frontend

En DevTools, ve a la pestaña **Console**:
- Busca errores en rojo relacionados con `maplibre-gl` o carga de tiles
- Los mensajes típicos de error incluyen:
  - `Failed to load resource: net::ERR_CONNECTION_REFUSED` → El servidor de tiles no está corriendo
  - `Network error: 500 Internal Server Error` → Error en el servidor Rust
  - `Tile loading failed` → El archivo MBTiles no existe o está corrupto

## 3. Reviar la Consola de Rust (backend)

Los logs de Rust (`println!`) se muestran en la **salida estándar/stderr**. Dependiendo de cómo se inició la aplicación:

### Si la app fue lanzada desde el instalador:
- Los logs de Rust **no son visibles** por defecto (la app está empaquetada como MSI)
- Para ver los logs, necesitas ejecutar la app en modo desarrollo usando `tauri dev`

### Si ejecutas en modo desarrollo:
```bash
cd caros
npm run tauri dev
```
Los logs aparecerán en la terminal con el prefijo `[tiles_server]`, `[gps]`, o `[CarOS]`.

## 4. Verificar el Archivo MBTiles

### Verificar existencia del archivo:
1. Abre **Explorador de archivos** en la Surface Go 3
2. Navega a esta ruta:
   ```
   %APPDATA%\caros\tiles\
   ```
   - Equivalente típico: `C:\Users\<TuUsuario>\AppData\Roaming\caros\tiles\`
3. Verifica que existe un archivo con extensión `.mbtiles`:
   ```
   ✅ thailand.mbtiles    (debe estar aquí)
   ```

### Si el archivo no existe:
1. Copia `thailand.mbtiles` desde el PC de desarrollo a la carpeta anterior
2. El archivo debe quedar en:
   ```
   C:\Users\<TuUsuario>\AppData\Roaming\caros\tiles\thailand.mbtiles
   ```

## 5. Probar el Endpoint de Health Check

Puedes verificar que el servidor de tiles está funcionando correctamente:

1. En la Surface Go 3, abre PowerShell
2. Ejecuta:
   ```powershell
   curl http://127.0.0.1:8765/health
   ```
3. Si todo funciona, deberías ver:
   ```json
   {"status":"ok","mbtiles_found":true,"tiles_dir":"C:\\Users\\<Usuario>\\AppData\\Roaming\\caros\\tiles",...}
   ```

## 6. Mensajes de Log Clave

Busca estos mensajes en la consola de Rust:

| Mensaje | Significado | Acción |
|---------|-------------|--------|
| `[tiles_server] Iniciando servidor de tiles...` | El servidor se está iniciando | Esperar |
| `[tiles_server] Archivo MBTiles encontrado: ...` | MBTiles detectado correctamente | OK |
| `[tiles_server] WARNING: No se encontró archivo .mbtiles` | El archivo falta | Copiar MBTiles |
| `[tiles_server] Servidor escuchando en http://127.0.0.1:8765` | Servidor listo | OK |
| `[CarOS] ERROR: No se pudo iniciar el servidor de tiles` | Error crítico | Ver logs detallados |
| `[gps] No se encontraron puertos GPS` | GPS no conectado | Verificar conexión serial |

## 7. Solución de Problemas Comunes

### Error 500 — No se encuentra el archivo MBTiles
1. Verifica que `thailand.mbtiles` esté en `%APPDATA%\caros\tiles\`
2. Reinicia la aplicación
3. Verifica el endpoint `/health`

### El mapa se carga pero no muestra tiles
1. Verifica el endpoint `/health` retorna `"status":"ok"`
2. Verifica que el archivo MBTiles no esté corrupto:
   ```powershell
   # Instala sqlite3 si no está disponible
   npm install -g sqlite3
   sqlite3 "%APPDATA%\caros\tiles\thailand.mbtiles" "SELECT COUNT(*) FROM tiles;"
   ```
3. Si la consulta falla, el archivo está corrupto. Descarga uno nuevo.

### El marcador GPS no se mueve
1. Verifica que haya un dispositivo GPS conectado
2. En la Surface, abre Device Manager y verifica puertos COM
3. Los logs mostrarán `[gps] Escaneando puertos seriales...` y `[gps] Puertos encontrados: [...]`
4. Si no encuentra puertos, verifica la conexión USB del GPS

## 8. Contacto para Soporte

Si los problemas persisten después de seguir estas instrucciones, contacta al equipo de desarrollo con:
- Captura de la consola de DevTools (pestaña Console)
- Copia de los logs de Rust
- Captura del endpoint `/health`

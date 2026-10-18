# CarOS Debug Log Viewer
# Ejecute este script desde PowerShell para ver los logs del backend Rust
# Uso: .\scripts\view-logs.ps1

$logPath = "$env:TEMP\caros_debug.log"

Write-Host ""
Write-Host "=== CAROS DEBUG LOG VIEWER ===" -ForegroundColor Cyan
Write-Host ""

if (Test-Path $logPath) {
    Write-Host "Archivo de log encontrado en: $logPath" -ForegroundColor Green
    Write-Host "Tamaño: $((Get-Item $logPath).Length / 1024) KB" -ForegroundColor Gray
    Write-Host ""
    Write-Host "--- Últimas 50 líneas del log ---" -ForegroundColor Yellow
    Write-Host ""
    Get-Content $logPath -Tail 50 | ForEach-Object {
        if ($_ -match "ERROR|❌|CRITICO") {
            Write-Host $_ -ForegroundColor Red
        } elseif ($_ -match "WARNING|⚠️") {
            Write-Host $_ -ForegroundColor Yellow
        } elseif ($_ -match "✅|OK") {
            Write-Host $_ -ForegroundColor Green
        } else {
            Write-Host $_ -ForegroundColor Gray
        }
    }
    Write-Host ""
    Write-Host "--- Fin del log ---" -ForegroundColor Yellow
} else {
    Write-Host "❌ No se encontró el archivo de log en: $logPath" -ForegroundColor Red
    Write-Host ""
    Write-Host "Posibles causas:" -ForegroundColor Yellow
    Write-Host "  1. La aplicación CarOS no se ha abierto aún" -ForegroundColor Gray
    Write-Host "  2. La aplicación no pudo crear el archivo de log" -ForegroundColor Gray
    Write-Host "  3. El directorio TEMP no es accesible" -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== COMANDOS ÚTILES ===" -ForegroundColor Cyan
Write-Host "  Ver todos los logs:  Get-Content '$logPath'" -ForegroundColor Gray
Write-Host "  Ver logs en vivo:    Get-Content '$logPath' -Wait" -ForegroundColor Gray
Write-Host "  Limpiar logs:        Remove-Item '$logPath'" -ForegroundColor Gray
Write-Host ""

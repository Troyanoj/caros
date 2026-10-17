use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsData {
    pub latitude: f64,
    pub longitude: f64,
    pub speed_kmh: f32,
    pub altitude_m: f32,
    pub valid: bool,
    pub timestamp: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsDevice {
    pub port_name: String,
    pub vendor: String,
}

pub fn scan_serial_ports() -> Vec<GpsDevice> {
    println!("[gps] Escaneando puertos seriales...");
    let ports = serialport::available_ports().unwrap_or_default();

    let filtered: Vec<_> = ports
        .into_iter()
        .filter(|p| {
            p.port_name.contains("ttyUSB")
                || p.port_name.contains("ttyACM")
                || p.port_name.contains("COM")
        })
        .collect();

    let port_names: Vec<_> = filtered.iter().map(|p| &p.port_name).collect();
    println!("[gps] Puertos encontrados: {:?}", port_names);

    filtered
        .into_iter()
        .map(|p| GpsDevice {
            port_name: p.port_name,
            vendor: "Unknown".to_string(),
        })
        .collect()
}

pub fn parse_nmea_sentence(sentence: &str) -> Option<GpsData> {
    let sentence = sentence.trim();

    if !sentence.starts_with('$') {
        return None;
    }

    if let Some(checksum_pos) = sentence.find('*') {
        let data = &sentence[1..checksum_pos];
        let expected_checksum = u8::from_str_radix(&sentence[checksum_pos + 1..], 16).ok()?;
        let calculated_checksum = data.bytes().fold(0u8, |acc, b| acc ^ b);

        if calculated_checksum != expected_checksum {
            return None;
        }
    }

    let parts: Vec<&str> = sentence.split(',').collect();

    match parts[0] {
        "$GPRMC" | "$GNRMC" => parse_gprmc(&parts),
        "$GPGGA" | "$GNGGA" => parse_gpgga(&parts),
        _ => None,
    }
}

fn parse_gprmc(parts: &[&str]) -> Option<GpsData> {
    if parts.len() < 12 {
        return None;
    }

    let status = parts[2];
    if status != "A" {
        return Some(GpsData {
            latitude: 0.0,
            longitude: 0.0,
            speed_kmh: 0.0,
            altitude_m: 0.0,
            valid: false,
            timestamp: parts[1].to_string(),
            source: "serial".to_string(),
        });
    }

    let lat = parse_nmea_coordinate(parts[3], parts[4])?;
    let lon = parse_nmea_coordinate(parts[5], parts[6])?;
    let speed_knots: f32 = parts[7].parse().ok()?;
    let speed_kmh = speed_knots * 1.852;

    Some(GpsData {
        latitude: lat,
        longitude: lon,
        speed_kmh,
        altitude_m: 0.0,
        valid: true,
        timestamp: parts[1].to_string(),
        source: "serial".to_string(),
    })
}

fn parse_gpgga(parts: &[&str]) -> Option<GpsData> {
    if parts.len() < 10 {
        return None;
    }

    let lat = parse_nmea_coordinate(parts[2], parts[3])?;
    let lon = parse_nmea_coordinate(parts[4], parts[5])?;
    let altitude: f32 = parts[9].parse().ok()?;

    Some(GpsData {
        latitude: lat,
        longitude: lon,
        speed_kmh: 0.0,
        altitude_m: altitude,
        valid: true,
        timestamp: parts[1].to_string(),
        source: "serial".to_string(),
    })
}

fn parse_nmea_coordinate(value: &str, hemisphere: &str) -> Option<f64> {
    if value.len() < 4 {
        return None;
    }

    let dot_pos = value.find('.')?;
    let degrees: f64 = value[..dot_pos - 2].parse().ok()?;
    let minutes: f64 = value[dot_pos - 2..].parse().ok()?;

    let decimal = degrees + minutes / 60.0;

    match hemisphere {
        "S" | "W" => Some(-decimal),
        "N" | "E" => Some(decimal),
        _ => None,
    }
}

pub fn read_gps_from_port(port_name: &str) -> Result<GpsData, String> {
    println!("[gps] Leyendo GPS desde puerto: {}", port_name);

    let port = serialport::new(port_name, 4800)
        .timeout(Duration::from_secs(5))
        .open()
        .map_err(|e| format!("Error opening port {}: {}", port_name, e))?;

    let mut reader = BufReader::new(port);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Err("EOF reached".to_string()),
            Ok(_) => {
                if let Some(gps_data) = parse_nmea_sentence(&line) {
                    if gps_data.valid {
                        println!(
                            "[gps] GPS válido recibido: lat={}, lng={}",
                            gps_data.latitude, gps_data.longitude
                        );
                        return Ok(gps_data);
                    }
                }
            }
            Err(e) => return Err(format!("Read error: {}", e)),
        }
    }
}

#[tauri::command]
pub fn get_gps_position() -> Result<GpsData, String> {
    println!("[gps] Solicitada posición GPS actual");
    let ports = scan_serial_ports();

    if ports.is_empty() {
        println!("[gps] No se encontraron puertos GPS");
    }

    for port in ports {
        println!("[gps] Intentando leer del puerto: {}", port.port_name);
        if let Ok(gps_data) = read_gps_from_port(&port.port_name) {
            if gps_data.valid {
                println!(
                    "[gps] GPS válido: lat={}, lng={}, speed={}km/h",
                    gps_data.latitude, gps_data.longitude, gps_data.speed_kmh
                );
                return Ok(gps_data);
            }
        }
    }

    println!("[gps] No se obtuvo datos GPS válidos de ningún puerto");
    Err("No valid GPS data from any serial port".to_string())
}

#[tauri::command]
pub fn scan_gps_devices() -> Result<Vec<GpsDevice>, String> {
    Ok(scan_serial_ports())
}

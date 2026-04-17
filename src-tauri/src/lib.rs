use base64::{Engine as _, engine::general_purpose};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

// ---- Auth verify ----

#[derive(Serialize, Deserialize)]
struct VerifyResult {
    email: String,
    tier: String,
}

#[tauri::command]
async fn verify_license(server_url: String, api_key: String) -> Result<VerifyResult, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/auth/verify", server_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    match resp.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err("401: Ogiltig API-nyckel".to_string()),
        reqwest::StatusCode::PAYMENT_REQUIRED => {
            return Err("402: Prenumerationen har gått ut".to_string())
        }
        s if !s.is_success() => return Err(format!("Serverfel: {}", s)),
        _ => {}
    }

    resp.json::<VerifyResult>().await.map_err(|e| e.to_string())
}

// ---- Manifest check ----

#[derive(Deserialize)]
struct ManifestFileEntry {
    name: String,
    size: i64,
    sha256: String,
}

#[derive(Deserialize)]
struct ManifestResponse {
    files: HashMap<String, ManifestFileEntry>,
}

#[derive(Serialize)]
struct DbFileInfo {
    name: String,
    size: i64,
    sha256: String,
}

#[derive(Serialize)]
struct ManifestCheckResult {
    needs_update: bool,
    file: Option<DbFileInfo>,
    etag: String,
}

fn tier_to_manifest_key(tier: &str) -> &str {
    if tier == "desktop" {
        "pro"
    } else {
        tier
    }
}

#[tauri::command]
async fn check_manifest(
    server_url: String,
    tier: String,
    current_etag: String,
    current_sha256: String,
) -> Result<ManifestCheckResult, String> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/manifest", server_url));
    if !current_etag.is_empty() {
        req = req.header("If-None-Match", current_etag.clone());
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;

    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(ManifestCheckResult {
            needs_update: false,
            file: None,
            etag: current_etag,
        });
    }

    if !resp.status().is_success() {
        return Err(format!("Serverfel: {}", resp.status()));
    }

    let etag = resp
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let manifest: ManifestResponse = resp.json().await.map_err(|e| e.to_string())?;
    let key = tier_to_manifest_key(&tier);
    let entry = manifest
        .files
        .get(key)
        .ok_or_else(|| format!("Ingen fil tillgänglig för tier '{}'", tier))?;

    let needs_update = entry.sha256 != current_sha256;

    Ok(ManifestCheckResult {
        needs_update,
        file: if needs_update {
            Some(DbFileInfo {
                name: entry.name.clone(),
                size: entry.size,
                sha256: entry.sha256.clone(),
            })
        } else {
            None
        },
        etag,
    })
}

// ---- Download ----

#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

#[tauri::command]
async fn download_db(
    app: AppHandle,
    server_url: String,
    api_key: String,
    expected_sha256: String,
    file_name: String,
) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&data_dir)
        .await
        .map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/download", server_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    match resp.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err("401: Ogiltig API-nyckel".to_string()),
        reqwest::StatusCode::PAYMENT_REQUIRED => {
            return Err("402: Prenumerationen har gått ut".to_string())
        }
        s if !s.is_success() => return Err(format!("Serverfel: {}", s)),
        _ => {}
    }

    let total = resp.content_length().unwrap_or(0);
    let temp_path = data_dir.join("download.tmp");
    let mut out_file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut downloaded = 0u64;
    let mut hasher = sha2::Sha256::new();
    let mut response = resp;

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        out_file
            .write_all(&chunk)
            .await
            .map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        let _ = app.emit(
            "download-progress",
            DownloadProgress { downloaded, total },
        );
    }

    let computed = format!("{:x}", hasher.finalize());
    if computed != expected_sha256 {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err("SHA256-kontroll misslyckades – filen kan vara skadad".to_string());
    }

    let final_path = if file_name.ends_with(".zip") {
        let out_path = data_dir.join("foretagsdatabasen.sqlite");
        let tmp = temp_path.clone();
        let out = out_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let f = std::fs::File::open(&tmp).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
                let name = entry.name().to_string();
                if name.ends_with(".sqlite") || name.ends_with(".db") {
                    let mut dst =
                        std::fs::File::create(&out).map_err(|e| e.to_string())?;
                    std::io::copy(&mut entry, &mut dst).map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            Err("Ingen .sqlite-fil hittades i zip-arkivet".to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        let _ = tokio::fs::remove_file(&temp_path).await;
        out_path
    } else {
        let out_path = data_dir.join("foretagsdatabasen.sqlite");
        let _ = tokio::fs::remove_file(&out_path).await;
        tokio::fs::rename(&temp_path, &out_path)
            .await
            .map_err(|e| e.to_string())?;
        out_path
    };

    final_path
        .to_str()
        .ok_or_else(|| "Ogiltig sökväg".to_string())
        .map(|s| s.to_string())
}

// ---- Schema ----

#[tauri::command]
async fn get_schema(db_path: String) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .map_err(|e| e.to_string())?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut schema = std::collections::HashMap::new();
        for table in tables {
            let mut col_stmt = conn
                .prepare(&format!("PRAGMA table_info(\"{}\")", table))
                .map_err(|e| e.to_string())?;
            let cols: Vec<String> = col_stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            schema.insert(table, cols);
        }
        Ok(schema)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---- Query DB ----

#[derive(Serialize)]
struct QueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
    truncated: bool,
}

const MAX_ROWS: usize = 50_000;

#[tauri::command]
async fn query_db(db_path: String, sql: String) -> Result<QueryResult, String> {
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let columns: Vec<String> = stmt.column_names().iter().map(|&s| s.to_string()).collect();
        let col_count = columns.len();

        let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut raw = stmt.query([]).map_err(|e| e.to_string())?;
        let mut truncated = false;

        while let Some(row) = raw.next().map_err(|e| e.to_string())? {
            if rows.len() >= MAX_ROWS {
                truncated = true;
                break;
            }
            let mut values = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val = match row.get_ref(i).map_err(|e| e.to_string())? {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(n) => serde_json::json!(n),
                    rusqlite::types::ValueRef::Real(f) => {
                        serde_json::Number::from_f64(f)
                            .map(serde_json::Value::Number)
                            .unwrap_or_else(|| serde_json::Value::String(f.to_string()))
                    }
                    rusqlite::types::ValueRef::Text(s) => {
                        serde_json::Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    rusqlite::types::ValueRef::Blob(b) => {
                        serde_json::Value::String(format!("<blob {} bytes>", b.len()))
                    }
                };
                values.push(val);
            }
            rows.push(values);
        }

        Ok(QueryResult { columns, rows, truncated })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_file(filename: String, content: String, extension: Option<String>) -> Result<(), String> {
    let ext = extension.as_deref().unwrap_or("json");
    let filter_name = match ext { "csv" => "CSV", _ => "JSON" };
    let path = rfd::AsyncFileDialog::new()
        .set_file_name(&filename)
        .add_filter(filter_name, &[ext])
        .save_file()
        .await
        .ok_or_else(|| "Avbruten".to_string())?;
    tokio::fs::write(path.path(), content.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_file_binary(filename: String, data: String) -> Result<(), String> {
    let bytes = general_purpose::STANDARD.decode(&data).map_err(|e| e.to_string())?;
    let ext = filename.rsplit('.').next().unwrap_or("bin");
    let filter_name = match ext { "xlsx" => "Excel", _ => "Fil" };
    let path = rfd::AsyncFileDialog::new()
        .set_file_name(&filename)
        .add_filter(filter_name, &[ext])
        .save_file()
        .await
        .ok_or_else(|| "Avbruten".to_string())?;
    tokio::fs::write(path.path(), bytes)
        .await
        .map_err(|e| e.to_string())
}

// ---- Ollama ----

const OLLAMA_BASE: &str = "http://localhost:11434";

#[tauri::command]
async fn check_ollama() -> bool {
    reqwest::Client::new()
        .get(OLLAMA_BASE)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelInfo>,
}

#[derive(Deserialize, Serialize)]
struct OllamaModelInfo {
    name: String,
    size: u64,
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<OllamaModelInfo>, String> {
    let resp = reqwest::Client::new()
        .get(format!("{}/api/tags", OLLAMA_BASE))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let data: OllamaTagsResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.models)
}

#[derive(Serialize, Clone)]
struct PullProgress {
    status: String,
    completed: Option<u64>,
    total: Option<u64>,
}

#[derive(Deserialize)]
struct PullLine {
    status: String,
    completed: Option<u64>,
    total: Option<u64>,
}

#[tauri::command]
async fn pull_ollama_model(app: AppHandle, model: String) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/pull", OLLAMA_BASE))
        .json(&serde_json::json!({ "name": model }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            if line.is_empty() { continue; }
            if let Ok(parsed) = serde_json::from_str::<PullLine>(&line) {
                let done = parsed.status == "success";
                let _ = app.emit("ollama-pull-progress", PullProgress {
                    status: parsed.status,
                    completed: parsed.completed,
                    total: parsed.total,
                });
                if done { return Ok(()); }
            }
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[tauri::command]
fn get_os() -> &'static str {
    std::env::consts::OS
}

#[tauri::command]
async fn delete_ollama_model(model: String) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .delete(format!("{}/api/delete", OLLAMA_BASE))
        .json(&serde_json::json!({ "name": model }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Kunde inte ta bort modell: {}", resp.status()));
    }
    Ok(())
}

#[tauri::command]
async fn query_ollama(model: String, prompt: String) -> Result<String, String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/generate", OLLAMA_BASE))
        .json(&OllamaGenerateRequest { model, prompt, stream: false })
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Ollama-fel: {}", resp.status()));
    }

    let data: OllamaGenerateResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            verify_license,
            check_manifest,
            download_db,
            query_db,
            get_schema,
            save_file,
            save_file_binary,
            get_os,
            check_ollama,
            list_ollama_models,
            pull_ollama_model,
            delete_ollama_model,
            query_ollama
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

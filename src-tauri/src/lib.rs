use base64::{Engine as _, engine::general_purpose};
use lettre::{
    message::header::ContentType, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
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
            .filter(|t| t != "ai_expl")
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

#[tauri::command]
async fn get_ai_explanations(db_path: String) -> Result<std::collections::HashMap<String, std::collections::HashMap<String, String>>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ai_expl'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .unwrap_or(false);
        if !exists {
            return Ok(std::collections::HashMap::new());
        }
        let mut stmt = conn
            .prepare("SELECT tabell, kolumn, beskrivning FROM ai_expl")
            .map_err(|e| e.to_string())?;
        let mut result: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.filter_map(|r| r.ok()) {
            result.entry(row.0).or_default().insert(row.1, row.2);
        }
        Ok(result)
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
fn register_ulow(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "ulow", 1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx: &rusqlite::functions::Context| {
            let s: String = ctx.get(0)?;
            Ok(s.to_lowercase())
        },
    )
}

#[tauri::command]
async fn query_db(db_path: String, sql: String) -> Result<QueryResult, String> {
    tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        register_ulow(&conn).map_err(|e| e.to_string())?;
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
async fn install_ollama(app: AppHandle) -> Result<(), String> {
    match std::env::consts::OS {
        "linux" => {
            let wrapper = "#!/bin/bash\ncurl -fsSL https://ollama.com/install.sh | sh\nsudo mkdir -p /usr/share/ollama\nsudo chown ollama:ollama /usr/share/ollama\nsudo systemctl restart ollama\necho\necho 'Ollama installerat. Tryck Enter för att stänga.'\nread\n";
            let wrapper_path = "/tmp/ollama_install_run.sh";
            std::fs::write(wrapper_path, wrapper).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(wrapper_path, std::fs::Permissions::from_mode(0o755));
            }
            let terminals: &[(&str, &[&str])] = &[
                ("x-terminal-emulator", &["-e", wrapper_path]),
                ("gnome-terminal",      &["--", wrapper_path]),
                ("konsole",             &["-e", wrapper_path]),
                ("xterm",               &["-e", wrapper_path]),
            ];
            for (term, args) in terminals {
                if std::process::Command::new(term).args(*args).spawn().is_ok() {
                    let _ = app.emit("ollama-install-status", "terminal-opened");
                    return Ok(());
                }
            }
            Err("Ingen terminal hittades".to_string())
        }
        "windows" => {
            let _ = app.emit("ollama-install-status", "Laddar ner installerare...");
            let bytes = reqwest::get("https://ollama.com/download/OllamaSetup.exe")
                .await
                .map_err(|e| format!("Kunde inte ladda ner: {e}"))?
                .bytes()
                .await
                .map_err(|e| e.to_string())?;
            let temp_path = std::env::temp_dir().join("OllamaSetup.exe");
            std::fs::write(&temp_path, &bytes).map_err(|e| e.to_string())?;
            let _ = app.emit("ollama-install-status", "Startar installerare...");
            std::process::Command::new(&temp_path)
                .spawn()
                .map_err(|e| format!("Kunde inte starta installerare: {e}"))?;
            Ok(())
        }
        _ => Err("unsupported".to_string()),
    }
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
fn quit(app: AppHandle) {
    app.exit(0);
}

#[derive(Deserialize)]
struct GeminiPart { text: String }
#[derive(Deserialize)]
struct GeminiContent { parts: Vec<GeminiPart> }
#[derive(Deserialize)]
struct GeminiCandidate { content: GeminiContent }
#[derive(Deserialize)]
struct GeminiResponse { candidates: Vec<GeminiCandidate> }

#[derive(Deserialize, Serialize)]
struct GeminiModelInfo {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "supportedGenerationMethods", default)]
    supported_generation_methods: Vec<String>,
}
#[derive(Deserialize)]
struct GeminiModelsResponse { models: Vec<GeminiModelInfo> }

#[tauri::command]
async fn list_gemini_models(api_key: String) -> Result<Vec<GeminiModelInfo>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini-fel: {}", body));
    }

    let data: GeminiModelsResponse = resp.json().await.map_err(|e| e.to_string())?;
    let models = data.models.into_iter()
        .filter(|m| m.supported_generation_methods.iter().any(|s| s == "generateContent"))
        .map(|mut m| { m.name = m.name.trim_start_matches("models/").to_string(); m })
        .collect();
    Ok(models)
}

#[tauri::command]
async fn query_gemini(api_key: String, model: String, prompt: String) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}]
        }))
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini-fel {}: {}", status, body));
    }

    let data: GeminiResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.candidates.into_iter().next()
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .unwrap_or_default())
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
#[tauri::command]
async fn send_test_email(
    host: String,
    port: u16,
    encryption: String, // "starttls" | "tls" | "none"
    username: String,
    password: String,
    from_name: String,
    from_email: String,
    to_email: String,
) -> Result<(), String> {
    let email = Message::builder()
        .from(format!("{} <{}>", from_name, from_email).parse().map_err(|e| format!("Ogiltig avsändaradress: {e}"))?)
        .to(to_email.parse().map_err(|e| format!("Ogiltig mottagaradress: {e}"))?)
        .subject("Testmail från Företagsdatabasen")
        .header(ContentType::TEXT_PLAIN)
        .body("Det här är ett testmail. SMTP-konfigurationen fungerar.".to_string())
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(username, password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> = match encryption.as_str() {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|e| e.to_string())?
            .port(port)
            .credentials(creds)
            .build(),
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
            .port(port)
            .credentials(creds)
            .build(),
        _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| e.to_string())?
            .port(port)
            .credentials(creds)
            .build(),
    };

    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}

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
            get_ai_explanations,
            save_file,
            save_file_binary,
            get_os,
            check_ollama,
            list_ollama_models,
            pull_ollama_model,
            delete_ollama_model,
            query_ollama,
            query_gemini,
            list_gemini_models,
            install_ollama,
            quit,
            send_test_email
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

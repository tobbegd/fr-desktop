use serde::{Deserialize, Serialize};

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
        reqwest::StatusCode::PAYMENT_REQUIRED => return Err("402: Prenumerationen har gått ut".to_string()),
        s if !s.is_success() => return Err(format!("Serverfel: {}", s)),
        _ => {}
    }

    resp.json::<VerifyResult>().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![verify_license])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::blocking::multipart;
use serde::Deserialize;

const GITHUB_REPO: &str = "tobbegd/fr-desktop";
const WORKFLOW_FILE: &str = "build.yml";
const GITHUB_API: &str = "https://api.github.com";

// Plattform → (artifact-namn, updater-arkiv-suffix, plattform-id till fr-web)
// Tauri v2 signerar installern direkt (ej inzippad som i v1)
const PLATFORMS: &[(&str, &str, &str)] = &[
    ("fr-app-ubuntu-22.04",   ".AppImage",   "linux-x86_64"),
    ("fr-app-windows-latest", ".msi",        "windows-x86_64"),
    ("fr-app-macos-latest",   ".app.tar.gz", "darwin-universal"),
];

#[derive(Parser)]
#[command(
    name = "fr-release",
    about = "Bygg och publicera fr-app",
    long_about = "Bygger fr-app med Tauri, signerar binären och laddar upp till servern.

FÖRUTSÄTTNINGAR
  - bun (JavaScript-pakethanterare)
  - Rust + Cargo
  - Tauri CLI: bun add -D @tauri-apps/cli  (i fr-app/)
  - Signeringsnyckel: bunx tauri signer generate -w ~/.tauri/fr-app.key
    (publik nyckel läggs in i fr-app/src-tauri/tauri.conf.json → plugins.updater.pubkey)

GITHUB ACTIONS-LÄGE (alla plattformar på en gång)
  Triggar build.yml, väntar på att alla tre plattformar bygger klart,
  laddar sedan ner artefakterna och laddar upp dem till fr-web.

  Kräver:
    - GitHub PAT med actions: read+write (env FR_GITHUB_TOKEN eller --github-token)
    - Upload-nyckel (env FR_UPLOAD_KEY eller --upload-key)
    - Signing-nycklarna som GitHub Secrets: TAURI_SIGNING_PRIVATE_KEY +
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD

  Exempel:
    FR_GITHUB_TOKEN=ghp_... FR_UPLOAD_KEY=abc123 \\
      fr-release --github --notes \"Buggfixar\"

LOKALT LINUX-LÄGE
  Bygger AppImage lokalt, signerar och laddar upp (befintligt beteende).

FÖRSTA GÅNGEN (en gång per nyckelpar)
  1. Generera nyckelpar:
       bunx tauri signer generate -w ~/.tauri/fr-app.key  (i fr-app/)
  2. Kopiera publik nyckel ur ~/.tauri/fr-app.key.pub till
     tauri.conf.json → plugins.updater.pubkey
  3. Exportera privat nyckel till lösenordshanteraren:
       cat ~/.tauri/fr-app.key
     Spara hela utskriften (två rader) som en hemlighet, t.ex. 'FR_SIGNING_KEY'
  4. Sätt upload-nyckeln i miljövariabeln FR_UPLOAD_KEY (se config.yaml på servern)

NY MASKIN (ingen filkopiering behövs)
  Hämta FR_SIGNING_KEY ur lösenordshanteraren och exportera den:
    export FR_SIGNING_KEY=\"<två rader ur lösenordshanteraren>\"
  Ingen fil på disk krävs — verktyget använder strängen direkt.

RELEASE-FLÖDE (GitHub Actions, rekommenderat)
  1. Bumpa version i fr-app/src-tauri/tauri.conf.json och fr-app/src-tauri/Cargo.toml
  2. Committa och pusha till main
  3. Kör:
       FR_GITHUB_TOKEN=... FR_UPLOAD_KEY=... fr-release --github --notes \"Release-text\"
  4. Alla tre plattformar laddas upp som staging-byggen i fr-web
  5. Aktivera i fr-web admin (/admin/desktop) när du vill releasa till kunder

SERVER-ENDPOINTS (fr-web)
  GET  /output/desktop/latest.json            Tauri updater läser denna vid varje appstart
  GET  /output/desktop/download/<fil>         Serverar arkivet vid uppdatering
  POST /api/output/desktop/upload             Skyddad endpoint, används av detta verktyg",
    after_help = "MILJÖVARIABLER
  FR_UPLOAD_KEY      Upload-nyckel (alternativ till --upload-key)
  FR_SIGNING_KEY     Privat signeringsnyckel som sträng (lokalt läge)
  FR_GITHUB_TOKEN    GitHub PAT med actions: read+write (GitHub Actions-läge)

EXPORTERA SIGNERINGSNYCKELN (en gång, från maskinen med nyckelfilen)
  cat ~/.tauri/fr-app.key
  → Kopiera utskriften (två rader) och spara i lösenordshanteraren som FR_SIGNING_KEY

EXEMPEL
  # GitHub Actions-release (alla plattformar) — rekommenderat
  FR_GITHUB_TOKEN=ghp_... FR_UPLOAD_KEY=abc123 fr-release --github --notes \"Ny funktion\"

  # Lokalt Linux-bygge
  FR_UPLOAD_KEY=abc123 fr-release --upload --notes \"Buggfix\"

  # Release mot lokal server (test)
  FR_UPLOAD_KEY=abc123 fr-release --upload --server http://localhost:8080 --notes \"Test\"

  # Ladda bara upp utan att bygga om (om bygget redan gjorts)
  FR_UPLOAD_KEY=abc123 fr-release --upload --no-build"
)]
struct Cli {
    /// Sökväg till fr-app-katalogen
    #[arg(long, default_value = "../fr-app")]
    app_dir: PathBuf,

    /// Server-URL att ladda upp till
    #[arg(long, default_value = "https://foretagsdatabasen.se")]
    server: String,

    /// Upload-nyckel (Bearer token), krävs vid uppladdning
    #[arg(long, env = "FR_UPLOAD_KEY")]
    upload_key: Option<String>,

    /// Sökväg till privat signeringsnyckel (lokalt läge)
    #[arg(long, default_value = "~/.tauri/fr-app.key")]
    key_path: String,

    /// Lösenord till signeringsnyckeln (lokalt läge)
    #[arg(long, env = "FR_SIGNING_KEY_PASSWORD")]
    key_password: Option<String>,

    /// Plattform att rapportera i latest.json (lokalt läge)
    #[arg(long, default_value = "linux-x86_64")]
    platform: String,

    /// Release-noter (valfri)
    #[arg(long, default_value = "")]
    notes: String,

    /// Hoppa över bygget och använd senaste artifact direkt (lokalt läge)
    #[arg(long)]
    no_build: bool,

    /// Ladda upp till servern efter bygge och signering (lokalt läge)
    #[arg(long)]
    upload: bool,

    /// Bygg alla plattformar via GitHub Actions och ladda upp med --upload-key
    #[arg(long)]
    github: bool,

    /// Hämta senaste lyckade GitHub Actions-bygge och ladda upp (ingen ny build)
    #[arg(long)]
    push_latest: bool,

    /// GitHub PAT med actions: read+write (alternativt env-var FR_GITHUB_TOKEN)
    #[arg(long, env = "FR_GITHUB_TOKEN")]
    github_token: Option<String>,
}

#[derive(Deserialize)]
struct TauriConf {
    version: String,
    #[serde(rename = "productName")]
    product_name: String,
}

// ─── Hjälpfunktioner ──────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(path)
    }
}

fn read_tauri_conf(app_dir: &Path) -> Result<TauriConf> {
    let path = app_dir.join("src-tauri/tauri.conf.json");
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("Kunde inte läsa {}", path.display()))?;
    serde_json::from_str(&data).context("Kunde inte tolka tauri.conf.json")
}

fn find_artifact(bundle_dir: &Path, suffix: &str) -> Result<PathBuf> {
    for entry in std::fs::read_dir(bundle_dir)
        .with_context(|| format!("Kunde inte läsa {}", bundle_dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(suffix) {
            return Ok(entry.path());
        }
    }
    bail!("Hittade ingen fil med suffix '{}' i {}", suffix, bundle_dir.display());
}

/// Skriver ut alla filer rekursivt relativt `base` (för felsökning).
fn list_files_recursive(dir: &Path, base: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                list_files_recursive(&path, base);
            } else {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                println!("    {}", rel.display());
            }
        }
    }
}

/// Söker rekursivt efter den första filen vars namn slutar med `suffix`.
fn find_file_recursive(dir: &Path, suffix: &str) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_file_recursive(&path, suffix) {
                    return Some(found);
                }
            } else if path.to_string_lossy().ends_with(suffix) {
                return Some(path);
            }
        }
    }
    None
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Kunde inte öppna {}", path.display()))?;
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        sha2::Digest::update(&mut hasher, &buf[..n]);
    }
    Ok(format!("{:x}", sha2::Digest::finalize(hasher)))
}

// ─── Lokalt bygge (befintlig logik) ───────────────────────────────────────────

fn build(app_dir: &Path) -> Result<()> {
    println!("→ Bygger fr-app...");
    let status = Command::new("bunx")
        .args(["tauri", "build", "--bundles", "appimage"])
        .current_dir(app_dir)
        .status()
        .context("Kunde inte starta 'bunx tauri build'")?;
    if !status.success() {
        bail!("Bygget misslyckades (exit {})", status);
    }
    println!("✓ Bygget klart");
    Ok(())
}

fn create_archive(appimage: &Path) -> Result<PathBuf> {
    let archive = appimage.with_extension("AppImage.tar.gz");
    println!("→ Skapar arkiv {}...", archive.file_name().unwrap().to_string_lossy());
    let status = Command::new("tar")
        .args([
            "czf",
            archive.to_str().unwrap(),
            "-C",
            appimage.parent().unwrap().to_str().unwrap(),
            appimage.file_name().unwrap().to_str().unwrap(),
        ])
        .status()
        .context("Kunde inte köra tar")?;
    if !status.success() {
        bail!("tar misslyckades");
    }
    let checksum = sha256_file(&archive)?;
    let sha_path = PathBuf::from(format!("{}.sha256", archive.display()));
    std::fs::write(&sha_path, format!("{}  {}\n", checksum, archive.file_name().unwrap().to_string_lossy()))?;
    println!("✓ Arkiv skapat (sha256: {})", &checksum[..16]);
    Ok(archive)
}

fn sign_archive(archive: &Path, key_str: Option<&str>, key_path: &Path, key_password: Option<&str>, app_dir: &Path) -> Result<String> {
    println!("→ Signerar...");
    let mut cmd = Command::new("bunx");
    cmd.args(["tauri", "signer", "sign"]).current_dir(app_dir);
    if let Some(k) = key_str {
        cmd.args(["-k", k]);
    } else {
        cmd.args(["-f", key_path.to_str().unwrap()]);
    }
    cmd.args(["--password", key_password.unwrap_or("")]);
    cmd.arg(archive.to_str().unwrap());
    let status = cmd.status().context("Kunde inte köra tauri signer sign")?;
    if !status.success() {
        bail!("Signering misslyckades");
    }
    let sig_path = PathBuf::from(format!("{}.sig", archive.display()));
    let sig = std::fs::read_to_string(&sig_path)
        .with_context(|| format!("Hittade inte signaturfilen: {}", sig_path.display()))?;
    println!("✓ Signering klar");
    Ok(sig.trim().to_string())
}

// ─── Uppladdning (delas av lokalt och GitHub-flödet) ─────────────────────────

fn upload(
    server: &str,
    upload_key: &str,
    platform: &str,
    notes: &str,
    version: &str,
    archive: &Path,
    sig: &str,
) -> Result<()> {
    let filename = archive
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let download_url = format!("{}/output/desktop/download/{}", server.trim_end_matches('/'), filename);

    println!("→ Laddar upp {} ({})...", filename, platform);

    let file_bytes = std::fs::read(archive)
        .with_context(|| format!("Kunde inte läsa {}", archive.display()))?;

    let sha_path = PathBuf::from(format!("{}.sha256", archive.display()));
    let sha_bytes = std::fs::read(&sha_path)
        .with_context(|| format!("Hittade inte checksumfilen: {}", sha_path.display()))?;

    let form = multipart::Form::new()
        .text("version", version.to_string())
        .text("platform", platform.to_string())
        .text("notes", notes.to_string())
        .text("signature", sig.to_string())
        .text("url", download_url.clone())
        .part("file", multipart::Part::bytes(file_bytes).file_name(filename.clone()))
        .part("sha256", multipart::Part::bytes(sha_bytes).file_name(format!("{}.sha256", filename)));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .context("Kunde inte skapa upload-klient")?;
    let resp = client
        .post(format!("{}/api/output/desktop/upload", server.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", upload_key))
        .multipart(form)
        .send()
        .context("HTTP-förfrågan misslyckades")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        bail!("Uppladdning misslyckades ({}): {}", status, body);
    }

    println!("✓ {platform} uppladdad — {download_url}");
    Ok(())
}

// ─── GitHub Actions-integration ──────────────────────────────────────────────

struct GhClient {
    inner: reqwest::blocking::Client,
    token: String,
}

impl GhClient {
    fn new(token: &str) -> Self {
        let inner = reqwest::blocking::Client::builder()
            .user_agent("fr-release/1.0")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Kunde inte skapa HTTP-klient");
        Self { inner, token: token.to_string() }
    }

    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let resp = self.inner.get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .with_context(|| format!("GET {url}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("GitHub API {url} — {status}: {body}");
        }
        resp.json().context("Kunde inte tolka GitHub-svar som JSON")
    }

    fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let resp = self.inner.post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .with_context(|| format!("POST {url}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("GitHub API POST {url} — {status}: {body}");
        }
        Ok(())
    }

    fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        // Artefakter kan vara >100 MB — använd separat klient med lång timeout
        let client = reqwest::blocking::Client::builder()
            .user_agent("fr-release/1.0")
            .timeout(Duration::from_secs(600))
            .build()
            .context("Kunde inte skapa download-klient")?;
        let resp = client.get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .with_context(|| format!("Nedladdning {url}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("Nedladdning {url} — {status}: {body}");
        }
        let total = resp.content_length();
        let mut buf = Vec::new();
        let mut reader = resp;
        let mut chunk = [0u8; 65536];
        loop {
            let n = std::io::Read::read(&mut reader, &mut chunk)
                .context("Fel vid läsning av nedladdning")?;
            if n == 0 { break; }
            buf.extend_from_slice(&chunk[..n]);
            let mb = buf.len() as f64 / 1_048_576.0;
            match total {
                Some(t) => print!("  {:.1} / {:.1} MB\r", mb, t as f64 / 1_048_576.0),
                None    => print!("  {:.1} MB\r", mb),
            }
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        println!("  {:.1} MB — klar", buf.len() as f64 / 1_048_576.0);
        Ok(buf)
    }
}

/// Hämtar det högsta run-id:t för workflowen (0 om inga körs finns).
fn latest_run_id(gh: &GhClient) -> Result<u64> {
    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/workflows/{WORKFLOW_FILE}/runs?per_page=1");
    let data = gh.get_json(&url)?;
    let runs = data["workflow_runs"].as_array()
        .context("Saknar 'workflow_runs' i GitHub-svaret")?;
    if runs.is_empty() {
        return Ok(0);
    }
    runs[0]["id"].as_u64().context("Saknar 'id' i workflow run")
}

/// Hämtar det senaste lyckade run-id:t för workflowen.
fn latest_successful_run_id(gh: &GhClient) -> Result<u64> {
    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/workflows/{WORKFLOW_FILE}/runs?status=success&per_page=1");
    let data = gh.get_json(&url)?;
    let runs = data["workflow_runs"].as_array()
        .context("Saknar 'workflow_runs' i GitHub-svaret")?;
    if runs.is_empty() {
        bail!("Hittade inget lyckat bygge för workflowen");
    }
    let run = &runs[0];
    let id = run["id"].as_u64().context("Saknar 'id' i workflow run")?;
    let created = run["created_at"].as_str().unwrap_or("?");
    let head_sha = run["head_sha"].as_str().unwrap_or("?");
    println!("  Senaste lyckade bygge: run_id={id}, skapad={created}, commit={}", &head_sha[..8]);
    Ok(id)
}

/// Triggar workflowen och returnerar det nya run-id:t.
fn trigger_workflow(gh: &GhClient) -> Result<u64> {
    let max_id_before = latest_run_id(gh)?;
    println!("→ Triggar GitHub Actions-workflow...");

    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/workflows/{WORKFLOW_FILE}/dispatches");
    gh.post_json(&url, &serde_json::json!({ "ref": "main" }))?;

    // Polla tills ett nytt run dyker upp (max 2 minuter)
    let deadline = SystemTime::now() + Duration::from_secs(120);
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let new_id = latest_run_id(gh)?;
        if new_id > max_id_before {
            println!("✓ Kör startad (run_id={new_id})");
            return Ok(new_id);
        }
        if SystemTime::now() > deadline {
            bail!("Timeout — nytt GitHub-run dök aldrig upp inom 2 minuter");
        }
        print!(".");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

/// Pollar run-status var 30:e sekund tills körnigen är klar.
fn wait_for_run(gh: &GhClient, run_id: u64) -> Result<()> {
    println!("→ Väntar på bygget (pollar var 30 s)...");
    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/runs/{run_id}");
    let deadline = SystemTime::now() + Duration::from_secs(90 * 60); // max 90 min

    loop {
        let data = match gh.get_json(&url) {
            Ok(d) => d,
            Err(e) => {
                if SystemTime::now() > deadline {
                    return Err(e);
                }
                eprintln!("  Nätverksfel (försöker igen om 30 s): {e:#}");
                std::thread::sleep(Duration::from_secs(30));
                continue;
            }
        };
        let status = data["status"].as_str().unwrap_or("unknown");
        let conclusion = data["conclusion"].as_str().unwrap_or("");

        match status {
            "completed" => {
                if conclusion == "success" {
                    println!("✓ Bygget klart (success)");
                    return Ok(());
                } else {
                    let url_html = data["html_url"].as_str().unwrap_or("");
                    bail!("Bygget misslyckades (conclusion={conclusion})\n  {url_html}");
                }
            }
            "queued" | "in_progress" | "waiting" | "requested" | "pending" => {
                if SystemTime::now() > deadline {
                    bail!("Timeout — GitHub-run {run_id} slutfördes inte inom 90 minuter");
                }
                let elapsed = {
                    let started = data["run_started_at"]
                        .as_str()
                        .unwrap_or("");
                    if started.is_empty() { "väntar...".to_string() }
                    else { format!("status={status}") }
                };
                print!("  [{elapsed}] nästa check om 30 s...\r");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                std::thread::sleep(Duration::from_secs(30));
            }
            other => bail!("Okänd GitHub run-status: {other}"),
        }
    }
}

/// Extraherar en zip-fil till en målkatalog.
fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .context("Kunde inte öppna zip-arkivet")?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .with_context(|| format!("Zip-entry {i}"))?;
        let out_path = dest.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)
                .with_context(|| format!("Kunde inte skapa {}", out_path.display()))?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// Laddar ner alla plattformars artefakter, extraherar och laddar upp till fr-web.
fn github_upload_all(
    gh: &GhClient,
    run_id: u64,
    server: &str,
    upload_key: &str,
    version: &str,
    notes: &str,
) -> Result<()> {
    // Hämta artifact-lista för run:en
    let url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/runs/{run_id}/artifacts?per_page=30");
    let data = gh.get_json(&url)?;
    let artifacts = data["artifacts"].as_array()
        .context("Saknar 'artifacts' i GitHub-svaret")?;

    let tmp = tempfile::tempdir().context("Kunde inte skapa temp-katalog")?;
    let mut any_ok = false;

    for (artifact_name, archive_suffix, platform_id) in PLATFORMS {
        // Hitta artefakten med rätt namn
        let artifact = artifacts.iter().find(|a| {
            a["name"].as_str().unwrap_or("") == *artifact_name
        });

        let Some(artifact) = artifact else {
            eprintln!("⚠  Artefakt '{artifact_name}' saknas — hoppar över {platform_id}");
            continue;
        };

        let artifact_id = artifact["id"].as_u64()
            .context("Saknar 'id' i artifact")?;

        // Ladda ner zip (GitHub redirectar till S3)
        println!("→ Laddar ner {artifact_name} (id={artifact_id})...");
        let zip_url = format!("{GITHUB_API}/repos/{GITHUB_REPO}/actions/artifacts/{artifact_id}/zip");
        let zip_bytes = gh.download_bytes(&zip_url)
            .with_context(|| format!("Nedladdning av {artifact_name}"))?;

        // Extrahera i en plattformsspecifik underkatalog
        let plat_dir = tmp.path().join(artifact_name);
        std::fs::create_dir_all(&plat_dir)?;
        extract_zip(&zip_bytes, &plat_dir)
            .with_context(|| format!("Extraktion av {artifact_name}"))?;

        // Lista alla filer i artefakten (hjälper vid felsökning)
        println!("  Filer i artefakten:");
        list_files_recursive(&plat_dir, &plat_dir);

        // Hitta updater-arkivet och signaturfilen
        let archive_path = match find_file_recursive(&plat_dir, archive_suffix) {
            Some(p) => p,
            None => {
                eprintln!("⚠  Hittade inte *{archive_suffix} i {artifact_name} — hoppar över {platform_id}");
                continue;
            }
        };

        let sig_suffix = format!("{archive_suffix}.sig");
        let sig_path = match find_file_recursive(&plat_dir, &sig_suffix) {
            Some(p) => p,
            None => {
                eprintln!("⚠  Hittade inte *{sig_suffix} i {artifact_name} — hoppar över {platform_id}");
                continue;
            }
        };

        let sig = std::fs::read_to_string(&sig_path)
            .with_context(|| format!("Kunde inte läsa signaturfilen {}", sig_path.display()))?;
        let sig = sig.trim();

        // Skapa sha256-fil bredvid arkivet (krävs av upload())
        let checksum = sha256_file(&archive_path)?;
        let archive_filename = archive_path.file_name().unwrap().to_string_lossy();
        let sha_path = PathBuf::from(format!("{}.sha256", archive_path.display()));
        std::fs::write(&sha_path, format!("{}  {}\n", checksum, archive_filename))?;

        if let Err(e) = upload(server, upload_key, platform_id, notes, version, &archive_path, sig) {
            eprintln!("⚠  Uppladdning av {platform_id} misslyckades: {e:#}");
        } else {
            any_ok = true;
        }
    }

    if !any_ok {
        bail!("Ingen plattform laddades upp");
    }
    Ok(())
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    let app_dir = cli.app_dir.canonicalize()
        .with_context(|| format!("Hittade inte app-katalogen: {}", cli.app_dir.display()))?;

    let conf = read_tauri_conf(&app_dir)?;
    println!("fr-release v{} | {} v{}", env!("CARGO_PKG_VERSION"), conf.product_name, conf.version);

    // ── GitHub Actions-läge ──────────────────────────────────────────────────
    if cli.github || cli.push_latest {
        let github_token = cli.github_token.as_deref()
            .context("FR_GITHUB_TOKEN eller --github-token krävs")?;
        let upload_key = cli.upload_key.as_deref()
            .context("FR_UPLOAD_KEY eller --upload-key krävs")?;

        let gh = GhClient::new(github_token);

        let run_id = if cli.push_latest {
            println!("→ Hämtar senaste lyckade bygge...");
            latest_successful_run_id(&gh)?
        } else {
            let id = trigger_workflow(&gh)?;
            wait_for_run(&gh, id)?;
            id
        };

        github_upload_all(&gh, run_id, &cli.server, upload_key, &conf.version, &cli.notes)?;

        println!("\n✓ Alla plattformar uppladdade som staging-byggen.");
        println!("  Aktivera i fr-web admin: {}/admin/desktop", cli.server.trim_end_matches('/'));
        return Ok(());
    }

    // ── Lokalt Linux-läge ────────────────────────────────────────────────────
    let signing_key_str = std::env::var("FR_SIGNING_KEY").ok();
    let key_path = expand_tilde(&cli.key_path);

    if signing_key_str.is_some() {
        println!("  Signeringsnyckel: miljövariabel FR_SIGNING_KEY");
    } else if key_path.exists() {
        println!("  Signeringsnyckel: {}", key_path.display());
    } else {
        bail!(
            "Ingen signeringsnyckel hittad.\n\
             Alternativ 1 — miljövariabel (rekommenderas på ny maskin):\n\
               export FR_SIGNING_KEY=\"$(cat ~/.tauri/fr-app.key)\"\n\
             Alternativ 2 — nyckelfil:\n\
               bunx tauri signer generate -w {p}    (generera ny)\n\
               eller kopiera befintlig nyckel till {p}",
            p = key_path.display()
        );
    }

    if !cli.no_build {
        build(&app_dir)?;
    }

    let bundle_dir = app_dir.join("src-tauri/target/release/bundle/appimage");
    let appimage = find_artifact(&bundle_dir, ".AppImage")
        .context("Hittade inte AppImage efter bygget")?;

    let appimage_name = appimage.file_name().unwrap().to_string_lossy().to_string();
    if !appimage_name.contains(&conf.version) {
        eprintln!(
            "Varning: '{}' innehåller inte version '{}' — är bygget aktuellt?",
            appimage_name, conf.version
        );
    }

    let archive = create_archive(&appimage)?;
    let sig = sign_archive(&archive, signing_key_str.as_deref(), &key_path, cli.key_password.as_deref(), &app_dir)?;

    if cli.upload {
        let upload_key = cli.upload_key.as_deref().context(
            "FR_UPLOAD_KEY eller --upload-key krävs när --upload används",
        )?;
        upload(
            &cli.server,
            upload_key,
            &cli.platform,
            &cli.notes,
            &conf.version,
            &archive,
            &sig,
        )?;
    } else {
        println!("  (Uppladdning hoppas över — lägg till --upload för att ladda upp)");
    }

    Ok(())
}

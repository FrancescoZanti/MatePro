// Prevent console window on Windows in release builds
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod agent;
mod mcp_sql;

use agent::{AgentSystem, ToolCall, ToolResult};
use anyhow::Result;
use calamine::{open_workbook, Ods, Reader, Xls, Xlsx};
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[cfg(target_os = "windows")]
use semver::Version;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use tokio::fs::File;
#[cfg(target_os = "windows")]
use tokio::io::AsyncWriteExt;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum UpdateStatus {
    UpToDate {
        current_version: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
        download_url: String,
        asset_name: String,
    },
    Unsupported,
    Error {
        message: String,
    },
}

// ============ TYPES ============

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub hidden: bool,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
}

impl ModelInfo {
    fn size_gb(&self) -> f64 {
        self.size as f64 / 1_073_741_824.0
    }

    fn weight_category(&self) -> &'static str {
        let gb = self.size_gb();
        if gb < 4.0 {
            "light"
        } else if gb < 8.0 {
            "medium"
        } else {
            "heavy"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfoResponse {
    pub name: String,
    pub size: u64,
    pub size_gb: f64,
    pub category: String,
}

// ============ STATE ============

struct AppState {
    ollama_url: Mutex<String>,
    client: reqwest::Client,
    agent_system: Mutex<AgentSystem>,
    sql_manager: mcp_sql::SqlConnectionManager,
    last_sql_connection_id: Arc<Mutex<Option<String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        let sql_manager = mcp_sql::SqlConnectionManager::new();
        let last_sql_connection_id = Arc::new(Mutex::new(None));
        let agent =
            AgentSystem::with_shared_state(sql_manager.clone(), last_sql_connection_id.clone());

        Self {
            ollama_url: Mutex::new("http://localhost:11434".to_string()),
            client: reqwest::Client::new(),
            agent_system: Mutex::new(agent),
            sql_manager,
            last_sql_connection_id,
        }
    }
}

// ============ UPDATE SUPPORT ============

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[cfg(target_os = "windows")]
async fn latest_windows_release() -> Result<UpdateStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("MatePro-Updater")
        .build()
        .map_err(|e| format!("Impossibile creare client HTTP: {}", e))?;

    let release: GitHubRelease = client
        .get("https://api.github.com/repos/FrancescoZanti/MatePro/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Errore richiesta GitHub: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Risposta GitHub non valida: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Errore parsing risposta GitHub: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v');
    let current_version = env!("CARGO_PKG_VERSION");

    let latest_semver = Version::parse(latest_version)
        .map_err(|e| format!("Versione release non valida '{}': {}", latest_version, e))?;
    let current_semver = Version::parse(current_version)
        .map_err(|e| format!("Versione corrente non valida '{}': {}", current_version, e))?;

    if latest_semver <= current_semver {
        return Ok(UpdateStatus::UpToDate {
            current_version: current_version.to_string(),
        });
    }

    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.contains("windows") && asset.name.ends_with(".exe"))
        .ok_or_else(|| {
            format!(
                "Nessun installer Windows trovato per la release {}",
                latest_version
            )
        })?;

    Ok(UpdateStatus::UpdateAvailable {
        current_version: current_version.to_string(),
        latest_version: latest_semver.to_string(),
        download_url: asset.browser_download_url,
        asset_name: asset.name,
    })
}

#[cfg(target_os = "windows")]
async fn download_installer(url: &str, version: &str) -> Result<std::path::PathBuf, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent("MatePro-Updater")
        .build()
        .map_err(|e| format!("Impossibile creare client HTTP: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Errore download installer: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Download fallito: {}", e))?;

    let mut installer_path = std::env::temp_dir();
    installer_path.push(format!("matepro-update-{}-installer.exe", version));

    let mut file = File::create(&installer_path)
        .await
        .map_err(|e| format!("Impossibile creare file temporaneo: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Errore lettura dati installer: {}", e))?;

    file.write_all(&bytes)
        .await
        .map_err(|e| format!("Impossibile salvare installer: {}", e))?;
    file.flush()
        .await
        .map_err(|e| format!("Impossibile completare scrittura installer: {}", e))?;

    Ok(installer_path)
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn check_for_updates() -> Result<UpdateStatus, String> {
    match latest_windows_release().await {
        Ok(status) => Ok(status),
        Err(message) => Ok(UpdateStatus::Error { message }),
    }
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn check_for_updates() -> Result<UpdateStatus, String> {
    Ok(UpdateStatus::Unsupported)
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn download_and_install_update(url: String, version: String) -> Result<(), String> {
    let installer_path = download_installer(&url, &version).await?;

    std::process::Command::new(&installer_path)
        .spawn()
        .map_err(|e| format!("Impossibile avviare l'installer: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn download_and_install_update(_url: String, _version: String) -> Result<(), String> {
    Err("Gli aggiornamenti automatici sono disponibili solo su Windows".to_string())
}

// ============ HELPER FUNCTIONS ============

fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let total_seconds = now + 3600; // +1 hour for CET
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds % 3600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

fn extract_text_from_pdf(path: &PathBuf) -> Result<String> {
    let doc = Document::load(path)?;
    let mut text = String::new();

    for page_num in 1..=doc.get_pages().len() {
        if let Ok(page_text) = doc.extract_text(&[page_num as u32]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    if text.trim().is_empty() {
        anyhow::bail!("Impossibile estrarre testo dal PDF");
    }

    Ok(text)
}

fn extract_text_from_excel(path: &PathBuf) -> Result<String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut text = String::new();

    match extension.to_lowercase().as_str() {
        "xlsx" => {
            let mut workbook: Xlsx<_> = open_workbook(path)?;
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    text.push_str(&format!("=== Foglio: {} ===\n", sheet_name));
                    for row in range.rows() {
                        let row_text: Vec<String> =
                            row.iter().map(|cell| format!("{}", cell)).collect();
                        text.push_str(&row_text.join("\t"));
                        text.push('\n');
                    }
                    text.push('\n');
                }
            }
        }
        "xls" => {
            let mut workbook: Xls<_> = open_workbook(path)?;
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    text.push_str(&format!("=== Foglio: {} ===\n", sheet_name));
                    for row in range.rows() {
                        let row_text: Vec<String> =
                            row.iter().map(|cell| format!("{}", cell)).collect();
                        text.push_str(&row_text.join("\t"));
                        text.push('\n');
                    }
                    text.push('\n');
                }
            }
        }
        "ods" => {
            let mut workbook: Ods<_> = open_workbook(path)?;
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    text.push_str(&format!("=== Foglio: {} ===\n", sheet_name));
                    for row in range.rows() {
                        let row_text: Vec<String> =
                            row.iter().map(|cell| format!("{}", cell)).collect();
                        text.push_str(&row_text.join("\t"));
                        text.push('\n');
                    }
                    text.push('\n');
                }
            }
        }
        _ => anyhow::bail!("Formato non supportato: {}", extension),
    }

    if text.trim().is_empty() {
        anyhow::bail!("Il file è vuoto");
    }

    Ok(text)
}

fn extract_text_from_file(path: &PathBuf) -> Result<String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "pdf" => extract_text_from_pdf(path),
        "xlsx" | "xls" | "ods" => extract_text_from_excel(path),
        "txt" | "md" | "csv" => {
            let content = fs::read_to_string(path)?;
            Ok(content)
        }
        _ => anyhow::bail!("Formato file non supportato: {}", extension),
    }
}

async fn check_server(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
        .unwrap();

    match client.get(format!("{}/api/tags", url)).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

// ============ TAURI COMMANDS ============

#[tauri::command]
async fn scan_network() -> Vec<String> {
    let mut servers = Vec::new();

    // Check localhost
    if check_server("http://localhost:11434").await {
        servers.push("http://localhost:11434".to_string());
    }

    // Check 127.0.0.1
    if check_server("http://127.0.0.1:11434").await
        && !servers.contains(&"http://127.0.0.1:11434".to_string())
    {
        servers.push("http://127.0.0.1:11434".to_string());
    }

    // Get local IP and scan network
    if let Ok(local_ip) = local_ip_address::local_ip() {
        if let IpAddr::V4(ip) = local_ip {
            let octets = ip.octets();
            let base = format!("{}.{}.{}", octets[0], octets[1], octets[2]);

            let mut handles = vec![];
            for i in 1..255 {
                let url = format!("http://{}.{}:11434", base, i);
                let handle = tokio::spawn(async move {
                    if check_server(&url).await {
                        Some(url)
                    } else {
                        None
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                if let Ok(Some(url)) = handle.await {
                    if !servers.contains(&url) {
                        servers.push(url);
                    }
                }
            }
        }
    }

    servers
}

#[tauri::command]
async fn connect_to_server(state: State<'_, Arc<AppState>>, url: String) -> Result<(), String> {
    if !check_server(&url).await {
        return Err("Impossibile connettersi al server Ollama".to_string());
    }

    let mut ollama_url = state.ollama_url.lock().await;
    *ollama_url = url;
    Ok(())
}

#[tauri::command]
async fn list_models(state: State<'_, Arc<AppState>>) -> Result<Vec<ModelInfoResponse>, String> {
    let url = state.ollama_url.lock().await;
    let response = state
        .client
        .get(format!("{}/api/tags", *url))
        .send()
        .await
        .map_err(|e| format!("Errore connessione: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Errore risposta: {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Errore parsing JSON: {}", e))?;

    let models: Vec<ModelInfoResponse> = json["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_string();
            let size = m["size"].as_u64().unwrap_or(0);
            let model = ModelInfo {
                name: name.clone(),
                size,
            };
            Some(ModelInfoResponse {
                name,
                size,
                size_gb: model.size_gb(),
                category: model.weight_category().to_string(),
            })
        })
        .collect();

    Ok(models)
}

#[tauri::command]
async fn chat(
    state: State<'_, Arc<AppState>>,
    model: String,
    messages: Vec<Message>,
) -> Result<Message, String> {
    let url = state.ollama_url.lock().await;
    let request = ChatRequest {
        model,
        messages,
        stream: false,
    };

    let response = state
        .client
        .post(format!("{}/api/chat", *url))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Errore richiesta: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Errore risposta: {}", response.status()));
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Errore parsing risposta: {}", e))?;

    Ok(Message {
        role: chat_response.message.role,
        content: chat_response.message.content,
        hidden: false,
        timestamp: Some(get_timestamp()),
    })
}

#[tauri::command]
async fn read_file(path: String) -> Result<(String, String), String> {
    let path_buf = PathBuf::from(&path);

    // Validate path doesn't contain directory traversal
    let path_str = path_buf.to_string_lossy();
    if path_str.contains("..") {
        return Err("Path non valido: directory traversal non permesso".to_string());
    }

    // Validate the file exists
    if !path_buf.exists() {
        return Err(format!("File non trovato: {}", path));
    }

    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let content =
        extract_text_from_file(&path_buf).map_err(|e| format!("Errore lettura file: {}", e))?;

    Ok((filename, content))
}

#[tauri::command]
async fn get_tools_description(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let agent = state.agent_system.lock().await;
    Ok(agent.get_tools_description())
}

#[tauri::command]
async fn parse_tool_calls(
    state: State<'_, Arc<AppState>>,
    response: String,
) -> Result<Vec<ToolCall>, String> {
    let agent = state.agent_system.lock().await;
    Ok(agent.parse_tool_calls(&response))
}

#[tauri::command]
async fn execute_tool(
    state: State<'_, Arc<AppState>>,
    tool_call: ToolCall,
) -> Result<ToolResult, String> {
    let mut agent = state.agent_system.lock().await;
    agent
        .execute_tool(&tool_call)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_allow_dangerous(state: State<'_, Arc<AppState>>, allow: bool) -> Result<(), String> {
    let mut agent = state.agent_system.lock().await;
    agent.set_allow_dangerous(allow);
    Ok(())
}

#[tauri::command]
async fn check_tool_dangerous(
    state: State<'_, Arc<AppState>>,
    tool_name: String,
) -> Result<bool, String> {
    let agent = state.agent_system.lock().await;
    Ok(agent
        .tools
        .get(&tool_name)
        .map(|t| t.dangerous)
        .unwrap_or(false))
}

#[tauri::command]
async fn sql_connect(
    state: State<'_, Arc<AppState>>,
    server: String,
    database: String,
    auth_method: String,
    username: Option<String>,
    password: Option<String>,
    trust_server_certificate: Option<bool>,
) -> Result<String, String> {
    let connection_id = format!("sql_{}", uuid::Uuid::new_v4());
    let trust_server_certificate = trust_server_certificate.unwrap_or(false);

    let _client = if auth_method == "windows" {
        mcp_sql::connect_windows_auth(&server, &database, trust_server_certificate)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let user = username.as_deref().ok_or("Username richiesto")?;
        let pass = password.as_deref().ok_or("Password richiesta")?;
        mcp_sql::connect_sql_auth(&server, &database, user, pass, trust_server_certificate)
            .await
            .map_err(|e| e.to_string())?
    };

    let conn_info = mcp_sql::SqlConnection {
        connection_id: connection_id.clone(),
        server,
        database,
        auth_type: auth_method,
        username,
        password,
        trust_server_certificate,
    };

    state.sql_manager.add_connection(conn_info);

    let mut last_conn = state.last_sql_connection_id.lock().await;
    *last_conn = Some(connection_id.clone());

    Ok(connection_id)
}

#[tauri::command]
async fn sql_query(
    state: State<'_, Arc<AppState>>,
    connection_id: Option<String>,
    query: String,
) -> Result<mcp_sql::QueryResult, String> {
    let conn_id = match connection_id {
        Some(id) => id,
        None => {
            let last = state.last_sql_connection_id.lock().await;
            last.clone().ok_or("Nessuna connessione SQL attiva")?
        }
    };

    let conn_info = state
        .sql_manager
        .get_connection(&conn_id)
        .ok_or("Connessione non trovata")?;

    let mut client = mcp_sql::connect_with_info(&conn_info)
        .await
        .map_err(|e| e.to_string())?;

    mcp_sql::run_query(&mut client, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn sql_list_tables(
    state: State<'_, Arc<AppState>>,
    connection_id: Option<String>,
) -> Result<mcp_sql::QueryResult, String> {
    let conn_id = match connection_id {
        Some(id) => id,
        None => {
            let last = state.last_sql_connection_id.lock().await;
            last.clone().ok_or("Nessuna connessione SQL attiva")?
        }
    };

    let conn_info = state
        .sql_manager
        .get_connection(&conn_id)
        .ok_or("Connessione non trovata")?;

    let mut client = mcp_sql::connect_with_info(&conn_info)
        .await
        .map_err(|e| e.to_string())?;

    mcp_sql::list_tables(&mut client)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn sql_describe_table(
    state: State<'_, Arc<AppState>>,
    connection_id: Option<String>,
    schema: String,
    table: String,
) -> Result<mcp_sql::QueryResult, String> {
    let conn_id = match connection_id {
        Some(id) => id,
        None => {
            let last = state.last_sql_connection_id.lock().await;
            last.clone().ok_or("Nessuna connessione SQL attiva")?
        }
    };

    let conn_info = state
        .sql_manager
        .get_connection(&conn_id)
        .ok_or("Connessione non trovata")?;

    let mut client = mcp_sql::connect_with_info(&conn_info)
        .await
        .map_err(|e| e.to_string())?;

    mcp_sql::describe_table(&mut client, &schema, &table)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn sql_disconnect(
    state: State<'_, Arc<AppState>>,
    connection_id: Option<String>,
) -> Result<(), String> {
    let conn_id = match connection_id {
        Some(id) => id,
        None => {
            let last = state.last_sql_connection_id.lock().await;
            last.clone().ok_or("Nessuna connessione SQL attiva")?
        }
    };

    state
        .sql_manager
        .remove_connection(&conn_id)
        .ok_or("Connessione non trovata")?;

    let mut last = state.last_sql_connection_id.lock().await;
    if last.as_ref() == Some(&conn_id) {
        *last = None;
    }

    Ok(())
}

#[tauri::command]
fn get_timestamp_cmd() -> String {
    get_timestamp()
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============ MAIN ============

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            scan_network,
            connect_to_server,
            list_models,
            chat,
            read_file,
            get_tools_description,
            parse_tool_calls,
            execute_tool,
            set_allow_dangerous,
            check_tool_dangerous,
            sql_connect,
            sql_query,
            sql_list_tables,
            sql_describe_table,
            sql_disconnect,
            get_timestamp_cmd,
            get_app_version,
            check_for_updates,
            download_and_install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

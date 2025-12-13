// Prevent console window on Windows in release builds
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod agent;
mod aiconnect;
mod calendar_integration;
mod local_storage;
mod mcp_sql;

use agent::{AgentSystem, ToolCall, ToolResult};
use aiconnect::{
    AiConnectClient, AiConnectNode, AuthMethod, BackendConfig, BackendKind, DiscoveredService,
};
use anyhow::Result;
use calamine::{open_workbook, Ods, Reader, Xls, Xlsx};
use chrono::{DateTime, Utc};
use calendar_integration::{
    CalendarIntegrationStatus, CreateRemoteEventRequest, OutlookDeviceFlowPoll,
    OutlookDeviceFlowStart, RemoteCalendarEvent,
};
use local_storage::{CalendarEvent, CustomSystemPrompt, LocalMemory, MemoryMessage};
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
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

#[derive(Debug, Serialize)]
struct UserProfile {
    username: String,
    display_name: Option<String>,
    primary_language: Option<String>,
}

// Calendar input structures for commands
#[derive(Debug, Serialize, Deserialize, Clone)]
struct CalendarEventInput {
    pub id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub start: String,
    pub end: Option<String>,
    pub source_text: Option<String>,
}

// ============ STATE ============

struct AppState {
    ollama_url: Mutex<String>,
    client: reqwest::Client,
    agent_system: Mutex<AgentSystem>,
    sql_manager: mcp_sql::SqlConnectionManager,
    last_sql_connection_id: Arc<Mutex<Option<String>>>,
    aiconnect_client: AiConnectClient,
    backend_config: Mutex<BackendConfig>,
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
            aiconnect_client: AiConnectClient::new(),
            backend_config: Mutex::new(BackendConfig::default()),
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
    let pages = doc.get_pages();

    for page_num in pages.keys() {
        if let Ok(page_text) = doc.extract_text(&[*page_num]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    if text.trim().is_empty() {
        if let Some(fallback_text) = extract_text_from_pdf_with_pdftotext(path) {
            return Ok(fallback_text);
        }
        anyhow::bail!(
            "Impossibile estrarre testo dal PDF. Il file potrebbe contenere solo immagini o testo protetto."
        );
    }

    Ok(text)
}

fn extract_text_from_pdf_with_pdftotext(path: &PathBuf) -> Option<String> {
    let output = Command::new("pdftotext")
        .arg("-layout")
        .arg("-nopgbrk")
        .arg(path.as_os_str())
        .arg("-")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
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
    let mut messages = messages;

    if let Some(last_user_index) = messages
        .iter()
        .rposition(|message| message.role == "user" && !message.hidden)
    {
        let last_user_content = messages[last_user_index].content.clone();
        let context = {
            let agent = state.agent_system.lock().await;
            agent
                .build_web_search_context(&last_user_content)
                .await
        };

        if let Some(context_text) = context {
            let context_message = Message {
                role: "system".to_string(),
                content: context_text,
                hidden: true,
                timestamp: Some(get_timestamp()),
            };
            messages.insert(last_user_index, context_message);
        }
    }

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

#[tauri::command]
fn get_user_profile() -> UserProfile {
    let username = whoami::username();
    let realname = whoami::realname();
    let trimmed_realname = realname.trim();
    let display_name = if trimmed_realname.is_empty() || trimmed_realname == username {
        None
    } else {
        Some(trimmed_realname.to_string())
    };

    let primary_language = ["LANG", "LC_ALL", "LC_MESSAGES"].iter().find_map(|key| {
        std::env::var(key).ok().and_then(|value| {
            let lang = value.split('.').next().unwrap_or("").trim().to_string();
            if lang.is_empty() {
                None
            } else {
                Some(lang)
            }
        })
    });

    UserProfile {
        username,
        display_name,
        primary_language,
    }
}

// ============ LOCAL STORAGE COMMANDS ============

/// Load conversation memory from local storage
#[tauri::command]
fn load_memory() -> Result<LocalMemory, String> {
    local_storage::load_memory().map_err(|e| e.to_string())
}

/// Save conversation memory to local storage
#[tauri::command]
fn save_memory(memory: LocalMemory) -> Result<(), String> {
    local_storage::save_memory(&memory).map_err(|e| e.to_string())
}

/// Load custom system prompt from local storage
#[tauri::command]
fn load_custom_system_prompt() -> Result<CustomSystemPrompt, String> {
    local_storage::load_custom_system_prompt().map_err(|e| e.to_string())
}

/// Save custom system prompt to local storage
#[tauri::command]
fn save_custom_system_prompt(prompt: CustomSystemPrompt) -> Result<(), String> {
    local_storage::save_custom_system_prompt(&prompt).map_err(|e| e.to_string())
}

/// Add a new conversation to memory
#[tauri::command]
fn add_conversation_to_memory(
    title: String,
    messages: Vec<MemoryMessage>,
    model: Option<String>,
) -> Result<String, String> {
    local_storage::add_conversation(title, messages, model).map_err(|e| e.to_string())
}

/// Update an existing conversation in memory
#[tauri::command]
fn update_conversation_in_memory(
    id: String,
    messages: Vec<MemoryMessage>,
) -> Result<(), String> {
    local_storage::update_conversation(&id, messages).map_err(|e| e.to_string())
}

/// Delete a conversation from memory
#[tauri::command]
fn delete_conversation_from_memory(id: String) -> Result<(), String> {
    local_storage::delete_conversation(&id).map_err(|e| e.to_string())
}

/// Clear all conversations from memory
#[tauri::command]
fn clear_all_conversations() -> Result<(), String> {
    local_storage::clear_all_conversations().map_err(|e| e.to_string())
}

/// Get the path to the data directory
#[tauri::command]
fn get_data_directory() -> Result<String, String> {
    local_storage::get_data_directory().map_err(|e| e.to_string())
}

// ============ CALENDAR COMMANDS ============

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Data non valida: {}", e))
}

#[tauri::command]
fn load_calendar_events() -> Result<Vec<CalendarEvent>, String> {
    local_storage::load_calendar_events().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_calendar_event(event: CalendarEventInput) -> Result<String, String> {
    let start = parse_datetime(&event.start)?;
    let end = match event.end {
        Some(ref end_str) if !end_str.is_empty() => Some(parse_datetime(end_str)?),
        _ => None,
    };

    local_storage::add_calendar_event(
        event.title,
        event.description,
        start,
        end,
        event.source_text,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_calendar_event(event: CalendarEventInput) -> Result<(), String> {
    let id = event
        .id
        .clone()
        .ok_or_else(|| "ID evento mancante".to_string())?;
    let start = parse_datetime(&event.start)?;
    let end = match event.end {
        Some(ref end_str) if !end_str.is_empty() => Some(parse_datetime(end_str)?),
        _ => None,
    };

    let current_events = local_storage::load_calendar_events().map_err(|e| e.to_string())?;
    let original = current_events
        .into_iter()
        .find(|ev| ev.id == id)
        .ok_or_else(|| "Evento non trovato".to_string())?;

    let updated = CalendarEvent {
        id: original.id,
        title: event.title,
        description: event.description,
        start,
        end,
        source_text: event.source_text,
        created_at: original.created_at,
        updated_at: Utc::now(),
    };

    local_storage::update_calendar_event(updated).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_calendar_event(id: String) -> Result<(), String> {
    local_storage::delete_calendar_event(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_calendar_events() -> Result<(), String> {
    local_storage::clear_calendar_events().map_err(|e| e.to_string())
}

#[tauri::command]
fn export_calendar_to_ics() -> Result<String, String> {
    local_storage::export_calendar_to_ics().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_calendar_integrations_status() -> Result<CalendarIntegrationStatus, String> {
    calendar_integration::get_calendar_status().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_outlook_calendar_credentials(
    client_id: String,
    tenant: Option<String>,
) -> Result<CalendarIntegrationStatus, String> {
    calendar_integration::set_outlook_credentials(client_id, tenant).map_err(|e| e.to_string())
}

#[tauri::command]
fn disconnect_outlook_calendar() -> Result<CalendarIntegrationStatus, String> {
    calendar_integration::disconnect_outlook().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_google_calendar_credentials(
    client_id: String,
    client_secret: Option<String>,
    calendar_id: Option<String>,
) -> Result<CalendarIntegrationStatus, String> {
    let _ = client_secret;
    calendar_integration::set_google_credentials(client_id, calendar_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn disconnect_google_calendar() -> Result<CalendarIntegrationStatus, String> {
    calendar_integration::disconnect_google().map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_outlook_calendar_device_flow() -> Result<OutlookDeviceFlowStart, String> {
    calendar_integration::start_outlook_device_flow()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_google_calendar_device_flow() -> Result<OutlookDeviceFlowStart, String> {
    calendar_integration::start_google_device_flow()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn poll_outlook_calendar_device_flow() -> Result<OutlookDeviceFlowPoll, String> {
    calendar_integration::poll_outlook_device_flow()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn poll_google_calendar_device_flow() -> Result<OutlookDeviceFlowPoll, String> {
    calendar_integration::poll_google_device_flow()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_outlook_calendar_events(
    limit: Option<usize>,
) -> Result<Vec<RemoteCalendarEvent>, String> {
    calendar_integration::list_outlook_events(limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_google_calendar_events(
    limit: Option<usize>,
) -> Result<Vec<RemoteCalendarEvent>, String> {
    calendar_integration::list_google_events(limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_outlook_calendar_event(
    event: CreateRemoteEventRequest,
) -> Result<RemoteCalendarEvent, String> {
    calendar_integration::create_outlook_event(event)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_google_calendar_event(
    event: CreateRemoteEventRequest,
) -> Result<RemoteCalendarEvent, String> {
    calendar_integration::create_google_event(event)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn sync_calendar_event_to_integrations(id: String) -> Result<(), String> {
    let events = local_storage::load_calendar_events().map_err(|e| e.to_string())?;
    let event = events
        .into_iter()
        .find(|ev| ev.id == id)
        .ok_or_else(|| "Evento non trovato".to_string())?;

    let mut errors: Vec<String> = Vec::new();

    if let Err(err) = calendar_integration::push_local_event_to_outlook(&event).await {
        errors.push(format!("Outlook: {}", err));
    }

    if let Err(err) = calendar_integration::push_local_event_to_google(&event).await {
        errors.push(format!("Google: {}", err));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

#[tauri::command]
async fn is_outlook_calendar_connected() -> Result<bool, String> {
    calendar_integration::is_outlook_connected()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn is_google_calendar_connected() -> Result<bool, String> {
    calendar_integration::is_google_connected()
        .await
        .map_err(|e| e.to_string())
}

// ============ AICONNECT COMMANDS ============

/// Discovery result for AIConnect and Ollama services
#[derive(Debug, Clone, Serialize)]
struct DiscoveryResult {
    aiconnect_found: bool,
    aiconnect_services: Vec<DiscoveredService>,
    ollama_servers: Vec<String>,
    recommended_backend: BackendKind,
}

/// Scan network for AIConnect and Ollama services
#[tauri::command]
async fn scan_services() -> DiscoveryResult {
    use std::time::Duration;

    let mut aiconnect_services = Vec::new();
    let mut ollama_servers = Vec::new();
    let mut aiconnect_found = false;

    // Try mDNS discovery for AIConnect (with 2 second timeout)
    if let Ok(services) = aiconnect::discover_aiconnect(Duration::from_secs(2)).await {
        aiconnect_services = services;
        aiconnect_found = !aiconnect_services.is_empty();
    }

    // Discover Ollama instances advertised via mDNS
    if let Ok(services) = aiconnect::discover_ollama(Duration::from_secs(2)).await {
        for service in services {
            let url = service.base_url();
            if check_server(&url).await && !ollama_servers.contains(&url) {
                ollama_servers.push(url);
            }
        }
    }

    // Fall back to subnet scan (includes localhost) to preserve legacy behaviour
    let scanned_servers = scan_network().await;
    for server in scanned_servers {
        if !ollama_servers.contains(&server) {
            ollama_servers.push(server);
        }
    }

    // Determine recommended backend
    let recommended_backend = if aiconnect_found {
        BackendKind::AiConnect
    } else {
        BackendKind::OllamaLocal
    };

    DiscoveryResult {
        aiconnect_found,
        aiconnect_services,
        ollama_servers,
        recommended_backend,
    }
}

/// Get the current backend configuration
#[tauri::command]
async fn get_backend_config(state: State<'_, Arc<AppState>>) -> Result<BackendConfig, String> {
    let config = state.backend_config.lock().await;
    Ok(config.clone())
}

/// Set the backend configuration
#[tauri::command]
async fn set_backend_config(
    state: State<'_, Arc<AppState>>,
    config: BackendConfig,
) -> Result<(), String> {
    // Update the backend config
    {
        let mut backend = state.backend_config.lock().await;
        *backend = config.clone();
    }

    // Also update the AIConnect client configuration
    state.aiconnect_client.set_config(config.clone()).await;

    // Update ollama_url for backward compatibility
    {
        let mut ollama_url = state.ollama_url.lock().await;
        *ollama_url = config.endpoint;
    }

    Ok(())
}

/// Connect to AIConnect backend
#[tauri::command]
async fn connect_aiconnect(
    state: State<'_, Arc<AppState>>,
    endpoint: String,
    auth_method: Option<String>,
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), String> {
    // Build auth method
    let auth = match auth_method.as_deref() {
        Some("bearer") => {
            let token = token.ok_or("Token richiesto per autenticazione Bearer")?;
            AuthMethod::Bearer { token }
        }
        Some("basic") => {
            let username = username.ok_or("Username richiesto per autenticazione Basic")?;
            let password = password.ok_or("Password richiesta per autenticazione Basic")?;
            AuthMethod::Basic { username, password }
        }
        _ => AuthMethod::None,
    };

    // Check if AIConnect is reachable
    if !aiconnect::check_aiconnect_health(&endpoint, &auth).await {
        return Err("Impossibile connettersi ad AIConnect".to_string());
    }

    // Update configuration
    let config = BackendConfig {
        kind: BackendKind::AiConnect,
        endpoint: endpoint.clone(),
        auth,
        aiconnect_service: None,
    };

    // Update state
    {
        let mut backend = state.backend_config.lock().await;
        *backend = config.clone();
    }

    state.aiconnect_client.set_config(config).await;

    // Update ollama_url for backward compatibility with chat/models
    {
        let mut ollama_url = state.ollama_url.lock().await;
        *ollama_url = endpoint;
    }

    Ok(())
}

/// Get AIConnect nodes (only works when backend is AIConnect)
#[tauri::command]
async fn get_aiconnect_nodes(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AiConnectNode>, String> {
    let config = state.backend_config.lock().await;

    if config.kind != BackendKind::AiConnect {
        return Err("Questa funzione è disponibile solo con backend AIConnect".to_string());
    }

    drop(config);

    state
        .aiconnect_client
        .get_nodes()
        .await
        .map_err(|e| format!("Errore recupero nodi AIConnect: {}", e))
}

/// Check backend health (AIConnect or Ollama)
#[tauri::command]
async fn check_backend_health(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let config = state.backend_config.lock().await;

    let is_healthy = match config.kind {
        BackendKind::AiConnect => {
            aiconnect::check_aiconnect_health(&config.endpoint, &config.auth).await
        }
        BackendKind::OllamaLocal => aiconnect::check_ollama_health(&config.endpoint).await,
    };

    Ok(is_healthy)
}

/// Auto-discover and configure the best available backend
#[tauri::command]
async fn auto_configure(state: State<'_, Arc<AppState>>) -> Result<BackendConfig, String> {
    use std::time::Duration;

    let fallback_url = "http://localhost:11434";
    let config = aiconnect::auto_configure_backend(Duration::from_secs(3), fallback_url).await;

    // Update state
    {
        let mut backend = state.backend_config.lock().await;
        *backend = config.clone();
    }

    state.aiconnect_client.set_config(config.clone()).await;

    // Update ollama_url for backward compatibility
    {
        let mut ollama_url = state.ollama_url.lock().await;
        *ollama_url = config.endpoint.clone();
    }

    Ok(config)
}

// ============ MAIN ============

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
            get_user_profile,
            check_for_updates,
            download_and_install_update,
            // Local storage commands
            load_memory,
            save_memory,
            load_custom_system_prompt,
            save_custom_system_prompt,
            add_conversation_to_memory,
            update_conversation_in_memory,
            delete_conversation_from_memory,
            clear_all_conversations,
            get_data_directory,
            // Calendar commands
            load_calendar_events,
            add_calendar_event,
            update_calendar_event,
            delete_calendar_event,
            clear_calendar_events,
            export_calendar_to_ics,
            get_calendar_integrations_status,
            set_outlook_calendar_credentials,
            disconnect_outlook_calendar,
            start_outlook_calendar_device_flow,
            poll_outlook_calendar_device_flow,
            list_outlook_calendar_events,
            create_outlook_calendar_event,
            set_google_calendar_credentials,
            disconnect_google_calendar,
            start_google_calendar_device_flow,
            poll_google_calendar_device_flow,
            list_google_calendar_events,
            create_google_calendar_event,
            sync_calendar_event_to_integrations,
            is_outlook_calendar_connected,
            is_google_calendar_connected,
            // AIConnect commands
            scan_services,
            get_backend_config,
            set_backend_config,
            connect_aiconnect,
            get_aiconnect_nodes,
            check_backend_health,
            auto_configure,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

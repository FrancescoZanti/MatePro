// Agent module - Tool system for agentic features
// Migrated from egui app to Tauri backend

use crate::mcp_sql;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;
use uuid::Uuid;
use walkdir::WalkDir;

/// Tool definition with name, description and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub dangerous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Tool call extracted from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub raw_text: String,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub tool_name: String,
}

impl ToolResult {
    pub fn to_markdown(&self) -> String {
        if self.success {
            format!(
                "✅ **{}** eseguito con successo:\n```\n{}\n```",
                self.tool_name, self.output
            )
        } else {
            format!(
                "❌ **{}** fallito:\n```\n{}\n```",
                self.tool_name,
                self.error
                    .as_ref()
                    .unwrap_or(&"Errore sconosciuto".to_string())
            )
        }
    }
}

/// Agent system that manages tools
#[derive(Clone)]
pub struct AgentSystem {
    pub tools: HashMap<String, ToolDefinition>,
    pub allow_dangerous: bool,
    sql_manager: mcp_sql::SqlConnectionManager,
    last_sql_connection_id: Arc<Mutex<Option<String>>>,
}

impl AgentSystem {
    pub fn new() -> Self {
        let sql_manager = mcp_sql::SqlConnectionManager::new();
        let last_sql_connection_id = Arc::new(Mutex::new(None));
        Self::with_shared_state(sql_manager, last_sql_connection_id)
    }

    pub fn with_shared_state(
        sql_manager: mcp_sql::SqlConnectionManager,
        last_sql_connection_id: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let mut tools = HashMap::new();

        // Tool: ShellExecute
        tools.insert(
            "shell_execute".to_string(),
            ToolDefinition {
                name: "shell_execute".to_string(),
                description: "Esegue un comando shell. USALO per operazioni sul sistema."
                    .to_string(),
                parameters: vec![ToolParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Il comando bash da eseguire".to_string(),
                    required: true,
                }],
                dangerous: true,
            },
        );

        // Tool: FileRead
        tools.insert(
            "file_read".to_string(),
            ToolDefinition {
                name: "file_read".to_string(),
                description: "Legge il contenuto di un file.".to_string(),
                parameters: vec![ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Percorso del file da leggere".to_string(),
                    required: true,
                }],
                dangerous: false,
            },
        );

        // Tool: FileWrite
        tools.insert(
            "file_write".to_string(),
            ToolDefinition {
                name: "file_write".to_string(),
                description: "Scrive contenuto in un file (crea o sovrascrive).".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: "Percorso del file da scrivere".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "content".to_string(),
                        param_type: "string".to_string(),
                        description: "Contenuto da scrivere nel file".to_string(),
                        required: true,
                    },
                ],
                dangerous: true,
            },
        );

        // Tool: FileList
        tools.insert(
            "file_list".to_string(),
            ToolDefinition {
                name: "file_list".to_string(),
                description: "Lista file e directory in un percorso.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: "Percorso della directory da esplorare".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "recursive".to_string(),
                        param_type: "boolean".to_string(),
                        description: "Se true, cerca ricorsivamente".to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: ProcessList
        tools.insert(
            "process_list".to_string(),
            ToolDefinition {
                name: "process_list".to_string(),
                description: "Lista i processi attivi nel sistema.".to_string(),
                parameters: vec![],
                dangerous: false,
            },
        );

        // Tool: SystemInfo
        tools.insert(
            "system_info".to_string(),
            ToolDefinition {
                name: "system_info".to_string(),
                description: "Ottiene informazioni sul sistema (CPU, RAM, disco).".to_string(),
                parameters: vec![],
                dangerous: false,
            },
        );

        // Tool: BrowserOpen
        tools.insert(
            "browser_open".to_string(),
            ToolDefinition {
                name: "browser_open".to_string(),
                description: "Apre un URL nel browser predefinito.".to_string(),
                parameters: vec![ToolParameter {
                    name: "url".to_string(),
                    param_type: "string".to_string(),
                    description: "URL completo da aprire".to_string(),
                    required: true,
                }],
                dangerous: false,
            },
        );

        // Tool: WebSearch
        tools.insert(
            "web_search".to_string(),
            ToolDefinition {
                name: "web_search".to_string(),
                description: "Esegue una ricerca su Google.".to_string(),
                parameters: vec![ToolParameter {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "La query di ricerca".to_string(),
                    required: true,
                }],
                dangerous: false,
            },
        );

        // Tool: MapOpen
        tools.insert(
            "map_open".to_string(),
            ToolDefinition {
                name: "map_open".to_string(),
                description: "Apre Google Maps con una località o percorso.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "location".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome della località, indirizzo o coordinate".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "mode".to_string(),
                        param_type: "string".to_string(),
                        description: "Modalità: 'search' (default), 'directions' per percorsi"
                            .to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: YouTubeSearch
        tools.insert(
            "youtube_search".to_string(),
            ToolDefinition {
                name: "youtube_search".to_string(),
                description: "Cerca video su YouTube.".to_string(),
                parameters: vec![ToolParameter {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "La query di ricerca su YouTube".to_string(),
                    required: true,
                }],
                dangerous: false,
            },
        );

        // MCP SQL Server tools
        tools.insert(
            "sql_connect".to_string(),
            ToolDefinition {
                name: "sql_connect".to_string(),
                description: "Connette a un database SQL Server.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "server".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome o IP del server SQL".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "database".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome del database".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "auth_method".to_string(),
                        param_type: "string".to_string(),
                        description: "'windows' o 'sql'".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "username".to_string(),
                        param_type: "string".to_string(),
                        description: "Username SQL".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "password".to_string(),
                        param_type: "string".to_string(),
                        description: "Password SQL".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "trust_server_certificate".to_string(),
                        param_type: "boolean".to_string(),
                        description: "Imposta true per accettare certificati TLS non attendibili"
                            .to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        tools.insert(
            "sql_query".to_string(),
            ToolDefinition {
                name: "sql_query".to_string(),
                description: "Esegue query SELECT su database SQL Server (SOLO LETTURA)."
                    .to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: "Query SQL SELECT da eseguire".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        tools.insert(
            "sql_list_tables".to_string(),
            ToolDefinition {
                name: "sql_list_tables".to_string(),
                description: "Lista tutte le tabelle del database SQL Server.".to_string(),
                parameters: vec![ToolParameter {
                    name: "connection_id".to_string(),
                    param_type: "string".to_string(),
                    description: "ID della connessione SQL".to_string(),
                    required: false,
                }],
                dangerous: false,
            },
        );

        tools.insert(
            "sql_describe_table".to_string(),
            ToolDefinition {
                name: "sql_describe_table".to_string(),
                description: "Mostra struttura di una tabella SQL.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "schema".to_string(),
                        param_type: "string".to_string(),
                        description: "Schema della tabella (es: dbo)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "table".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome della tabella".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        tools.insert(
            "sql_disconnect".to_string(),
            ToolDefinition {
                name: "sql_disconnect".to_string(),
                description: "Chiude connessione SQL Server.".to_string(),
                parameters: vec![ToolParameter {
                    name: "connection_id".to_string(),
                    param_type: "string".to_string(),
                    description: "ID della connessione SQL da chiudere".to_string(),
                    required: false,
                }],
                dangerous: false,
            },
        );

        Self {
            tools,
            allow_dangerous: false,
            sql_manager,
            last_sql_connection_id,
        }
    }

    pub fn get_tools_description(&self) -> String {
        let mut desc = String::from(
            "**TOOLS DISPONIBILI** - Puoi usare questi tool per interagire con il sistema.\n\n",
        );
        desc.push_str("Per usare un tool, rispondi con il seguente formato JSON:\n");
        desc.push_str("```json\n{\n  \"tool\": \"nome_tool\",\n  \"parameters\": {\n    \"param1\": \"valore1\"\n  }\n}\n```\n\n");
        desc.push_str("**Lista Tool:**\n\n");

        for tool in self.tools.values() {
            desc.push_str(&format!("### {}\n", tool.name));
            desc.push_str(&format!("{}\n", tool.description));

            if !tool.parameters.is_empty() {
                desc.push_str("**Parametri:**\n");
                for param in &tool.parameters {
                    let required = if param.required {
                        "obbligatorio"
                    } else {
                        "opzionale"
                    };
                    desc.push_str(&format!(
                        "- `{}` ({}): {} - {}\n",
                        param.name, param.param_type, required, param.description
                    ));
                }
            }

            if tool.dangerous {
                desc.push_str("⚠️ *Tool pericoloso: richiede conferma utente*\n");
            }
            desc.push('\n');
        }

        desc
    }

    pub fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        let json_regex = regex::Regex::new(r"```json\s*(\{[^`]*\})\s*```").unwrap();

        for cap in json_regex.captures_iter(response) {
            if let Some(json_str) = cap.get(1) {
                let json_text = json_str.as_str();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_text) {
                    if let Some(tool_name) = value.get("tool").and_then(|v| v.as_str()) {
                        let parameters = value
                            .get("parameters")
                            .and_then(|v| v.as_object())
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();

                        calls.push(ToolCall {
                            tool_name: tool_name.to_string(),
                            parameters,
                            raw_text: json_text.to_string(),
                        });
                    }
                }
            }
        }

        calls
    }

    pub async fn execute_tool(&mut self, call: &ToolCall) -> Result<ToolResult> {
        let tool_def = self
            .tools
            .get(&call.tool_name)
            .context("Tool non trovato")?;

        if tool_def.dangerous && !self.allow_dangerous {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Tool pericoloso: conferma richiesta".to_string()),
                tool_name: call.tool_name.clone(),
            });
        }

        let result = match call.tool_name.as_str() {
            "shell_execute" => self.execute_shell(&call.parameters).await,
            "file_read" => self.execute_file_read(&call.parameters).await,
            "file_write" => self.execute_file_write(&call.parameters).await,
            "file_list" => self.execute_file_list(&call.parameters).await,
            "process_list" => self.execute_process_list().await,
            "system_info" => self.execute_system_info().await,
            "browser_open" => self.execute_browser_open(&call.parameters).await,
            "web_search" => self.execute_web_search(&call.parameters).await,
            "map_open" => self.execute_map_open(&call.parameters).await,
            "youtube_search" => self.execute_youtube_search(&call.parameters).await,
            "sql_connect" => self.execute_sql_connect(&call.parameters).await,
            "sql_query" => self.execute_sql_query(&call.parameters).await,
            "sql_list_tables" => self.execute_sql_list_tables(&call.parameters).await,
            "sql_describe_table" => self.execute_sql_describe_table(&call.parameters).await,
            "sql_disconnect" => self.execute_sql_disconnect(&call.parameters).await,
            _ => Err(anyhow::anyhow!("Tool non implementato: {}", call.tool_name)),
        };

        let tool_result = match result {
            Ok(output) => ToolResult {
                success: true,
                output,
                error: None,
                tool_name: call.tool_name.clone(),
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                tool_name: call.tool_name.clone(),
            },
        };

        if tool_def.dangerous {
            self.allow_dangerous = false;
        }

        Ok(tool_result)
    }

    pub fn set_allow_dangerous(&mut self, allow: bool) {
        self.allow_dangerous = allow;
    }

    async fn execute_shell(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .context("Parametro 'command' mancante")?;

        let output = if cfg!(target_os = "windows") {
            match Command::new("pwsh")
                .arg("-NoLogo")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
            {
                Ok(output) => output,
                Err(err) if err.kind() == ErrorKind::NotFound => Command::new("powershell")
                    .arg("-NoLogo")
                    .arg("-NoProfile")
                    .arg("-Command")
                    .arg(command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .with_context(|| "Errore esecuzione comando con PowerShell")?,
                Err(err) => {
                    return Err(anyhow!("Errore esecuzione comando pwsh: {}", err));
                }
            }
        } else {
            Command::new("bash")
                .arg("-lc")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .context("Errore esecuzione comando")?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(format!("{}{}", stdout, stderr))
        } else {
            Err(anyhow::anyhow!(
                "Comando fallito (exit {}): {}{}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            ))
        }
    }

    async fn execute_file_read(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        let content =
            fs::read_to_string(path).context(format!("Impossibile leggere file: {}", path))?;
        Ok(content)
    }

    async fn execute_file_write(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .context("Parametro 'content' mancante")?;

        fs::write(path, content).context(format!("Impossibile scrivere file: {}", path))?;
        Ok(format!("File scritto: {} ({} bytes)", path, content.len()))
    }

    async fn execute_file_list(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        let recursive = params
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut entries = Vec::new();

        if recursive {
            for entry in WalkDir::new(path).max_depth(5) {
                if let Ok(e) = entry {
                    entries.push(e.path().display().to_string());
                }
            }
        } else {
            for entry in fs::read_dir(path)? {
                if let Ok(e) = entry {
                    entries.push(e.path().display().to_string());
                }
            }
        }

        Ok(entries.join("\n"))
    }

    async fn execute_process_list(&self) -> Result<String> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut processes: Vec<String> = sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                format!(
                    "PID: {} | Name: {} | CPU: {:.1}% | Memory: {} MB",
                    pid,
                    process.name(),
                    process.cpu_usage(),
                    process.memory() / 1024 / 1024
                )
            })
            .collect();

        processes.sort();
        processes.truncate(50);
        Ok(processes.join("\n"))
    }

    async fn execute_system_info(&self) -> Result<String> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let total_memory = sys.total_memory() / 1024 / 1024;
        let used_memory = sys.used_memory() / 1024 / 1024;
        let total_swap = sys.total_swap() / 1024 / 1024;
        let used_swap = sys.used_swap() / 1024 / 1024;

        let info = format!(
            "Sistema: {}\nKernel: {}\nCPU: {} cores\nRAM: {} MB / {} MB ({:.1}%)\nSwap: {} MB / {} MB\nProcessi attivi: {}",
            System::name().unwrap_or_else(|| "Unknown".to_string()),
            System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            sys.cpus().len(),
            used_memory,
            total_memory,
            (used_memory as f64 / total_memory as f64) * 100.0,
            used_swap,
            total_swap,
            sys.processes().len()
        );

        Ok(info)
    }

    async fn execute_browser_open(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let url_str = params
            .get("url")
            .and_then(|v| v.as_str())
            .context("Parametro 'url' mancante")?;

        // URL will be opened by the frontend via tauri-plugin-opener
        Ok(format!("URL: {}", url_str))
    }

    async fn execute_web_search(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .context("Parametro 'query' mancante")?;

        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://www.google.com/search?q={}", encoded_query);
        Ok(format!("URL: {}", search_url))
    }

    async fn execute_map_open(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let location = params
            .get("location")
            .and_then(|v| v.as_str())
            .context("Parametro 'location' mancante")?;

        let mode = params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("search");

        let encoded_location = urlencoding::encode(location);

        let map_url = match mode {
            "directions" => format!(
                "https://www.google.com/maps/dir/?api=1&destination={}",
                encoded_location
            ),
            _ => format!(
                "https://www.google.com/maps/search/?api=1&query={}",
                encoded_location
            ),
        };

        Ok(format!("URL: {}", map_url))
    }

    async fn execute_youtube_search(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .context("Parametro 'query' mancante")?;

        let encoded_query = urlencoding::encode(query);
        let youtube_url = format!(
            "https://www.youtube.com/results?search_query={}",
            encoded_query
        );
        Ok(format!("URL: {}", youtube_url))
    }

    async fn execute_sql_connect(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let server = params
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'server' mancante"))?;

        let database = params
            .get("database")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'database' mancante"))?;

        let auth_method = params
            .get("auth_method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'auth_method' mancante (usa 'windows' o 'sql')"))?;

        let trust_server_certificate = params
            .get("trust_server_certificate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let connection_id = format!("sql_{}", Uuid::new_v4());

        let mut stored_username = None;
        let mut stored_password = None;

        let client = if auth_method.eq_ignore_ascii_case("windows") {
            mcp_sql::connect_windows_auth(server, database, trust_server_certificate).await?
        } else if auth_method.eq_ignore_ascii_case("sql") {
            let username = params
                .get("username")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Parametro 'username' richiesto per SQL auth"))?;

            let password = params
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Parametro 'password' richiesto per SQL auth"))?;

            stored_username = Some(username.to_string());
            stored_password = Some(password.to_string());

            mcp_sql::connect_sql_auth(
                server,
                database,
                username,
                password,
                trust_server_certificate,
            )
            .await?
        } else {
            return Err(anyhow!("auth_method non valido: usa 'windows' o 'sql'"));
        };

        drop(client);

        let conn_info = mcp_sql::SqlConnection {
            connection_id: connection_id.clone(),
            server: server.to_string(),
            database: database.to_string(),
            auth_type: auth_method.to_string(),
            username: stored_username,
            password: stored_password,
            trust_server_certificate,
        };

        self.sql_manager.add_connection(conn_info);

        {
            let mut last_conn = self.last_sql_connection_id.lock().await;
            *last_conn = Some(connection_id.clone());
        }

        Ok(format!(
            "✅ Connessione riuscita\nConnection ID: {}\nServer: {}\nDatabase: {}\nAutenticazione: {}\nTrust certificato TLS: {}",
            connection_id,
            server,
            database,
            auth_method,
            if trust_server_certificate { "disabilitata" } else { "attiva" }
        ))
    }

    async fn execute_sql_query(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = self.resolve_connection_id(params).await?;

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'query' mancante"))?;

        let conn_info = self
            .sql_manager
            .get_connection(&connection_id)
            .ok_or_else(|| {
                anyhow!(
                    "Connessione '{}' non trovata. Esegui prima sql_connect.",
                    connection_id
                )
            })?;

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;

        let result = mcp_sql::run_query(&mut client, query).await?;
        let summary = summarize_query_result(&result);
        let table_preview = render_result_table(&result, 20);
        let payload = json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&payload)?;

        let mut response = String::new();
        response.push_str("📊 Risultato query\n");
        response.push_str(&summary);

        if let Some(table) = table_preview {
            response.push_str("\n\n**Anteprima dati**\n");
            response.push_str(&table);
        }

        response.push_str("\n\n**JSON completo**\n```json\n");
        response.push_str(&json_pretty);
        response.push_str("\n```\n");

        Ok(response)
    }

    async fn execute_sql_list_tables(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = self.resolve_connection_id(params).await?;

        let conn_info = self
            .sql_manager
            .get_connection(&connection_id)
            .ok_or_else(|| {
                anyhow!(
                    "Connessione '{}' non trovata. Esegui prima sql_connect.",
                    connection_id
                )
            })?;

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;
        let result = mcp_sql::list_tables(&mut client).await?;

        let total_items = result.rows.len();
        let base_tables = result
            .rows
            .iter()
            .filter(|row| matches_ignore_case(row.get("Type"), "BASE TABLE"))
            .count();
        let views = result
            .rows
            .iter()
            .filter(|row| matches_ignore_case(row.get("Type"), "VIEW"))
            .count();
        let table_preview = render_result_table(&result, 40);
        let payload = json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&payload)?;

        let mut response = String::new();
        response.push_str("📋 Tabelle disponibili\n");
        response.push_str(&format!(
            "- elementi totali: {}\n- tabelle: {}\n- viste: {}\n",
            total_items, base_tables, views
        ));

        if let Some(table) = table_preview {
            response.push_str("\n**Dettaglio**\n");
            response.push_str(&table);
        }

        response.push_str("\n\n**JSON completo**\n```json\n");
        response.push_str(&json_pretty);
        response.push_str("\n```\n");

        Ok(response)
    }

    async fn execute_sql_describe_table(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = self.resolve_connection_id(params).await?;

        let schema = params
            .get("schema")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'schema' mancante"))?;

        let table = params
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'table' mancante"))?;

        let conn_info = self
            .sql_manager
            .get_connection(&connection_id)
            .ok_or_else(|| {
                anyhow!(
                    "Connessione '{}' non trovata. Esegui prima sql_connect.",
                    connection_id
                )
            })?;

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;
        let result = mcp_sql::describe_table(&mut client, schema, table).await?;

        let total_columns = result.rows.len();
        let highlights: Vec<String> = result
            .rows
            .iter()
            .take(5)
            .map(|row| describe_column_row(row))
            .collect();
        let highlight_text = if highlights.is_empty() {
            "nessuna colonna trovata".to_string()
        } else {
            highlights.join("; ")
        };
        let table_preview = render_result_table(&result, 50);
        let payload = json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&payload)?;

        let mut response = String::new();
        response.push_str(&format!("🔍 Struttura tabella {}.{}\n", schema, table));
        response.push_str(&format!(
            "- colonne totali: {}\n- anteprima: {}\n",
            total_columns, highlight_text
        ));

        if let Some(table_markdown) = table_preview {
            response.push_str("\n**Dettaglio**\n");
            response.push_str(&table_markdown);
        }

        response.push_str("\n\n**JSON completo**\n```json\n");
        response.push_str(&json_pretty);
        response.push_str("\n```\n");

        Ok(response)
    }

    async fn execute_sql_disconnect(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = if let Some(id) = params.get("connection_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            let last = self.last_sql_connection_id.lock().await;
            last.clone()
                .ok_or_else(|| anyhow!("Nessuna connessione SQL attiva da chiudere"))?
        };

        let removed = self.sql_manager.remove_connection(&connection_id);
        if removed.is_none() {
            return Err(anyhow!("Connessione '{}' non trovata", connection_id));
        }

        {
            let mut last = self.last_sql_connection_id.lock().await;
            if last.as_ref() == Some(&connection_id) {
                *last = None;
            }
        }

        Ok(format!(
            "✅ Connessione '{}' chiusa correttamente.",
            connection_id
        ))
    }

    async fn resolve_connection_id(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        if let Some(id) = params.get("connection_id").and_then(|v| v.as_str()) {
            let mut last = self.last_sql_connection_id.lock().await;
            let id_string = id.to_string();
            *last = Some(id_string.clone());
            Ok(id_string)
        } else {
            let last = self.last_sql_connection_id.lock().await;
            if let Some(id) = last.as_ref() {
                Ok(id.clone())
            } else {
                Err(anyhow!(
                    "Nessun connection_id fornito e nessuna connessione SQL attiva trovata. Esegui prima sql_connect."
                ))
            }
        }
    }
}

fn summarize_query_result(result: &mcp_sql::QueryResult) -> String {
    let total_rows = result.rows.len();
    let total_columns = result.columns.len();

    let column_highlights: Vec<String> = result
        .columns
        .iter()
        .take(3)
        .map(|column| summarize_column(result, column))
        .collect();

    let highlight_text = if column_highlights.is_empty() {
        "nessun dato disponibile".to_string()
    } else {
        column_highlights.join("; ")
    };

    format!(
        "- righe: {}\n- colonne: {}\n- colonne principali: {}\n",
        total_rows, total_columns, highlight_text
    )
}

fn summarize_column(result: &mcp_sql::QueryResult, column: &mcp_sql::SqlColumnInfo) -> String {
    if is_numeric_type(&column.data_type) {
        let mut numeric_values = Vec::new();
        for row in &result.rows {
            if let Some(value) = row.get(&column.name) {
                if let Some(number) = value_to_f64(value) {
                    numeric_values.push(number);
                }
            }
        }

        if !numeric_values.is_empty() {
            let count = numeric_values.len();
            let (min, max, mean) = compute_basic_stats(&numeric_values);
            return format!(
                "{} {}: {} valori, min {:.3}, max {:.3}, media {:.3}",
                column.name, column.data_type, count, min, max, mean
            );
        }
    }

    if column.data_type == "bit" {
        let mut true_count = 0usize;
        let mut false_count = 0usize;
        for row in &result.rows {
            if let Some(value) = row.get(&column.name) {
                if let Some(flag) = value.as_bool() {
                    if flag {
                        true_count += 1;
                    } else {
                        false_count += 1;
                    }
                }
            }
        }
        if true_count + false_count > 0 {
            return format!(
                "{} bit: {} veri, {} falsi",
                column.name, true_count, false_count
            );
        }
    }

    let mut samples = Vec::new();
    for row in &result.rows {
        if let Some(value) = row.get(&column.name) {
            if value.is_null() {
                continue;
            }
            let display = value_to_display(value);
            if display.is_empty() {
                continue;
            }
            if !samples.contains(&display) {
                samples.push(display);
            }
            if samples.len() == 3 {
                break;
            }
        }
    }

    if samples.is_empty() {
        format!("{} {}: solo valori null", column.name, column.data_type)
    } else {
        format!(
            "{} {}: esempi {}",
            column.name,
            column.data_type,
            samples.join(", ")
        )
    }
}

fn render_result_table(result: &mcp_sql::QueryResult, max_rows: usize) -> Option<String> {
    if result.columns.is_empty() {
        return None;
    }

    let headers: Vec<String> = result
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect();

    let mut table = String::new();
    table.push_str("| ");
    table.push_str(&headers.join(" | "));
    table.push_str(" |");
    table.push('\n');
    table.push_str("| ");
    table.push_str(
        &headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    table.push_str(" |");
    table.push('\n');

    for row in result.rows.iter().take(max_rows) {
        table.push_str("| ");
        let mut cells = Vec::new();
        for column in &result.columns {
            let value = row.get(&column.name).unwrap_or(&serde_json::Value::Null);
            let display = escape_markdown_cell(&value_to_display(value));
            cells.push(display);
        }
        table.push_str(&cells.join(" | "));
        table.push_str(" |");
        table.push('\n');
    }

    if result.rows.len() > max_rows {
        table.push_str(&format!(
            "_{} righe aggiuntive non mostrate_\n",
            result.rows.len() - max_rows
        ));
    }

    Some(table)
}

fn matches_ignore_case(value: Option<&serde_json::Value>, expected: &str) -> bool {
    value
        .and_then(|val| val.as_str())
        .map(|val| val.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

fn describe_column_row(row: &HashMap<String, serde_json::Value>) -> String {
    let column = row
        .get("Column")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    let data_type = row
        .get("Type")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    let nullable = row
        .get("Nullable")
        .and_then(|value| value.as_str())
        .map(|value| {
            if value.eq_ignore_ascii_case("YES") {
                "NULL"
            } else {
                "NOT NULL"
            }
        })
        .unwrap_or("sconosciuto");

    format!("{} {} ({})", column, data_type, nullable)
}

fn value_to_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(flag) => flag.to_string(),
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::String(text) => truncate_string(text, 60),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            truncate_string(&value.to_string(), 60)
        }
    }
}

fn truncate_string(input: &str, max_len: usize) -> String {
    let sanitized = input.replace('\n', " ").trim().to_string();
    if sanitized.chars().count() > max_len {
        let mut truncated = sanitized
            .chars()
            .take(max_len.saturating_sub(3))
            .collect::<String>();
        truncated.push_str("...");
        truncated
    } else {
        sanitized
    }
}

fn escape_markdown_cell(text: &str) -> String {
    text.replace('|', "\\|")
        .replace('\n', " ")
        .trim()
        .to_string()
}

fn value_to_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn compute_basic_stats(values: &[f64]) -> (f64, f64, f64) {
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    let mut sum = 0.0f64;

    for value in values {
        if *value < min_value {
            min_value = *value;
        }
        if *value > max_value {
            max_value = *value;
        }
        sum += *value;
    }

    let mean = if values.is_empty() {
        0.0
    } else {
        sum / values.len() as f64
    };

    (min_value, max_value, mean)
}

fn is_numeric_type(data_type: &str) -> bool {
    matches!(data_type, "int" | "float" | "decimal" | "money")
}

impl Default for AgentSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls() {
        let agent = AgentSystem::new();
        let response = r#"
Per eseguire questo compito userò il tool shell_execute:

```json
{
  "tool": "shell_execute",
  "parameters": {
    "command": "ls -la"
  }
}
```

Questo comando lista tutti i file.
        "#;

        let calls = agent.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "shell_execute");
    }
}

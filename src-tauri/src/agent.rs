// Agent module - Tool system for agentic features
// Migrated from egui app to Tauri backend

use crate::mcp_sql;
use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook, Data, Ods, Range, Reader, Xls, Xlsx};
use lazy_static::lazy_static;
use lopdf::Document;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::io::{BufReader, ErrorKind, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;
use uuid::Uuid;
use walkdir::WalkDir;
use zip::read::ZipArchive;

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

        tools.insert(
            "text_translate".to_string(),
            ToolDefinition {
                name: "text_translate".to_string(),
                description:
                    "Traduce un testo in un'altra lingua utilizzando servizi di traduzione online"
                        .to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "text".to_string(),
                        param_type: "string".to_string(),
                        description: "Testo da tradurre (max 1500 caratteri)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "target_language".to_string(),
                        param_type: "string".to_string(),
                        description: "Lingua di destinazione (ISO code es: it, en, es)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "source_language".to_string(),
                        param_type: "string".to_string(),
                        description: "Lingua sorgente (ISO code). Default rilevamento automatico"
                            .to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        tools.insert(
            "document_summarize".to_string(),
            ToolDefinition {
                name: "document_summarize".to_string(),
                description:
                    "Crea un riassunto compatto di un documento di testo, PDF, Excel o Word."
                        .to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: "Percorso del file da riassumere".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "max_sentences".to_string(),
                        param_type: "integer".to_string(),
                        description: "Numero massimo di frasi nel riassunto (default 5)"
                            .to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        tools.insert(
            "excel_improve".to_string(),
            ToolDefinition {
                name: "excel_improve".to_string(),
                description:
                    "Analizza un file Excel e suggerisce miglioramenti (metriche, grafici, pulizia dati)."
                        .to_string(),
                parameters: vec![ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Percorso del file Excel (.xlsx o .xls)".to_string(),
                    required: true,
                }],
                dangerous: false,
            },
        );

        tools.insert(
            "word_improve".to_string(),
            ToolDefinition {
                name: "word_improve".to_string(),
                description:
                    "Analizza un documento Word (.docx) e propone miglioramenti di stile e leggibilità."
                        .to_string(),
                parameters: vec![ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Percorso del file Word".to_string(),
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
            "text_translate" => self.execute_text_translate(&call.parameters).await,
            "document_summarize" => self.execute_document_summarize(&call.parameters).await,
            "excel_improve" => self.execute_excel_improve(&call.parameters).await,
            "word_improve" => self.execute_word_improve(&call.parameters).await,
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

    async fn execute_text_translate(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Parametro 'text' mancante o vuoto"))?;

        if text.chars().count() > 1_500 {
            anyhow::bail!("Testo troppo lungo: massimo 1500 caratteri");
        }

        let target_language = params
            .get("target_language")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Parametro 'target_language' mancante"))?
            .to_lowercase();

        let source_language = params
            .get("source_language")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("auto");

        let encoded_text = urlencoding::encode(text);
        let langpair = format!("{}|{}", source_language, target_language);

        let url = format!(
            "https://api.mymemory.translated.net/get?q={}&langpair={}",
            encoded_text, langpair
        );

        let client = Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Errore richiesta traduzione")?
            .error_for_status()
            .context("Risposta traduzione non valida")?;

        let payload: serde_json::Value = response
            .json()
            .await
            .context("Errore parsing risposta traduzione")?;

        let translated = payload["responseData"]["translatedText"]
            .as_str()
            .unwrap_or_default()
            .trim();

        if translated.is_empty() {
            anyhow::bail!("Traduzione non disponibile");
        }

        let mut output = String::new();
        output.push_str("🌐 Traduzione completata\n");
        output.push_str(&format!("- Sorgente: {}\n", source_language));
        output.push_str(&format!("- Destinazione: {}\n\n", target_language));
        output.push_str("**Risultato**\n");
        output.push_str(translated);

        Ok(output)
    }

    async fn execute_document_summarize(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'path' mancante"))?;

        let max_sentences = params
            .get("max_sentences")
            .and_then(|v| v.as_i64())
            .map(|n| n.max(1).min(10) as usize)
            .unwrap_or(5);

        let text = extract_text_from_path(Path::new(path))
            .with_context(|| format!("Impossibile leggere il documento: {}", path))?;

        if text.trim().is_empty() {
            anyhow::bail!("Il documento non contiene testo analizzabile");
        }

        let summary = summarize_text(&text, max_sentences);
        let stats = compute_text_statistics(&text);

        let mut output = String::new();
        output.push_str("📝 Riassunto documento\n");
        output.push_str(&format!(
            "- parole totali: {}\n- frasi stimate: {}\n- lunghezza media frase: {:.1} parole\n\n",
            stats.word_count, stats.sentence_count, stats.avg_sentence_len
        ));
        output.push_str("**Riassunto**\n");
        output.push_str(&summary);

        Ok(output)
    }

    async fn execute_excel_improve(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'path' mancante"))?;

        let improvement = analyze_excel(Path::new(path))
            .with_context(|| format!("Impossibile analizzare il file Excel: {}", path))?;

        Ok(improvement)
    }

    async fn execute_word_improve(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Parametro 'path' mancante"))?;

        let improvement = analyze_word_document(Path::new(path))
            .with_context(|| format!("Impossibile analizzare il file Word: {}", path))?;

        Ok(improvement)
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

        let requested_trust = params
            .get("trust_server_certificate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let connection_id = format!("sql_{}", Uuid::new_v4());

        let mut stored_username = None;
        let mut stored_password = None;

        let (client, effective_trust, auto_trust_applied) =
            if auth_method.eq_ignore_ascii_case("windows") {
                connect_with_optional_trust(
                    |trust| mcp_sql::connect_windows_auth(server, database, trust),
                    requested_trust,
                )
                .await?
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

                connect_with_optional_trust(
                    |trust| mcp_sql::connect_sql_auth(server, database, username, password, trust),
                    requested_trust,
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
            trust_server_certificate: effective_trust,
        };

        self.sql_manager.add_connection(conn_info);

        {
            let mut last_conn = self.last_sql_connection_id.lock().await;
            *last_conn = Some(connection_id.clone());
        }

        let mut response = format!(
            "✅ Connessione riuscita\nConnection ID: {}\nServer: {}\nDatabase: {}\nAutenticazione: {}\nTrust certificato TLS: {}",
            connection_id,
            server,
            database,
            auth_method,
            if effective_trust { "disabilitata" } else { "attiva" }
        );

        if auto_trust_applied && !requested_trust {
            response.push_str(
                "\n⚠️ Il certificato TLS presentato dal server non è attendibile. La connessione è stata completata accettando il certificato per proseguire.",
            );
        }

        Ok(response)
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

async fn connect_with_optional_trust<F, Fut>(
    mut connect_fn: F,
    requested_trust: bool,
) -> Result<(mcp_sql::SqlClient, bool, bool)>
where
    F: FnMut(bool) -> Fut,
    Fut: Future<Output = Result<mcp_sql::SqlClient>>,
{
    let mut trust = requested_trust;
    let mut fallback_used = false;

    match connect_fn(trust).await {
        Ok(client) => Ok((client, trust, fallback_used)),
        Err(err) => {
            if !trust && is_certificate_error(&err) {
                let first_err_msg = err.to_string();
                trust = true;
                fallback_used = true;
                match connect_fn(trust).await {
                    Ok(client) => Ok((client, trust, fallback_used)),
                    Err(second_err) => Err(anyhow!(
                        "Connessione TLS fallita ({}). Ritentativo accettando il certificato non riuscito: {}",
                        first_err_msg,
                        second_err
                    )),
                }
            } else {
                Err(err)
            }
        }
    }
}

fn is_certificate_error(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_lowercase();
    message.contains("certificate")
        || message.contains("unknownissuer")
        || message.contains("unknown issuer")
        || message.contains("tls")
}

#[derive(Debug, Clone)]
struct TextStatistics {
    word_count: usize,
    sentence_count: usize,
    avg_sentence_len: f64,
}

fn extract_text_from_path(path: &Path) -> Result<String> {
    if !path.exists() {
        anyhow::bail!("File non trovato: {}", path.display());
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let text = match extension.as_str() {
        "pdf" => extract_text_from_pdf(path)?,
        "xlsx" | "xls" | "ods" => extract_text_from_spreadsheet(path)?,
        "docx" => extract_text_from_docx(path)?,
        "txt" | "md" | "csv" => fs::read_to_string(path)?,
        other => anyhow::bail!("Formato file non supportato per riassunto: {}", other),
    };

    Ok(normalize_whitespace(&text))
}

fn extract_text_from_pdf(path: &Path) -> Result<String> {
    let doc = Document::load(path)?;
    let mut text = String::new();

    for page_num in 1..=doc.get_pages().len() {
        if let Ok(page_text) = doc.extract_text(&[page_num as u32]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    Ok(text)
}

fn extract_text_from_spreadsheet(path: &Path) -> Result<String> {
    let mut output = String::new();

    if path
        .extension()
        .and_then(|e| e.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("xlsx"))
    {
        let mut workbook: Xlsx<_> = open_workbook(path)?;
        let sheet_names = workbook.sheet_names();
        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                append_range_text(&mut output, &sheet_name, &range);
            }
        }
    } else if path
        .extension()
        .and_then(|e| e.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("xls"))
    {
        let mut workbook: Xls<_> = open_workbook(path)?;
        let sheet_names = workbook.sheet_names();
        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                append_range_text(&mut output, &sheet_name, &range);
            }
        }
    } else {
        let mut workbook: Ods<_> = open_workbook(path)?;
        let sheet_names = workbook.sheet_names();
        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                append_range_text(&mut output, &sheet_name, &range);
            }
        }
    }

    Ok(output)
}

fn append_range_text(output: &mut String, sheet_name: &str, range: &Range<Data>) {
    output.push_str(&format!("=== Foglio: {} ===\n", sheet_name));
    for row in range.rows() {
        let values: Vec<String> = row
            .iter()
            .map(|cell| match cell {
                Data::Empty => String::new(),
                Data::String(s) => s.to_string(),
                Data::Float(f) => format!("{:.4}", f),
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::DateTime(dt) => dt.to_string(),
                Data::DateTimeIso(dt) => dt.to_string(),
                _ => cell.to_string(),
            })
            .collect();
        output.push_str(&values.join("\t"));
        output.push('\n');
    }
    output.push('\n');
}

fn extract_text_from_docx(path: &Path) -> Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut document = archive.by_name("word/document.xml")?;
    let mut xml_content = String::new();
    document.read_to_string(&mut xml_content)?;

    lazy_static! {
        static ref DOCX_TAG_REGEX: Regex = Regex::new(r"<[^>]+>").unwrap();
    }

    let text = DOCX_TAG_REGEX.replace_all(&xml_content, " ");
    Ok(normalize_whitespace(&text))
}

fn summarize_text(text: &str, max_sentences: usize) -> String {
    let sentences = sentence_tokenize(text);
    if sentences.len() <= max_sentences {
        return sentences.join(" \n");
    }

    let mut frequencies: HashMap<String, usize> = HashMap::new();
    for sentence in &sentences {
        for token in tokenize_sentence(sentence) {
            if token.len() > 2 && !STOPWORDS.contains(token.as_str()) {
                *frequencies.entry(token).or_insert(0) += 1;
            }
        }
    }

    let mut scored: Vec<(usize, f64)> = sentences
        .iter()
        .enumerate()
        .map(|(idx, sentence)| {
            let mut score = 0.0;
            for token in tokenize_sentence(sentence) {
                if let Some(freq) = frequencies.get(&token) {
                    score += *freq as f64;
                }
            }
            (idx, score)
        })
        .collect();

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut selected: Vec<usize> = scored
        .into_iter()
        .take(max_sentences)
        .map(|(idx, _)| idx)
        .collect();
    selected.sort_unstable();

    selected
        .into_iter()
        .map(|idx| sentences[idx].clone())
        .collect::<Vec<_>>()
        .join(" \n")
}

fn compute_text_statistics(text: &str) -> TextStatistics {
    let sentences = sentence_tokenize(text);
    let sentence_count = sentences.len().max(1);
    let word_count: usize = sentences.iter().map(|s| s.split_whitespace().count()).sum();

    TextStatistics {
        word_count,
        sentence_count,
        avg_sentence_len: word_count as f64 / sentence_count as f64,
    }
}

fn analyze_excel(path: &Path) -> Result<String> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "xlsx" => {
            let workbook: Xlsx<_> = open_workbook(path)?;
            analyze_excel_with_workbook(workbook)
        }
        "xls" => {
            let workbook: Xls<_> = open_workbook(path)?;
            analyze_excel_with_workbook(workbook)
        }
        "ods" => {
            let workbook: Ods<_> = open_workbook(path)?;
            analyze_excel_with_workbook(workbook)
        }
        other => anyhow::bail!("Formato non supportato per Excel: {}", other),
    }
}

fn analyze_excel_with_workbook<W>(mut workbook: W) -> Result<String>
where
    W: Reader<BufReader<fs::File>>,
{
    let sheet_names = workbook.sheet_names();

    if sheet_names.is_empty() {
        anyhow::bail!("Il file non contiene fogli");
    }

    let mut report = String::new();
    report.push_str("📊 Miglioramento Excel\n");

    for sheet in sheet_names {
        let range = workbook
            .worksheet_range(&sheet)
            .map_err(|err| anyhow!("Foglio non leggibile {}: {:?}", sheet, err))?;

        let mut row_count = 0usize;
        let mut column_count = 0usize;
        let mut numeric_columns: HashSet<usize> = HashSet::new();
        let mut text_columns: HashSet<usize> = HashSet::new();
        let mut empty_cells = 0usize;
        let mut total_cells = 0usize;

        for row in range.rows() {
            row_count += 1;
            column_count = column_count.max(row.len());

            for (idx, cell) in row.iter().enumerate() {
                total_cells += 1;
                match cell {
                    Data::Empty => empty_cells += 1,
                    Data::Float(_) | Data::Int(_) | Data::Bool(_) => {
                        numeric_columns.insert(idx);
                    }
                    Data::DateTime(_) | Data::DateTimeIso(_) => {
                        numeric_columns.insert(idx);
                    }
                    Data::String(s) => {
                        if s.trim().parse::<f64>().is_ok() {
                            numeric_columns.insert(idx);
                        } else if !s.trim().is_empty() {
                            text_columns.insert(idx);
                        }
                    }
                    _ => {}
                }
            }
        }

        let fill_ratio = if total_cells == 0 {
            0.0
        } else {
            1.0 - (empty_cells as f64 / total_cells as f64)
        };

        report.push_str(&format!(
            "\n**Foglio: {}**\n- righe: {}\n- colonne utilizzate: {}\n- riempimento celle: {:.0}%\n",
            sheet,
            row_count,
            column_count,
            fill_ratio * 100.0
        ));

        let mut suggestions: Vec<String> = Vec::new();

        if column_count > 15 {
            suggestions
                .push("Valuta la suddivisione del foglio o l'uso di una tabella pivot".to_string());
        }

        if !numeric_columns.is_empty() {
            let columns_list = numeric_columns
                .iter()
                .map(|idx| column_name_from_index(*idx))
                .collect::<Vec<_>>()
                .join(", ");
            suggestions.push(format!(
                "Numeri nelle colonne {}: puoi aggiungere grafici, subtotali o KPI",
                columns_list
            ));
        }

        if fill_ratio < 0.7 {
            suggestions
                .push("Molte celle vuote: valuta la pulizia o la convalida dei dati".to_string());
        }

        if !text_columns.is_empty() {
            suggestions.push(
                "Aggiungi filtri e regole di formattazione per le colonne testuali rilevanti"
                    .to_string(),
            );
        }

        if suggestions.is_empty() {
            report.push_str("- Nessun miglioramento consigliato: struttura già ottimizzata\n");
        } else {
            report.push_str("- Suggerimenti:\n");
            for suggestion in suggestions {
                report.push_str(&format!("  • {}\n", suggestion));
            }
        }
    }

    Ok(report)
}

fn analyze_word_document(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let raw_text = match ext.as_str() {
        "docx" => extract_text_from_docx(path)?,
        "txt" | "md" => fs::read_to_string(path)?,
        other => anyhow::bail!("Formato non supportato per miglioramento Word: {}", other),
    };

    let text = normalize_whitespace(&raw_text);
    if text.is_empty() {
        anyhow::bail!("Il documento è vuoto o non contiene testo leggibile");
    }

    let stats = compute_text_statistics(&text);
    let sentences = sentence_tokenize(&text);
    let long_sentences: Vec<&str> = sentences
        .iter()
        .filter(|s| s.split_whitespace().count() > 25)
        .take(3)
        .map(|s| s.as_str())
        .collect();

    let mut suggestions: Vec<String> = Vec::new();

    if stats.avg_sentence_len > 20.0 {
        suggestions.push(
            "Riduci la lunghezza media delle frasi per migliorare la leggibilità (target < 20 parole)"
                .to_string(),
        );
    }

    if long_sentences.len() >= 3 {
        suggestions.push("Spezza le frasi molto lunghe per agevolare la comprensione".to_string());
    }

    let repeated_words = detect_repeated_words(&text);
    if !repeated_words.is_empty() {
        suggestions.push(format!(
            "Varietà lessicale: sostituisci parole ripetute frequentemente ({})",
            repeated_words.join(", ")
        ));
    }

    let mut report = String::new();
    report.push_str("🖋️ Miglioramento documento Word\n");
    report.push_str(&format!(
        "- parole totali: {}\n- frasi: {}\n- lunghezza media frase: {:.1} parole\n\n",
        stats.word_count, stats.sentence_count, stats.avg_sentence_len
    ));

    if suggestions.is_empty() {
        report.push_str("Il documento ha già una buona struttura e stile.\n");
    } else {
        report.push_str("**Suggerimenti**\n");
        for suggestion in suggestions {
            report.push_str(&format!("- {}\n", suggestion));
        }
    }

    if !long_sentences.is_empty() {
        report.push_str("\n**Frasi lunghe da rivedere**\n");
        for sentence in long_sentences {
            report.push_str(&format!("- {}\n", sentence));
        }
    }

    Ok(report)
}

fn detect_repeated_words(text: &str) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in tokenize_sentence(text) {
        if token.len() > 4 {
            *counts.entry(token).or_insert(0) += 1;
        }
    }

    let mut repeated: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, count)| *count >= 5)
        .collect();

    repeated.sort_by(|a, b| b.1.cmp(&a.1));
    repeated.into_iter().take(5).map(|(word, _)| word).collect()
}

fn normalize_whitespace(input: &str) -> String {
    input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn sentence_tokenize(text: &str) -> Vec<String> {
    lazy_static! {
        static ref SENTENCE_REGEX: Regex = Regex::new(r"(?m)(?<=[.!?])\s+(?=[A-ZÀ-Ý])").unwrap();
    }

    SENTENCE_REGEX
        .split(text)
        .map(|sentence| sentence.trim())
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_string())
        .collect()
}

fn tokenize_sentence(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| !c.is_alphanumeric() && c != '’')
        .filter_map(|word| {
            let token = word.trim().to_lowercase();
            if token.is_empty() {
                None
            } else {
                Some(token)
            }
        })
        .collect()
}

fn column_name_from_index(index: usize) -> String {
    let mut index = index;
    let mut name = String::new();

    loop {
        let rem = index % 26;
        name.insert(0, (b'A' + rem as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }

    name
}

lazy_static! {
    static ref STOPWORDS: HashSet<&'static str> = {
        let words = [
            "a", "about", "anche", "and", "are", "as", "at", "be", "che", "con", "della", "delle",
            "dello", "di", "e", "for", "from", "gli", "i", "il", "in", "is", "la", "le", "lo",
            "loro", "ma", "nel", "non", "o", "of", "on", "per", "quella", "quello", "sono", "that",
            "the", "their", "this", "to", "una", "uno", "with", "you",
        ];
        words.iter().copied().collect()
    };
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

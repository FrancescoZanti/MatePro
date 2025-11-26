use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use plotters::chart::SeriesLabelPosition;
use plotters::prelude::*;
use plotters::series::LineSeries;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use sysinfo::System;
use walkdir::WalkDir;

/// Definizione di un tool con nome, descrizione e parametri
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub dangerous: bool, // Se richiede conferma
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Chiamata a un tool estratta dalla risposta del LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub raw_text: String, // Testo originale per debug
}

/// Risultato dell'esecuzione di un tool
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

/// Sistema agentico che gestisce i tool
#[derive(Clone)]
pub struct AgentSystem {
    pub tools: HashMap<String, ToolDefinition>,
    execution_log: Vec<ToolResult>,
    pub allow_dangerous: bool,
}

impl AgentSystem {
    pub fn new() -> Self {
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
                description: "Apre un URL nel browser predefinito. USALO quando l'utente chiede di visualizzare informazioni web, mappe, grafici, documenti online, video, etc.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        description: "URL completo da aprire (deve iniziare con http:// o https://)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "description".to_string(),
                        param_type: "string".to_string(),
                        description: "Breve descrizione di cosa si sta aprendo".to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: WebSearch
        tools.insert(
            "web_search".to_string(),
            ToolDefinition {
                name: "web_search".to_string(),
                description: "Esegue una ricerca su Google aprendo il browser. USALO quando l'utente chiede informazioni che richiedono ricerca web.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: "La query di ricerca".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: MapOpen
        tools.insert(
            "map_open".to_string(),
            ToolDefinition {
                name: "map_open".to_string(),
                description: "Apre Google Maps con una località o percorso. USALO per visualizzare mappe, indicazioni, luoghi.".to_string(),
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
                        description: "Modalità: 'search' (default), 'directions' per percorsi".to_string(),
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
                description: "Cerca video su YouTube aprendo il browser. USALO quando l'utente chiede di vedere video o tutorial.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: "La query di ricerca su YouTube".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: DocumentView
        tools.insert(
            "document_view".to_string(),
            ToolDefinition {
                name: "document_view".to_string(),
                description: "Apre un documento/file locale nel programma predefinito. USALO per visualizzare PDF, immagini, documenti.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: "Percorso completo del file da aprire".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        // ========== TOOL MCP SQL SERVER ==========

        // Tool: SqlConnect
        tools.insert(
            "sql_connect".to_string(),
            ToolDefinition {
                name: "sql_connect".to_string(),
                description: "Connette a un database SQL Server. USALO per stabilire connessione prima di query. Supporta autenticazione Windows (dominio) e SQL.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "server".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome o IP del server SQL (es: localhost, 192.168.1.10, server.domain.com)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "database".to_string(),
                        param_type: "string".to_string(),
                        description: "Nome del database a cui connettersi".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "auth_method".to_string(),
                        param_type: "string".to_string(),
                        description: "Metodo autenticazione: 'windows' (usa credenziali dominio/PC) o 'sql' (username/password)".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "username".to_string(),
                        param_type: "string".to_string(),
                        description: "Username SQL (solo se auth_method='sql')".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "password".to_string(),
                        param_type: "string".to_string(),
                        description: "Password SQL (solo se auth_method='sql')".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "trust_server_certificate".to_string(),
                        param_type: "boolean".to_string(),
                        description: "Imposta true per accettare certificati TLS non attendibili (self-signed)".to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: SqlQuery
        tools.insert(
            "sql_query".to_string(),
            ToolDefinition {
                name: "sql_query".to_string(),
                description: "Esegue query SELECT su database SQL Server connesso. SOLO LETTURA - UPDATE/INSERT/DELETE non permessi. Se non specifichi connection_id verrà usata l'ultima connessione attiva.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL (ottenuto da sql_connect). Se omesso, usa l'ultima connessione attiva.".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: "Query SQL SELECT da eseguire (solo lettura)".to_string(),
                        required: true,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: SqlListTables
        tools.insert(
            "sql_list_tables".to_string(),
            ToolDefinition {
                name: "sql_list_tables".to_string(),
                description: "Lista tutte le tabelle e view del database SQL Server. Se connection_id non è fornito usa l'ultima connessione attiva.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL. Se omesso, usa l'ultima connessione attiva.".to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        // Tool: SqlDescribeTable
        tools.insert(
            "sql_describe_table".to_string(),
            ToolDefinition {
                name: "sql_describe_table".to_string(),
                description: "Mostra struttura di una tabella (colonne, tipi, nullable). Se connection_id non è fornito usa l'ultima connessione attiva.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL. Se omesso, usa l'ultima connessione attiva.".to_string(),
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

        // Tool: SqlDisconnect
        tools.insert(
            "sql_disconnect".to_string(),
            ToolDefinition {
                name: "sql_disconnect".to_string(),
                description: "Chiude connessione SQL Server. Se non specifichi connection_id verrà chiusa l'ultima connessione attiva.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "connection_id".to_string(),
                        param_type: "string".to_string(),
                        description: "ID della connessione SQL da chiudere. Se omesso, chiude l'ultima connessione attiva.".to_string(),
                        required: false,
                    },
                ],
                dangerous: false,
            },
        );

        Self {
            tools,
            execution_log: Vec::new(),
            allow_dangerous: false,
        }
    }

    /// Ottiene la descrizione di tutti i tool in formato markdown per il prompt
    pub fn get_tools_description(&self) -> String {
        let mut desc = String::from(
            "**TOOLS DISPONIBILI** - Puoi usare questi tool per interagire con il sistema.\n\n",
        );
        desc.push_str("Per usare un tool, rispondi con il seguente formato JSON:\n");
        desc.push_str("```json\n{\n  \"tool\": \"nome_tool\",\n  \"parameters\": {\n    \"param1\": \"valore1\"\n  }\n}\n```\n\n");
        desc.push_str("**Lista Tool:**\n\n");

        for (_, tool) in &self.tools {
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
            desc.push_str("\n");
        }

        desc
    }

    /// Parse la risposta del LLM per trovare tool calls
    pub fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();

        // Cerca blocchi JSON nel formato ```json ... ```
        let json_regex = regex::Regex::new(r"```json\s*(\{[^`]*\})\s*```").unwrap();

        for cap in json_regex.captures_iter(response) {
            if let Some(json_str) = cap.get(1) {
                let json_text = json_str.as_str();

                // Prova a parsare come tool call
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

    /// Esegue un tool call e restituisce il risultato
    pub async fn execute_tool(&mut self, call: &ToolCall) -> Result<ToolResult> {
        // Verifica che il tool esista
        let tool_def = self
            .tools
            .get(&call.tool_name)
            .context("Tool non trovato")?;

        // Verifica permessi per tool pericolosi
        if tool_def.dangerous && !self.allow_dangerous {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Tool pericoloso: conferma richiesta".to_string()),
                tool_name: call.tool_name.clone(),
            });
        }

        // Esegui il tool specifico
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
            "document_view" => self.execute_document_view(&call.parameters).await,
            // MCP SQL Server tools
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

        self.execution_log.push(tool_result.clone());
        Ok(tool_result)
    }

    pub fn set_allow_dangerous(&mut self, allow: bool) {
        self.allow_dangerous = allow;
    }

    // Implementazioni specifiche dei tool

    async fn execute_shell(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .context("Parametro 'command' mancante")?;

        let output = Command::new("bash")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Errore esecuzione comando")?;

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
        processes.truncate(50); // Limita a 50 processi per non sovraccaricare

        Ok(processes.join("\n"))
    }

    async fn execute_system_info(&self) -> Result<String> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let total_memory = sys.total_memory() / 1024 / 1024; // MB
        let used_memory = sys.used_memory() / 1024 / 1024;
        let total_swap = sys.total_swap() / 1024 / 1024;
        let used_swap = sys.used_swap() / 1024 / 1024;

        let info = format!(
            "Sistema: {}\n\
             Kernel: {}\n\
             CPU: {} cores\n\
             RAM: {} MB / {} MB ({:.1}%)\n\
             Swap: {} MB / {} MB\n\
             Processi attivi: {}",
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

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("pagina web");

        // Valida URL
        let parsed_url = url::Url::parse(url_str).context("URL non valido")?;

        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            anyhow::bail!("Solo URL http:// o https:// sono supportati");
        }

        // Apri nel browser
        webbrowser::open(url_str).context("Impossibile aprire il browser")?;

        Ok(format!("Browser aperto con {} - {}", description, url_str))
    }

    async fn execute_web_search(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .context("Parametro 'query' mancante")?;

        // Crea URL di ricerca Google
        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://www.google.com/search?q={}", encoded_query);

        webbrowser::open(&search_url).context("Impossibile aprire il browser")?;

        Ok(format!("Ricerca Google avviata per: '{}'", query))
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

        webbrowser::open(&map_url).context("Impossibile aprire Google Maps")?;

        Ok(format!(
            "Google Maps aperto per: '{}' (modalità: {})",
            location, mode
        ))
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

        webbrowser::open(&youtube_url).context("Impossibile aprire YouTube")?;

        Ok(format!("YouTube aperto con ricerca: '{}'", query))
    }

    async fn execute_document_view(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        // Verifica che il file esista
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("File non trovato: {}", path);
        }

        // Apri con il programma predefinito del sistema
        webbrowser::open(path).context("Impossibile aprire il file")?;

        Ok(format!("File aperto: {}", path))
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

// ============ IMPLEMENTAZIONI TOOL MCP SQL SERVER ============

use crate::mcp_sql;

// Gestore globale connessioni SQL (statico thread-safe)
lazy_static::lazy_static! {
    pub static ref SQL_MANAGER: std::sync::Arc<tokio::sync::Mutex<mcp_sql::SqlConnectionManager>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(mcp_sql::SqlConnectionManager::new()));

    // Tiene traccia dell'ultima connessione SQL usata così da poterla riutilizzare
    pub static ref LAST_SQL_CONNECTION_ID: std::sync::Arc<tokio::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(None));
}

impl AgentSystem {
    /// Connette a SQL Server e memorizza la connessione
    async fn execute_sql_connect(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let server = params
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parametro 'server' mancante"))?;

        let database = params
            .get("database")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parametro 'database' mancante"))?;

        let auth_method = params
            .get("auth_method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Parametro 'auth_method' mancante (usa 'windows' o 'sql')")
            })?;

        let trust_server_certificate = params
            .get("trust_server_certificate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Genera ID connessione unico
        let connection_id = format!("sql_{}", uuid::Uuid::new_v4().to_string());

        let mut stored_username = None;
        let mut stored_password = None;

        // Connetti in base al metodo di autenticazione
        let client = if auth_method == "windows" {
            mcp_sql::connect_windows_auth(server, database, trust_server_certificate).await?
        } else if auth_method == "sql" {
            let username = params
                .get("username")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Parametro 'username' richiesto per SQL auth"))?;

            let password = params
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Parametro 'password' richiesto per SQL auth"))?;

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
            return Err(anyhow::anyhow!(
                "auth_method non valido: usa 'windows' o 'sql'"
            ));
        };

        let conn_info = mcp_sql::SqlConnection {
            connection_id: connection_id.clone(),
            server: server.to_string(),
            database: database.to_string(),
            auth_type: auth_method.to_string(),
            username: stored_username,
            password: stored_password,
            trust_server_certificate,
        };

        // Rilascia il client dopo aver validato la connessione
        drop(client);

        let manager = SQL_MANAGER.lock().await;
        manager.add_connection(conn_info);

        // Memorizza come connessione attiva predefinita
        let mut last_conn = LAST_SQL_CONNECTION_ID.lock().await;
        *last_conn = Some(connection_id.clone());

        Ok(format!(
            "✅ Connesso a SQL Server!\n\
            Connection ID: {}\n\
            Server: {}\n\
            Database: {}\n\
            Autenticazione: {}\n\
            Verifica certificato TLS: {}\n\n\
            Usa questo connection_id per le query successive.",
            connection_id,
            server,
            database,
            auth_method,
            if trust_server_certificate {
                "disabilitata"
            } else {
                "attiva"
            }
        ))
    }

    /// Esegue query SQL SELECT
    async fn execute_sql_query(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = match params.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let last = LAST_SQL_CONNECTION_ID.lock().await;
                last.clone().ok_or_else(|| anyhow::anyhow!(
                    "Nessun connection_id fornito e nessuna connessione SQL attiva trovata. Esegui prima sql_connect."
                ))?
            }
        };

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parametro 'query' mancante"))?;

        // Aggiorna la connessione predefinita per i prossimi comandi
        {
            let mut last_store = LAST_SQL_CONNECTION_ID.lock().await;
            *last_store = Some(connection_id.clone());
        }

        let conn_info = {
            let manager = SQL_MANAGER.lock().await;
            manager.get_connection(&connection_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Connessione '{}' non trovata. Usa sql_connect prima.",
                    connection_id
                )
            })?
        };

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;

        let result = mcp_sql::run_query(&mut client, query).await?;
        let summary = summarize_query_result(&result);
        let table_preview = render_result_table(&result, 20);
        let chart_markdown = match generate_numeric_chart(&result) {
            Ok(chart) => chart,
            Err(_) => None,
        };
        let json_payload = serde_json::json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&json_payload)?;

        let mut response = String::new();
        response.push_str("📊 Analisi query\n");
        response.push_str(&summary);

        if let Some(table) = table_preview {
            response.push_str("\n\n**Anteprima dati**\n");
            response.push_str(&table);
        }

        if let Some(chart) = chart_markdown {
            response.push_str("\n\n**Visualizzazione rapida**\n");
            response.push_str(&chart);
        }

        response.push_str("\n\n**JSON completo**\n```json\n");
        response.push_str(&json_pretty);
        response.push_str("\n```\n");

        Ok(response)
    }

    /// Lista tutte le tabelle del database
    async fn execute_sql_list_tables(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = match params.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let last = LAST_SQL_CONNECTION_ID.lock().await;
                last.clone().ok_or_else(|| anyhow::anyhow!(
                    "Nessun connection_id fornito e nessuna connessione SQL attiva trovata. Esegui prima sql_connect."
                ))?
            }
        };

        {
            let mut last_store = LAST_SQL_CONNECTION_ID.lock().await;
            *last_store = Some(connection_id.clone());
        }

        let conn_info = {
            let manager = SQL_MANAGER.lock().await;
            manager
                .get_connection(&connection_id)
                .ok_or_else(|| anyhow::anyhow!("Connessione '{}' non trovata", connection_id))?
        };

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;

        let result = mcp_sql::list_tables(&mut client).await?;
        let total_items = result.rows.len();
        let base_tables = result
            .rows
            .iter()
            .filter(|row| {
                row.get("Type")
                    .and_then(|value| value.as_str())
                    .map(|value| value.eq_ignore_ascii_case("BASE TABLE"))
                    .unwrap_or(false)
            })
            .count();
        let views = result
            .rows
            .iter()
            .filter(|row| {
                row.get("Type")
                    .and_then(|value| value.as_str())
                    .map(|value| value.eq_ignore_ascii_case("VIEW"))
                    .unwrap_or(false)
            })
            .count();
        let table_preview = render_result_table(&result, 40);
        let json_payload = serde_json::json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&json_payload)?;

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

    /// Descrive struttura di una tabella
    async fn execute_sql_describe_table(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = match params.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let last = LAST_SQL_CONNECTION_ID.lock().await;
                last.clone().ok_or_else(|| anyhow::anyhow!(
                    "Nessun connection_id fornito e nessuna connessione SQL attiva trovata. Esegui prima sql_connect."
                ))?
            }
        };

        let schema = params
            .get("schema")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parametro 'schema' mancante (es: 'dbo')"))?;

        let table = params
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parametro 'table' mancante"))?;

        {
            let mut last_store = LAST_SQL_CONNECTION_ID.lock().await;
            *last_store = Some(connection_id.clone());
        }

        let conn_info = {
            let manager = SQL_MANAGER.lock().await;
            manager
                .get_connection(&connection_id)
                .ok_or_else(|| anyhow::anyhow!("Connessione '{}' non trovata", connection_id))?
        };

        let mut client = mcp_sql::connect_with_info(&conn_info).await?;

        let result = mcp_sql::describe_table(&mut client, schema, table).await?;
        let total_columns = result.rows.len();
        let highlights: Vec<String> = result
            .rows
            .iter()
            .take(5)
            .map(|row| {
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
            })
            .collect();
        let highlight_text = if highlights.is_empty() {
            "nessuna colonna trovata".to_string()
        } else {
            highlights.join("; ")
        };
        let table_preview = render_result_table(&result, 50);
        let json_payload = serde_json::json!({
            "columns": result.columns,
            "rows": result.rows,
        });
        let json_pretty = serde_json::to_string_pretty(&json_payload)?;

        let mut response = String::new();
        response.push_str(&format!("🔍 Struttura tabella {}.{}\n", schema, table));
        response.push_str(&format!(
            "- colonne totali: {}\n- anteprima: {}\n",
            total_columns, highlight_text
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

    /// Chiude connessione SQL
    async fn execute_sql_disconnect(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let connection_id = match params.get("connection_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let last = LAST_SQL_CONNECTION_ID.lock().await;
                last.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Nessun connection_id fornito e nessuna connessione SQL attiva trovata."
                    )
                })?
            }
        };

        // Rimuovi info dal manager
        let manager = SQL_MANAGER.lock().await;
        let removed = manager.remove_connection(&connection_id);

        if removed.is_none() {
            return Err(anyhow::anyhow!(
                "Connessione '{}' non trovata",
                connection_id
            ));
        }

        {
            let mut last_store = LAST_SQL_CONNECTION_ID.lock().await;
            if last_store.as_ref() == Some(&connection_id) {
                *last_store = None;
            }
        }

        Ok(format!(
            "✅ Connessione '{}' chiusa correttamente.",
            connection_id
        ))
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

fn generate_numeric_chart(result: &mcp_sql::QueryResult) -> Result<Option<String>> {
    let numeric_column = result
        .columns
        .iter()
        .find(|column| is_numeric_type(&column.data_type))
        .cloned();

    let Some(column) = numeric_column else {
        return Ok(None);
    };

    let mut series = Vec::new();
    for row in &result.rows {
        if let Some(value) = row.get(&column.name) {
            if let Some(number) = value_to_f64(value) {
                series.push(number);
            }
        }
    }

    if series.len() < 2 {
        return Ok(None);
    }

    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("matepro_chart_{}.png", uuid::Uuid::new_v4()));

    let width = 800;
    let height = 480;

    {
        let drawing_area = BitMapBackend::new(&path, (width, height)).into_drawing_area();
        drawing_area.fill(&WHITE).map_err(map_plotters_error)?;

        let (min_value, max_value) = compute_numeric_range(&series);
        let x_range = 0..(series.len() as i32);
        let y_range = adjust_numeric_range(min_value, max_value);

        let mut chart = ChartBuilder::on(&drawing_area)
            .caption(format!("Trend {}", column.name), ("sans-serif", 28))
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(x_range.clone(), y_range.clone())
            .map_err(map_plotters_error)?;

        chart
            .configure_mesh()
            .x_desc("Indice")
            .y_desc(&column.name)
            .light_line_style(&WHITE.mix(0.0))
            .draw()
            .map_err(map_plotters_error)?;

        chart
            .draw_series(LineSeries::new(
                series
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index as i32, *value)),
                &BLUE,
            ))
            .map_err(map_plotters_error)?
            .label(&column.name)
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

        chart
            .draw_series(
                series
                    .iter()
                    .enumerate()
                    .map(|(index, value)| Circle::new((index as i32, *value), 3, BLUE.filled())),
            )
            .map_err(map_plotters_error)?;

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .legend_area_size(20)
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()
            .map_err(map_plotters_error)?;

        drawing_area.present().map_err(map_plotters_error)?;
    }

    let bytes = fs::read(&path).context("Impossibile leggere il grafico generato")?;
    let _ = fs::remove_file(&path);
    let encoded = general_purpose::STANDARD.encode(bytes);

    Ok(Some(format!(
        "![Trend {}](data:image/png;base64,{})",
        column.name, encoded
    )))
}

fn map_plotters_error<E: std::error::Error + Send + Sync + 'static>(
    err: plotters::drawing::DrawingAreaErrorKind<E>,
) -> anyhow::Error {
    anyhow::anyhow!("Errore generazione grafico: {}", err)
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

fn compute_numeric_range(values: &[f64]) -> (f64, f64) {
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;

    for value in values {
        if *value < min_value {
            min_value = *value;
        }
        if *value > max_value {
            max_value = *value;
        }
    }

    (min_value, max_value)
}

fn adjust_numeric_range(min_value: f64, max_value: f64) -> Range<f64> {
    if (max_value - min_value).abs() < f64::EPSILON {
        let padding = if min_value.abs() < 1.0 {
            1.0
        } else {
            min_value.abs() * 0.1
        };
        (min_value - padding)..(max_value + padding)
    } else {
        min_value..max_value
    }
}

fn value_to_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
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

fn is_numeric_type(data_type: &str) -> bool {
    matches!(data_type, "int" | "float" | "decimal" | "money")
}

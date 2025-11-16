use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};
use sysinfo::System;
use walkdir::WalkDir;

/// Definizione di un tool con nome, descrizione e parametri
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub dangerous: bool,  // Se richiede conferma
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
    pub raw_text: String,  // Testo originale per debug
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
                self.error.as_ref().unwrap_or(&"Errore sconosciuto".to_string())
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
                description: "Esegue un comando shell. USALO per operazioni sul sistema.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: "Il comando bash da eseguire".to_string(),
                        required: true,
                    },
                ],
                dangerous: true,
            },
        );

        // Tool: FileRead
        tools.insert(
            "file_read".to_string(),
            ToolDefinition {
                name: "file_read".to_string(),
                description: "Legge il contenuto di un file.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: "Percorso del file da leggere".to_string(),
                        required: true,
                    },
                ],
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

        Self {
            tools,
            execution_log: Vec::new(),
            allow_dangerous: false,
        }
    }

    /// Ottiene la descrizione di tutti i tool in formato markdown per il prompt
    pub fn get_tools_description(&self) -> String {
        let mut desc = String::from("**TOOLS DISPONIBILI** - Puoi usare questi tool per interagire con il sistema.\n\n");
        desc.push_str("Per usare un tool, rispondi con il seguente formato JSON:\n");
        desc.push_str("```json\n{\n  \"tool\": \"nome_tool\",\n  \"parameters\": {\n    \"param1\": \"valore1\"\n  }\n}\n```\n\n");
        desc.push_str("**Lista Tool:**\n\n");

        for (_, tool) in &self.tools {
            desc.push_str(&format!("### {}\n", tool.name));
            desc.push_str(&format!("{}\n", tool.description));
            
            if !tool.parameters.is_empty() {
                desc.push_str("**Parametri:**\n");
                for param in &tool.parameters {
                    let required = if param.required { "obbligatorio" } else { "opzionale" };
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
                            .map(|obj| {
                                obj.iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect()
                            })
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

    async fn execute_file_read(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        let content = fs::read_to_string(path)
            .context(format!("Impossibile leggere file: {}", path))?;

        Ok(content)
    }

    async fn execute_file_write(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .context("Parametro 'content' mancante")?;

        fs::write(path, content)
            .context(format!("Impossibile scrivere file: {}", path))?;

        Ok(format!("File scritto: {} ({} bytes)", path, content.len()))
    }

    async fn execute_file_list(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
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

    async fn execute_browser_open(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let url_str = params
            .get("url")
            .and_then(|v| v.as_str())
            .context("Parametro 'url' mancante")?;

        let description = params
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("pagina web");

        // Valida URL
        let parsed_url = url::Url::parse(url_str)
            .context("URL non valido")?;

        if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
            anyhow::bail!("Solo URL http:// o https:// sono supportati");
        }

        // Apri nel browser
        webbrowser::open(url_str)
            .context("Impossibile aprire il browser")?;

        Ok(format!("Browser aperto con {} - {}", description, url_str))
    }

    async fn execute_web_search(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .context("Parametro 'query' mancante")?;

        // Crea URL di ricerca Google
        let encoded_query = urlencoding::encode(query);
        let search_url = format!("https://www.google.com/search?q={}", encoded_query);

        webbrowser::open(&search_url)
            .context("Impossibile aprire il browser")?;

        Ok(format!("Ricerca Google avviata per: '{}'", query))
    }

    async fn execute_map_open(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
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
            "directions" => format!("https://www.google.com/maps/dir/?api=1&destination={}", encoded_location),
            _ => format!("https://www.google.com/maps/search/?api=1&query={}", encoded_location),
        };

        webbrowser::open(&map_url)
            .context("Impossibile aprire Google Maps")?;

        Ok(format!("Google Maps aperto per: '{}' (modalità: {})", location, mode))
    }

    async fn execute_youtube_search(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .context("Parametro 'query' mancante")?;

        let encoded_query = urlencoding::encode(query);
        let youtube_url = format!("https://www.youtube.com/results?search_query={}", encoded_query);

        webbrowser::open(&youtube_url)
            .context("Impossibile aprire YouTube")?;

        Ok(format!("YouTube aperto con ricerca: '{}'", query))
    }

    async fn execute_document_view(&self, params: &HashMap<String, serde_json::Value>) -> Result<String> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .context("Parametro 'path' mancante")?;

        // Verifica che il file esista
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("File non trovato: {}", path);
        }

        // Apri con il programma predefinito del sistema
        webbrowser::open(path)
            .context("Impossibile aprire il file")?;

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

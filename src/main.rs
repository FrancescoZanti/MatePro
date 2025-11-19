use anyhow::{Context, Result};
use eframe::egui;
use poll_promise::Promise;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::PathBuf;
use std::fs;
use lopdf::Document;
use calamine::{Reader, open_workbook, Xlsx, Xls, Ods};

mod agent;
mod mcp_sql;
use agent::{AgentSystem, ToolCall, ToolResult};

// Helper per ottenere timestamp formattato
fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Calcola ore e minuti dal timestamp Unix (considera fuso orario UTC+1 per Italia)
    let total_seconds = now + 3600; // +1 ora per CET/CEST
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds % 3600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
    #[serde(skip)]
    hidden: bool,  // Se true, non mostrare nella chat UI
    #[serde(skip)]
    timestamp: Option<String>,  // Orario del messaggio
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Message,
}

#[derive(Debug, Clone)]
struct ModelInfo {
    name: String,
    size: u64,  // Size in bytes
}

impl ModelInfo {
    fn size_gb(&self) -> f64 {
        self.size as f64 / 1_073_741_824.0  // Convert bytes to GB
    }
    
    fn weight_category(&self) -> (&str, egui::Color32) {
        let gb = self.size_gb();
        if gb < 4.0 {
            ("🟢", egui::Color32::from_rgb(52, 199, 89))  // Verde - leggero
        } else if gb < 8.0 {
            ("🟡", egui::Color32::from_rgb(255, 204, 0))  // Giallo - medio
        } else {
            ("🔴", egui::Color32::from_rgb(255, 59, 48))  // Rosso - pesante
        }
    }
}

#[derive(Clone)]
struct OllamaClient {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaClient {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Impossibile connettersi a Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Errore nella risposta di Ollama: {}", response.status());
        }

        let json: serde_json::Value = response.json().await?;
        let models = json["models"]
            .as_array()
            .context("Formato risposta non valido")?
            .iter()
            .filter_map(|m| {
                let name = m["name"].as_str()?.to_string();
                let size = m["size"].as_u64().unwrap_or(0);
                Some(ModelInfo { name, size })
            })
            .collect();

        Ok(models)
    }

    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Errore durante l'invio della richiesta")?;

        if !response.status().is_success() {
            anyhow::bail!("Errore nella risposta di Ollama: {}", response.status());
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response.message.content)
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
}

async fn scan_local_network() -> Vec<String> {
    let mut servers = Vec::new();
    
    // Controlla localhost
    if OllamaClient::check_server("http://localhost:11434").await {
        servers.push("http://localhost:11434".to_string());
    }
    
    // Controlla 127.0.0.1
    if OllamaClient::check_server("http://127.0.0.1:11434").await 
        && !servers.contains(&"http://127.0.0.1:11434".to_string()) {
        servers.push("http://127.0.0.1:11434".to_string());
    }
    
    // Ottieni l'IP locale
    if let Ok(local_ip) = local_ip_address::local_ip() {
        match local_ip {
            IpAddr::V4(ip) => {
                let octets = ip.octets();
                let base = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
                
                // Scansiona gli IP comuni nella rete locale (range ristretto per velocità)
                let mut handles = vec![];
                
                for i in 1..255 {
                    let url = format!("http://{}.{}:11434", base, i);
                    let handle = tokio::spawn(async move {
                        if OllamaClient::check_server(&url).await {
                            Some(url)
                        } else {
                            None
                        }
                    });
                    handles.push(handle);
                }
                
                // Raccogli i risultati
                for handle in handles {
                    if let Ok(Some(url)) = handle.await {
                        if !servers.contains(&url) {
                            servers.push(url);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    servers
}

// Funzioni per estrarre testo dai file
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
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    let mut text = String::new();
    
    match extension.to_lowercase().as_str() {
        "xlsx" => {
            let mut workbook: Xlsx<_> = open_workbook(path)?;
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    text.push_str(&format!("=== Foglio: {} ===\n", sheet_name));
                    for row in range.rows() {
                        let row_text: Vec<String> = row.iter()
                            .map(|cell| format!("{}", cell))
                            .collect();
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
                        let row_text: Vec<String> = row.iter()
                            .map(|cell| format!("{}", cell))
                            .collect();
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
                        let row_text: Vec<String> = row.iter()
                            .map(|cell| format!("{}", cell))
                            .collect();
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
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
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

#[derive(PartialEq)]
enum AppState {
    Setup,
    ScanningNetwork,
    LoadingModels,
    Chat,
}

struct OllamaChatApp {
    state: AppState,
    ollama_url: String,
    discovered_servers: Vec<String>,
    available_models: Vec<ModelInfo>,
    selected_model: Option<String>,
    conversation: Vec<Message>,
    input_text: String,
    error_message: Option<String>,
    client: Option<OllamaClient>,
    scanning_promise: Option<Promise<Vec<String>>>,
    loading_models_promise: Option<Promise<Result<Vec<ModelInfo>>>>,
    chat_promise: Option<Promise<Result<String>>>,
    scroll_to_bottom: bool,
    markdown_cache: CommonMarkCache,
    system_prompt_added: bool,
    attached_files: Vec<(String, String)>, // (nome_file, contenuto)
    file_loading_promise: Option<Promise<Result<(String, String)>>>,
    // Nuovi campi per funzionalità agentiche
    agent_system: AgentSystem,
    agent_mode_enabled: bool,
    tool_execution_promise: Option<Promise<Result<Vec<ToolResult>>>>,
    pending_tool_calls: Vec<ToolCall>,
    awaiting_confirmation: Option<ToolCall>,
    max_agent_iterations: usize,
    current_agent_iteration: usize,
    // Campi per configurazione SQL Server
    show_sql_config: bool,
    sql_server: String,
    sql_database: String,
    sql_auth_method: String, // "windows" o "sql"
    sql_username: String,
    sql_password: String,
    sql_connection_status: Option<String>, // None, Some("connecting"), Some("connected"), Some("error: ...")
    sql_test_promise: Option<Promise<Result<String>>>,
}

impl Default for OllamaChatApp {
    fn default() -> Self {
        Self {
            state: AppState::Setup,
            ollama_url: "http://localhost:11434".to_string(),
            discovered_servers: Vec::new(),
            available_models: Vec::new(),
            selected_model: None,
            conversation: Vec::new(),
            input_text: String::new(),
            error_message: None,
            client: None,
            scanning_promise: None,
            loading_models_promise: None,
            chat_promise: None,
            scroll_to_bottom: false,
            markdown_cache: CommonMarkCache::default(),
            system_prompt_added: false,
            attached_files: Vec::new(),
            file_loading_promise: None,
            agent_system: AgentSystem::new(),
            agent_mode_enabled: false,
            tool_execution_promise: None,
            pending_tool_calls: Vec::new(),
            awaiting_confirmation: None,
            max_agent_iterations: 5,
            current_agent_iteration: 0,
            show_sql_config: false,
            sql_server: "localhost".to_string(),
            sql_database: String::new(),
            sql_auth_method: "windows".to_string(),
            sql_username: String::new(),
            sql_password: String::new(),
            sql_connection_status: None,
            sql_test_promise: None,
        }
    }
}

impl OllamaChatApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.start_network_scan();
        app
    }
    
    fn start_network_scan(&mut self) {
        self.state = AppState::ScanningNetwork;
        self.scanning_promise = Some(Promise::spawn_thread("scan_network", move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(scan_local_network())
        }));
    }
    
    fn load_models(&mut self) {
        let client = OllamaClient::new(self.ollama_url.clone());
        let client_clone = client.clone();
        
        self.client = Some(client);
        self.state = AppState::LoadingModels;
        self.error_message = None;
        
        self.loading_models_promise = Some(Promise::spawn_thread("load_models", move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(client_clone.list_models())
        }));
    }

    fn open_file_dialog(&mut self) {
        self.file_loading_promise = Some(Promise::spawn_thread("file_picker", move || {
            // Usa il dialog sincrono invece di async
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Documenti", &["pdf", "xlsx", "xls", "ods", "txt", "md", "csv"])
                .pick_file()
            {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                
                match extract_text_from_file(&path) {
                    Ok(content) => Ok((filename, content)),
                    Err(e) => Err(e),
                }
            } else {
                Err(anyhow::anyhow!("Nessun file selezionato"))
            }
        }));
    }

    fn process_next_tool_call(&mut self) {
        if let Some(tool_call) = self.pending_tool_calls.first() {
            // Controlla se il tool richiede conferma
            if let Some(tool_def) = self.agent_system.tools.get(&tool_call.tool_name) {
                if tool_def.dangerous && !self.agent_system.allow_dangerous {
                    self.awaiting_confirmation = Some(tool_call.clone());
                    return;
                }
            }
            
            // Esegui il tool
            self.execute_pending_tools();
        }
    }

    fn execute_pending_tools(&mut self) {
        let tools_to_execute = std::mem::take(&mut self.pending_tool_calls);
        let mut agent_system = self.agent_system.clone();
        
        self.tool_execution_promise = Some(Promise::spawn_thread("execute_tools", move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let mut results = Vec::new();
            for tool_call in tools_to_execute {
                match rt.block_on(agent_system.execute_tool(&tool_call)) {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        results.push(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some(e.to_string()),
                            tool_name: tool_call.tool_name.clone(),
                        });
                    }
                }
            }
            Ok(results)
        }));
    }

    fn continue_agent_loop(&mut self) {
        // L'agente ha eseguito i tool, ora chiedi al LLM di continuare
        if let (Some(client), Some(model)) = (&self.client, &self.selected_model) {
            let client_clone = client.clone();
            let model_clone = model.clone();
            let messages = self.conversation.clone();

            self.chat_promise = Some(Promise::spawn_thread("chat", move || {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(client_clone.chat(&model_clone, &messages))
            }));
        }
    }

    fn confirm_dangerous_tool(&mut self) {
        if let Some(tool_call) = self.awaiting_confirmation.take() {
            self.agent_system.set_allow_dangerous(true);
            self.pending_tool_calls = vec![tool_call];
            self.execute_pending_tools();
        }
    }

    fn cancel_dangerous_tool(&mut self) {
        self.awaiting_confirmation = None;
        self.conversation.push(Message {
            role: "system".to_string(),
            content: "❌ Operazione annullata dall'utente".to_string(),
            hidden: false,
            timestamp: Some(get_timestamp()),
        });
    }

    fn test_sql_connection(&mut self) {
        self.sql_connection_status = Some("connecting".to_string());
        
        let server = self.sql_server.clone();
        let database = self.sql_database.clone();
        let auth_method = self.sql_auth_method.clone();
        let username = self.sql_username.clone();
        let password = self.sql_password.clone();
        
        self.sql_test_promise = Some(Promise::spawn_thread("test_sql", move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    // Crea una connessione di test usando il modulo mcp_sql
                    let connection_result = if auth_method == "windows" {
                        mcp_sql::connect_windows_auth(&server, &database).await
                    } else {
                        mcp_sql::connect_sql_auth(&server, &database, &username, &password).await
                    };
                    
                    match connection_result {
                        Ok(client) => {
                            // Genera un ID per la connessione
                            let connection_id = format!("conn_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
                            
                            // Salva il client nel manager globale
                            let mut clients = agent::SQL_CLIENTS.lock().await;
                            clients.insert(connection_id.clone(), client);
                            drop(clients);
                            
                            // Registra nel manager
                            let conn_info = mcp_sql::SqlConnection {
                                connection_id: connection_id.clone(),
                                server: server.clone(),
                                database: database.clone(),
                                auth_type: auth_method.clone(),
                            };
                            
                            let manager = agent::SQL_MANAGER.lock().await;
                            manager.add_connection(conn_info);
                            
                            Ok(connection_id)
                        }
                        Err(e) => Err(e),
                    }
                })
        }));
    }

    fn send_message(&mut self) {
        if self.input_text.trim().is_empty() && self.attached_files.is_empty() {
            return;
        }
        
        // Resetta il contatore di iterazioni per nuova richiesta utente
        self.current_agent_iteration = 0;
        
        // Aggiungi istruzioni di formattazione solo alla prima interazione
        if !self.system_prompt_added && self.conversation.is_empty() {
            // Usa un approccio user/assistant per garantire che il modello capisca
            let mut instruction_content = "IMPORTANTE: Per questa conversazione, quando devi mostrare formule matematiche NON usare LaTeX (no \\frac, \\sqrt, \\( \\), ecc). Usa SOLO:

• Caratteri Unicode: √ ² ³ ∫ ∑ π ∞ ≤ ≥ ≠ ± × ÷
• Notazione testuale: sqrt(), ^2, ^3, /
• Esempi: 
  - x = (-b ± √(b² - 4ac)) / (2a)
  - a² + b² = c²
  - lim(x→∞) f(x)

Conferma che userai solo Unicode e notazione testuale, MAI LaTeX.".to_string();

            // Se la modalità agente è abilitata, aggiungi descrizione tools e linee guida
            if self.agent_mode_enabled {
                instruction_content.push_str("\n\n");
                instruction_content.push_str(&self.agent_system.get_tools_description());
                instruction_content.push_str("\n**LINEE GUIDA PER AZIONI COMPLESSE:**\n\n");
                instruction_content.push_str("1. **Visualizzazioni Web**: Se l'utente chiede di vedere/visualizzare qualcosa online, usa `browser_open`, `web_search`, `map_open` o `youtube_search`\n");
                instruction_content.push_str("2. **Informazioni in Tempo Reale**: Per meteo, notizie, risultati sportivi, usa `web_search` per aprire risultati aggiornati\n");
                instruction_content.push_str("3. **Mappe e Luoghi**: Per indirizzi, ristoranti, percorsi stradali, usa `map_open`\n");
                instruction_content.push_str("4. **Video e Tutorial**: Per guide, musica, film, usa `youtube_search`\n");
                instruction_content.push_str("5. **Documenti Locali**: Per PDF, immagini, file esistenti, usa `document_view`\n");
                instruction_content.push_str("6. **Task Multi-Step**: Combina più tool in sequenza per completare task complessi\n");
                instruction_content.push_str("7. **Spiega Prima**: Prima di usare tool, spiega brevemente cosa farai\n\n");
                instruction_content.push_str("**ESEMPI DI RICONOSCIMENTO AZIONI COMPLESSE:**\n");
                instruction_content.push_str("- \"mostrami il meteo\" → web_search per meteo in tempo reale\n");
                instruction_content.push_str("- \"apri Google Maps con Milano\" → map_open\n");
                instruction_content.push_str("- \"cerca video tutorial Python\" → youtube_search\n");
                instruction_content.push_str("- \"vai su Wikipedia\" → browser_open con URL Wikipedia\n");
                instruction_content.push_str("- \"come arrivo a Roma da Milano\" → map_open con mode=directions\n");
                instruction_content.push_str("- \"mostrami il sito di GitHub\" → browser_open\n");
            }
            
            let instruction = Message {
                role: "user".to_string(),
                content: instruction_content,
                hidden: true,  // Non mostrare nella UI
                timestamp: None,  // Messaggi di sistema senza timestamp
            };
            
            let confirmation = Message {
                role: "assistant".to_string(),
                content: "Perfetto! Userò solo caratteri Unicode (√, ², ³, π, ±, ecc.) e notazione testuale chiara (sqrt, ^2, /) per le formule matematiche. Non userò LaTeX. Sono pronto ad aiutarti!".to_string(),
                hidden: true,  // Non mostrare nella UI
                timestamp: None,  // Messaggi di sistema senza timestamp
            };
            
            self.conversation.push(instruction);
            self.conversation.push(confirmation);
            self.system_prompt_added = true;
        }

        // Costruisci il messaggio per Ollama includendo i file allegati
        let mut full_content = String::new();
        
        if !self.attached_files.is_empty() {
            full_content.push_str("File allegati:\n\n");
            for (filename, file_content) in &self.attached_files {
                full_content.push_str(&format!("=== {} ===\n{}\n\n", filename, file_content));
            }
            full_content.push_str("---\n\n");
        }
        
        full_content.push_str(self.input_text.trim());

        // Messaggio che l'utente vede (solo testo, senza contenuto file)
        let display_content = if !self.attached_files.is_empty() {
            let files_list: Vec<String> = self.attached_files.iter()
                .map(|(name, _)| format!("📎 {}", name))
                .collect();
            format!("{}\n\n{}", files_list.join("\n"), self.input_text.trim())
        } else {
            self.input_text.trim().to_string()
        };

        // Aggiungi alla conversazione per visualizzazione
        let user_message_display = Message {
            role: "user".to_string(),
            content: display_content,
            hidden: false,
            timestamp: Some(get_timestamp()),
        };
        self.conversation.push(user_message_display);
        
        self.input_text.clear();
        self.attached_files.clear(); // Pulisci i file allegati dopo l'invio
        self.scroll_to_bottom = true;

        if let (Some(client), Some(model)) = (&self.client, &self.selected_model) {
            let client_clone = client.clone();
            let model_clone = model.clone();
            
            // Crea una copia della conversazione con il contenuto completo per l'ultimo messaggio
            let mut messages_for_api = self.conversation.clone();
            if let Some(last_msg) = messages_for_api.last_mut() {
                last_msg.content = full_content;
            }

            self.chat_promise = Some(Promise::spawn_thread("chat", move || {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(client_clone.chat(&model_clone, &messages_for_api))
            }));
        }
    }
}

impl eframe::App for OllamaChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Usa il tema di sistema (chiaro/scuro)
        let is_dark = ctx.style().visuals.dark_mode;
        
        let mut style = (*ctx.style()).clone();
        
        // Font più grandi e leggibili
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(22.0, egui::FontFamily::Proportional),
        );
        
        // Spaziatura generosa
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(16.0);
        
        // Arrotondamenti più pronunciati
        style.visuals.window_rounding = egui::Rounding::same(12.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        
        // Colori adattivi al tema
        if is_dark {
            // Tema scuro
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(44, 44, 46);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(44, 44, 46);
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(58, 58, 60);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(58, 58, 60);
        } else {
            // Tema chiaro
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(242, 242, 247);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(242, 242, 247);
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(229, 229, 234);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(229, 229, 234);
        }
        
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 122, 255);
        
        // Ombre sottili
        style.visuals.window_shadow = egui::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 16.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(20),
        };
        style.visuals.popup_shadow = egui::Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 24.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(30),
        };
        
        ctx.set_style(style);
        
        // Controlla promise per il caricamento dei modelli
        if let Some(promise) = &self.loading_models_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(models) => {
                        if models.is_empty() {
                            self.error_message = Some(
                                "Nessun modello disponibile. Scarica un modello con 'ollama pull <model>'".to_string()
                            );
                            self.state = AppState::Setup;
                        } else {
                            self.available_models = models.clone();
                            self.selected_model = Some(models[0].name.clone());
                            self.state = AppState::Chat;
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Errore: {}. Assicurati che Ollama sia in esecuzione.", e));
                        self.state = AppState::Setup;
                    }
                }
                self.loading_models_promise = None;
            }
        }

        // Controlla promise per il caricamento file
        if let Some(promise) = &self.file_loading_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok((filename, content)) => {
                        self.attached_files.push((filename.clone(), content.clone()));
                    }
                    Err(e) => {
                        if e.to_string() != "Nessun file selezionato" {
                            self.error_message = Some(format!("Errore caricamento file: {}", e));
                        }
                    }
                }
                self.file_loading_promise = None;
            }
        }

        // Controlla promise per la chat
        if let Some(promise) = &self.chat_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(response) => {
                        self.conversation.push(Message {
                            role: "assistant".to_string(),
                            content: response.clone(),
                            hidden: false,
                            timestamp: Some(get_timestamp()),
                        });
                        self.scroll_to_bottom = true;
                        self.attached_files.clear(); // Pulisci file dopo invio
                        
                        // Se modalità agente abilitata, cerca tool calls nella risposta
                        if self.agent_mode_enabled {
                            let tool_calls = self.agent_system.parse_tool_calls(response);
                            if !tool_calls.is_empty() {
                                self.pending_tool_calls = tool_calls;
                                self.process_next_tool_call();
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Errore: {}", e));
                        self.conversation.pop(); // Rimuovi il messaggio utente
                    }
                }
                self.chat_promise = None;
            }
        }

        // Controlla promise per l'esecuzione dei tool
        if let Some(promise) = &self.tool_execution_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(results) => {
                        // Aggiungi i risultati alla conversazione come messaggio nascosto per il context
                        let mut tool_results_text = String::from("**Risultati Tool:**\n\n");
                        for result in results {
                            tool_results_text.push_str(&result.to_markdown());
                            tool_results_text.push_str("\n\n");
                            
                            // Mostra anche un messaggio visibile all'utente
                            self.conversation.push(Message {
                                role: "system".to_string(),
                                content: format!("🔧 {}", result.to_markdown()),
                                hidden: false,
                                timestamp: Some(get_timestamp()),
                            });
                        }
                        
                        // Aggiungi i risultati al context per il LLM
                        self.conversation.push(Message {
                            role: "user".to_string(),
                            content: tool_results_text,
                            hidden: true,
                            timestamp: None,
                        });
                        
                        self.scroll_to_bottom = true;
                        
                        // Incrementa iterazioni e continua il ciclo agentico se necessario
                        self.current_agent_iteration += 1;
                        if self.current_agent_iteration < self.max_agent_iterations {
                            self.continue_agent_loop();
                        } else {
                            self.error_message = Some("Raggiunto limite massimo di iterazioni agentiche".to_string());
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Errore esecuzione tool: {}", e));
                    }
                }
                self.tool_execution_promise = None;
            }
        }

        // Controlla promise per la scansione di rete
        if let Some(promise) = &self.scanning_promise {
            if let Some(servers) = promise.ready() {
                self.discovered_servers = servers.clone();
                self.state = AppState::Setup;
                
                // Se c'è almeno un server, usa il primo come default
                if !servers.is_empty() {
                    self.ollama_url = servers[0].clone();
                }
                
                self.scanning_promise = None;
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
            match self.state {
                AppState::ScanningNetwork => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(150.0);
                        ui.spinner();
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("🔍 Ricerca server Ollama in corso...").size(18.0));
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Scansione della rete locale")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(142, 142, 147))
                        );
                    });
                }
                AppState::Setup => {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("🤖 MatePro");
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Connettiti a un'istanza Ollama per iniziare")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(142, 142, 147)));
                        ui.add_space(40.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_space(40.0);
                            ui.vertical(|ui| {
                                ui.set_min_width(400.0);
                                
                                // Mostra server scoperti
                                if !self.discovered_servers.is_empty() {
                                    ui.label("Server Ollama trovati:");
                                    ui.add_space(8.0);
                                    
                                    for server in &self.discovered_servers {
                                        let is_selected = &self.ollama_url == server;
                                        let button_text = if server.contains("localhost") || server.contains("127.0.0.1") {
                                            format!("🏠 {} (locale)", server)
                                        } else {
                                            format!("🌐 {}", server)
                                        };
                                        
                                        let button = if is_selected {
                                            egui::Button::new(egui::RichText::new(&button_text).color(egui::Color32::WHITE))
                                                .fill(egui::Color32::from_rgb(0, 122, 255))
                                                .min_size(egui::vec2(400.0, 36.0))
                                        } else {
                                            egui::Button::new(&button_text)
                                                .min_size(egui::vec2(400.0, 36.0))
                                        };
                                        
                                        if ui.add(button).clicked() {
                                            self.ollama_url = server.clone();
                                        }
                                        ui.add_space(4.0);
                                    }
                                    
                                    ui.add_space(16.0);
                                    ui.label("Oppure inserisci un URL personalizzato:");
                                    ui.add_space(6.0);
                                } else {
                                    ui.label("URL dell'istanza Ollama");
                                    ui.add_space(6.0);
                                }
                                
                                let text_edit = egui::TextEdit::singleline(&mut self.ollama_url)
                                    .desired_width(f32::INFINITY)
                                    .min_size(egui::vec2(400.0, 44.0))
                                    .font(egui::TextStyle::Body);
                                ui.add(text_edit);
                                
                                ui.add_space(20.0);
                                
                                ui.horizontal(|ui| {
                                    let connect_button = egui::Button::new(
                                        egui::RichText::new("Connetti").size(16.0).color(egui::Color32::WHITE)
                                    )
                                    .fill(egui::Color32::from_rgb(0, 122, 255))
                                    .min_size(egui::vec2(280.0, 44.0));
                                    
                                    if ui.add(connect_button).clicked() {
                                        self.load_models();
                                    }
                                    
                                    ui.add_space(8.0);
                                    
                                    let rescan_button = egui::Button::new(
                                        egui::RichText::new("🔄 Ricarica").size(16.0)
                                    )
                                    .min_size(egui::vec2(110.0, 44.0));
                                    
                                    if ui.add(rescan_button).clicked() {
                                        self.start_network_scan();
                                    }
                                });

                                if let Some(error) = &self.error_message {
                                    ui.add_space(16.0);
                                    ui.colored_label(egui::Color32::from_rgb(255, 59, 48), error);
                                }
                            });
                        });
                    });
                }
                AppState::LoadingModels => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(150.0);
                        ui.spinner();
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Caricamento modelli...").size(18.0));
                    });
                }
                AppState::Chat => {
                    // Header elegante con selezione modello
                    let is_dark = ui.style().visuals.dark_mode;
                    let header_bg = if is_dark {
                        egui::Color32::from_rgb(28, 28, 30)
                    } else {
                        egui::Color32::from_rgb(248, 248, 248)
                    };
                    
                    egui::Frame::none()
                        .fill(header_bg)
                        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading("💬");
                                ui.add_space(8.0);
                                
                                egui::ComboBox::new("model_selector", "")
                                    .selected_text(egui::RichText::new(self.selected_model.as_ref().unwrap()).size(16.0))
                                    .width(280.0)
                                    .show_ui(ui, |ui| {
                                        for model in &self.available_models {
                                            let (indicator, color) = model.weight_category();
                                            let size_text = format!("{:.1} GB", model.size_gb());
                                            
                                            ui.horizontal(|ui| {
                                                let response = ui.selectable_value(
                                                    &mut self.selected_model,
                                                    Some(model.name.clone()),
                                                    &model.name,
                                                );
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.label(egui::RichText::new(size_text).size(11.0).color(egui::Color32::GRAY));
                                                    ui.label(egui::RichText::new(indicator).color(color));
                                                });
                                                
                                                response
                                            });
                                        }
                                    });
                                
                                ui.add_space(12.0);
                                
                                // Toggle per modalità agente
                                let agent_color = if self.agent_mode_enabled {
                                    egui::Color32::from_rgb(52, 199, 89)
                                } else {
                                    egui::Color32::from_rgb(142, 142, 147)
                                };
                                
                                ui.toggle_value(&mut self.agent_mode_enabled, 
                                    egui::RichText::new("🤖 Modalità Agente")
                                        .color(agent_color)
                                        .size(14.0));
                                
                                if self.agent_mode_enabled {
                                    ui.label(
                                        egui::RichText::new(format!("({}/{})", self.current_agent_iteration, self.max_agent_iterations))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY)
                                    );
                                }
                                
                                ui.add_space(12.0);
                                
                                // Pulsante configurazione SQL Server
                                let sql_btn_text = if self.sql_connection_status.as_ref().map(|s| s.as_str()) == Some("connected") {
                                    egui::RichText::new("🗄️ SQL (✓)")
                                        .color(egui::Color32::from_rgb(52, 199, 89))
                                        .size(14.0)
                                } else {
                                    egui::RichText::new("🗄️ SQL")
                                        .color(egui::Color32::from_rgb(142, 142, 147))
                                        .size(14.0)
                                };
                                
                                if ui.button(sql_btn_text).on_hover_text("Configura database SQL Server").clicked() {
                                    self.show_sql_config = true;
                                }
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let disconnect_btn = egui::Button::new(
                                        egui::RichText::new("✕").size(20.0).strong()
                                    )
                                    .frame(false);
                                    
                                    if ui.add(disconnect_btn).on_hover_text("Disconnetti").clicked() {
                                        *self = Self::default();
                                    }
                                    
                                    ui.add_space(8.0);
                                    
                                    let new_chat_btn = egui::Button::new(
                                        egui::RichText::new("⟲").size(20.0).strong()
                                    )
                                    .frame(false);
                                    
                                    if ui.add(new_chat_btn).on_hover_text("Nuova chat").clicked() {
                                        self.conversation.clear();
                                        self.error_message = None;
                                        self.system_prompt_added = false;
                                        self.current_agent_iteration = 0;
                                        self.agent_system = AgentSystem::new();
                                    }
                                });
                            });
                        });

                    ui.add_space(4.0);

                    // Area messaggi con più spazio
                    let available_height = ui.available_height() - 150.0;
                    
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add_space(12.0);
                            
                            if self.conversation.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(60.0);
                                    ui.label(
                                        egui::RichText::new("Inizia una conversazione")
                                            .size(20.0)
                                            .color(egui::Color32::from_rgb(142, 142, 147))
                                    );
                                    ui.add_space(8.0);
                                    ui.label(
                                        egui::RichText::new("Scrivi un messaggio per iniziare")
                                            .size(14.0)
                                            .color(egui::Color32::from_rgb(174, 174, 178))
                                    );
                                });
                            }
                            
                            for message in &self.conversation {
                                // Salta i messaggi nascosti (istruzioni di sistema)
                                if message.hidden {
                                    continue;
                                }
                                
                                let is_user = message.role == "user";
                                let is_dark = ui.style().visuals.dark_mode;
                                
                                ui.horizontal_top(|ui| {
                                    let max_bubble_width = ui.available_width() * 0.7;
                                    
                                    if is_user {
                                        // Spazio a sinistra per messaggi utente
                                        ui.allocate_space(egui::vec2(ui.available_width() - max_bubble_width, 0.0));
                                    }
                                    
                                    let frame_color = if is_user {
                                        egui::Color32::from_rgb(0, 122, 255)
                                    } else if is_dark {
                                        egui::Color32::from_rgb(58, 58, 60)
                                    } else {
                                        egui::Color32::from_rgb(229, 229, 234)
                                    };
                                    
                                    let text_color = if is_user {
                                        egui::Color32::WHITE
                                    } else if is_dark {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::BLACK
                                    };
                                    
                                    egui::Frame::none()
                                        .fill(frame_color)
                                        .rounding(egui::Rounding::same(18.0))
                                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                                        .show(ui, |ui| {
                                            ui.set_max_width(max_bubble_width);
                                            
                                            if is_user {
                                                // Messaggi utente semplici senza markdown
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&message.content)
                                                            .color(text_color)
                                                            .size(14.5)
                                                    );
                                                    
                                                    // Timestamp in basso a destra
                                                    if let Some(timestamp) = &message.timestamp {
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                                            ui.label(
                                                                egui::RichText::new(timestamp)
                                                                    .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 180))
                                                                    .size(10.0)
                                                            );
                                                        });
                                                    }
                                                });
                                            } else {
                                                // Messaggi assistente con rendering Markdown migliorato
                                                {
                                                    let style = ui.style_mut();
                                                    style.visuals.hyperlink_color = egui::Color32::from_rgb(0, 122, 255);
                                                    
                                                    // Aumenta la dimensione del font per migliore leggibilità
                                                    style.text_styles.insert(
                                                        egui::TextStyle::Body,
                                                        egui::FontId::new(15.0, egui::FontFamily::Proportional),
                                                    );
                                                    style.text_styles.insert(
                                                        egui::TextStyle::Monospace,
                                                        egui::FontId::new(14.0, egui::FontFamily::Monospace),
                                                    );
                                                    style.text_styles.insert(
                                                        egui::TextStyle::Heading,
                                                        egui::FontId::new(18.0, egui::FontFamily::Proportional),
                                                    );
                                                    
                                                    // Aumenta la spaziatura tra elementi
                                                    style.spacing.item_spacing = egui::vec2(8.0, 10.0);
                                                }
                                                
                                                // Rendering markdown con sintassi codice e formule (Unicode)
                                                ui.vertical(|ui| {
                                                    CommonMarkViewer::new().show(
                                                        ui,
                                                        &mut self.markdown_cache,
                                                        &message.content,
                                                    );
                                                    
                                                    // Timestamp in basso a sinistra per l'assistente
                                                    if let Some(timestamp) = &message.timestamp {
                                                        ui.label(
                                                            egui::RichText::new(timestamp)
                                                                .color(egui::Color32::from_rgb(142, 142, 147))
                                                                .size(10.0)
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                });
                                
                                ui.add_space(10.0);
                            }

                            // Indicatore di caricamento elegante
                            if self.chat_promise.is_some() {
                                let is_dark = ui.style().visuals.dark_mode;
                                let loading_bg = if is_dark {
                                    egui::Color32::from_rgb(58, 58, 60)
                                } else {
                                    egui::Color32::from_rgb(229, 229, 234)
                                };
                                
                                ui.horizontal(|ui| {
                                    ui.add_space(0.0);
                                    egui::Frame::none()
                                        .fill(loading_bg)
                                        .rounding(egui::Rounding::same(18.0))
                                        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(egui::RichText::new("Sto pensando...").size(14.5));
                                            });
                                        });
                                });
                                ui.add_space(10.0);
                            }

                            if self.scroll_to_bottom {
                                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                self.scroll_to_bottom = false;
                            }
                        });

                    // Mostra errori eleganti
                    if let Some(error) = &self.error_message {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(255, 239, 239))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.colored_label(egui::Color32::from_rgb(255, 59, 48), format!("⚠️ {}", error));
                            });
                    }

                    ui.add_space(12.0);

                    // Input area spaziosa e moderna - stile Apple
                    let is_dark = ui.style().visuals.dark_mode;
                    let input_bg = if is_dark {
                        egui::Color32::from_rgb(28, 28, 30)
                    } else {
                        egui::Color32::from_rgb(248, 248, 248)
                    };
                    
                    egui::Frame::none()
                        .fill(input_bg)
                        .inner_margin(egui::Margin::symmetric(20.0, 12.0))
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width() - 8.0); // Margine interno extra
                            ui.vertical(|ui| {
                                // Mostra file allegati
                                if !self.attached_files.is_empty() {
                                    let mut to_remove = None;
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0; // Spaziatura tra chip
                                        for (i, (filename, _)) in self.attached_files.iter().enumerate() {
                                            let chip_color = if is_dark {
                                                egui::Color32::from_rgb(48, 48, 50)
                                            } else {
                                                egui::Color32::from_rgb(229, 229, 234)
                                            };
                                            
                                            egui::Frame::none()
                                                .fill(chip_color)
                                                .rounding(egui::Rounding::same(12.0))
                                                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new("📎").size(12.0));
                                                        ui.label(egui::RichText::new(filename).size(12.0));
                                                        
                                                        let remove_btn = egui::Button::new(
                                                            egui::RichText::new("✕").size(10.0)
                                                        )
                                                        .frame(false)
                                                        .small();
                                                        
                                                        if ui.add(remove_btn).clicked() {
                                                            to_remove = Some(i);
                                                        }
                                                    });
                                                });
                                        }
                                    });
                                    
                                    if let Some(index) = to_remove {
                                        self.attached_files.remove(index);
                                    }
                                    
                                    ui.add_space(8.0);
                                }
                                
                                ui.horizontal(|ui| {
                                    // Area di testo multilinea grande e confortevole
                                    let text_edit = egui::TextEdit::multiline(&mut self.input_text)
                                        .desired_rows(3)
                                        .hint_text("Scrivi un messaggio...")
                                        .font(egui::TextStyle::Body);
                                    
                                    // Calcola larghezza considerando i pulsanti (circa 100px) + margini
                                    let buttons_width = 100.0;
                                    let response = ui.add_sized(
                                        egui::vec2(ui.available_width() - buttons_width, 80.0),
                                        text_edit
                                    );
                                    
                                    // Invia con Cmd+Enter o Ctrl+Enter
                                    let modifiers = ui.input(|i| i.modifiers);
                                    if response.has_focus() 
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        && (modifiers.command || modifiers.ctrl) {
                                        self.send_message();
                                        response.request_focus();
                                    }

                                    ui.add_space(8.0);
                                    
                                    // Pulsanti orizzontali
                                    ui.horizontal(|ui| {
                                        // Pulsante allegato file
                                        let attach_color = if is_dark {
                                            egui::Color32::from_rgb(99, 99, 102)
                                        } else {
                                            egui::Color32::from_rgb(174, 174, 178)
                                        };
                                        
                                        let attach_button = egui::Button::new(
                                            egui::RichText::new("📎").size(16.0)
                                        )
                                        .fill(attach_color)
                                        .rounding(egui::Rounding::same(22.0))
                                        .min_size(egui::vec2(44.0, 44.0));

                                        if ui.add(attach_button)
                                            .on_hover_text("Allega file (PDF, Excel, TXT)")
                                            .clicked() {
                                            self.open_file_dialog();
                                        }
                                        
                                        ui.add_space(4.0);
                                        
                                        // Pulsante di invio grande e tondeggiante
                                        let button_enabled = self.chat_promise.is_none() 
                                            && (!self.input_text.trim().is_empty() || !self.attached_files.is_empty());
                                        let button_color = if button_enabled {
                                            egui::Color32::from_rgb(0, 122, 255)
                                        } else {
                                            egui::Color32::from_rgb(142, 142, 147)
                                        };
                                        
                                        let send_button = egui::Button::new(
                                            egui::RichText::new("▶").size(18.0).color(egui::Color32::WHITE).strong()
                                        )
                                        .fill(button_color)
                                        .rounding(egui::Rounding::same(22.0))
                                        .min_size(egui::vec2(44.0, 44.0));

                                        if ui.add_enabled(button_enabled, send_button)
                                            .on_hover_text("Invia (Ctrl+Enter)")
                                            .clicked() {
                                            self.send_message();
                                        }
                                    });
                                });
                                
                                // Suggerimento tasti rapidi
                                ui.add_space(4.0);
                                let hint_color = if is_dark {
                                    egui::Color32::from_rgb(142, 142, 147)
                                } else {
                                    egui::Color32::from_rgb(142, 142, 147)
                                };
                                ui.label(
                                    egui::RichText::new("Premi Ctrl+Enter per inviare")
                                        .size(11.0)
                                        .color(hint_color)
                                );
                            });
                        });
                }
            }
        });

        // Modale di conferma per tool pericolosi
        if let Some(tool_call) = self.awaiting_confirmation.clone() {
            let mut should_confirm = false;
            let mut should_cancel = false;
            
            egui::Window::new("⚠️ Conferma Operazione")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(400.0);
                    
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("L'agente vuole eseguire un'operazione potenzialmente pericolosa:")
                                .size(15.0)
                        );
                        ui.add_space(12.0);
                        
                        // Mostra dettagli del tool
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(248, 248, 248))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format!("Tool: {}", tool_call.tool_name)).strong());
                                ui.add_space(8.0);
                                ui.label("Parametri:");
                                for (key, value) in &tool_call.parameters {
                                    ui.label(format!("  {}: {}", key, value));
                                }
                            });
                        
                        ui.add_space(16.0);
                        
                        ui.horizontal(|ui| {
                            let allow_btn = egui::Button::new(
                                egui::RichText::new("✓ Consenti").size(14.0).color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(52, 199, 89))
                            .min_size(egui::vec2(150.0, 36.0));
                            
                            if ui.add(allow_btn).on_hover_text("Esegui l'operazione").clicked() {
                                should_confirm = true;
                            }
                            
                            ui.add_space(8.0);
                            
                            let cancel_btn = egui::Button::new(
                                egui::RichText::new("✕ Annulla").size(14.0)
                            )
                            .fill(egui::Color32::from_rgb(255, 59, 48))
                            .min_size(egui::vec2(150.0, 36.0));
                            
                            if ui.add(cancel_btn).on_hover_text("Non eseguire").clicked() {
                                should_cancel = true;
                            }
                        });
                        
                        ui.add_space(10.0);
                    });
                });
            
            if should_confirm {
                self.confirm_dangerous_tool();
            } else if should_cancel {
                self.cancel_dangerous_tool();
            }
        }

        // Finestra configurazione SQL Server
        if self.show_sql_config {
            let mut should_close = false;
            let mut should_test = false;
            
            egui::Window::new("🗄️ Configurazione SQL Server")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(500.0);
                    
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        
                        // Server
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Server:").size(14.0).strong());
                            ui.add_space(8.0);
                            ui.text_edit_singleline(&mut self.sql_server);
                        });
                        ui.label(egui::RichText::new("  (es: localhost, 192.168.1.10, server.domain.com)").size(11.0).color(egui::Color32::GRAY));
                        
                        ui.add_space(8.0);
                        
                        // Database
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Database:").size(14.0).strong());
                            ui.add_space(8.0);
                            ui.text_edit_singleline(&mut self.sql_database);
                        });
                        
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                        
                        // Metodo autenticazione
                        ui.label(egui::RichText::new("Autenticazione:").size(14.0).strong());
                        ui.add_space(8.0);
                        
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.sql_auth_method, "windows".to_string(), 
                                egui::RichText::new("🪟 Windows (Integrated)").size(13.0));
                            ui.add_space(16.0);
                            ui.radio_value(&mut self.sql_auth_method, "sql".to_string(), 
                                egui::RichText::new("🔑 SQL Authentication").size(13.0));
                        });
                        
                        if self.sql_auth_method == "windows" {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("  ℹ️ Su Windows con dominio, verranno usate le credenziali dell'utente corrente.")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(0, 122, 255)));
                        }
                        
                        // Username e Password (solo per SQL Auth)
                        if self.sql_auth_method == "sql" {
                            ui.add_space(12.0);
                            
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Username:").size(14.0));
                                ui.add_space(8.0);
                                ui.text_edit_singleline(&mut self.sql_username);
                            });
                            
                            ui.add_space(8.0);
                            
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Password:").size(14.0));
                                ui.add_space(8.0);
                                let password_edit = egui::TextEdit::singleline(&mut self.sql_password)
                                    .password(true);
                                ui.add(password_edit);
                            });
                        }
                        
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                        
                        // Status connessione
                        if let Some(status) = &self.sql_connection_status {
                            let (icon, color) = if status == "connected" {
                                ("✓", egui::Color32::from_rgb(52, 199, 89))
                            } else if status == "connecting" {
                                ("⟳", egui::Color32::from_rgb(0, 122, 255))
                            } else if status.starts_with("error:") {
                                ("✕", egui::Color32::from_rgb(255, 59, 48))
                            } else {
                                ("", egui::Color32::GRAY)
                            };
                            
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(icon).size(16.0).color(color));
                                ui.label(egui::RichText::new(status).size(13.0).color(color));
                            });
                            
                            ui.add_space(12.0);
                        }
                        
                        // Nota read-only
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(255, 249, 196))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("🔒 SOLO LETTURA: Le query sono limitate a SELECT. UPDATE, INSERT, DELETE non sono permesse.")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(138, 109, 0)));
                            });
                        
                        ui.add_space(16.0);
                        
                        // Pulsanti
                        ui.horizontal(|ui| {
                            let test_btn = egui::Button::new(
                                egui::RichText::new("🔌 Test Connessione").size(14.0).color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(0, 122, 255))
                            .min_size(egui::vec2(160.0, 36.0));
                            
                            if ui.add(test_btn).clicked() && self.sql_test_promise.is_none() {
                                should_test = true;
                            }
                            
                            ui.add_space(8.0);
                            
                            let close_btn = egui::Button::new(
                                egui::RichText::new("Chiudi").size(14.0)
                            )
                            .min_size(egui::vec2(100.0, 36.0));
                            
                            if ui.add(close_btn).clicked() {
                                should_close = true;
                            }
                        });
                        
                        ui.add_space(10.0);
                    });
                });
            
            if should_close {
                self.show_sql_config = false;
            }
            
            if should_test {
                self.test_sql_connection();
            }
        }
        
        // Controlla promise test connessione SQL
        if let Some(promise) = &self.sql_test_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(connection_id) => {
                        self.sql_connection_status = Some("connected".to_string());
                        // Mostra anche l'ID connessione nel log (opzionale)
                        println!("✅ Connessione SQL stabilita: {}", connection_id);
                    }
                    Err(e) => {
                        self.sql_connection_status = Some(format!("error: {}", e));
                    }
                }
                self.sql_test_promise = None;
            }
        }

        // Richiedi un nuovo frame se ci sono promise in corso
        if self.scanning_promise.is_some() 
            || self.loading_models_promise.is_some() 
            || self.chat_promise.is_some()
            || self.file_loading_promise.is_some()
            || self.tool_execution_promise.is_some()
            || self.sql_test_promise.is_some() {
            ctx.request_repaint();
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 500.0])
            .with_title("MatePro"),
        ..Default::default()
    };

    eframe::run_native(
        "MatePro",
        options,
        Box::new(|cc| Ok(Box::new(OllamaChatApp::new(cc)))),
    )
}

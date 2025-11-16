use anyhow::{Context, Result};
use eframe::egui;
use poll_promise::Promise;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

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
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Message,
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

    async fn list_models(&self) -> Result<Vec<String>> {
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
            .filter_map(|m| m["name"].as_str().map(String::from))
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
    available_models: Vec<String>,
    selected_model: Option<String>,
    conversation: Vec<Message>,
    input_text: String,
    error_message: Option<String>,
    client: Option<OllamaClient>,
    scanning_promise: Option<Promise<Vec<String>>>,
    loading_models_promise: Option<Promise<Result<Vec<String>>>>,
    chat_promise: Option<Promise<Result<String>>>,
    scroll_to_bottom: bool,
    markdown_cache: CommonMarkCache,
    system_prompt_added: bool,
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

    fn send_message(&mut self) {
        if self.input_text.trim().is_empty() {
            return;
        }
        
        // Aggiungi istruzioni di formattazione solo alla prima interazione
        if !self.system_prompt_added && self.conversation.is_empty() {
            // Usa un approccio user/assistant per garantire che il modello capisca
            let instruction = Message {
                role: "user".to_string(),
                content: "IMPORTANTE: Per questa conversazione, quando devi mostrare formule matematiche NON usare LaTeX (no \\frac, \\sqrt, \\( \\), ecc). Usa SOLO:

• Caratteri Unicode: √ ² ³ ∫ ∑ π ∞ ≤ ≥ ≠ ± × ÷
• Notazione testuale: sqrt(), ^2, ^3, /
• Esempi: 
  - x = (-b ± √(b² - 4ac)) / (2a)
  - a² + b² = c²
  - lim(x→∞) f(x)

Conferma che userai solo Unicode e notazione testuale, MAI LaTeX.".to_string(),
            };
            
            let confirmation = Message {
                role: "assistant".to_string(),
                content: "Perfetto! Userò solo caratteri Unicode (√, ², ³, π, ±, ecc.) e notazione testuale chiara (sqrt, ^2, /) per le formule matematiche. Non userò LaTeX. Sono pronto ad aiutarti!".to_string(),
            };
            
            self.conversation.push(instruction);
            self.conversation.push(confirmation);
            self.system_prompt_added = true;
        }

        let user_message = Message {
            role: "user".to_string(),
            content: self.input_text.trim().to_string(),
        };

        self.conversation.push(user_message);
        self.input_text.clear();
        self.scroll_to_bottom = true;

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
                            self.selected_model = Some(models[0].clone());
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

        // Controlla promise per la chat
        if let Some(promise) = &self.chat_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(response) => {
                        self.conversation.push(Message {
                            role: "assistant".to_string(),
                            content: response.clone(),
                        });
                        self.scroll_to_bottom = true;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Errore: {}", e));
                        self.conversation.pop(); // Rimuovi il messaggio utente
                    }
                }
                self.chat_promise = None;
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

        egui::CentralPanel::default().show(ctx, |ui| {
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
                                    .width(200.0)
                                    .show_ui(ui, |ui| {
                                        for model in &self.available_models {
                                            ui.selectable_value(
                                                &mut self.selected_model,
                                                Some(model.clone()),
                                                model,
                                            );
                                        }
                                    });
                                
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
                                                ui.label(
                                                    egui::RichText::new(&message.content)
                                                        .color(text_color)
                                                        .size(14.5)
                                                );
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
                                                CommonMarkViewer::new().show(
                                                    ui,
                                                    &mut self.markdown_cache,
                                                    &message.content,
                                                );
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
                        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    // Area di testo multilinea grande e confortevole
                                    let text_edit = egui::TextEdit::multiline(&mut self.input_text)
                                        .desired_rows(3)
                                        .hint_text("Scrivi un messaggio...")
                                        .font(egui::TextStyle::Body);
                                    
                                    let response = ui.add_sized(
                                        egui::vec2(ui.available_width() - 62.0, 80.0),
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
                                    
                                    // Pulsante di invio grande e tondeggiante - allineato in basso
                                    ui.vertical(|ui| {
                                        ui.add_space(30.0); // Sposta il pulsante in basso
                                        
                                        let button_enabled = self.chat_promise.is_none() && !self.input_text.trim().is_empty();
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

        // Richiedi un nuovo frame se ci sono promise in corso
        if self.scanning_promise.is_some() 
            || self.loading_models_promise.is_some() 
            || self.chat_promise.is_some() {
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

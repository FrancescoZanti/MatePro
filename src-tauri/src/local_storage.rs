// Local Storage Module
// Handles local persistence of conversation memory and custom system prompt
// Data is stored on the PC running MatePro, independent of the server

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Directory name for MatePro data
const DATA_DIR_NAME: &str = "MatePro";
/// File name for storing conversation memory
const MEMORY_FILE_NAME: &str = "memory.json";
/// File name for storing custom system prompt
const SYSTEM_PROMPT_FILE_NAME: &str = "system_prompt.json";
/// File name for storing calendar integrations
const CALENDAR_INTEGRATIONS_FILE_NAME: &str = "calendar_integrations.json";
/// File name for storing calendar events
const CALENDAR_FILE_NAME: &str = "calendar.json";

/// A single conversation entry stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    /// Unique identifier for the conversation
    pub id: String,
    /// Title or summary of the conversation
    pub title: String,
    /// Messages in the conversation
    pub messages: Vec<MemoryMessage>,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// Model used for this conversation
    pub model: Option<String>,
}

/// A message stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub hidden: bool,
    pub timestamp: Option<String>,
}

/// Local memory storage containing all conversations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalMemory {
    /// Version of the memory format for future migrations
    pub version: u32,
    /// List of conversations
    pub conversations: Vec<ConversationEntry>,
}

impl LocalMemory {
    pub fn new() -> Self {
        Self {
            version: 1,
            conversations: Vec::new(),
        }
    }
}

/// Custom system prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSystemPrompt {
    /// Whether the custom system prompt is enabled
    pub enabled: bool,
    /// The custom system prompt text
    pub content: String,
    /// When the prompt was last updated
    pub updated_at: DateTime<Utc>,
}

/// Calendar event stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Unique identifier of the event
    pub id: String,
    /// Event title
    pub title: String,
    /// Optional description/details
    #[serde(default)]
    pub description: Option<String>,
    /// Start time of the event (UTC)
    pub start: DateTime<Utc>,
    /// Optional end time (UTC)
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
    /// Raw text fragment that generated this event
    #[serde(default)]
    pub source_text: Option<String>,
    /// Timestamp metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Calendar storage wrapper
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarData {
    /// Version for potential migrations
    pub version: u32,
    /// Stored events
    pub events: Vec<CalendarEvent>,
}

impl CalendarData {
    pub fn new() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
        }
    }
}

/// Pending device flow information for OAuth-based integrations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingDeviceFlow {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expires_at: DateTime<Utc>,
    pub interval: u64,
    pub message: Option<String>,
}

/// Pending PKCE authorization flow (OAuth2 Authorization Code + PKCE)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingPkceFlow {
    /// Full authorization URL to open in the browser
    pub authorization_url: String,
    /// Redirect URI bound locally (loopback)
    pub redirect_uri: String,
    /// PKCE verifier (kept locally)
    pub code_verifier: String,
    /// CSRF state
    pub state: String,
    /// Authorization code ricevuto dal redirect (quando presente)
    #[serde(default)]
    pub authorization_code: Option<String>,
    /// Eventuale errore restituito dal provider OAuth
    #[serde(default)]
    pub error: Option<String>,
    /// When the flow expires
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expires_at: DateTime<Utc>,
    /// Optional message surfaced to the UI
    pub message: Option<String>,
}

/// Configuration for the Outlook calendar integration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutlookIntegrationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub tenant: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub pending: Option<PendingDeviceFlow>,
    #[serde(default)]
    pub pending_pkce: Option<PendingPkceFlow>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

/// Configuration for the Google Calendar integration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleCalendarIntegrationConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub calendar_id: Option<String>,
    #[serde(default)]
    pub pending: Option<PendingDeviceFlow>,
    #[serde(default)]
    pub pending_pkce: Option<PendingPkceFlow>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

/// Stored calendar integrations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarIntegrations {
    pub version: u32,
    #[serde(default)]
    pub outlook: Option<OutlookIntegrationConfig>,
    #[serde(default)]
    pub google: Option<GoogleCalendarIntegrationConfig>,
}

impl CalendarIntegrations {
    pub fn new() -> Self {
        Self {
            version: 1,
            outlook: None,
            google: None,
        }
    }
}

impl Default for CustomSystemPrompt {
    fn default() -> Self {
        Self {
            enabled: false,
            content: String::new(),
            updated_at: Utc::now(),
        }
    }
}

/// Get the data directory for MatePro
fn get_data_dir() -> Result<PathBuf> {
    let base_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
        .context("Impossibile determinare la directory dati dell'utente")?;

    let data_dir = base_dir.join(DATA_DIR_NAME);

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .context("Impossibile creare la directory dati di MatePro")?;
    }

    Ok(data_dir)
}

/// Load the local memory from disk
pub fn load_memory() -> Result<LocalMemory> {
    let data_dir = get_data_dir()?;
    let memory_path = data_dir.join(MEMORY_FILE_NAME);

    if !memory_path.exists() {
        return Ok(LocalMemory::new());
    }

    let content = fs::read_to_string(&memory_path)
        .context("Impossibile leggere il file di memoria")?;

    let memory: LocalMemory = serde_json::from_str(&content)
        .context("Impossibile analizzare il file di memoria")?;

    Ok(memory)
}

/// Save the local memory to disk
pub fn save_memory(memory: &LocalMemory) -> Result<()> {
    let data_dir = get_data_dir()?;
    let memory_path = data_dir.join(MEMORY_FILE_NAME);

    let content = serde_json::to_string_pretty(memory)
        .context("Impossibile serializzare la memoria")?;

    fs::write(&memory_path, content)
        .context("Impossibile salvare il file di memoria")?;

    Ok(())
}

/// Load the custom system prompt from disk
pub fn load_custom_system_prompt() -> Result<CustomSystemPrompt> {
    let data_dir = get_data_dir()?;
    let prompt_path = data_dir.join(SYSTEM_PROMPT_FILE_NAME);

    if !prompt_path.exists() {
        return Ok(CustomSystemPrompt::default());
    }

    let content = fs::read_to_string(&prompt_path)
        .context("Impossibile leggere il file del system prompt")?;

    let prompt: CustomSystemPrompt = serde_json::from_str(&content)
        .context("Impossibile analizzare il file del system prompt")?;

    Ok(prompt)
}

/// Save the custom system prompt to disk
pub fn save_custom_system_prompt(prompt: &CustomSystemPrompt) -> Result<()> {
    let data_dir = get_data_dir()?;
    let prompt_path = data_dir.join(SYSTEM_PROMPT_FILE_NAME);

    let content = serde_json::to_string_pretty(prompt)
        .context("Impossibile serializzare il system prompt")?;

    fs::write(&prompt_path, content)
        .context("Impossibile salvare il file del system prompt")?;

    Ok(())
}

/// Add a new conversation to memory
pub fn add_conversation(title: String, messages: Vec<MemoryMessage>, model: Option<String>) -> Result<String> {
    let mut memory = load_memory()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let entry = ConversationEntry {
        id: id.clone(),
        title,
        messages,
        created_at: now,
        updated_at: now,
        model,
    };

    memory.conversations.push(entry);
    save_memory(&memory)?;

    Ok(id)
}

/// Update an existing conversation in memory
pub fn update_conversation(id: &str, messages: Vec<MemoryMessage>) -> Result<()> {
    let mut memory = load_memory()?;

    if let Some(entry) = memory.conversations.iter_mut().find(|e| e.id == id) {
        entry.messages = messages;
        entry.updated_at = Utc::now();
        save_memory(&memory)?;
        Ok(())
    } else {
        anyhow::bail!("Conversazione non trovata: {}", id)
    }
}

/// Delete a conversation from memory
pub fn delete_conversation(id: &str) -> Result<()> {
    let mut memory = load_memory()?;
    let initial_len = memory.conversations.len();

    memory.conversations.retain(|e| e.id != id);

    if memory.conversations.len() == initial_len {
        anyhow::bail!("Conversazione non trovata: {}", id)
    }

    save_memory(&memory)?;
    Ok(())
}

/// Clear all conversations from memory
pub fn clear_all_conversations() -> Result<()> {
    let memory = LocalMemory::new();
    save_memory(&memory)?;
    Ok(())
}

/// Get the path to the data directory (for debugging/information purposes)
pub fn get_data_directory() -> Result<String> {
    let data_dir = get_data_dir()?;
    Ok(data_dir.to_string_lossy().to_string())
}

fn load_calendar_integrations_data() -> Result<CalendarIntegrations> {
    let data_dir = get_data_dir()?;
    let integrations_path = data_dir.join(CALENDAR_INTEGRATIONS_FILE_NAME);

    if !integrations_path.exists() {
        return Ok(CalendarIntegrations::new());
    }

    let content = fs::read_to_string(&integrations_path)
        .context("Impossibile leggere il file delle integrazioni calendario")?;

    let integrations: CalendarIntegrations = serde_json::from_str(&content)
        .context("Impossibile analizzare il file delle integrazioni calendario")?;

    Ok(integrations)
}

fn save_calendar_integrations_data(integrations: &CalendarIntegrations) -> Result<()> {
    let data_dir = get_data_dir()?;
    let integrations_path = data_dir.join(CALENDAR_INTEGRATIONS_FILE_NAME);

    let content = serde_json::to_string_pretty(integrations)
        .context("Impossibile serializzare le integrazioni calendario")?;

    fs::write(&integrations_path, content)
        .context("Impossibile salvare il file delle integrazioni calendario")?;

    Ok(())
}

fn load_calendar_data() -> Result<CalendarData> {
    let data_dir = get_data_dir()?;
    let calendar_path = data_dir.join(CALENDAR_FILE_NAME);

    if !calendar_path.exists() {
        return Ok(CalendarData::new());
    }

    let content = fs::read_to_string(&calendar_path)
        .context("Impossibile leggere il file del calendario")?;

    let calendar: CalendarData = serde_json::from_str(&content)
        .context("Impossibile analizzare il file del calendario")?;

    Ok(calendar)
}

fn save_calendar_data(calendar: &CalendarData) -> Result<()> {
    let data_dir = get_data_dir()?;
    let calendar_path = data_dir.join(CALENDAR_FILE_NAME);

    let content = serde_json::to_string_pretty(calendar)
        .context("Impossibile serializzare il calendario")?;

    fs::write(&calendar_path, content)
        .context("Impossibile salvare il file del calendario")?;

    Ok(())
}

/// Load all calendar events
pub fn load_calendar_events() -> Result<Vec<CalendarEvent>> {
    let calendar = load_calendar_data()?;
    Ok(calendar.events)
}

/// Add a new calendar event returning its id
pub fn add_calendar_event(
    title: String,
    description: Option<String>,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    source_text: Option<String>,
) -> Result<String> {
    let mut calendar = load_calendar_data()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    let event = CalendarEvent {
        id: id.clone(),
        title,
        description,
        start,
        end,
        source_text,
        created_at: now,
        updated_at: now,
    };

    calendar.events.push(event);
    save_calendar_data(&calendar)?;

    Ok(id)
}

/// Update an existing calendar event
pub fn update_calendar_event(event: CalendarEvent) -> Result<()> {
    let mut calendar = load_calendar_data()?;

    if let Some(existing) = calendar.events.iter_mut().find(|e| e.id == event.id) {
        *existing = CalendarEvent {
            updated_at: Utc::now(),
            ..event
        };
        save_calendar_data(&calendar)?;
        Ok(())
    } else {
        anyhow::bail!("Evento non trovato: {}", event.id)
    }
}

/// Delete a calendar event by id
pub fn delete_calendar_event(id: &str) -> Result<()> {
    let mut calendar = load_calendar_data()?;
    let initial_len = calendar.events.len();
    calendar.events.retain(|event| event.id != id);

    if calendar.events.len() == initial_len {
        anyhow::bail!("Evento non trovato: {}", id)
    }

    save_calendar_data(&calendar)?;
    Ok(())
}

/// Clear all stored calendar events
pub fn clear_calendar_events() -> Result<()> {
    let calendar = CalendarData::new();
    save_calendar_data(&calendar)?;
    Ok(())
}

fn escape_ics_text(text: &str) -> String {
    text.replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

/// Export events to an ICS file and return its path
pub fn export_calendar_to_ics() -> Result<String> {
    let calendar = load_calendar_data()?;
    let data_dir = get_data_dir()?;
    let ics_path = data_dir.join("calendar.ics");

    let mut lines = Vec::new();
    lines.push("BEGIN:VCALENDAR".to_string());
    lines.push("VERSION:2.0".to_string());
    lines.push("PRODID:-//MatePro//Calendar//EN".to_string());

    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    for event in calendar.events {
        let start_str = event.start.format("%Y%m%dT%H%M%SZ").to_string();
        let end_dt = event
            .end
            .unwrap_or_else(|| event.start + chrono::Duration::hours(1));
        let end_str = end_dt.format("%Y%m%dT%H%M%SZ").to_string();

        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}@matepro", event.id));
        lines.push(format!("DTSTAMP:{}", now));
        lines.push(format!("DTSTART:{}", start_str));
        lines.push(format!("DTEND:{}", end_str));
        lines.push(format!("SUMMARY:{}", escape_ics_text(&event.title)));
        if let Some(desc) = event.description.as_ref() {
            lines.push(format!(
                "DESCRIPTION:{}",
                escape_ics_text(desc)
            ));
        }
        if let Some(src) = event.source_text.as_ref() {
            lines.push(format!(
                "X-MATEPRO-SOURCE:{}",
                escape_ics_text(src)
            ));
        }
        lines.push("END:VEVENT".to_string());
    }

    lines.push("END:VCALENDAR".to_string());

    let ics_content = lines.join("\r\n");
    fs::write(&ics_path, ics_content)
        .context("Impossibile scrivere il file ICS")?;

    Ok(ics_path.to_string_lossy().to_string())
}

/// Load stored calendar integrations
pub fn load_calendar_integrations() -> Result<CalendarIntegrations> {
    load_calendar_integrations_data()
}

/// Save calendar integrations to disk
pub fn save_calendar_integrations(integrations: &CalendarIntegrations) -> Result<()> {
    save_calendar_integrations_data(integrations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_memory_serialization() {
        let memory = LocalMemory::new();
        let json = serde_json::to_string(&memory).unwrap();
        let parsed: LocalMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert!(parsed.conversations.is_empty());
    }

    #[test]
    fn test_custom_system_prompt_serialization() {
        let prompt = CustomSystemPrompt {
            enabled: true,
            content: "Test prompt".to_string(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&prompt).unwrap();
        let parsed: CustomSystemPrompt = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.content, "Test prompt");
    }
}

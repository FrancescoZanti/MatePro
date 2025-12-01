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

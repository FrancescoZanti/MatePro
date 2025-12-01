// AIConnect Module
// Integration with AIConnect orchestrator via mDNS discovery
// Supports fallback to local Ollama when AIConnect is unavailable

use anyhow::{anyhow, Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// Service types for mDNS discovery
pub const AICONNECT_SERVICE_TYPE: &str = "_aiconnect._tcp.local.";
pub const OLLAMA_SERVICE_TYPE: &str = "_ollama._tcp.local.";

/// Backend kind for the application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    AiConnect,
    #[default]
    OllamaLocal,
}

/// Authentication method for AIConnect
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    #[default]
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
}

/// Discovered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub service_type: String,
    pub properties: HashMap<String, String>,
}

impl DiscoveredService {
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// AIConnect node information from /internal/nodes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConnectNode {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub address: Option<String>,
}

/// Response from AIConnect /internal/nodes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesResponse {
    pub nodes: Vec<AiConnectNode>,
}

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub kind: BackendKind,
    pub endpoint: String,
    pub auth: AuthMethod,
    #[serde(default)]
    pub aiconnect_service: Option<DiscoveredService>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            kind: BackendKind::OllamaLocal,
            endpoint: "http://localhost:11434".to_string(),
            auth: AuthMethod::None,
            aiconnect_service: None,
        }
    }
}

/// AIConnect client with authentication support
pub struct AiConnectClient {
    http_client: reqwest::Client,
    config: Arc<Mutex<BackendConfig>>,
}

impl AiConnectClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            config: Arc::new(Mutex::new(BackendConfig::default())),
        }
    }

    pub fn with_config(config: BackendConfig) -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            config: Arc::new(Mutex::new(config)),
        }
    }

    /// Get the current backend configuration
    pub async fn get_config(&self) -> BackendConfig {
        self.config.lock().await.clone()
    }

    /// Update the backend configuration
    pub async fn set_config(&self, config: BackendConfig) {
        let mut guard = self.config.lock().await;
        *guard = config;
    }

    /// Get the current endpoint URL
    pub async fn get_endpoint(&self) -> String {
        self.config.lock().await.endpoint.clone()
    }

    /// Get the current backend kind
    pub async fn get_backend_kind(&self) -> BackendKind {
        self.config.lock().await.kind.clone()
    }

    /// Build authorization headers based on the auth method
    fn build_auth_headers(auth: &AuthMethod) -> HeaderMap {
        let mut headers = HeaderMap::new();

        match auth {
            AuthMethod::None => {}
            AuthMethod::Bearer { token } => {
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                    headers.insert(AUTHORIZATION, value);
                }
            }
            AuthMethod::Basic { username, password } => {
                use base64::Engine;
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                if let Ok(value) = HeaderValue::from_str(&format!("Basic {}", encoded)) {
                    headers.insert(AUTHORIZATION, value);
                }
            }
        }

        headers
    }

    /// Make an authenticated GET request
    pub async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let config = self.config.lock().await;
        let url = format!("{}{}", config.endpoint, path);
        let headers = Self::build_auth_headers(&config.auth);

        self.http_client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context(format!("GET request to {} failed", url))
    }

    /// Make an authenticated POST request with JSON body
    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<reqwest::Response> {
        let config = self.config.lock().await;
        let url = format!("{}{}", config.endpoint, path);
        let headers = Self::build_auth_headers(&config.auth);

        self.http_client
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .context(format!("POST request to {} failed", url))
    }

    /// Check if the backend is reachable
    pub async fn is_reachable(&self) -> bool {
        let config = self.config.lock().await;

        match config.kind {
            BackendKind::AiConnect => {
                // AIConnect uses /api/health or similar
                let url = format!("{}/api/health", config.endpoint);
                let headers = Self::build_auth_headers(&config.auth);

                match self.http_client.get(&url).headers(headers).send().await {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
            BackendKind::OllamaLocal => {
                // Ollama uses /api/tags
                let url = format!("{}/api/tags", config.endpoint);

                match self.http_client.get(&url).send().await {
                    Ok(response) => response.status().is_success(),
                    Err(_) => false,
                }
            }
        }
    }

    /// Get active nodes from AIConnect (only available when backend is AIConnect)
    pub async fn get_nodes(&self) -> Result<Vec<AiConnectNode>> {
        let config = self.config.lock().await;

        if config.kind != BackendKind::AiConnect {
            return Err(anyhow!(
                "get_nodes is only available when using AIConnect backend"
            ));
        }

        let url = format!("{}/internal/nodes", config.endpoint);
        let headers = Self::build_auth_headers(&config.auth);

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("Failed to fetch AIConnect nodes")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "AIConnect nodes request failed with status: {}",
                response.status()
            ));
        }

        let nodes_response: NodesResponse = response
            .json()
            .await
            .context("Failed to parse AIConnect nodes response")?;

        Ok(nodes_response.nodes)
    }
}

impl Default for AiConnectClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover services via mDNS
pub async fn discover_services(
    service_type: &str,
    timeout: Duration,
) -> Result<Vec<DiscoveredService>> {
    let mdns = ServiceDaemon::new().context("Failed to create mDNS daemon")?;
    let receiver = mdns
        .browse(service_type)
        .context("Failed to start mDNS browse")?;

    let mut services = Vec::new();
    let deadline = std::time::Instant::now() + timeout;

    loop {
        let now = std::time::Instant::now();
        if now > deadline {
            break;
        }

        // Use checked subtraction to avoid potential panics
        let remaining = deadline
            .checked_duration_since(now)
            .unwrap_or(Duration::from_millis(0))
            .min(Duration::from_millis(100));

        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, async {
            receiver.recv().ok()
        })
        .await
        {
            Ok(Some(event)) => match event {
                ServiceEvent::ServiceResolved(info) => {
                    let mut properties = HashMap::new();
                    for prop in info.get_properties().iter() {
                        let val_str = prop.val_str();
                        if !val_str.is_empty() {
                            properties.insert(prop.key().to_string(), val_str.to_string());
                        }
                    }

                    // Get the first address
                    let host = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| info.get_hostname().to_string());

                    services.push(DiscoveredService {
                        name: info.get_fullname().to_string(),
                        host,
                        port: info.get_port(),
                        service_type: service_type.to_string(),
                        properties,
                    });
                }
                ServiceEvent::SearchStopped(_) => break,
                _ => {}
            },
            Ok(None) => break,
            Err(_) => continue, // Timeout, continue loop
        }
    }

    // Stop the daemon
    let _ = mdns.stop_browse(service_type);
    let _ = mdns.shutdown();

    Ok(services)
}

/// Discover AIConnect services via mDNS
pub async fn discover_aiconnect(timeout: Duration) -> Result<Vec<DiscoveredService>> {
    discover_services(AICONNECT_SERVICE_TYPE, timeout).await
}

/// Discover Ollama services via mDNS
pub async fn discover_ollama(timeout: Duration) -> Result<Vec<DiscoveredService>> {
    discover_services(OLLAMA_SERVICE_TYPE, timeout).await
}

/// Auto-configure backend: prefer AIConnect, fallback to Ollama
pub async fn auto_configure_backend(
    aiconnect_timeout: Duration,
    fallback_ollama_url: &str,
) -> BackendConfig {
    // Try to discover AIConnect first
    if let Ok(aiconnect_services) = discover_aiconnect(aiconnect_timeout).await {
        if let Some(service) = aiconnect_services.into_iter().next() {
            let endpoint = service.base_url();

            // Check if AIConnect is reachable
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(_) => {
                    return BackendConfig {
                        kind: BackendKind::OllamaLocal,
                        endpoint: fallback_ollama_url.to_string(),
                        auth: AuthMethod::None,
                        aiconnect_service: None,
                    };
                }
            };

            let health_url = format!("{}/api/health", endpoint);
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    return BackendConfig {
                        kind: BackendKind::AiConnect,
                        endpoint,
                        auth: AuthMethod::None, // User can configure auth later
                        aiconnect_service: Some(service),
                    };
                }
            }
        }
    }

    // Fallback to local Ollama
    BackendConfig {
        kind: BackendKind::OllamaLocal,
        endpoint: fallback_ollama_url.to_string(),
        auth: AuthMethod::None,
        aiconnect_service: None,
    }
}

/// Check if AIConnect is available at the given endpoint
pub async fn check_aiconnect_health(endpoint: &str, auth: &AuthMethod) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!("{}/api/health", endpoint);
    let headers = AiConnectClient::build_auth_headers(auth);

    match client.get(&url).headers(headers).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Check if Ollama is available at the given endpoint
pub async fn check_ollama_health(endpoint: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!("{}/api/tags", endpoint);

    match client.get(&url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_kind_default() {
        let kind = BackendKind::default();
        assert_eq!(kind, BackendKind::OllamaLocal);
    }

    #[test]
    fn test_discovered_service_base_url() {
        let service = DiscoveredService {
            name: "test".to_string(),
            host: "192.168.1.100".to_string(),
            port: 8080,
            service_type: "_aiconnect._tcp.local.".to_string(),
            properties: HashMap::new(),
        };

        assert_eq!(service.base_url(), "http://192.168.1.100:8080");
    }

    #[test]
    fn test_auth_method_serialization() {
        let bearer = AuthMethod::Bearer {
            token: "test_token".to_string(),
        };
        let json = serde_json::to_string(&bearer).unwrap();
        assert!(json.contains("bearer"));
        assert!(json.contains("test_token"));
    }
}

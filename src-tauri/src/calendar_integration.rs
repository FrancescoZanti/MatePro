use crate::local_storage::{
    self, CalendarEvent, CalendarIntegrations, GoogleCalendarIntegrationConfig, OutlookIntegrationConfig,
    PendingDeviceFlow, PendingPkceFlow,
};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use url::Url;
use uuid::Uuid;

const GRAPH_SCOPE: &str = "offline_access Calendars.ReadWrite";
const GRAPH_ENDPOINT: &str = "https://graph.microsoft.com/v1.0";
const DEFAULT_TIME_ZONE: &str = "UTC";

const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
#[allow(dead_code)]
const GOOGLE_DEVICE_CODE_ENDPOINT: &str = "https://oauth2.googleapis.com/device/code";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_CALENDAR_API: &str = "https://www.googleapis.com/calendar/v3";

const LOOPBACK_CALLBACK_PATH: &str = "/";
const PKCE_POLL_INTERVAL_SECS: u64 = 2;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Impossibile creare il client HTTP per le integrazioni calendario");
}

fn sanitize_optional_string(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn pkce_base64_url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_pkce_verifier() -> String {
    // 32 bytes -> 43 chars base64url (no padding), within PKCE limits (43..128)
    let mut raw = Vec::with_capacity(32);
    raw.extend_from_slice(Uuid::new_v4().as_bytes());
    raw.extend_from_slice(Uuid::new_v4().as_bytes());
    pkce_base64_url(&raw)
}

fn pkce_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    pkce_base64_url(&hasher.finalize())
}

fn generate_state() -> String {
    pkce_base64_url(Uuid::new_v4().as_bytes())
}

async fn bind_loopback_listener() -> Result<(TcpListener, String)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("Impossibile aprire la porta locale per il redirect OAuth")?;
    let port = listener
        .local_addr()
        .context("Impossibile determinare la porta locale")?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}{LOOPBACK_CALLBACK_PATH}");
    Ok((listener, redirect_uri))
}

async fn accept_single_http_request(
    listener: TcpListener,
    timeout_secs: u64,
) -> Result<(TcpStream, HashMap<String, String>)> {
    let (stream, _) = timeout(StdDuration::from_secs(timeout_secs), listener.accept())
        .await
        .context("Timeout in attesa del redirect OAuth")?
        .context("Errore nell'accettare la connessione di redirect OAuth")?;

    let mut stream = stream;
    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .context("Impossibile leggere la richiesta di redirect OAuth")?;

    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or_default();
    let uri = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or(LOOPBACK_CALLBACK_PATH);

    let url = Url::parse(&format!("http://localhost{uri}"))
        .context("Impossibile parsare la URL di redirect OAuth")?;

    let mut params = HashMap::new();
    for (k, v) in url.query_pairs() {
        params.insert(k.to_string(), v.to_string());
    }

    Ok((stream, params))
}

async fn respond_simple_html(mut stream: TcpStream, title: &str, body: &str) {
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h3>{}</h3><p>{}</p></body></html>",
        title, title, body
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.as_bytes().len(),
        html
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

async fn store_google_pkce_callback(params: HashMap<String, String>) -> Result<()> {
    let code = params.get("code").cloned();
    let state = params.get("state").cloned();
    let error = params.get("error").cloned();
    let error_description = params.get("error_description").cloned();

    let mut integrations = load_integrations()?;
    {
        let google = get_google_config_mut(&mut integrations);
        let pending = google
            .pending_pkce
            .as_mut()
            .ok_or_else(|| anyhow!("Nessun flusso PKCE Google in corso"))?;

        if pending.expires_at <= Utc::now() {
            pending.error = Some("expired".to_string());
            pending.message = Some("Autorizzazione scaduta, avvia nuovamente la procedura".to_string());
            store_integrations(&integrations)?;
            return Err(anyhow!("Autorizzazione scaduta"));
        }

        let returned_state = state.ok_or_else(|| anyhow!("Parametro state mancante"))?;
        if returned_state != pending.state {
            pending.error = Some("state_mismatch".to_string());
            pending.message = Some("State non valido, ripeti la procedura".to_string());
            store_integrations(&integrations)?;
            return Err(anyhow!("State non valido"));
        }

        if let Some(err) = error {
            pending.error = Some(err.clone());
            pending.message = Some(
                error_description
                    .clone()
                    .unwrap_or_else(|| "Autorizzazione rifiutata o fallita".to_string()),
            );
            store_integrations(&integrations)?;
            return Err(anyhow!("Errore OAuth: {}", err));
        }

        let code = code.ok_or_else(|| anyhow!("Parametro code mancante"))?;
        pending.authorization_code = Some(code);
        pending.message = Some("Autorizzazione ricevuta, sto completando il collegamento...".to_string());
    }

    store_integrations(&integrations)?;
    Ok(())
}

async fn store_outlook_pkce_callback(params: HashMap<String, String>) -> Result<()> {
    let code = params.get("code").cloned();
    let state = params.get("state").cloned();
    let error = params.get("error").cloned();
    let error_description = params.get("error_description").cloned();

    let mut integrations = load_integrations()?;
    {
        let outlook = get_outlook_config_mut(&mut integrations);
        let pending = outlook
            .pending_pkce
            .as_mut()
            .ok_or_else(|| anyhow!("Nessun flusso PKCE Outlook in corso"))?;

        if pending.expires_at <= Utc::now() {
            pending.error = Some("expired".to_string());
            pending.message = Some("Autorizzazione scaduta, avvia nuovamente la procedura".to_string());
            store_integrations(&integrations)?;
            return Err(anyhow!("Autorizzazione scaduta"));
        }

        let returned_state = state.ok_or_else(|| anyhow!("Parametro state mancante"))?;
        if returned_state != pending.state {
            pending.error = Some("state_mismatch".to_string());
            pending.message = Some("State non valido, ripeti la procedura".to_string());
            store_integrations(&integrations)?;
            return Err(anyhow!("State non valido"));
        }

        if let Some(err) = error {
            pending.error = Some(err.clone());
            pending.message = Some(
                error_description
                    .clone()
                    .unwrap_or_else(|| "Autorizzazione rifiutata o fallita".to_string()),
            );
            store_integrations(&integrations)?;
            return Err(anyhow!("Errore OAuth: {}", err));
        }

        let code = code.ok_or_else(|| anyhow!("Parametro code mancante"))?;
        pending.authorization_code = Some(code);
        pending.message = Some("Autorizzazione ricevuta, sto completando il collegamento...".to_string());
    }

    store_integrations(&integrations)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct OutlookStatus {
    pub configured: bool,
    pub connected: bool,
    pub pending: bool,
    pub tenant: Option<String>,
    pub client_id: Option<String>,
    pub expires_at: Option<String>,
    pub message: Option<String>,
    pub interval: Option<u64>,
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GoogleCalendarStatus {
    pub configured: bool,
    pub connected: bool,
    pub pending: bool,
    pub client_id: Option<String>,
    pub calendar_id: Option<String>,
    pub expires_at: Option<String>,
    pub message: Option<String>,
    pub interval: Option<u64>,
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CalendarIntegrationStatus {
    pub outlook: OutlookStatus,
    pub google: GoogleCalendarStatus,
}

#[derive(Debug, Serialize)]
pub struct OutlookDeviceFlowStart {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
    pub interval: u64,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct OutlookDeviceFlowPoll {
    pub status: String,
    pub message: Option<String>,
    pub retry_in: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteCalendarEvent {
    pub id: String,
    pub subject: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
    pub web_link: Option<String>,
    pub body_preview: Option<String>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRemoteEventRequest {
    pub subject: String,
    pub start: String,
    pub end: String,
    pub body: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    #[allow(dead_code)]
    verification_uri_complete: Option<String>,
    expires_in: i64,
    interval: Option<u64>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenSuccessResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: Option<String>,
    expires_in: i64,
    #[serde(default)]
    #[allow(dead_code)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleDeviceCodeResponse {
    device_code: String,
    user_code: String,
    #[serde(rename = "verification_url")]
    verification_url: String,
    expires_in: i64,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    error_uri: Option<String>,
}

fn build_outlook_status(outlook: &OutlookIntegrationConfig) -> OutlookStatus {
    let configured = sanitize_optional_string(&outlook.client_id)
        .as_ref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false);

    let connected = outlook
        .access_token
        .as_ref()
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false);

    let pending = outlook.pending.is_some() || outlook.pending_pkce.is_some();

    let (message, interval, verification_uri, user_code) = if let Some(p) = outlook.pending_pkce.as_ref() {
        (
            p.message.clone(),
            Some(PKCE_POLL_INTERVAL_SECS),
            Some(p.authorization_url.clone()),
            None,
        )
    } else {
        (
            outlook.pending.as_ref().and_then(|p| p.message.clone()),
            outlook.pending.as_ref().map(|p| p.interval),
            outlook.pending.as_ref().map(|p| p.verification_uri.clone()),
            outlook.pending.as_ref().map(|p| p.user_code.clone()),
        )
    };

    OutlookStatus {
        configured,
        connected,
        pending,
        tenant: sanitize_optional_string(&outlook.tenant),
        client_id: sanitize_optional_string(&outlook.client_id),
        expires_at: outlook
            .expires_at
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        message,
        interval,
        user_code,
        verification_uri,
    }
}

fn build_google_status(google: &GoogleCalendarIntegrationConfig) -> GoogleCalendarStatus {
    let configured = sanitize_optional_string(&google.client_id)
        .as_ref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false);

    let connected = google
        .access_token
        .as_ref()
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false);

    let pending = google.pending.is_some() || google.pending_pkce.is_some();

    let (message, interval, verification_uri, user_code) = if let Some(p) = google.pending_pkce.as_ref() {
        (
            p.message.clone(),
            Some(PKCE_POLL_INTERVAL_SECS),
            Some(p.authorization_url.clone()),
            None,
        )
    } else {
        (
            google.pending.as_ref().and_then(|p| p.message.clone()),
            google.pending.as_ref().map(|p| p.interval),
            google.pending.as_ref().map(|p| p.verification_uri.clone()),
            google.pending.as_ref().map(|p| p.user_code.clone()),
        )
    };

    GoogleCalendarStatus {
        configured,
        connected,
        pending,
        client_id: sanitize_optional_string(&google.client_id),
        calendar_id: sanitize_optional_string(&google.calendar_id)
            .or_else(|| Some("primary".to_string())),
        expires_at: google
            .expires_at
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        message,
        interval,
        user_code,
        verification_uri,
    }
}

fn load_integrations() -> Result<CalendarIntegrations> {
    let mut integrations = local_storage::load_calendar_integrations()?;
    if integrations.version == 0 {
        integrations.version = 1;
    }
    if integrations.outlook.is_none() {
        integrations.outlook = Some(OutlookIntegrationConfig::default());
    }
    if integrations.google.is_none() {
        integrations.google = Some(GoogleCalendarIntegrationConfig::default());
    }
    Ok(integrations)
}

fn store_integrations(integrations: &CalendarIntegrations) -> Result<()> {
    local_storage::save_calendar_integrations(integrations)
}

fn get_outlook_config_mut(integrations: &mut CalendarIntegrations) -> &mut OutlookIntegrationConfig {
    integrations
        .outlook
        .get_or_insert_with(OutlookIntegrationConfig::default)
}

fn get_google_config_mut(integrations: &mut CalendarIntegrations) -> &mut GoogleCalendarIntegrationConfig {
    integrations
        .google
        .get_or_insert_with(GoogleCalendarIntegrationConfig::default)
}

fn ensure_client_and_tenant(outlook: &OutlookIntegrationConfig) -> Result<(String, String)> {
    let client_id = outlook
        .client_id
        .as_ref()
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .ok_or_else(|| anyhow!("Client ID Outlook non configurato"))?;

    let tenant = outlook
        .tenant
        .as_ref()
        .map(|tenant| tenant.trim().to_string())
        .filter(|tenant| !tenant.is_empty())
        .unwrap_or_else(|| "common".to_string());

    Ok((client_id, tenant))
}

fn ensure_scopes(outlook: &mut OutlookIntegrationConfig) {
    if outlook.scopes.is_empty() {
        outlook.scopes = vec![GRAPH_SCOPE.to_string()];
    }
}

fn ensure_google_scopes(google: &mut GoogleCalendarIntegrationConfig) {
    if google.scopes.is_empty() {
        google.scopes = vec![GOOGLE_SCOPE.to_string()];
    }
    if google.calendar_id.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        google.calendar_id = Some("primary".to_string());
    }
}

fn ensure_google_client_id(google: &GoogleCalendarIntegrationConfig) -> Result<String> {
    sanitize_optional_string(&google.client_id)
        .ok_or_else(|| anyhow!("Client ID Google non configurato"))
}

fn ensure_google_client(google: &GoogleCalendarIntegrationConfig) -> Result<(String, String)> {
    let client_id = ensure_google_client_id(google)?;
    let client_secret = google
        .client_secret
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!(
            "Client Secret Google mancante (necessario solo per il vecchio device flow)"
        ))?;
    Ok((client_id, client_secret))
}

pub fn get_calendar_status() -> Result<CalendarIntegrationStatus> {
    let integrations = load_integrations()?;
    let outlook = integrations.outlook.as_ref().cloned().unwrap_or_default();
    let google = integrations.google.as_ref().cloned().unwrap_or_default();

    Ok(CalendarIntegrationStatus {
        outlook: build_outlook_status(&outlook),
        google: build_google_status(&google),
    })
}

pub fn set_outlook_credentials(client_id: String, tenant: Option<String>) -> Result<CalendarIntegrationStatus> {
    let mut integrations = load_integrations()?;
    let status = {
        let outlook = get_outlook_config_mut(&mut integrations);

        outlook.client_id = Some(client_id.trim().to_string());
        outlook.tenant = Some(
            tenant
                .unwrap_or_else(|| "common".to_string())
                .trim()
                .to_string(),
        );
        outlook.enabled = true;
        outlook.pending = None;
        outlook.pending_pkce = None;
        outlook.access_token = None;
        outlook.refresh_token = None;
        outlook.expires_at = None;
        ensure_scopes(outlook);

        build_outlook_status(outlook)
    };

    store_integrations(&integrations)?;

    let google = integrations.google.as_ref().cloned().unwrap_or_default();

    Ok(CalendarIntegrationStatus {
        outlook: status,
        google: build_google_status(&google),
    })
}

pub fn disconnect_outlook() -> Result<CalendarIntegrationStatus> {
    let mut integrations = load_integrations()?;
    let status = {
        let outlook = get_outlook_config_mut(&mut integrations);

        outlook.enabled = false;
        outlook.access_token = None;
        outlook.refresh_token = None;
        outlook.expires_at = None;
        outlook.pending = None;
        outlook.pending_pkce = None;

        build_outlook_status(outlook)
    };

    store_integrations(&integrations)?;
    let google = integrations.google.as_ref().cloned().unwrap_or_default();

    Ok(CalendarIntegrationStatus {
        outlook: status,
        google: build_google_status(&google),
    })
}

pub fn set_google_credentials(client_id: String, calendar_id: Option<String>) -> Result<CalendarIntegrationStatus> {
    let mut integrations = load_integrations()?;
    let status = {
        let google = get_google_config_mut(&mut integrations);
        ensure_google_scopes(google);

        google.client_id = Some(client_id.trim().to_string());
        // PKCE public client: il secret non è necessario
        google.client_secret = None;
        if let Some(calendar_id) = calendar_id {
            let trimmed = calendar_id.trim();
            if !trimmed.is_empty() {
                google.calendar_id = Some(trimmed.to_string());
            }
        }

        google.enabled = true;
        google.pending = None;
        google.pending_pkce = None;
        google.access_token = None;
        google.refresh_token = None;
        google.expires_at = None;

        build_google_status(google)
    };

    store_integrations(&integrations)?;

    let outlook = integrations.outlook.as_ref().cloned().unwrap_or_default();

    Ok(CalendarIntegrationStatus {
        outlook: build_outlook_status(&outlook),
        google: status,
    })
}

pub fn disconnect_google() -> Result<CalendarIntegrationStatus> {
    let mut integrations = load_integrations()?;
    let status = {
        let google = get_google_config_mut(&mut integrations);

        google.enabled = false;
        google.access_token = None;
        google.refresh_token = None;
        google.expires_at = None;
        google.pending = None;
        google.pending_pkce = None;

        build_google_status(google)
    };

    store_integrations(&integrations)?;
    let outlook = integrations.outlook.as_ref().cloned().unwrap_or_default();

    Ok(CalendarIntegrationStatus {
        outlook: build_outlook_status(&outlook),
        google: status,
    })
}

pub async fn start_google_device_flow() -> Result<OutlookDeviceFlowStart> {
    // Authorization Code + PKCE (public client, niente client secret)
    let (listener, redirect_uri) = bind_loopback_listener().await?;
    let mut integrations = load_integrations()?;

    let (authorization_url, expires_at) = {
        let google = get_google_config_mut(&mut integrations);
        ensure_google_scopes(google);
        let client_id = ensure_google_client_id(google)?;
        let scope = google.scopes.join(" ");

        let verifier = generate_pkce_verifier();
        let challenge = pkce_challenge_s256(&verifier);
        let state = generate_state();
        let expires_at = Utc::now() + Duration::minutes(10);

        let mut url = Url::parse(GOOGLE_AUTH_ENDPOINT)
            .context("Impossibile costruire URL autorizzazione Google")?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id.as_str())
            .append_pair("redirect_uri", redirect_uri.as_str())
            .append_pair("response_type", "code")
            .append_pair("scope", scope.as_str())
            .append_pair("state", state.as_str())
            .append_pair("code_challenge", challenge.as_str())
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        let authorization_url = url.to_string();

        google.pending = None;
        google.pending_pkce = Some(PendingPkceFlow {
            authorization_url: authorization_url.clone(),
            redirect_uri,
            code_verifier: verifier,
            state,
            authorization_code: None,
            error: None,
            expires_at,
            message: Some("Attendo che tu completi l'autorizzazione nel browser".to_string()),
        });
        google.enabled = true;
        google.access_token = None;
        google.refresh_token = None;
        google.expires_at = None;

        (authorization_url, expires_at)
    };

    store_integrations(&integrations)?;

    tokio::spawn(async move {
        let accept_result = accept_single_http_request(listener, 10 * 60).await;
        match accept_result {
            Ok((stream, params)) => {
                let result = store_google_pkce_callback(params).await;
                match result {
                    Ok(_) => {
                        respond_simple_html(
                            stream,
                            "Autorizzazione ricevuta",
                            "Puoi tornare su MatePro: completo il collegamento in automatico.",
                        )
                        .await;
                    }
                    Err(err) => {
                        respond_simple_html(
                            stream,
                            "Errore collegamento Google Calendar",
                            &format!("Dettagli: {}", err),
                        )
                        .await;
                    }
                }
            }
            Err(_) => {}
        }
    });

    Ok(OutlookDeviceFlowStart {
        user_code: String::new(),
        verification_uri: authorization_url,
        message: "Autorizzazione Google Calendar".to_string(),
        interval: PKCE_POLL_INTERVAL_SECS,
        expires_at: expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    })
}

async fn refresh_google_token(google: &mut GoogleCalendarIntegrationConfig) -> Result<()> {
    let client_id = ensure_google_client_id(google)?;
    let refresh_token = google
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow!("Refresh token mancante per Google Calendar"))?;

    let mut form = vec![
        ("grant_type", "refresh_token".to_string()),
        ("client_id", client_id),
        ("refresh_token", refresh_token.to_string()),
    ];
    if let Some(secret) = google
        .client_secret
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        form.push(("client_secret", secret));
    }

    let response = HTTP_CLIENT
        .post(GOOGLE_TOKEN_ENDPOINT)
        .form(&form)
        .send()
        .await
        .context("Richiesta refresh token Google fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Refresh token Google fallito: stato {} - {}", status, body));
    }

    #[derive(Debug, Deserialize)]
    struct GoogleTokenSuccess {
        access_token: String,
        expires_in: i64,
        #[serde(default)]
        #[allow(dead_code)]
        scope: Option<String>,
        #[serde(default)]
        #[allow(dead_code)]
        token_type: Option<String>,
    }

    let token: GoogleTokenSuccess = response
        .json()
        .await
        .context("Impossibile decodificare risposta refresh Google")?;

    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));
    google.access_token = Some(token.access_token);
    google.expires_at = Some(expires_at);
    Ok(())
}

async fn ensure_google_access_token(google: &mut GoogleCalendarIntegrationConfig) -> Result<String> {
    let expires_at = google.expires_at.unwrap_or_else(|| Utc::now() - Duration::seconds(1));
    let needs_refresh = expires_at <= (Utc::now() + Duration::seconds(60));
    if needs_refresh {
        refresh_google_token(google).await?;
    }

    google
        .access_token
        .clone()
        .ok_or_else(|| anyhow!("Token Google Calendar non disponibile"))
}

pub async fn poll_google_device_flow() -> Result<OutlookDeviceFlowPoll> {
    let mut integrations = load_integrations()?;

    let (result, should_store) = {
        let google = get_google_config_mut(&mut integrations);

        if let Some(pending) = google.pending_pkce.clone() {
            if pending.expires_at <= Utc::now() {
                google.pending_pkce = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "expired".to_string(),
                        message: Some(
                            "Autorizzazione scaduta, avvia nuovamente la procedura".to_string(),
                        ),
                        retry_in: None,
                    },
                    true,
                )
            } else if let Some(err) = pending.error.clone() {
                google.pending_pkce = None;
                let status = if err == "access_denied" { "declined" } else { "error" };
                (
                    OutlookDeviceFlowPoll {
                        status: status.to_string(),
                        message: pending
                            .message
                            .clone()
                            .or_else(|| Some(format!("Errore OAuth: {}", err))),
                        retry_in: None,
                    },
                    true,
                )
            } else if let Some(code) = pending.authorization_code.clone() {
                let client_id = ensure_google_client_id(google)?;
                let mut form = vec![
                    ("grant_type", "authorization_code".to_string()),
                    ("client_id", client_id),
                    ("code", code),
                    ("code_verifier", pending.code_verifier.clone()),
                    ("redirect_uri", pending.redirect_uri.clone()),
                ];
                if let Some(secret) = google
                    .client_secret
                    .as_ref()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                {
                    form.push(("client_secret", secret));
                }

                let response = HTTP_CLIENT
                    .post(GOOGLE_TOKEN_ENDPOINT)
                    .form(&form)
                    .send()
                    .await
                    .context("Richiesta token Google (PKCE) fallita")?;

                if response.status().is_success() {
                    let token: TokenSuccessResponse = response
                        .json()
                        .await
                        .context("Impossibile decodificare risposta token Google")?;

                    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));

                    google.pending_pkce = None;
                    google.access_token = Some(token.access_token);
                    if let Some(refresh) = token.refresh_token {
                        google.refresh_token = Some(refresh);
                    }
                    google.expires_at = Some(expires_at);
                    google.enabled = true;

                    (
                        OutlookDeviceFlowPoll {
                            status: "completed".to_string(),
                            message: Some("Google Calendar collegato con successo".to_string()),
                            retry_in: None,
                        },
                        true,
                    )
                } else {
                    let status_code = response.status();
                    let body_text = response.text().await.unwrap_or_default();
                    google.pending_pkce = None;
                    (
                        OutlookDeviceFlowPoll {
                            status: "error".to_string(),
                            message: Some(format!(
                                "Errore token Google (stato {}): {}",
                                status_code, body_text
                            )),
                            retry_in: None,
                        },
                        true,
                    )
                }
            } else if google.enabled && google.access_token.is_some() {
                google.pending_pkce = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "completed".to_string(),
                        message: Some("Google Calendar collegato con successo".to_string()),
                        retry_in: None,
                    },
                    true,
                )
            } else {
                (
                    OutlookDeviceFlowPoll {
                        status: "pending".to_string(),
                        message: pending.message.clone(),
                        retry_in: Some(PKCE_POLL_INTERVAL_SECS),
                    },
                    false,
                )
            }
        } else if let Some(pending) = google.pending.clone() {
            // Legacy device flow (richiede client_secret)
            if pending.expires_at <= Utc::now() {
                google.pending = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "expired".to_string(),
                        message: Some("Codice scaduto, avvia nuovamente la procedura".to_string()),
                        retry_in: None,
                    },
                    true,
                )
            } else {
                let (client_id, client_secret) = ensure_google_client(google)?;

                let response = HTTP_CLIENT
                    .post(GOOGLE_TOKEN_ENDPOINT)
                    .form(&[
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ("client_id", client_id.as_str()),
                        ("client_secret", client_secret.as_str()),
                        ("device_code", pending.device_code.as_str()),
                    ])
                    .send()
                    .await
                    .context("Richiesta polling token Google fallita")?;

                if response.status().is_success() {
                    #[derive(Debug, Deserialize)]
                    struct GoogleTokenSuccess {
                        access_token: String,
                        expires_in: i64,
                        #[serde(default)]
                        refresh_token: Option<String>,
                        #[serde(default)]
                        #[allow(dead_code)]
                        scope: Option<String>,
                        #[serde(default)]
                        #[allow(dead_code)]
                        token_type: Option<String>,
                    }

                    let token: GoogleTokenSuccess = response
                        .json()
                        .await
                        .context("Impossibile decodificare risposta token Google")?;

                    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));

                    google.pending = None;
                    google.access_token = Some(token.access_token);
                    if let Some(refresh) = token.refresh_token {
                        google.refresh_token = Some(refresh);
                    }
                    google.expires_at = Some(expires_at);
                    google.enabled = true;

                    (
                        OutlookDeviceFlowPoll {
                            status: "completed".to_string(),
                            message: Some("Google Calendar collegato con successo".to_string()),
                            retry_in: None,
                        },
                        true,
                    )
                } else {
                    let status_code = response.status();
                    let body_text = response.text().await.unwrap_or_default();

                    if let Ok(error) = serde_json::from_str::<GoogleTokenErrorResponse>(&body_text) {
                        match error.error.as_str() {
                            "authorization_pending" => {
                                return Ok(OutlookDeviceFlowPoll {
                                    status: "pending".to_string(),
                                    message: error.error_description,
                                    retry_in: Some(pending.interval.max(1)),
                                })
                            }
                            "slow_down" => {
                                let retry = pending.interval + 2;
                                if let Some(current) = google.pending.as_mut() {
                                    current.interval = retry;
                                }
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "pending".to_string(),
                                        message: error.error_description,
                                        retry_in: Some(retry),
                                    },
                                    true,
                                )
                            }
                            "access_denied" => {
                                google.pending = None;
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "declined".to_string(),
                                        message: Some(
                                            "Richiesta rifiutata dall'utente. Ripeti la procedura se desideri collegare Google Calendar.".to_string(),
                                        ),
                                        retry_in: None,
                                    },
                                    true,
                                )
                            }
                            "expired_token" => {
                                google.pending = None;
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "expired".to_string(),
                                        message: Some(
                                            "Codice scaduto, avvia nuovamente la procedura".to_string(),
                                        ),
                                        retry_in: None,
                                    },
                                    true,
                                )
                            }
                            other => {
                                return Ok(OutlookDeviceFlowPoll {
                                    status: "error".to_string(),
                                    message: Some(format!("Errore Google: {}", other)),
                                    retry_in: Some(pending.interval.max(1)),
                                })
                            }
                        }
                    } else {
                        return Err(anyhow!(
                            "Errore polling Google (stato {}): {}",
                            status_code,
                            body_text
                        ));
                    }
                }
            }
        } else {
            if google.enabled && google.access_token.is_some() {
                return Ok(OutlookDeviceFlowPoll {
                    status: "completed".to_string(),
                    message: Some("Google Calendar collegato con successo".to_string()),
                    retry_in: None,
                });
            }
            return Ok(OutlookDeviceFlowPoll {
                status: "idle".to_string(),
                message: Some("Nessuna richiesta di collegamento in corso".to_string()),
                retry_in: None,
            });
        }
    };

    if should_store {
        store_integrations(&integrations)?;
    }

    Ok(result)
}

pub async fn list_google_events(limit: usize) -> Result<Vec<RemoteCalendarEvent>> {
    let mut integrations = load_integrations()?;
    let (token, calendar_id) = {
        let google = get_google_config_mut(&mut integrations);
        if !google.enabled {
            return Err(anyhow!("Google Calendar non è abilitato"));
        }
        if google.access_token.is_none() {
            return Err(anyhow!("Google Calendar non è collegato"));
        }
        ensure_google_scopes(google);
        let token = ensure_google_access_token(google).await?;
        let calendar_id = google
            .calendar_id
            .clone()
            .unwrap_or_else(|| "primary".to_string());
        (token, calendar_id)
    };

    store_integrations(&integrations)?;

    let time_min = (Utc::now() - Duration::hours(12)).to_rfc3339();
    let max_results = limit.max(1).min(50);

    let response = HTTP_CLIENT
        .get(format!(
            "{GOOGLE_CALENDAR_API}/calendars/{}/events",
            urlencoding::encode(&calendar_id)
        ))
        .query(&[
            ("maxResults", max_results.to_string()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
            ("timeMin", time_min),
        ])
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .context("Richiesta eventi Google Calendar fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Impossibile recuperare eventi Google Calendar (stato {}): {}",
            status,
            body
        ));
    }

    #[derive(Debug, Deserialize)]
    struct GoogleEventsResponse {
        #[serde(default)]
        items: Vec<GoogleEvent>,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleEvent {
        id: String,
        #[serde(default)]
        summary: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        start: Option<GoogleEventDateTime>,
        #[serde(default)]
        end: Option<GoogleEventDateTime>,
        #[serde(default)]
        location: Option<String>,
        #[serde(default)]
        html_link: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleEventDateTime {
        #[serde(default, rename = "dateTime")]
        date_time: Option<String>,
        #[serde(default, rename = "date")]
        date: Option<String>,
        #[serde(default)]
        #[allow(dead_code)]
        time_zone: Option<String>,
    }

    let events: GoogleEventsResponse = response
        .json()
        .await
        .context("Impossibile decodificare eventi Google Calendar")?;

    let mapped = events
        .items
        .into_iter()
        .map(|event| {
            let subject = sanitize_optional_string(&event.summary)
                .unwrap_or_else(|| "(Senza titolo)".to_string());
            let start = event
                .start
                .as_ref()
                .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone()))
                .unwrap_or_else(|| Utc::now().to_rfc3339());
            let end = event
                .end
                .as_ref()
                .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone()))
                .unwrap_or_else(|| Utc::now().to_rfc3339());

            RemoteCalendarEvent {
                id: event.id,
                subject,
                start,
                end,
                location: sanitize_optional_string(&event.location),
                web_link: sanitize_optional_string(&event.html_link),
                body_preview: sanitize_optional_string(&event.description),
                source: "google",
            }
        })
        .collect();

    Ok(mapped)
}

pub async fn create_google_event(request: CreateRemoteEventRequest) -> Result<RemoteCalendarEvent> {
    let mut integrations = load_integrations()?;
    let (token, calendar_id) = {
        let google = get_google_config_mut(&mut integrations);
        if !google.enabled {
            return Err(anyhow!("Google Calendar non è abilitato"));
        }
        if google.access_token.is_none() {
            return Err(anyhow!("Google Calendar non è collegato"));
        }
        ensure_google_scopes(google);
        let token = ensure_google_access_token(google).await?;
        let calendar_id = google
            .calendar_id
            .clone()
            .unwrap_or_else(|| "primary".to_string());
        (token, calendar_id)
    };

    store_integrations(&integrations)?;

    #[derive(Serialize)]
    struct GoogleEventBody<'a> {
        summary: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        start: GoogleDateTime<'a>,
        end: GoogleDateTime<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<&'a str>,
    }

    #[derive(Serialize)]
    struct GoogleDateTime<'a> {
        #[serde(rename = "dateTime")]
        date_time: &'a str,
        #[serde(rename = "timeZone")]
        time_zone: &'a str,
    }

    let body = GoogleEventBody {
        summary: request.subject.as_str(),
        description: request.body.as_deref(),
        start: GoogleDateTime {
            date_time: request.start.as_str(),
            time_zone: DEFAULT_TIME_ZONE,
        },
        end: GoogleDateTime {
            date_time: request.end.as_str(),
            time_zone: DEFAULT_TIME_ZONE,
        },
        location: request.location.as_deref(),
    };

    let response = HTTP_CLIENT
        .post(format!(
            "{GOOGLE_CALENDAR_API}/calendars/{}/events",
            urlencoding::encode(&calendar_id)
        ))
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .context("Creazione evento Google Calendar fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Impossibile creare evento Google Calendar (stato {}): {}",
            status,
            body
        ));
    }

    #[derive(Debug, Deserialize)]
    struct GoogleCreatedEvent {
        id: String,
        #[serde(default)]
        summary: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        location: Option<String>,
        #[serde(default)]
        html_link: Option<String>,
        #[serde(default)]
        start: Option<GoogleCreatedDateTime>,
        #[serde(default)]
        end: Option<GoogleCreatedDateTime>,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleCreatedDateTime {
        #[serde(default, rename = "dateTime")]
        date_time: Option<String>,
        #[serde(default, rename = "date")]
        date: Option<String>,
    }

    let event: GoogleCreatedEvent = response
        .json()
        .await
        .context("Impossibile decodificare evento creato su Google Calendar")?;

    Ok(RemoteCalendarEvent {
        id: event.id,
        subject: sanitize_optional_string(&event.summary).unwrap_or_else(|| "Evento".to_string()),
        start: event
            .start
            .as_ref()
            .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone()))
            .unwrap_or_else(|| request.start.clone()),
        end: event
            .end
            .as_ref()
            .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone()))
            .unwrap_or_else(|| request.end.clone()),
        location: sanitize_optional_string(&event.location),
        web_link: sanitize_optional_string(&event.html_link),
        body_preview: sanitize_optional_string(&event.description),
        source: "google",
    })
}

pub async fn is_google_connected() -> Result<bool> {
    let integrations = load_integrations()?;
    let google = integrations.google.unwrap_or_default();
    Ok(google.enabled && google.access_token.is_some())
}

pub async fn start_outlook_device_flow() -> Result<OutlookDeviceFlowStart> {
    // Authorization Code + PKCE (public client, niente client secret)
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("Impossibile aprire la porta locale per il redirect OAuth")?;
    let port = listener
        .local_addr()
        .context("Impossibile determinare la porta locale")?
        .port();
    // Usa localhost per compatibilità con le redirect URI tipiche di Entra ID
    let redirect_uri = format!("http://localhost:{port}{LOOPBACK_CALLBACK_PATH}");

    let mut integrations = load_integrations()?;

    let (authorization_url, expires_at) = {
        let outlook = get_outlook_config_mut(&mut integrations);
        ensure_scopes(outlook);
        let (client_id, tenant) = ensure_client_and_tenant(outlook)?;
        let scope = outlook.scopes.join(" ");

        let verifier = generate_pkce_verifier();
        let challenge = pkce_challenge_s256(&verifier);
        let state = generate_state();
        let expires_at = Utc::now() + Duration::minutes(10);

        let mut url = Url::parse(&format!(
            "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize"
        ))
        .context("Impossibile costruire URL autorizzazione Outlook")?;

        url.query_pairs_mut()
            .append_pair("client_id", client_id.as_str())
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri.as_str())
            .append_pair("response_mode", "query")
            .append_pair("scope", scope.as_str())
            .append_pair("state", state.as_str())
            .append_pair("code_challenge", challenge.as_str())
            .append_pair("code_challenge_method", "S256")
            .append_pair("prompt", "select_account");

        let authorization_url = url.to_string();

        outlook.pending = None;
        outlook.pending_pkce = Some(PendingPkceFlow {
            authorization_url: authorization_url.clone(),
            redirect_uri: redirect_uri.clone(),
            code_verifier: verifier,
            state,
            authorization_code: None,
            error: None,
            expires_at,
            message: Some("Attendo che tu completi l'autorizzazione nel browser".to_string()),
        });
        outlook.enabled = true;
        outlook.access_token = None;
        outlook.refresh_token = None;
        outlook.expires_at = None;

        (authorization_url, expires_at)
    };

    store_integrations(&integrations)?;

    tokio::spawn(async move {
        let accept_result = accept_single_http_request(listener, 10 * 60).await;
        match accept_result {
            Ok((stream, params)) => {
                let result = store_outlook_pkce_callback(params).await;
                match result {
                    Ok(_) => {
                        respond_simple_html(
                            stream,
                            "Autorizzazione ricevuta",
                            "Puoi tornare su MatePro: completo il collegamento in automatico.",
                        )
                        .await;
                    }
                    Err(err) => {
                        respond_simple_html(
                            stream,
                            "Errore collegamento Outlook Calendar",
                            &format!("Dettagli: {}", err),
                        )
                        .await;
                    }
                }
            }
            Err(_) => {}
        }
    });

    Ok(OutlookDeviceFlowStart {
        user_code: String::new(),
        verification_uri: authorization_url,
        message: "Autorizzazione Outlook Calendar".to_string(),
        interval: PKCE_POLL_INTERVAL_SECS,
        expires_at: expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    })
}

async fn refresh_outlook_token(outlook: &mut OutlookIntegrationConfig) -> Result<()> {
    let (client_id, tenant) = ensure_client_and_tenant(outlook)?;
    let refresh_token = outlook
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow!("Refresh token mancante per Outlook"))?;

    let response = HTTP_CLIENT
        .post(format!(
            "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"
        ))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("scope", GRAPH_SCOPE),
        ])
        .send()
        .await
        .context("Richiesta refresh token Outlook fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Refresh token Outlook fallito: stato {} - {}", status, body));
    }

    let token: TokenSuccessResponse = response
        .json()
        .await
        .context("Impossibile decodificare risposta refresh Outlook")?;

    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));
    outlook.access_token = Some(token.access_token);
    if let Some(refresh) = token.refresh_token {
        outlook.refresh_token = Some(refresh);
    }
    outlook.expires_at = Some(expires_at);
    Ok(())
}

async fn ensure_outlook_access_token(outlook: &mut OutlookIntegrationConfig) -> Result<String> {
    let expires_at = outlook.expires_at.unwrap_or_else(|| Utc::now() - Duration::seconds(1));
    let needs_refresh = expires_at <= (Utc::now() + Duration::seconds(60));
    if needs_refresh {
        refresh_outlook_token(outlook).await?;
    }

    outlook
        .access_token
        .clone()
        .ok_or_else(|| anyhow!("Token Outlook non disponibile"))
}

pub async fn poll_outlook_device_flow() -> Result<OutlookDeviceFlowPoll> {
    let mut integrations = load_integrations()?;
    let (result, should_store) = {
        let outlook = get_outlook_config_mut(&mut integrations);
        if let Some(pending) = outlook.pending_pkce.clone() {
            if pending.expires_at <= Utc::now() {
                outlook.pending_pkce = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "expired".to_string(),
                        message: Some(
                            "Autorizzazione scaduta, avvia nuovamente la procedura".to_string(),
                        ),
                        retry_in: None,
                    },
                    true,
                )
            } else if let Some(err) = pending.error.clone() {
                outlook.pending_pkce = None;
                let status = if err == "access_denied" { "declined" } else { "error" };
                (
                    OutlookDeviceFlowPoll {
                        status: status.to_string(),
                        message: pending
                            .message
                            .clone()
                            .or_else(|| Some(format!("Errore OAuth: {}", err))),
                        retry_in: None,
                    },
                    true,
                )
            } else if let Some(code) = pending.authorization_code.clone() {
                let (client_id, tenant) = ensure_client_and_tenant(outlook)?;
                let scope = if outlook.scopes.is_empty() {
                    GRAPH_SCOPE.to_string()
                } else {
                    outlook.scopes.join(" ")
                };

                let response = HTTP_CLIENT
                    .post(format!(
                        "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"
                    ))
                    .form(&[
                        ("grant_type", "authorization_code"),
                        ("client_id", client_id.as_str()),
                        ("code", code.as_str()),
                        ("redirect_uri", pending.redirect_uri.as_str()),
                        ("code_verifier", pending.code_verifier.as_str()),
                        ("scope", scope.as_str()),
                    ])
                    .send()
                    .await
                    .context("Richiesta token Outlook (PKCE) fallita")?;

                if response.status().is_success() {
                    let token: TokenSuccessResponse = response
                        .json()
                        .await
                        .context("Impossibile decodificare risposta token Outlook")?;

                    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));

                    outlook.pending_pkce = None;
                    outlook.access_token = Some(token.access_token);
                    if let Some(refresh) = token.refresh_token {
                        outlook.refresh_token = Some(refresh);
                    }
                    outlook.expires_at = Some(expires_at);
                    outlook.enabled = true;

                    (
                        OutlookDeviceFlowPoll {
                            status: "completed".to_string(),
                            message: Some("Outlook collegato con successo".to_string()),
                            retry_in: None,
                        },
                        true,
                    )
                } else {
                    let status_code = response.status();
                    let body_text = response.text().await.unwrap_or_default();
                    outlook.pending_pkce = None;

                    if let Ok(error) = serde_json::from_str::<TokenErrorResponse>(&body_text) {
                        (
                            OutlookDeviceFlowPoll {
                                status: "error".to_string(),
                                message: error
                                    .error_description
                                    .or_else(|| Some(format!("Errore Outlook: {}", error.error))),
                                retry_in: None,
                            },
                            true,
                        )
                    } else {
                        (
                            OutlookDeviceFlowPoll {
                                status: "error".to_string(),
                                message: Some(format!(
                                    "Errore token Outlook (stato {}): {}",
                                    status_code, body_text
                                )),
                                retry_in: None,
                            },
                            true,
                        )
                    }
                }
            } else if outlook.enabled && outlook.access_token.is_some() {
                outlook.pending_pkce = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "completed".to_string(),
                        message: Some("Outlook collegato con successo".to_string()),
                        retry_in: None,
                    },
                    true,
                )
            } else {
                (
                    OutlookDeviceFlowPoll {
                        status: "pending".to_string(),
                        message: pending.message.clone(),
                        retry_in: Some(PKCE_POLL_INTERVAL_SECS),
                    },
                    false,
                )
            }
        } else if let Some(pending) = outlook.pending.clone() {
            // Legacy device flow (se ancora presente nello storage)
            if pending.expires_at <= Utc::now() {
                outlook.pending = None;
                (
                    OutlookDeviceFlowPoll {
                        status: "expired".to_string(),
                        message: Some("Codice scaduto, avvia nuovamente la procedura".to_string()),
                        retry_in: None,
                    },
                    true,
                )
            } else {
                let (client_id, tenant) = ensure_client_and_tenant(outlook)?;
                let response = HTTP_CLIENT
                    .post(format!(
                        "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"
                    ))
                    .form(&[
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ("client_id", client_id.as_str()),
                        ("device_code", pending.device_code.as_str()),
                    ])
                    .send()
                    .await
                    .context("Richiesta polling token Outlook fallita")?;

                if response.status().is_success() {
                    let token: TokenSuccessResponse = response
                        .json()
                        .await
                        .context("Impossibile decodificare risposta token Outlook")?;

                    let expires_at = Utc::now() + Duration::seconds(token.expires_in.max(0));

                    outlook.pending = None;
                    outlook.access_token = Some(token.access_token);
                    if let Some(refresh) = token.refresh_token {
                        outlook.refresh_token = Some(refresh);
                    }
                    outlook.expires_at = Some(expires_at);
                    outlook.enabled = true;

                    (
                        OutlookDeviceFlowPoll {
                            status: "completed".to_string(),
                            message: Some("Outlook collegato con successo".to_string()),
                            retry_in: None,
                        },
                        true,
                    )
                } else {
                    let status_code = response.status();
                    let body_text = response.text().await.unwrap_or_default();

                    if let Ok(error) = serde_json::from_str::<TokenErrorResponse>(&body_text) {
                        match error.error.as_str() {
                            "authorization_pending" => {
                                return Ok(OutlookDeviceFlowPoll {
                                    status: "pending".to_string(),
                                    message: error.error_description,
                                    retry_in: Some(pending.interval.max(1)),
                                })
                            }
                            "slow_down" => {
                                let retry = pending.interval + 2;
                                if let Some(current) = outlook.pending.as_mut() {
                                    current.interval = retry;
                                }
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "pending".to_string(),
                                        message: error.error_description,
                                        retry_in: Some(retry),
                                    },
                                    true,
                                )
                            }
                            "access_denied" => {
                                outlook.pending = None;
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "declined".to_string(),
                                        message: Some(
                                            "Richiesta rifiutata dall'utente. Ripeti la procedura se desideri collegare Outlook.".to_string(),
                                        ),
                                        retry_in: None,
                                    },
                                    true,
                                )
                            }
                            "expired_token" => {
                                outlook.pending = None;
                                (
                                    OutlookDeviceFlowPoll {
                                        status: "expired".to_string(),
                                        message: Some(
                                            "Codice scaduto, avvia nuovamente la procedura".to_string(),
                                        ),
                                        retry_in: None,
                                    },
                                    true,
                                )
                            }
                            other => {
                                return Ok(OutlookDeviceFlowPoll {
                                    status: "error".to_string(),
                                    message: Some(format!("Errore Outlook: {}", other)),
                                    retry_in: Some(pending.interval.max(1)),
                                })
                            }
                        }
                    } else {
                        return Err(anyhow!(
                            "Errore polling Outlook (stato {}): {}",
                            status_code,
                            body_text
                        ));
                    }
                }
            }
        } else {
            if outlook.enabled && outlook.access_token.is_some() {
                return Ok(OutlookDeviceFlowPoll {
                    status: "completed".to_string(),
                    message: Some("Outlook collegato con successo".to_string()),
                    retry_in: None,
                });
            }
            return Ok(OutlookDeviceFlowPoll {
                status: "idle".to_string(),
                message: Some("Nessuna richiesta di collegamento in corso".to_string()),
                retry_in: None,
            });
        }
    };

    if should_store {
        store_integrations(&integrations)?;
    }

    Ok(result)
}

pub async fn list_outlook_events(limit: usize) -> Result<Vec<RemoteCalendarEvent>> {
    let mut integrations = load_integrations()?;
    let token = {
        let outlook = get_outlook_config_mut(&mut integrations);
        if !outlook.enabled {
            return Err(anyhow!("Outlook non è abilitato"));
        }
        if outlook.access_token.is_none() {
            return Err(anyhow!("Outlook non è collegato"));
        }
        ensure_scopes(outlook);
        ensure_outlook_access_token(outlook).await?
    };

    store_integrations(&integrations)?;

    let max_results = limit.max(1).min(50);

    let response = HTTP_CLIENT
        .get(format!("{GRAPH_ENDPOINT}/me/events"))
        .query(&[
            ("$top", max_results.to_string()),
            ("$orderby", "start/dateTime".to_string()),
            (
                "$select",
                "id,subject,bodyPreview,start,end,location,webLink".to_string(),
            ),
        ])
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .context("Richiesta eventi Outlook fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Impossibile recuperare eventi Outlook (stato {}): {}",
            status,
            body
        ));
    }

    #[derive(Debug, Deserialize)]
    struct GraphEventsResponse {
        #[serde(default)]
        value: Vec<GraphEvent>,
    }

    #[derive(Debug, Deserialize)]
    struct GraphEvent {
        id: String,
        #[serde(default)]
        subject: Option<String>,
        #[serde(default, rename = "bodyPreview")]
        body_preview: Option<String>,
        #[serde(default)]
        start: Option<GraphEventDateTime>,
        #[serde(default)]
        end: Option<GraphEventDateTime>,
        #[serde(default)]
        location: Option<GraphLocation>,
        #[serde(default, rename = "webLink")]
        web_link: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GraphEventDateTime {
        #[serde(default, rename = "dateTime")]
        date_time: Option<String>,
        #[serde(default, rename = "timeZone")]
        #[allow(dead_code)]
        time_zone: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GraphLocation {
        #[serde(default, rename = "displayName")]
        display_name: Option<String>,
    }

    let events: GraphEventsResponse = response
        .json()
        .await
        .context("Impossibile decodificare eventi Outlook")?;

    let mapped = events
        .value
        .into_iter()
        .map(|event| {
            let subject = sanitize_optional_string(&event.subject).unwrap_or_else(|| "Evento".to_string());
            let start = event
                .start
                .as_ref()
                .and_then(|dt| dt.date_time.clone())
                .unwrap_or_else(|| Utc::now().to_rfc3339());
            let end = event
                .end
                .as_ref()
                .and_then(|dt| dt.date_time.clone())
                .unwrap_or_else(|| Utc::now().to_rfc3339());

            RemoteCalendarEvent {
                id: event.id,
                subject,
                start,
                end,
                location: event
                    .location
                    .as_ref()
                    .and_then(|l| sanitize_optional_string(&l.display_name)),
                web_link: sanitize_optional_string(&event.web_link),
                body_preview: sanitize_optional_string(&event.body_preview),
                source: "outlook",
            }
        })
        .collect();

    Ok(mapped)
}

pub async fn create_outlook_event(request: CreateRemoteEventRequest) -> Result<RemoteCalendarEvent> {
    let mut integrations = load_integrations()?;
    let token = {
        let outlook = get_outlook_config_mut(&mut integrations);
        if !outlook.enabled {
            return Err(anyhow!("Outlook non è abilitato"));
        }
        if outlook.access_token.is_none() {
            return Err(anyhow!("Outlook non è collegato"));
        }
        ensure_scopes(outlook);
        ensure_outlook_access_token(outlook).await?
    };

    store_integrations(&integrations)?;

    #[derive(Serialize)]
    struct GraphBody<'a> {
        #[serde(rename = "contentType")]
        content_type: &'a str,
        content: &'a str,
    }

    #[derive(Serialize)]
    struct GraphDateTime<'a> {
        #[serde(rename = "dateTime")]
        date_time: &'a str,
        #[serde(rename = "timeZone")]
        time_zone: &'a str,
    }

    #[derive(Serialize)]
    struct GraphLocationBody<'a> {
        #[serde(rename = "displayName")]
        display_name: &'a str,
    }

    #[derive(Serialize)]
    struct GraphCreateEvent<'a> {
        subject: &'a str,
        start: GraphDateTime<'a>,
        end: GraphDateTime<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<GraphBody<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<GraphLocationBody<'a>>,
    }

    let body = GraphCreateEvent {
        subject: request.subject.as_str(),
        start: GraphDateTime {
            date_time: request.start.as_str(),
            time_zone: DEFAULT_TIME_ZONE,
        },
        end: GraphDateTime {
            date_time: request.end.as_str(),
            time_zone: DEFAULT_TIME_ZONE,
        },
        body: request
            .body
            .as_deref()
            .map(|content| GraphBody { content_type: "text", content }),
        location: request
            .location
            .as_deref()
            .map(|name| GraphLocationBody { display_name: name }),
    };

    let response = HTTP_CLIENT
        .post(format!("{GRAPH_ENDPOINT}/me/events"))
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .context("Creazione evento Outlook fallita")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Impossibile creare evento Outlook (stato {}): {}",
            status,
            body
        ));
    }

    #[derive(Debug, Deserialize)]
    struct GraphCreatedEvent {
        id: String,
        #[serde(default)]
        subject: Option<String>,
        #[serde(default, rename = "bodyPreview")]
        body_preview: Option<String>,
        #[serde(default)]
        start: Option<GraphEventDateTime>,
        #[serde(default)]
        end: Option<GraphEventDateTime>,
        #[serde(default)]
        location: Option<GraphLocation>,
        #[serde(default, rename = "webLink")]
        web_link: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GraphEventDateTime {
        #[serde(default, rename = "dateTime")]
        date_time: Option<String>,
        #[serde(default, rename = "timeZone")]
        #[allow(dead_code)]
        time_zone: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct GraphLocation {
        #[serde(default, rename = "displayName")]
        display_name: Option<String>,
    }

    let event: GraphCreatedEvent = response
        .json()
        .await
        .context("Impossibile decodificare evento creato su Outlook")?;

    Ok(RemoteCalendarEvent {
        id: event.id,
        subject: sanitize_optional_string(&event.subject).unwrap_or_else(|| "Evento".to_string()),
        start: event
            .start
            .as_ref()
            .and_then(|dt| dt.date_time.clone())
            .unwrap_or_else(|| request.start.clone()),
        end: event
            .end
            .as_ref()
            .and_then(|dt| dt.date_time.clone())
            .unwrap_or_else(|| request.end.clone()),
        location: event
            .location
            .as_ref()
            .and_then(|l| sanitize_optional_string(&l.display_name)),
        web_link: sanitize_optional_string(&event.web_link),
        body_preview: sanitize_optional_string(&event.body_preview),
        source: "outlook",
    })
}

pub async fn is_outlook_connected() -> Result<bool> {
    let integrations = load_integrations()?;
    let outlook = integrations.outlook.unwrap_or_default();
    Ok(outlook.enabled && outlook.access_token.is_some())
}

pub async fn push_local_event_to_outlook(event: &CalendarEvent) -> Result<()> {
    let subject = event.title.clone();
    let start = event.start.to_rfc3339();
    let end = event
        .end
        .unwrap_or_else(|| event.start + Duration::hours(1))
        .to_rfc3339();

    let description = event
        .description
        .clone()
        .or_else(|| event.source_text.clone())
        .unwrap_or_else(|| "Evento creato da MatePro".to_string());

    let request = CreateRemoteEventRequest {
        subject,
        start,
        end,
        body: Some(description),
        location: None,
    };

    let _ = create_outlook_event(request).await?;
    Ok(())
}

pub async fn push_local_event_to_google(event: &CalendarEvent) -> Result<()> {
    let subject = event.title.clone();
    let start = event.start.to_rfc3339();
    let end = event
        .end
        .unwrap_or_else(|| event.start + Duration::hours(1))
        .to_rfc3339();

    let description = event
        .description
        .clone()
        .or_else(|| event.source_text.clone())
        .unwrap_or_else(|| "Evento creato da MatePro".to_string());

    let request = CreateRemoteEventRequest {
        subject,
        start,
        end,
        body: Some(description),
        location: None,
    };

    let _ = create_google_event(request).await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn start_google_legacy_device_flow() -> Result<OutlookDeviceFlowStart> {
    // Supporto legacy, richiede client_secret configurato.
    let mut integrations = load_integrations()?;
    let start_payload = {
        let google = get_google_config_mut(&mut integrations);
        ensure_google_scopes(google);

        let (client_id, client_secret) = ensure_google_client(google)?;
        let scope = google.scopes.join(" ");

        let response = HTTP_CLIENT
            .post(GOOGLE_DEVICE_CODE_ENDPOINT)
            .form(&[("client_id", client_id.as_str()), ("scope", scope.as_str())])
            .send()
            .await
            .context("Richiesta device code Google fallita")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Device code Google fallito: stato {} - {}",
                status,
                body
            ));
        }

        let device: GoogleDeviceCodeResponse = response
            .json()
            .await
            .context("Impossibile decodificare la risposta device code Google")?;

        let interval = device.interval.unwrap_or(5);
        let expires_at = Utc::now() + Duration::seconds(device.expires_in.max(0));

        google.pending = Some(PendingDeviceFlow {
            device_code: device.device_code.clone(),
            user_code: device.user_code.clone(),
            verification_uri: device.verification_url.clone(),
            expires_at,
            interval,
            message: Some("Apri la pagina indicata e inserisci il codice".to_string()),
        });
        google.enabled = true;

        // Evita warning unused
        let _ = client_secret;

        OutlookDeviceFlowStart {
            user_code: device.user_code,
            verification_uri: device.verification_url,
            message: "Autorizzazione Google Calendar".to_string(),
            interval,
            expires_at: expires_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    };

    store_integrations(&integrations)?;

    Ok(start_payload)
}

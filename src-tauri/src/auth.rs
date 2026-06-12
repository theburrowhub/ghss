use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "ghss";
const KEYRING_USER: &str = "github_pat";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("gh CLI no disponible o sin sesión: {0}")]
    GhCli(String),
    #[error("keychain: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("red: {0}")]
    Network(#[from] reqwest::Error),
    #[error("device flow: {0}")]
    Device(String),
}

/// Obtiene el token de la sesión del CLI `gh` (gh auth token).
pub async fn gh_cli_token() -> Result<String, AuthError> {
    let out = tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
        .map_err(|e| AuthError::GhCli(e.to_string()))?;
    if !out.status.success() {
        return Err(AuthError::GhCli(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if token.is_empty() {
        return Err(AuthError::GhCli("token vacío".into()));
    }
    Ok(token)
}

pub fn save_pat(pat: &str) -> Result<(), AuthError> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?.set_password(pat)?;
    Ok(())
}

pub fn load_pat() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?.get_password().ok()
}

pub fn delete_pat() {
    if let Ok(e) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = e.delete_credential();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

#[derive(Clone, PartialEq)]
pub enum DevicePoll {
    Pending,
    Token(String),
}

impl std::fmt::Debug for DevicePoll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DevicePoll::Pending => write!(f, "DevicePoll::Pending"),
            DevicePoll::Token(_) => write!(f, "DevicePoll::Token([REDACTED])"),
        }
    }
}

pub fn github_oauth_base() -> String {
    "https://github.com".into()
}

pub async fn device_start(base: &str, client_id: &str) -> Result<DeviceStart, AuthError> {
    let resp: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/login/device/code"))
        .header("accept", "application/json")
        .json(&serde_json::json!({"client_id": client_id, "scope": "repo read:org"}))
        .send()
        .await?
        .json()
        .await?;
    let device_code = resp["device_code"].as_str().unwrap_or_default().to_string();
    if device_code.is_empty() {
        return Err(AuthError::Device("respuesta inválida de /login/device/code: falta device_code".into()));
    }
    Ok(DeviceStart {
        device_code,
        user_code: resp["user_code"].as_str().unwrap_or_default().into(),
        verification_uri: resp["verification_uri"].as_str().unwrap_or("https://github.com/login/device").into(),
        interval: resp["interval"].as_u64().unwrap_or(5),
    })
}

pub async fn device_poll_once(base: &str, client_id: &str, device_code: &str) -> Result<DevicePoll, AuthError> {
    let resp: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/login/oauth/access_token"))
        .header("accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        }))
        .send()
        .await?
        .json()
        .await?;
    if let Some(tok) = resp["access_token"].as_str() {
        return Ok(DevicePoll::Token(tok.into()));
    }
    match resp["error"].as_str() {
        Some("authorization_pending") | Some("slow_down") => Ok(DevicePoll::Pending),
        Some(e) => Err(AuthError::Device(e.into())),
        None => Err(AuthError::Device("respuesta inesperada".into())),
    }
}

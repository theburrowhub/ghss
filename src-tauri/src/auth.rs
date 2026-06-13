use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "ghss";
const KEYRING_USER: &str = "github_pat";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("GitHub CLI not available or not signed in: {0}")]
    GhCli(String),
    #[error("keychain: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("device flow: {0}")]
    Device(String),
}

/// Reads the real PATH from the user's login shell. macOS GUI apps launched from
/// Finder/.app do not inherit that PATH, so `gh` (in Homebrew, MacPorts, mise/asdf shims…)
/// is not found. Running the shell with `-lc` gets the PATH with the full configuration.
async fn login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").ok()?;
    let fut = tokio::process::Command::new(&shell)
        .args(["-lc", "printf %s \"$PATH\""])
        .output();
    let out = tokio::time::timeout(std::time::Duration::from_secs(4), fut).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!p.is_empty()).then_some(p)
}

/// Merges the base PATH with common binary locations, deduplicating entries.
fn merge_paths(base: &str) -> String {
    let common = ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin", "/bin"];
    let mut seen = std::collections::HashSet::new();
    let mut parts: Vec<String> = Vec::new();
    for p in common.iter().map(|s| s.to_string()).chain(base.split(':').map(str::to_string)) {
        if !p.is_empty() && seen.insert(p.clone()) {
            parts.push(p);
        }
    }
    parts.join(":")
}

/// Retrieves the token from the `gh` CLI session (gh auth token), locating `gh` using a
/// robust PATH (login shell + common locations).
pub async fn gh_cli_token() -> Result<String, AuthError> {
    let base = login_shell_path().await.unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
    let out = tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .env("PATH", merge_paths(&base))
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AuthError::GhCli("could not find the «gh» binary in PATH. Install GitHub CLI (https://cli.github.com) or use a token (PAT).".into())
            } else {
                AuthError::GhCli(e.to_string())
            }
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if stderr.is_empty() {
            "gh has no active session. Run «gh auth login» in a terminal.".to_string()
        } else {
            stderr
        };
        return Err(AuthError::GhCli(msg));
    }
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if token.is_empty() {
        return Err(AuthError::GhCli("gh returned an empty token. Run «gh auth login».".into()));
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
        return Err(AuthError::Device("invalid response from /login/device/code: missing device_code".into()));
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
        None => Err(AuthError::Device("unexpected response".into())),
    }
}

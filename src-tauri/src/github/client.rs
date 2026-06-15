use crate::model::RepoInfo;
use reqwest::{Method, Response, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// TTL de la caché de snapshots: dentro de esta ventana, auditar→sincronizar o re-auditar
/// reutiliza el snapshot en memoria sin volver a llamar a la API.
const SNAPSHOT_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum GhError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("GitHub responded {status}: {body}")]
    Api { status: StatusCode, body: String },
    #[error("401 invalid session: the token is incorrect or has expired")]
    Unauthorized,
}

pub type GhResult<T> = Result<T, GhError>;

#[derive(Clone)]
struct CachedResponse {
    etag: String,
    body: Value,
}

#[derive(Clone)]
pub struct GithubClient {
    http: reqwest::Client,
    base: String,
    token: String,
    // Caché de ETags para peticiones GET. Las respuestas 304 (Not Modified) NO cuentan
    // contra el rate limit de GitHub, así que re-auditar repos sin cambios es casi gratis.
    // Arc<Mutex<…>> para compartir la caché entre los clones del cliente (audit clona por repo).
    cache: Arc<Mutex<HashMap<String, CachedResponse>>>,
    // Caché de snapshots por repo con TTL: evita re-fetchear (round-trips incluidos) lo recién leído.
    snapshot_cache: Arc<Mutex<HashMap<String, (Instant, crate::model::RepoSettingsSnapshot)>>>,
}

impl GithubClient {
    pub fn new(base: impl Into<String>, token: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("ghss/0.1")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base: base.into(),
            token,
            cache: Arc::new(Mutex::new(HashMap::new())),
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Devuelve el snapshot cacheado del repo si sigue fresco (dentro del TTL).
    pub(crate) fn cached_snapshot(&self, repo: &str) -> Option<crate::model::RepoSettingsSnapshot> {
        let guard = self.snapshot_cache.lock().unwrap();
        guard
            .get(repo)
            .filter(|(t, _)| t.elapsed() < SNAPSHOT_TTL)
            .map(|(_, s)| s.clone())
    }

    pub(crate) fn store_snapshot(&self, repo: &str, snap: &crate::model::RepoSettingsSnapshot) {
        self.snapshot_cache.lock().unwrap().insert(repo.to_string(), (Instant::now(), snap.clone()));
    }

    /// Invalida el snapshot cacheado de un repo (tras escribir en él, para no servir estado viejo).
    pub(crate) fn invalidate_snapshot(&self, owner: &str, name: &str) {
        self.snapshot_cache.lock().unwrap().remove(&format!("{owner}/{name}"));
    }

    pub fn api_base() -> String {
        "https://api.github.com".into()
    }

    async fn send(&self, method: Method, path: &str, body: Option<&Value>, etag: Option<&str>) -> GhResult<Response> {
        let url = format!("{}{}", self.base, path);
        for attempt in 0..3u32 {
            let mut req = self
                .http
                .request(method.clone(), &url)
                .header("authorization", format!("Bearer {}", self.token))
                .header("accept", "application/vnd.github+json")
                .header("x-github-api-version", "2022-11-28");
            if let Some(e) = etag {
                req = req.header("if-none-match", e);
            }
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();
            // 304 Not Modified: el recurso no cambió. Lo devolvemos para que el caller use su
            // copia cacheada (no cuenta contra el rate limit). Va antes del check de !is_success.
            if status == StatusCode::NOT_MODIFIED {
                return Ok(resp);
            }
            // We only retry on 429 (secondary rate limit / abuse), which does honor retry-after.
            // A 403 from an exhausted primary rate limit (x-ratelimit-remaining: 0) does not
            // recover in seconds — it resets on the hour — so we fail fast with a clear message.
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let wait = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2)
                    .min(60);
                let _ = resp.bytes().await; // drain the body to reuse the connection
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            if status == StatusCode::UNAUTHORIZED {
                return Err(GhError::Unauthorized);
            }
            if !status.is_success() {
                // Rate limit primario agotado (403 con remaining 0, o 429): mensaje claro con el
                // tiempo de reset en vez del JSON crudo de GitHub.
                let remaining_zero = resp.headers().get("x-ratelimit-remaining").map(|v| v == "0").unwrap_or(false);
                let reset_epoch = resp
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());
                let body = resp.text().await.unwrap_or_default();
                let is_rate_limit = status == StatusCode::TOO_MANY_REQUESTS
                    || (status == StatusCode::FORBIDDEN && (remaining_zero || body.contains("rate limit")));
                if is_rate_limit {
                    let mins = reset_epoch.and_then(|r| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|now| r.saturating_sub(now.as_secs()).div_ceil(60))
                    });
                    let msg = match mins {
                        Some(m) => format!("rate limit exceeded; resets in ~{m} min"),
                        None => "rate limit exceeded".into(),
                    };
                    return Err(GhError::Api { status, body: msg });
                }
                return Err(GhError::Api { status, body });
            }
            return Ok(resp);
        }
        Err(GhError::Api { status: StatusCode::TOO_MANY_REQUESTS, body: "rate limit exhausted after retries".into() })
    }

    pub(crate) async fn get_json(&self, path: &str) -> GhResult<Value> {
        let key = format!("{}{}", self.base, path);
        let etag = self.cache.lock().unwrap().get(&key).map(|c| c.etag.clone());

        let resp = self.send(Method::GET, path, None, etag.as_deref()).await?;

        if resp.status() == StatusCode::NOT_MODIFIED {
            // Recurso sin cambios: devolvemos la copia cacheada (no consumió rate limit).
            return self
                .cache
                .lock()
                .unwrap()
                .get(&key)
                .map(|c| c.body.clone())
                .ok_or_else(|| GhError::Api { status: StatusCode::NOT_MODIFIED, body: "304 sin entrada en caché".into() });
        }

        let new_etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).map(String::from);
        let body: Value = resp.json().await?;
        if let Some(e) = new_etag {
            self.cache.lock().unwrap().insert(key, CachedResponse { etag: e, body: body.clone() });
        }
        Ok(body)
    }

    pub(crate) async fn send_json(&self, method: Method, path: &str, body: &Value) -> GhResult<Value> {
        let resp = self.send(method, path, Some(body), None).await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        Ok(resp.json().await?)
    }

    pub async fn get_user(&self) -> GhResult<Value> {
        self.get_json("/user").await
    }

    /// Validates the token and returns (user, scopes). Scopes come from the
    /// `X-OAuth-Scopes` header (present in classic/OAuth/gh tokens; empty in fine-grained PATs).
    pub async fn auth_check(&self) -> GhResult<(Value, Vec<String>)> {
        let resp = self.send(Method::GET, "/user", None, None).await?;
        let scopes = resp
            .headers()
            .get("x-oauth-scopes")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();
        let user: Value = resp.json().await?;
        Ok((user, scopes))
    }

    pub async fn update_repo(&self, owner: &str, name: &str, body: &Value) -> GhResult<Value> {
        self.invalidate_snapshot(owner, name);
        self.send_json(Method::PATCH, &format!("/repos/{owner}/{name}"), body).await
    }

    pub async fn create_ruleset(&self, owner: &str, name: &str, payload: &Value) -> GhResult<Value> {
        self.invalidate_snapshot(owner, name);
        self.send_json(Method::POST, &format!("/repos/{owner}/{name}/rulesets"), payload).await
    }

    pub async fn update_ruleset(&self, owner: &str, name: &str, id: u64, payload: &Value) -> GhResult<Value> {
        self.invalidate_snapshot(owner, name);
        self.send_json(Method::PUT, &format!("/repos/{owner}/{name}/rulesets/{id}"), payload).await
    }

    pub async fn put_branch_protection(&self, owner: &str, name: &str, branch: &str, config: &Value) -> GhResult<Value> {
        self.invalidate_snapshot(owner, name);
        self.send_json(Method::PUT, &format!("/repos/{owner}/{name}/branches/{branch}/protection"), config).await
    }

    pub async fn list_org_teams(&self, org: &str) -> GhResult<Vec<crate::model::TeamInfo>> {
        let mut teams = Vec::new();
        for page in 1.. {
            let path = format!("/orgs/{org}/teams?per_page=100&page={page}");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?)
                .map_err(|e| GhError::Api { status: StatusCode::OK, body: format!("unexpected response from /orgs/{org}/teams: {e}") })?;
            let n = batch.len();
            for t in batch {
                teams.push(crate::model::TeamInfo {
                    slug: t["slug"].as_str().unwrap_or_default().into(),
                    name: t["name"].as_str().unwrap_or_default().into(),
                });
            }
            if n < 100 {
                break;
            }
        }
        Ok(teams)
    }

    pub async fn list_team_repos(&self, org: &str, team_slug: &str) -> GhResult<Vec<String>> {
        let mut repos = Vec::new();
        for page in 1.. {
            let path = format!("/orgs/{org}/teams/{team_slug}/repos?per_page=100&page={page}");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?)
                .map_err(|e| GhError::Api { status: StatusCode::OK, body: format!("unexpected response from team repos: {e}") })?;
            let n = batch.len();
            for r in batch {
                if let Some(full) = r["full_name"].as_str() {
                    repos.push(full.to_string());
                }
            }
            if n < 100 {
                break;
            }
        }
        Ok(repos)
    }

    pub async fn list_repos(&self) -> GhResult<Vec<RepoInfo>> {
        let mut repos = Vec::new();
        for page in 1.. {
            let path = format!("/user/repos?per_page=100&page={page}&affiliation=owner,collaborator,organization_member&sort=full_name");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?)
                .map_err(|e| GhError::Api { status: StatusCode::OK, body: format!("unexpected response from /user/repos: {e}") })?;
            let n = batch.len();
            for r in batch {
                repos.push(RepoInfo {
                    full_name: r["full_name"].as_str().unwrap_or_default().into(),
                    owner: r["owner"]["login"].as_str().unwrap_or_default().into(),
                    name: r["name"].as_str().unwrap_or_default().into(),
                    private: r["private"].as_bool().unwrap_or(false),
                    admin: r["permissions"]["admin"].as_bool().unwrap_or(false),
                    archived: r["archived"].as_bool().unwrap_or(false),
                    default_branch: r["default_branch"].as_str().unwrap_or("main").into(),
                    description: r["description"].as_str().map(String::from),
                });
            }
            if n < 100 {
                break;
            }
        }
        Ok(repos)
    }
}

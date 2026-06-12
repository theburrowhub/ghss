use crate::model::RepoInfo;
use reqwest::{Method, Response, StatusCode};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum GhError {
    #[error("error de red: {0}")]
    Network(#[from] reqwest::Error),
    #[error("GitHub respondió {status}: {body}")]
    Api { status: StatusCode, body: String },
    #[error("no autenticado")]
    Unauthorized,
}

pub type GhResult<T> = Result<T, GhError>;

#[derive(Clone)]
pub struct GithubClient {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl GithubClient {
    pub fn new(base: impl Into<String>, token: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("ghss/0.1")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self { http, base: base.into(), token }
    }

    pub fn api_base() -> String {
        "https://api.github.com".into()
    }

    async fn send(&self, method: Method, path: &str, body: Option<&Value>) -> GhResult<Response> {
        let url = format!("{}{}", self.base, path);
        for attempt in 0..3u32 {
            let mut req = self
                .http
                .request(method.clone(), &url)
                .header("authorization", format!("Bearer {}", self.token))
                .header("accept", "application/vnd.github+json")
                .header("x-github-api-version", "2022-11-28");
            if let Some(b) = body {
                req = req.json(b);
            }
            let resp = req.send().await?;
            let status = resp.status();
            let rate_limited = status == StatusCode::TOO_MANY_REQUESTS
                || (status == StatusCode::FORBIDDEN
                    && resp.headers().get("x-ratelimit-remaining").map(|v| v == "0").unwrap_or(false));
            if rate_limited && attempt < 2 {
                let wait = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2)
                    .min(60);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            if status == StatusCode::UNAUTHORIZED {
                return Err(GhError::Unauthorized);
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(GhError::Api { status, body });
            }
            return Ok(resp);
        }
        unreachable!("el bucle de reintentos siempre retorna")
    }

    pub(crate) async fn get_json(&self, path: &str) -> GhResult<Value> {
        Ok(self.send(Method::GET, path, None).await?.json().await?)
    }

    pub(crate) async fn send_json(&self, method: Method, path: &str, body: &Value) -> GhResult<Value> {
        let resp = self.send(method, path, Some(body)).await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        Ok(resp.json().await?)
    }

    pub async fn get_user(&self) -> GhResult<Value> {
        self.get_json("/user").await
    }

    pub async fn list_repos(&self) -> GhResult<Vec<RepoInfo>> {
        let mut repos = Vec::new();
        for page in 1.. {
            let path = format!("/user/repos?per_page=100&page={page}&affiliation=owner,collaborator,organization_member&sort=full_name");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?).unwrap_or_default();
            let n = batch.len();
            for r in batch {
                repos.push(RepoInfo {
                    full_name: r["full_name"].as_str().unwrap_or_default().into(),
                    owner: r["owner"]["login"].as_str().unwrap_or_default().into(),
                    name: r["name"].as_str().unwrap_or_default().into(),
                    private: r["private"].as_bool().unwrap_or(false),
                    admin: r["permissions"]["admin"].as_bool().unwrap_or(false),
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

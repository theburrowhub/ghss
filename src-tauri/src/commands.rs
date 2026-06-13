use crate::auth;
use crate::diff::diff_snapshots;
use crate::github::{GhError, GithubClient};
use crate::model::*;
use crate::sync::{apply_actions, plan_actions, ActionResult};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

#[derive(Default)]
pub struct AppState {
    pub client: RwLock<Option<GithubClient>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub login: String,
    pub avatar_url: String,
}

type CmdResult<T> = Result<T, String>;

async fn login_with_token(state: &State<'_, AppState>, token: String) -> CmdResult<UserInfo> {
    let client = GithubClient::new(GithubClient::api_base(), token);
    let user = client.get_user().await.map_err(|e| e.to_string())?;
    let info = UserInfo {
        login: user["login"].as_str().unwrap_or_default().into(),
        avatar_url: user["avatar_url"].as_str().unwrap_or_default().into(),
    };
    *state.client.write().await = Some(client);
    Ok(info)
}

#[tauri::command]
pub async fn auth_with_gh(state: State<'_, AppState>) -> CmdResult<UserInfo> {
    let token = auth::gh_cli_token().await.map_err(|e| e.to_string())?;
    login_with_token(&state, token).await
}

#[tauri::command]
pub async fn auth_with_pat(state: State<'_, AppState>, pat: String, save: bool) -> CmdResult<UserInfo> {
    let info = login_with_token(&state, pat.clone()).await?;
    if save {
        auth::save_pat(&pat).map_err(|e| e.to_string())?;
    }
    Ok(info)
}

#[tauri::command]
pub async fn auth_load_saved(state: State<'_, AppState>) -> CmdResult<Option<UserInfo>> {
    if let Some(pat) = auth::load_pat() {
        if let Ok(info) = login_with_token(&state, pat).await {
            return Ok(Some(info));
        }
        auth::delete_pat();
    }
    if let Ok(token) = auth::gh_cli_token().await {
        if let Ok(info) = login_with_token(&state, token).await {
            return Ok(Some(info));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn auth_device_start(client_id: String) -> CmdResult<auth::DeviceStart> {
    auth::device_start(&auth::github_oauth_base(), &client_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn auth_device_poll(state: State<'_, AppState>, client_id: String, device_code: String) -> CmdResult<Option<UserInfo>> {
    match auth::device_poll_once(&auth::github_oauth_base(), &client_id, &device_code).await.map_err(|e| e.to_string())? {
        auth::DevicePoll::Pending => Ok(None),
        auth::DevicePoll::Token(tok) => login_with_token(&state, tok).await.map(Some),
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> CmdResult<()> {
    auth::delete_pat();
    *state.client.write().await = None;
    Ok(())
}

async fn client(state: &State<'_, AppState>) -> CmdResult<GithubClient> {
    state.client.read().await.clone().ok_or_else(|| "no autenticado".to_string())
}

#[tauri::command]
pub async fn list_repos(state: State<'_, AppState>) -> CmdResult<Vec<RepoInfo>> {
    client(&state).await?.list_repos().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_org_teams(state: State<'_, AppState>, org: String) -> CmdResult<Vec<TeamInfo>> {
    match client(&state).await?.list_org_teams(&org).await {
        Ok(teams) => Ok(teams),
        // Cuentas personales (o sin acceso a equipos) devuelven 404: sin equipos, no es error.
        Err(GhError::Api { status, .. }) if status == reqwest::StatusCode::NOT_FOUND => Ok(vec![]),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn list_team_repos(state: State<'_, AppState>, org: String, team_slug: String) -> CmdResult<Vec<String>> {
    client(&state).await?.list_team_repos(&org, &team_slug).await.map_err(|e| e.to_string())
}

fn split_full_name(full: &str) -> CmdResult<(String, String)> {
    full.split_once('/')
        .map(|(o, n)| (o.to_string(), n.to_string()))
        .ok_or_else(|| format!("nombre de repo inválido: {full}"))
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditResult {
    pub reference: RepoSettingsSnapshot,
    pub diffs: Vec<RepoDiff>,
    pub errors: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
struct AuditStarted {
    total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct AuditRepoEvent {
    repo: String,
    diff: Option<RepoDiff>,
    error: Option<String>,
}

#[tauri::command]
pub async fn audit(app: AppHandle, state: State<'_, AppState>, reference: String, targets: Vec<String>) -> CmdResult<AuditResult> {
    let c = client(&state).await?;
    let (ro, rn) = split_full_name(&reference)?;
    let ref_snap = c.fetch_snapshot(&ro, &rn).await.map_err(|e| e.to_string())?;

    let _ = app.emit("audit-started", AuditStarted { total: targets.len() });

    let fetches = targets.into_iter().map(|t| {
        let c = c.clone();
        async move {
            let (o, n) = match t.split_once('/') {
                Some((o, n)) => (o.to_string(), n.to_string()),
                None => return (t.clone(), Err("nombre inválido".to_string())),
            };
            (t.clone(), c.fetch_snapshot(&o, &n).await.map_err(|e| e.to_string()))
        }
    });
    // Concurrencia acotada: cientos de fetches simultáneos disparan los
    // secondary rate limits de GitHub. Emitimos cada repo en cuanto termina
    // (streaming) para que la UI vaya rellenando resultados sin congelarse.
    use futures::StreamExt;
    let mut stream = futures::stream::iter(fetches).buffer_unordered(8);

    let mut diffs = Vec::new();
    let mut errors = Vec::new();
    while let Some((repo, res)) = stream.next().await {
        match res {
            Ok(snap) => {
                let d = diff_snapshots(&ref_snap, &snap);
                let _ = app.emit("audit-repo", AuditRepoEvent { repo, diff: Some(d.clone()), error: None });
                diffs.push(d);
            }
            Err(e) => {
                let _ = app.emit("audit-repo", AuditRepoEvent { repo: repo.clone(), diff: None, error: Some(e.clone()) });
                errors.push((repo, e));
            }
        }
    }
    // buffer_unordered devuelve en orden de finalización; ordenamos el resultado
    // final autoritativo para una UI estable (el streaming es solo incremental).
    diffs.sort_by(|a, b| a.repo.cmp(&b.repo));
    errors.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(AuditResult { reference: ref_snap, diffs, errors })
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoSyncPlan {
    pub repo: String,
    pub changes: Vec<SettingChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoSyncResult {
    pub repo: String,
    pub results: Vec<ActionResult>,
    pub fatal: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProgressEvent {
    repo: String,
    action: String,
}

#[tauri::command]
pub async fn apply_sync(app: AppHandle, state: State<'_, AppState>, plans: Vec<RepoSyncPlan>) -> CmdResult<Vec<RepoSyncResult>> {
    let c = client(&state).await?;
    let mut all = Vec::new();
    for plan in plans {
        let (o, n) = match split_full_name(&plan.repo) {
            Ok(x) => x,
            Err(e) => {
                all.push(RepoSyncResult { repo: plan.repo, results: vec![], fatal: Some(e) });
                continue;
            }
        };
        // Snapshot fresco del destino para resolver create-vs-update de rulesets con ids actuales.
        let target = match c.fetch_snapshot(&o, &n).await {
            Ok(s) => s,
            Err(e) => {
                all.push(RepoSyncResult { repo: plan.repo, results: vec![], fatal: Some(e.to_string()) });
                continue;
            }
        };
        let actions = plan_actions(&plan.changes, &target);
        let repo_name = plan.repo.clone();
        let app2 = app.clone();
        let results = apply_actions(&c, &o, &n, &actions, |desc| {
            let _ = app2.emit("sync-progress", ProgressEvent { repo: repo_name.clone(), action: desc.to_string() });
        })
        .await;
        all.push(RepoSyncResult { repo: plan.repo, results, fatal: None });
    }
    Ok(all)
}

use super::client::{GhError, GhResult, GithubClient};
use super::transform::{normalize_ruleset, protection_get_to_put};
use crate::model::*;
use reqwest::StatusCode;
use serde_json::Value;

impl GithubClient {
    pub async fn fetch_snapshot(&self, owner: &str, name: &str) -> GhResult<RepoSettingsSnapshot> {
        let repo = self.get_json(&format!("/repos/{owner}/{name}")).await?;

        let mut branches = Vec::new();
        let mut protected = Vec::new();
        for page in 1.. {
            let path = format!("/repos/{owner}/{name}/branches?per_page=100&page={page}");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?)
                .map_err(|e| GhError::Api { status: StatusCode::OK, body: format!("respuesta inesperada de {path}: {e}") })?;
            let n = batch.len();
            for b in batch {
                let bname = b["name"].as_str().unwrap_or_default().to_string();
                if b["protected"].as_bool().unwrap_or(false) {
                    protected.push(bname.clone());
                }
                branches.push(bname);
            }
            if n < 100 {
                break;
            }
        }

        let mut summaries = Vec::new();
        for page in 1.. {
            let path = format!("/repos/{owner}/{name}/rulesets?per_page=100&page={page}&includes_parents=false");
            let batch: Vec<Value> = serde_json::from_value(self.get_json(&path).await?)
                .map_err(|e| GhError::Api { status: StatusCode::OK, body: format!("respuesta inesperada de {path}: {e}") })?;
            let n = batch.len();
            summaries.extend(batch);
            if n < 100 {
                break;
            }
        }
        let mut rulesets = Vec::new();
        for s in summaries {
            // Defensive: skip summaries without a numeric id (id 0 would produce an invalid PUT later)
            let Some(id) = s["id"].as_u64() else { continue };
            let full = self.get_json(&format!("/repos/{owner}/{name}/rulesets/{id}")).await?;
            rulesets.push(RulesetSummary {
                id,
                name: full["name"].as_str().unwrap_or_default().into(),
                target: full["target"].as_str().unwrap_or("branch").into(),
                payload: normalize_ruleset(full),
            });
        }

        let mut branch_protections = Vec::new();
        for b in &protected {
            let path = format!("/repos/{owner}/{name}/branches/{b}/protection");
            match self.get_json(&path).await {
                Ok(get) => branch_protections.push(BranchProtection { branch: b.clone(), config: protection_get_to_put(&get) }),
                // Rama protegida solo por rulesets: no hay protección clásica que snapshotear.
                Err(GhError::Api { status, .. }) if status == StatusCode::NOT_FOUND => {}
                Err(e) => return Err(e),
            }
        }

        let s = |k: &str| repo[k].as_str().map(String::from);
        let f = |k: &str| repo[k].as_bool().unwrap_or(false);
        Ok(RepoSettingsSnapshot {
            repo: format!("{owner}/{name}"),
            default_branch: repo["default_branch"].as_str().unwrap_or("main").into(),
            branches,
            features: Features {
                has_wiki: f("has_wiki"),
                has_issues: f("has_issues"),
                has_projects: f("has_projects"),
                has_discussions: f("has_discussions"),
                allow_forking: f("allow_forking"),
            },
            pull_requests: PullRequestSettings {
                allow_merge_commit: f("allow_merge_commit"),
                merge_commit_title: s("merge_commit_title"),
                merge_commit_message: s("merge_commit_message"),
                allow_squash_merge: f("allow_squash_merge"),
                squash_merge_commit_title: s("squash_merge_commit_title"),
                squash_merge_commit_message: s("squash_merge_commit_message"),
                allow_rebase_merge: f("allow_rebase_merge"),
                allow_update_branch: f("allow_update_branch"),
                allow_auto_merge: f("allow_auto_merge"),
                delete_branch_on_merge: f("delete_branch_on_merge"),
            },
            others: OtherSettings { web_commit_signoff_required: f("web_commit_signoff_required") },
            rulesets,
            branch_protections,
        })
    }
}

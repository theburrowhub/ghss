use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoInfo {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub private: bool,
    pub admin: bool,
    pub default_branch: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Features {
    pub has_wiki: bool,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_discussions: bool,
    pub allow_forking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PullRequestSettings {
    pub allow_merge_commit: bool,
    pub merge_commit_title: Option<String>,
    pub merge_commit_message: Option<String>,
    pub allow_squash_merge: bool,
    pub squash_merge_commit_title: Option<String>,
    pub squash_merge_commit_message: Option<String>,
    pub allow_rebase_merge: bool,
    pub allow_update_branch: bool,
    pub allow_auto_merge: bool,
    pub delete_branch_on_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OtherSettings {
    pub web_commit_signoff_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RulesetSummary {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchProtection {
    pub branch: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoSettingsSnapshot {
    pub repo: String,
    pub default_branch: String,
    pub branches: Vec<String>,
    pub features: Features,
    pub pull_requests: PullRequestSettings,
    pub others: OtherSettings,
    pub rulesets: Vec<RulesetSummary>,
    pub branch_protections: Vec<BranchProtection>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    DefaultBranch,
    Features,
    PullRequests,
    Others,
    Tags,
    Rules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettingChange {
    pub key: String,
    pub label: String,
    pub category: Category,
    pub current: Value,
    pub desired: Value,
    pub applicable: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoDiff {
    pub repo: String,
    pub changes: Vec<SettingChange>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serde_roundtrip() {
        let snap = RepoSettingsSnapshot {
            repo: "acme/ref".into(),
            default_branch: "main".into(),
            branches: vec!["main".into(), "dev".into()],
            features: Features { has_wiki: true, has_issues: true, has_projects: false, has_discussions: false, allow_forking: false },
            pull_requests: PullRequestSettings {
                allow_merge_commit: false,
                merge_commit_title: None,
                merge_commit_message: None,
                allow_squash_merge: true,
                squash_merge_commit_title: Some("PR_TITLE".into()),
                squash_merge_commit_message: Some("COMMIT_MESSAGES".into()),
                allow_rebase_merge: false,
                allow_update_branch: true,
                allow_auto_merge: false,
                delete_branch_on_merge: true,
            },
            others: OtherSettings { web_commit_signoff_required: false },
            rulesets: vec![RulesetSummary { id: 1, name: "protect-main".into(), target: "branch".into(), payload: serde_json::json!({"name": "protect-main"}) }],
            branch_protections: vec![BranchProtection { branch: "main".into(), config: serde_json::json!({"enforce_admins": true}) }],
        };
        let s = serde_json::to_string(&snap).unwrap();
        let back: RepoSettingsSnapshot = serde_json::from_str(&s).unwrap();
        assert_eq!(snap, back);
    }
}

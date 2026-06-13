use crate::model::*;
use serde_json::{json, Value};

/// Pushes a change if the values differ. Convention at ALL call sites: `current` = value from
/// the TARGET repo (t.*), `desired` = value from the REFERENCE repo (r.*).
fn push_scalar(changes: &mut Vec<SettingChange>, category: Category, key: &str, label: &str, current: Value, desired: Value) {
    if current != desired {
        changes.push(SettingChange {
            key: key.into(),
            label: label.into(),
            category,
            current,
            desired,
            applicable: true,
            note: None,
        });
    }
}

pub fn diff_snapshots(reference: &RepoSettingsSnapshot, target: &RepoSettingsSnapshot) -> RepoDiff {
    let mut ch = Vec::new();

    if reference.default_branch != target.default_branch {
        let exists = target.branches.contains(&reference.default_branch);
        ch.push(SettingChange {
            key: "default_branch".into(),
            label: "Default branch".into(),
            category: Category::DefaultBranch,
            current: json!(target.default_branch),
            desired: json!(reference.default_branch),
            applicable: exists,
            note: (!exists).then(|| format!("Branch «{}» does not exist in the target", reference.default_branch)),
        });
    }

    let (r, t) = (&reference.features, &target.features);
    push_scalar(&mut ch, Category::Features, "features.has_wiki", "Wikis", json!(t.has_wiki), json!(r.has_wiki));
    push_scalar(&mut ch, Category::Features, "features.has_issues", "Issues", json!(t.has_issues), json!(r.has_issues));
    push_scalar(&mut ch, Category::Features, "features.has_projects", "Projects", json!(t.has_projects), json!(r.has_projects));
    push_scalar(&mut ch, Category::Features, "features.has_discussions", "Discussions", json!(t.has_discussions), json!(r.has_discussions));
    push_scalar(&mut ch, Category::Features, "features.allow_forking", "Allow forking", json!(t.allow_forking), json!(r.allow_forking));

    let (r, t) = (&reference.pull_requests, &target.pull_requests);
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.allow_merge_commit", "Allow merge commits", json!(t.allow_merge_commit), json!(r.allow_merge_commit));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.merge_commit_title", "Merge commit title", json!(t.merge_commit_title), json!(r.merge_commit_title));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.merge_commit_message", "Merge commit message", json!(t.merge_commit_message), json!(r.merge_commit_message));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.allow_squash_merge", "Allow squash merging", json!(t.allow_squash_merge), json!(r.allow_squash_merge));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.squash_merge_commit_title", "Squash commit title", json!(t.squash_merge_commit_title), json!(r.squash_merge_commit_title));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.squash_merge_commit_message", "Squash commit message", json!(t.squash_merge_commit_message), json!(r.squash_merge_commit_message));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.allow_rebase_merge", "Allow rebase merging", json!(t.allow_rebase_merge), json!(r.allow_rebase_merge));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.allow_update_branch", "Always suggest updating PR branches", json!(t.allow_update_branch), json!(r.allow_update_branch));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.allow_auto_merge", "Allow auto-merge", json!(t.allow_auto_merge), json!(r.allow_auto_merge));
    push_scalar(&mut ch, Category::PullRequests, "pull_requests.delete_branch_on_merge", "Automatically delete head branches", json!(t.delete_branch_on_merge), json!(r.delete_branch_on_merge));

    push_scalar(&mut ch, Category::Others, "others.web_commit_signoff_required", "Require contributors to sign off on web-based commits", json!(target.others.web_commit_signoff_required), json!(reference.others.web_commit_signoff_required));

    for rs in &reference.rulesets {
        let category = if rs.target == "tag" { Category::Tags } else { Category::Rules };
        let key = format!("ruleset.{}.{}", rs.target, rs.name);
        match target.rulesets.iter().find(|x| x.name == rs.name && x.target == rs.target) {
            None => ch.push(SettingChange {
                key,
                label: format!("Ruleset «{}» (create)", rs.name),
                category,
                current: Value::Null,
                desired: rs.payload.clone(),
                applicable: true,
                note: None,
            }),
            Some(existing) if existing.payload != rs.payload => ch.push(SettingChange {
                key,
                label: format!("Ruleset «{}» (update)", rs.name),
                category,
                current: existing.payload.clone(),
                desired: rs.payload.clone(),
                applicable: true,
                note: None,
            }),
            _ => {}
        }
    }

    for bp in &reference.branch_protections {
        let key = format!("branch_protection.{}", bp.branch);
        let exists = target.branches.contains(&bp.branch);
        let current = target
            .branch_protections
            .iter()
            .find(|x| x.branch == bp.branch)
            .map(|x| x.config.clone())
            .unwrap_or(Value::Null);
        if !exists {
            ch.push(SettingChange {
                key,
                label: format!("Branch protection «{}»", bp.branch),
                category: Category::Rules,
                current,
                desired: bp.config.clone(),
                applicable: false,
                note: Some(format!("Branch «{}» does not exist in the target", bp.branch)),
            });
        } else if current != bp.config {
            ch.push(SettingChange {
                key,
                label: format!("Branch protection «{}»", bp.branch),
                category: Category::Rules,
                current,
                desired: bp.config.clone(),
                applicable: true,
                note: None,
            });
        }
    }

    RepoDiff { repo: target.repo.clone(), changes: ch }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base(repo: &str) -> RepoSettingsSnapshot {
        RepoSettingsSnapshot {
            repo: repo.into(),
            default_branch: "main".into(),
            branches: vec!["main".into()],
            features: Features { has_wiki: true, has_issues: true, has_projects: false, has_discussions: false, allow_forking: false },
            pull_requests: PullRequestSettings::default(),
            others: OtherSettings::default(),
            rulesets: vec![],
            branch_protections: vec![],
        }
    }

    #[test]
    fn identical_snapshots_have_no_changes() {
        let d = diff_snapshots(&base("a/ref"), &base("a/t1"));
        assert!(d.changes.is_empty());
    }

    #[test]
    fn scalar_differences_detected() {
        let reference = base("a/ref");
        let mut target = base("a/t1");
        target.features.has_wiki = false;
        target.pull_requests.allow_squash_merge = true;
        let d = diff_snapshots(&reference, &target);
        assert_eq!(d.changes.len(), 2);
        let wiki = d.changes.iter().find(|c| c.key == "features.has_wiki").unwrap();
        assert_eq!(wiki.current, json!(false));
        assert_eq!(wiki.desired, json!(true));
        assert_eq!(wiki.category, Category::Features);
        assert!(d.changes.iter().any(|c| c.key == "pull_requests.allow_squash_merge" && c.category == Category::PullRequests));
    }

    #[test]
    fn default_branch_missing_in_target_is_not_applicable() {
        let mut reference = base("a/ref");
        reference.default_branch = "develop".into();
        reference.branches = vec!["main".into(), "develop".into()];
        let target = base("a/t1"); // only has main
        let d = diff_snapshots(&reference, &target);
        let c = d.changes.iter().find(|c| c.key == "default_branch").unwrap();
        assert!(!c.applicable);
        assert!(c.note.is_some());
    }

    #[test]
    fn ruleset_create_update_and_tag_category() {
        let mut reference = base("a/ref");
        reference.rulesets = vec![
            RulesetSummary { id: 1, name: "branch-rules".into(), target: "branch".into(), payload: json!({"name": "branch-rules", "rules": [{"type": "deletion"}]}) },
            RulesetSummary { id: 2, name: "tag-rules".into(), target: "tag".into(), payload: json!({"name": "tag-rules"}) },
        ];
        let mut target = base("a/t1");
        target.rulesets = vec![
            RulesetSummary { id: 9, name: "branch-rules".into(), target: "branch".into(), payload: json!({"name": "branch-rules", "rules": []}) },
        ];
        let d = diff_snapshots(&reference, &target);
        let upd = d.changes.iter().find(|c| c.key == "ruleset.branch.branch-rules").unwrap();
        assert_eq!(upd.category, Category::Rules);
        assert_eq!(upd.current, json!({"name": "branch-rules", "rules": []}));
        let create = d.changes.iter().find(|c| c.key == "ruleset.tag.tag-rules").unwrap();
        assert_eq!(create.category, Category::Tags);
        assert_eq!(create.current, serde_json::Value::Null);
    }

    #[test]
    fn target_only_ruleset_produces_no_change() {
        let reference = base("a/ref");
        let mut target = base("a/t1");
        target.rulesets = vec![
            RulesetSummary { id: 5, name: "target-only".into(), target: "branch".into(), payload: json!({}) },
        ];
        let d = diff_snapshots(&reference, &target);
        assert!(d.changes.is_empty());
    }

    #[test]
    fn branch_protection_diff_and_missing_branch() {
        let mut reference = base("a/ref");
        reference.branches = vec!["main".into(), "release".into()];
        reference.branch_protections = vec![
            BranchProtection { branch: "main".into(), config: json!({"enforce_admins": true}) },
            BranchProtection { branch: "release".into(), config: json!({"enforce_admins": false}) },
        ];
        let target = base("a/t1"); // main without protection, release does not exist
        let d = diff_snapshots(&reference, &target);
        let main = d.changes.iter().find(|c| c.key == "branch_protection.main").unwrap();
        assert!(main.applicable);
        assert_eq!(main.current, serde_json::Value::Null);
        let release = d.changes.iter().find(|c| c.key == "branch_protection.release").unwrap();
        assert!(!release.applicable);
    }
}

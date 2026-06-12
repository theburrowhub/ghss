use crate::github::GithubClient;
use crate::model::*;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum SyncAction {
    PatchRepo(Value),
    CreateRuleset(Value),
    UpdateRuleset { id: u64, payload: Value },
    PutBranchProtection { branch: String, config: Value },
}

impl SyncAction {
    pub fn describe(&self) -> String {
        match self {
            SyncAction::PatchRepo(body) => {
                let fields: Vec<&str> = body.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
                format!("Actualizar settings del repo ({})", fields.join(", "))
            }
            SyncAction::CreateRuleset(p) => format!("Crear ruleset «{}»", p["name"].as_str().unwrap_or("?")),
            SyncAction::UpdateRuleset { payload, .. } => format!("Actualizar ruleset «{}»", payload["name"].as_str().unwrap_or("?")),
            SyncAction::PutBranchProtection { branch, .. } => format!("Aplicar branch protection a «{}»", branch),
        }
    }
}

/// Convierte los cambios seleccionados en acciones de API contra el repo destino.
/// Los cambios no aplicables se ignoran (la UI ya los muestra deshabilitados).
pub fn plan_actions(changes: &[SettingChange], target: &RepoSettingsSnapshot) -> Vec<SyncAction> {
    let mut patch = Map::new();
    let mut actions = Vec::new();

    for c in changes.iter().filter(|c| c.applicable) {
        if c.key == "default_branch" {
            patch.insert("default_branch".into(), c.desired.clone());
        } else if let Some(field) = c.key.strip_prefix("features.").or_else(|| c.key.strip_prefix("pull_requests.")).or_else(|| c.key.strip_prefix("others.")) {
            patch.insert(field.into(), c.desired.clone());
        } else if c.key.starts_with("ruleset.") {
            // Resolver por payload, no parseando la clave: los nombres de ruleset pueden
            // contener puntos y la clave es solo un identificador de UI.
            let Some(name) = c.desired["name"].as_str() else { continue; };
            let rs_target = c.desired["target"].as_str().unwrap_or("branch");
            match target.rulesets.iter().find(|x| x.name == name && x.target == rs_target) {
                Some(existing) => actions.push(SyncAction::UpdateRuleset { id: existing.id, payload: c.desired.clone() }),
                None => actions.push(SyncAction::CreateRuleset(c.desired.clone())),
            }
        } else if let Some(branch) = c.key.strip_prefix("branch_protection.") {
            actions.push(SyncAction::PutBranchProtection { branch: branch.into(), config: c.desired.clone() });
        }
    }

    if !patch.is_empty() {
        actions.insert(0, SyncAction::PatchRepo(Value::Object(patch)));
    }
    actions
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub description: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Aplica acciones secuencialmente. Un fallo no detiene el resto (espíritu del spec:
/// errores por ítem, sin abortar el batch). `on_progress` se invoca antes de cada acción.
pub async fn apply_actions(
    client: &GithubClient,
    owner: &str,
    name: &str,
    actions: &[SyncAction],
    mut on_progress: impl FnMut(&str),
) -> Vec<ActionResult> {
    let mut results = Vec::new();
    for action in actions {
        let description = action.describe();
        on_progress(&description);
        let outcome = match action {
            SyncAction::PatchRepo(body) => client.update_repo(owner, name, body).await,
            SyncAction::CreateRuleset(payload) => client.create_ruleset(owner, name, payload).await,
            SyncAction::UpdateRuleset { id, payload } => client.update_ruleset(owner, name, *id, payload).await,
            SyncAction::PutBranchProtection { branch, config } => client.put_branch_protection(owner, name, branch, config).await,
        };
        results.push(match outcome {
            Ok(_) => ActionResult { description, ok: true, error: None },
            Err(e) => ActionResult { description, ok: false, error: Some(e.to_string()) },
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn change(key: &str, category: Category, desired: serde_json::Value) -> SettingChange {
        SettingChange { key: key.into(), label: key.into(), category, current: serde_json::Value::Null, desired, applicable: true, note: None }
    }

    fn target_with_ruleset() -> RepoSettingsSnapshot {
        RepoSettingsSnapshot {
            repo: "a/t1".into(),
            default_branch: "main".into(),
            branches: vec!["main".into()],
            features: Features::default(),
            pull_requests: PullRequestSettings::default(),
            others: OtherSettings::default(),
            rulesets: vec![RulesetSummary { id: 77, name: "existing".into(), target: "branch".into(), payload: json!({"name": "existing"}) }],
            branch_protections: vec![],
        }
    }

    #[test]
    fn scalars_merge_into_single_patch() {
        let changes = vec![
            change("features.has_wiki", Category::Features, json!(true)),
            change("pull_requests.allow_squash_merge", Category::PullRequests, json!(true)),
            change("default_branch", Category::DefaultBranch, json!("develop")),
            change("others.web_commit_signoff_required", Category::Others, json!(true)),
        ];
        let actions = plan_actions(&changes, &target_with_ruleset());
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            SyncAction::PatchRepo(body) => {
                assert_eq!(*body, json!({"has_wiki": true, "allow_squash_merge": true, "default_branch": "develop", "web_commit_signoff_required": true}));
            }
            other => panic!("esperaba PatchRepo, fue {:?}", other),
        }
    }

    #[test]
    fn ruleset_upsert_resolves_create_vs_update() {
        let changes = vec![
            change("ruleset.branch.existing", Category::Rules, json!({"name": "existing", "target": "branch", "rules": [{"type": "deletion"}]})),
            change("ruleset.tag.newtag", Category::Tags, json!({"name": "newtag", "target": "tag"})),
        ];
        let actions = plan_actions(&changes, &target_with_ruleset());
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], SyncAction::UpdateRuleset { id: 77, .. }));
        assert!(matches!(&actions[1], SyncAction::CreateRuleset(_)));
    }

    #[test]
    fn branch_protection_and_non_applicable_skipped() {
        let changes = vec![
            change("branch_protection.main", Category::Rules, json!({"enforce_admins": true})),
            SettingChange { key: "default_branch".into(), label: "x".into(), category: Category::DefaultBranch, current: serde_json::Value::Null, desired: json!("develop"), applicable: false, note: None },
        ];
        let actions = plan_actions(&changes, &target_with_ruleset());
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            SyncAction::PutBranchProtection { branch, config } => {
                assert_eq!(branch, "main");
                assert_eq!(*config, json!({"enforce_admins": true}));
            }
            other => panic!("esperaba PutBranchProtection, fue {:?}", other),
        }
    }

    #[test]
    fn ruleset_with_dotted_name_resolves_update_via_payload() {
        let mut target = target_with_ruleset();
        target.rulesets.push(RulesetSummary { id: 88, name: "protect.main".into(), target: "branch".into(), payload: json!({"name": "protect.main"}) });
        let changes = vec![change("ruleset.branch.protect.main", Category::Rules, json!({"name": "protect.main", "target": "branch", "rules": []}))];
        let actions = plan_actions(&changes, &target);
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], SyncAction::UpdateRuleset { id: 88, .. }));
    }
}

use serde_json::{json, Value};

/// Rule `type`s that the GET ruleset endpoint can return but that the create/update
/// (POST/PUT) endpoint does NOT accept in its `rules` schema. Sending one of these makes
/// GitHub reject the whole payload with a 422 `Invalid property /rules/N: data matches no
/// possible input`. `copilot_code_review` is a preview rule surfaced on read but not yet
/// validated on write, so we drop it from the write payload (and from the diff, since both
/// reference and target normalize the same way it never produces a spurious change).
const WRITE_UNSUPPORTED_RULE_TYPES: &[&str] = &["copilot_code_review"];

/// Strips server-side fields from a ruleset so it can be compared and re-sent in POST/PUT.
/// Also drops rule types that the write endpoint rejects (see WRITE_UNSUPPORTED_RULE_TYPES):
/// the GET response includes read-only/preview rule types that, when echoed back to
/// POST/PUT, fail validation with `/rules/N: data matches no possible input`.
pub fn normalize_ruleset(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        for k in ["id", "node_id", "source", "source_type", "created_at", "updated_at", "_links", "current_user_can_bypass"] {
            obj.remove(k);
        }
        if let Some(rules) = obj.get_mut("rules").and_then(Value::as_array_mut) {
            rules.retain(|rule| {
                rule.get("type")
                    .and_then(Value::as_str)
                    .map(|t| !WRITE_UNSUPPORTED_RULE_TYPES.contains(&t))
                    .unwrap_or(true)
            });
        }
    }
    v
}

fn logins(v: &Value, field: &str) -> Value {
    json!(v.as_array().map(|a| a.iter().filter_map(|x| x[field].as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default())
}

/// Converts the GET branch protection response to the body required by PUT.
pub fn protection_get_to_put(get: &Value) -> Value {
    let b = |path: &str| get.pointer(path).and_then(Value::as_bool).unwrap_or(false);
    let mut put = json!({
        "required_status_checks": null,
        "enforce_admins": b("/enforce_admins/enabled"),
        "required_pull_request_reviews": null,
        "restrictions": null,
        "required_linear_history": b("/required_linear_history/enabled"),
        "allow_force_pushes": b("/allow_force_pushes/enabled"),
        "allow_deletions": b("/allow_deletions/enabled"),
        "required_conversation_resolution": b("/required_conversation_resolution/enabled"),
        "lock_branch": b("/lock_branch/enabled"),
        "allow_fork_syncing": b("/allow_fork_syncing/enabled"),
    });
    if let Some(rsc) = get.get("required_status_checks").filter(|v| !v.is_null()) {
        put["required_status_checks"] = json!({
            "strict": rsc["strict"].as_bool().unwrap_or(false),
            "contexts": rsc.get("contexts").cloned().unwrap_or_else(|| json!([])),
            "checks": rsc.get("checks").cloned().unwrap_or_else(|| json!([])),
        });
    }
    if let Some(rev) = get.get("required_pull_request_reviews").filter(|v| !v.is_null()) {
        let mut r = json!({
            "dismiss_stale_reviews": rev["dismiss_stale_reviews"].as_bool().unwrap_or(false),
            "require_code_owner_reviews": rev["require_code_owner_reviews"].as_bool().unwrap_or(false),
            "required_approving_review_count": rev["required_approving_review_count"].as_u64().unwrap_or(0),
            "require_last_push_approval": rev["require_last_push_approval"].as_bool().unwrap_or(false),
        });
        if let Some(dr) = rev.get("dismissal_restrictions").filter(|v| !v.is_null()) {
            r["dismissal_restrictions"] = json!({
                "users": logins(&dr["users"], "login"),
                "teams": logins(&dr["teams"], "slug"),
                "apps": logins(&dr["apps"], "slug"),
            });
        }
        if let Some(bpa) = rev.get("bypass_pull_request_allowances").filter(|v| !v.is_null()) {
            r["bypass_pull_request_allowances"] = json!({
                "users": logins(&bpa["users"], "login"),
                "teams": logins(&bpa["teams"], "slug"),
                "apps":  logins(&bpa["apps"],  "slug"),
            });
        }
        put["required_pull_request_reviews"] = r;
    }
    if let Some(res) = get.get("restrictions").filter(|v| !v.is_null()) {
        put["restrictions"] = json!({
            "users": logins(&res["users"], "login"),
            "teams": logins(&res["teams"], "slug"),
            "apps": logins(&res["apps"], "slug"),
        });
    }
    put
}

/// Normalizes a webhook's `config` object for stable comparison and re-sending.
/// Strips the volatile/secret fields that GitHub returns but that should never be
/// part of the diff: `secret` is never exposed by GitHub (it returns a placeholder
/// or omits it), and `created_at`/`updated_at`/`last_response` are server state.
pub fn webhook_get_to_config(get: &Value) -> Value {
    let mut config = get.get("config").cloned().unwrap_or_else(|| json!({}));
    if let Some(obj) = config.as_object_mut() {
        for k in ["secret", "created_at", "updated_at", "last_response"] {
            obj.remove(k);
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_ruleset_strips_server_fields() {
        let raw = json!({
            "id": 42, "node_id": "RRS_x", "source": "acme/ref", "source_type": "Repository",
            "created_at": "2026-01-01", "updated_at": "2026-01-02",
            "_links": {"self": {"href": "..."}}, "current_user_can_bypass": "never",
            "name": "protect-main", "target": "branch", "enforcement": "active",
            "bypass_actors": [], "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [{"type": "deletion"}]
        });
        let n = normalize_ruleset(raw);
        assert_eq!(n, json!({
            "name": "protect-main", "target": "branch", "enforcement": "active",
            "bypass_actors": [], "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [{"type": "deletion"}]
        }));
    }

    #[test]
    fn normalize_ruleset_drops_write_unsupported_rule_at_index_3() {
        // Real-world payload from GET /repos/{o}/{n}/rulesets/{id}: the 4th rule (index 3)
        // is `copilot_code_review`, which the write endpoint rejects with
        // `Invalid property /rules/3: data matches no possible input`.
        let raw = json!({
            "id": 9350827,
            "node_id": "RRS_x",
            "source": "freepik-company/ai-integration-wizard",
            "source_type": "Repository",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-02T00:00:00Z",
            "_links": {"self": {"href": "https://api.github.com/..."}},
            "current_user_can_bypass": "always",
            "name": "main",
            "target": "branch",
            "enforcement": "active",
            "bypass_actors": [],
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [
                {"type": "deletion"},
                {"type": "non_fast_forward"},
                {"type": "pull_request", "parameters": {
                    "allowed_merge_methods": ["squash"],
                    "dismiss_stale_reviews_on_push": false,
                    "require_code_owner_review": false,
                    "require_last_push_approval": false,
                    "required_approving_review_count": 1,
                    "required_review_thread_resolution": true,
                    "required_reviewers": []
                }},
                {"type": "copilot_code_review"}
            ]
        });
        let n = normalize_ruleset(raw);
        // Read-only top-level fields are gone.
        for k in ["id", "node_id", "source", "source_type", "created_at", "updated_at", "_links", "current_user_can_bypass"] {
            assert!(n.get(k).is_none(), "{k} must be stripped from the write payload");
        }
        // Only the keys the write endpoint accepts remain.
        let keys: Vec<&str> = n.as_object().unwrap().keys().map(String::as_str).collect();
        let mut keys_sorted = keys.clone();
        keys_sorted.sort_unstable();
        assert_eq!(keys_sorted, vec!["bypass_actors", "conditions", "enforcement", "name", "rules", "target"]);
        // The write-unsupported rule (copilot_code_review) at index 3 is dropped; the other
        // three are preserved in order.
        let rules = n["rules"].as_array().unwrap();
        let types: Vec<&str> = rules.iter().filter_map(|r| r["type"].as_str()).collect();
        assert_eq!(types, vec!["deletion", "non_fast_forward", "pull_request"]);
        assert!(!types.contains(&"copilot_code_review"), "copilot_code_review must be removed");
    }

    #[test]
    fn normalize_ruleset_keeps_all_supported_rules() {
        let raw = json!({
            "name": "main", "target": "branch", "enforcement": "active",
            "rules": [
                {"type": "deletion"},
                {"type": "non_fast_forward"},
                {"type": "required_status_checks", "parameters": {"required_status_checks": [], "strict_required_status_checks_policy": false}},
                {"type": "pull_request", "parameters": {"required_approving_review_count": 1}}
            ]
        });
        let n = normalize_ruleset(raw.clone());
        assert_eq!(n["rules"].as_array().unwrap().len(), 4, "no supported rule should be dropped");
    }

    #[test]
    fn protection_get_to_put_full() {
        let get = json!({
            "required_status_checks": {"strict": true, "contexts": ["ci"], "checks": [{"context": "ci", "app_id": null}]},
            "enforce_admins": {"enabled": true},
            "required_pull_request_reviews": {
                "dismiss_stale_reviews": true,
                "require_code_owner_reviews": false,
                "required_approving_review_count": 2,
                "require_last_push_approval": true,
                "dismissal_restrictions": {
                    "users": [{"login": "alice"}],
                    "teams": [{"slug": "platform"}],
                    "apps": [{"slug": "ci-app"}]
                },
                "bypass_pull_request_allowances": {
                    "users": [{"login": "carol"}],
                    "teams": [{"slug": "release"}],
                    "apps": [{"slug": "merge-bot"}]
                }
            },
            "restrictions": {"users": [{"login": "bob"}], "teams": [{"slug": "core"}], "apps": [{"slug": "ci-app"}]},
            "required_linear_history": {"enabled": true},
            "allow_force_pushes": {"enabled": false},
            "allow_deletions": {"enabled": false},
            "required_conversation_resolution": {"enabled": true},
            "lock_branch": {"enabled": false},
            "allow_fork_syncing": {"enabled": false}
        });
        let put = protection_get_to_put(&get);
        assert_eq!(put["enforce_admins"], json!(true));
        assert_eq!(put["required_status_checks"], json!({"strict": true, "contexts": ["ci"], "checks": [{"context": "ci", "app_id": null}]}));
        assert_eq!(put["required_pull_request_reviews"]["required_approving_review_count"], json!(2));
        assert_eq!(put["required_pull_request_reviews"]["dismissal_restrictions"], json!({"users": ["alice"], "teams": ["platform"], "apps": ["ci-app"]}));
        assert_eq!(put["required_pull_request_reviews"]["bypass_pull_request_allowances"], json!({"users": ["carol"], "teams": ["release"], "apps": ["merge-bot"]}));
        assert_eq!(put["restrictions"], json!({"users": ["bob"], "teams": ["core"], "apps": ["ci-app"]}));
        assert_eq!(put["required_linear_history"], json!(true));
        assert_eq!(put["lock_branch"], json!(false));
    }

    #[test]
    fn webhook_get_to_config_strips_secret_and_volatile() {
        let get = json!({
            "id": 12,
            "name": "web",
            "active": true,
            "events": ["push"],
            "config": {
                "url": "https://example.com/hook",
                "content_type": "json",
                "insecure_ssl": "0",
                "secret": "********",
                "created_at": "2026-01-01",
                "updated_at": "2026-01-02",
                "last_response": {"code": 200}
            }
        });
        let config = webhook_get_to_config(&get);
        assert_eq!(config, json!({
            "url": "https://example.com/hook",
            "content_type": "json",
            "insecure_ssl": "0"
        }));
        assert!(config.get("secret").is_none(), "el secret nunca debe incluirse");
    }

    #[test]
    fn protection_get_to_put_minimal() {
        let get = json!({"enforce_admins": {"enabled": false}});
        let put = protection_get_to_put(&get);
        assert_eq!(put["required_status_checks"], json!(null));
        assert_eq!(put["required_pull_request_reviews"], json!(null));
        assert_eq!(put["restrictions"], json!(null));
        assert_eq!(put["enforce_admins"], json!(false));
        assert_eq!(put["allow_force_pushes"], json!(false));
    }
}

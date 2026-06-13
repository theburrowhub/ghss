use serde_json::{json, Value};

/// Elimina campos de servidor de un ruleset para poder compararlo y reenviarlo en POST/PUT.
pub fn normalize_ruleset(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        for k in ["id", "node_id", "source", "source_type", "created_at", "updated_at", "_links", "current_user_can_bypass"] {
            obj.remove(k);
        }
    }
    v
}

fn logins(v: &Value, field: &str) -> Value {
    json!(v.as_array().map(|a| a.iter().filter_map(|x| x[field].as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default())
}

/// Convierte la respuesta de GET branch protection al cuerpo que exige el PUT.
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

use ghss_lib::github::GithubClient;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn mount_repo_fixture(server: &MockServer) {
    Mock::given(method("GET")).and(path("/repos/acme/ref"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "full_name": "acme/ref", "default_branch": "main",
            "has_wiki": true, "has_issues": true, "has_projects": false,
            "has_discussions": false, "allow_forking": false,
            "web_commit_signoff_required": false,
            "allow_merge_commit": false, "merge_commit_title": null, "merge_commit_message": null,
            "allow_squash_merge": true, "squash_merge_commit_title": "PR_TITLE",
            "squash_merge_commit_message": "COMMIT_MESSAGES",
            "allow_rebase_merge": false, "allow_update_branch": true,
            "allow_auto_merge": false, "delete_branch_on_merge": true
        })))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"name": "main", "protected": true},
            {"name": "dev", "protected": false}
        ])))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/rulesets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": 5, "name": "protect-main", "target": "branch"}
        ])))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/rulesets/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 5, "node_id": "x", "source": "acme/ref", "source_type": "Repository",
            "name": "protect-main", "target": "branch", "enforcement": "active",
            "bypass_actors": [], "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [{"type": "deletion"}], "created_at": "2026-01-01", "updated_at": "2026-01-01"
        })))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/branches/main/protection"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "enforce_admins": {"enabled": true},
            "required_linear_history": {"enabled": true},
            "allow_force_pushes": {"enabled": false},
            "allow_deletions": {"enabled": false}
        })))
        .mount(server).await;
}

#[tokio::test]
async fn fetch_snapshot_combines_all_sources() {
    let server = MockServer::start().await;
    mount_repo_fixture(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let snap = client.fetch_snapshot("acme", "ref").await.unwrap();

    assert_eq!(snap.repo, "acme/ref");
    assert_eq!(snap.default_branch, "main");
    assert_eq!(snap.branches, vec!["main".to_string(), "dev".to_string()]);
    assert!(snap.features.has_wiki);
    assert!(snap.pull_requests.allow_squash_merge);
    assert_eq!(snap.pull_requests.squash_merge_commit_title.as_deref(), Some("PR_TITLE"));
    assert_eq!(snap.rulesets.len(), 1);
    assert_eq!(snap.rulesets[0].name, "protect-main");
    assert!(snap.rulesets[0].payload.get("id").is_none(), "el payload debe estar normalizado");
    assert_eq!(snap.branch_protections.len(), 1);
    assert_eq!(snap.branch_protections[0].config["enforce_admins"], json!(true));
}

#[tokio::test]
async fn fetch_snapshot_ruleset_only_protected_branch_skips_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/repos/acme/ref"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "full_name": "acme/ref", "default_branch": "main"
        })))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"name": "feat", "protected": true}
        ])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/rulesets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/branches/feat/protection"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message": "Branch not protected"})))
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let snap = client.fetch_snapshot("acme", "ref").await.unwrap();

    assert_eq!(snap.branches, vec!["feat".to_string()]);
    assert!(snap.branch_protections.is_empty(), "la rama protegida solo por rulesets no aporta protección clásica");
}

#[tokio::test]
async fn write_methods_hit_expected_endpoints() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH")).and(path("/repos/acme/t1")).and(body_json(json!({"has_wiki": true})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/repos/acme/t1/rulesets"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 9})))
        .expect(1)
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/repos/acme/t1/rulesets/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 9})))
        .expect(1)
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/repos/acme/t1/branches/main/protection"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    client.update_repo("acme", "t1", &json!({"has_wiki": true})).await.unwrap();
    client.create_ruleset("acme", "t1", &json!({"name": "r"})).await.unwrap();
    client.update_ruleset("acme", "t1", 9, &json!({"name": "r"})).await.unwrap();
    client.put_branch_protection("acme", "t1", "main", &json!({"enforce_admins": true})).await.unwrap();
}

#[tokio::test]
async fn fetch_snapshot_tolerates_rulesets_403_on_free_private_repo() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/repos/acme/priv"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "full_name": "acme/priv", "default_branch": "main",
            "has_wiki": true, "has_issues": true, "has_projects": false,
            "has_discussions": false, "allow_forking": false,
            "web_commit_signoff_required": false,
            "allow_merge_commit": true, "allow_squash_merge": true,
            "allow_rebase_merge": true, "allow_update_branch": false,
            "allow_auto_merge": false, "delete_branch_on_merge": false
        })))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/repos/acme/priv/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{ "name": "main", "protected": false }])))
        .mount(&server).await;
    // Rulesets bloqueados por plan: 403 "Upgrade to GitHub Pro or make this repository public".
    Mock::given(method("GET")).and(path("/repos/acme/priv/rulesets"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "message": "Upgrade to GitHub Pro or make this repository public to enable this feature.",
            "status": "403"
        })))
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let snap = client.fetch_snapshot("acme", "priv").await.unwrap();
    assert_eq!(snap.repo, "acme/priv");
    assert!(snap.rulesets.is_empty(), "los rulesets bloqueados por plan se ignoran");
    assert!(snap.features.has_wiki);
}

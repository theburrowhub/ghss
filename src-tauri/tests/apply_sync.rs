use ghss_lib::github::GithubClient;
use ghss_lib::sync::{apply_actions, SyncAction};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn applies_all_actions_and_reports_per_action_errors() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH")).and(path("/repos/acme/t1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/repos/acme/t1/branches/main/protection"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({"message": "Forbidden"})))
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let actions = vec![
        SyncAction::PatchRepo(json!({"has_wiki": true})),
        SyncAction::PutBranchProtection { branch: "main".into(), config: json!({"enforce_admins": true}) },
    ];
    let mut seen = Vec::new();
    let results = apply_actions(&client, "acme", "t1", &actions, |desc| seen.push(desc.to_string())).await;

    assert_eq!(results.len(), 2);
    assert!(results[0].ok);
    assert!(!results[1].ok);
    assert!(results[1].error.as_deref().unwrap_or_default().contains("403"));
    assert_eq!(seen.len(), 2, "debe notificar progreso por acción");
}

#[tokio::test]
async fn applies_webhook_create_and_update_actions() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/repos/acme/t1/hooks"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 7})))
        .expect(1)
        .mount(&server).await;
    Mock::given(method("PATCH")).and(path("/repos/acme/t1/hooks/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 9})))
        .expect(1)
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let actions = vec![
        SyncAction::CreateWebhook { config: json!({"name": "web", "active": true, "events": ["push"], "config": {"url": "https://a.example/hook"}}) },
        SyncAction::UpdateWebhook { id: 9, config: json!({"name": "web", "active": false, "events": ["push"], "config": {"url": "https://b.example/hook"}}) },
    ];
    let mut seen = Vec::new();
    let results = apply_actions(&client, "acme", "t1", &actions, |desc| seen.push(desc.to_string())).await;

    assert_eq!(results.len(), 2);
    assert!(results[0].ok && results[1].ok);
    assert_eq!(seen.len(), 2);
    // wiremock verifica al drop que POST y PATCH se llamaron 1 vez cada uno.
}

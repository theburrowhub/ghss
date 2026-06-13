use ghss_lib::github::GithubClient;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn get_user_sends_token_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", "Bearer tok123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"login": "jamuriano", "avatar_url": "https://a/x.png"})))
        .mount(&server)
        .await;

    let client = GithubClient::new(server.uri(), "tok123".into());
    let user = client.get_user().await.unwrap();
    assert_eq!(user["login"], "jamuriano");
}

#[tokio::test]
async fn list_repos_paginates_and_maps_admin() {
    let server = MockServer::start().await;
    let page1: Vec<serde_json::Value> = (0..100)
        .map(|i| json!({
            "full_name": format!("acme/repo{i}"), "name": format!("repo{i}"),
            "owner": {"login": "acme"}, "private": true, "default_branch": "main",
            "description": null, "permissions": {"admin": i % 2 == 0}
        }))
        .collect();
    Mock::given(method("GET")).and(path("/user/repos")).and(query_param("page", "1"))
        .and(header("authorization", "Bearer tok"))
        .and(header("x-github-api-version", "2022-11-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&page1))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/user/repos")).and(query_param("page", "2"))
        .and(header("authorization", "Bearer tok"))
        .and(header("x-github-api-version", "2022-11-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "full_name": "jamuriano/solo", "name": "solo", "owner": {"login": "jamuriano"},
            "private": false, "default_branch": "master", "description": "d",
            "permissions": {"admin": true}
        }])))
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let repos = client.list_repos().await.unwrap();
    assert_eq!(repos.len(), 101);
    assert_eq!(repos[100].full_name, "jamuriano/solo");
    assert!(repos[100].admin);
    assert!(!repos[1].admin);
}

#[tokio::test]
async fn retries_on_rate_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/user"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"login": "ok"})))
        .mount(&server).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    assert_eq!(client.get_user().await.unwrap()["login"], "ok");
}

#[tokio::test]
async fn auth_check_parses_oauth_scopes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-oauth-scopes", "repo, read:org, gist")
                .set_body_json(json!({"login": "jamuriano"})),
        )
        .mount(&server)
        .await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let (user, scopes) = client.auth_check().await.unwrap();
    assert_eq!(user["login"], "jamuriano");
    assert_eq!(scopes, vec!["repo".to_string(), "read:org".to_string(), "gist".to_string()]);
}

#[tokio::test]
async fn auth_check_no_scopes_header_is_empty() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"login": "fine-grained"})))
        .mount(&server)
        .await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let (_user, scopes) = client.auth_check().await.unwrap();
    assert!(scopes.is_empty());
}

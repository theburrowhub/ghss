use ghss_lib::github::GithubClient;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Matches a request that does NOT carry an `If-None-Match` header, i.e. an unconditional GET.
/// After `clear_caches()` the client has no ETag stored, so the next call must be unconditional.
struct NoIfNoneMatch;
impl Match for NoIfNoneMatch {
    fn matches(&self, req: &Request) -> bool {
        !req.headers.contains_key("if-none-match")
    }
}

fn repo_json(full: &str, name: &str) -> serde_json::Value {
    json!({
        "full_name": full, "name": name, "owner": {"login": "acme"},
        "private": true, "default_branch": "main", "description": null,
        "permissions": {"admin": true}
    })
}

/// `clear_caches()` must drop the ETag cache: a subsequent conditional call (force=false) sends
/// no `If-None-Match`, so it can't be served a stale 304 and instead gets a fresh 200.
#[tokio::test]
async fn clear_caches_drops_etag_cache() {
    let server = MockServer::start().await;

    // 1) Initial conditional load: 200 with ETag, ONE repo. This gets cached.
    Mock::given(method("GET"))
        .and(path("/orgs/acme/repos"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"list-v1\"")
                .set_body_json(json!([repo_json("acme/old", "old")])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let first = client.list_repos_for_owner("acme", true, false).await.unwrap();
    assert_eq!(first.len(), 1);

    // Without clearing, a second conditional call would carry If-None-Match: "list-v1" and could
    // get a stale 304. clear_caches() must make it unconditional instead, so mount ONLY an
    // unconditional mock returning the updated list — a conditional request would not match it.
    client.clear_caches();
    Mock::given(method("GET"))
        .and(path("/orgs/acme/repos"))
        .and(query_param("page", "1"))
        .and(NoIfNoneMatch)
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"list-v2\"")
                .set_body_json(json!([repo_json("acme/old", "old"), repo_json("acme/new", "new")])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let after_clear = client.list_repos_for_owner("acme", true, false).await.unwrap();
    assert_eq!(after_clear.len(), 2, "cleared cache must not resurrect the stale ETag");
    assert!(after_clear.iter().any(|r| r.full_name == "acme/new"));
}

async fn mount_ref_fixture(server: &MockServer, default_branch: &str, times: u64) {
    // `up_to_n_times` (not just `expect`) so once this fixture's budget is used up, a later
    // mount_ref_fixture call's mock is the one that actually matches — otherwise both mocks
    // stay eligible and wiremock could keep serving the older (stale) body.
    Mock::given(method("GET"))
        .and(path("/repos/acme/ref"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "full_name": "acme/ref", "default_branch": default_branch,
            "has_wiki": true, "has_issues": true, "has_projects": false,
            "has_discussions": false, "allow_forking": false, "web_commit_signoff_required": false,
            "allow_merge_commit": true, "allow_squash_merge": true, "allow_rebase_merge": true,
            "allow_update_branch": false, "allow_auto_merge": false, "delete_branch_on_merge": false
        })))
        .up_to_n_times(times)
        .expect(times)
        .mount(server)
        .await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/rulesets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/repos/acme/ref/hooks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(server).await;
}

/// `clear_caches()` must also drop the snapshot cache (TTL 60s): a repo edited outside the app
/// (or via a stale ~60s-old read) must be re-fetched immediately after a clear, not served from
/// memory until the TTL expires.
#[tokio::test]
async fn clear_caches_drops_snapshot_cache() {
    let server = MockServer::start().await;
    mount_ref_fixture(&server, "main", 1).await;

    let client = GithubClient::new(server.uri(), "tok".into());
    let a = client.fetch_snapshot("acme", "ref").await.unwrap();
    let b = client.fetch_snapshot("acme", "ref").await.unwrap(); // served from memory, no new GET
    assert_eq!(a, b);
    assert_eq!(a.default_branch, "main");

    client.clear_caches();
    // Re-mount with a different default_branch: without the clear, this second mock would never
    // be hit and the cached "main" value would keep being returned.
    mount_ref_fixture(&server, "develop", 1).await;
    let c = client.fetch_snapshot("acme", "ref").await.unwrap();
    assert_eq!(c.default_branch, "develop", "cleared snapshot cache must be re-fetched, not served stale");
}

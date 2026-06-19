use ghss_lib::github::GithubClient;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Matches a request that does NOT carry an `If-None-Match` header, i.e. an unconditional GET.
/// A forced refresh must send no ETag so GitHub returns a fresh 200 instead of a cached 304.
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

/// A repo created after the list was first cached only shows up on a forced refresh:
/// the forced call must send NO `If-None-Match` (so it can't get a stale 304) and the fresh
/// result must overwrite the cache.
#[tokio::test]
async fn forced_refresh_bypasses_etag_and_updates_cache() {
    let server = MockServer::start().await;

    // 1) First (conditional) load: 200 with ETag, ONE repo. This is what gets cached.
    Mock::given(method("GET"))
        .and(path("/orgs/acme/repos"))
        .and(query_param("page", "1"))
        .and(NoIfNoneMatch) // first call has no ETag yet either
        .and(header_exists("authorization"))
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
    assert_eq!(first[0].full_name, "acme/old");

    // 2) A *conditional* re-fetch (force=false) would carry If-None-Match and get a stale 304.
    //    The forced refresh must instead hit this unconditional mock returning the NEW repo too.
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

    let refreshed = client.list_repos_for_owner("acme", true, true).await.unwrap();
    assert_eq!(refreshed.len(), 2, "forced refresh must surface the newly created repo");
    assert!(refreshed.iter().any(|r| r.full_name == "acme/new"));

    // 3) The forced result must have overwritten the cache with the new ETag ("list-v2"):
    //    a subsequent conditional call sends If-None-Match: "list-v2" and gets a 304 served from
    //    the fresh (2-repo) cache — proving the cache wasn't left stale or broken.
    Mock::given(method("GET"))
        .and(path("/orgs/acme/repos"))
        .and(query_param("page", "1"))
        .and(header("if-none-match", "\"list-v2\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    let cached = client.list_repos_for_owner("acme", true, false).await.unwrap();
    assert_eq!(cached.len(), 2, "cache must hold the fresh 2-repo list after the forced refresh");
    assert!(cached.iter().any(|r| r.full_name == "acme/new"));
}

use ghss_lib::auth;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn device_flow_start_and_poll() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "dc1", "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "interval": 1, "expires_in": 900
        })))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"error": "authorization_pending"})))
        .up_to_n_times(1)
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token": "gho_tok", "token_type": "bearer"})))
        .mount(&server).await;

    let start = auth::device_start(&server.uri(), "client123").await.unwrap();
    assert_eq!(start.user_code, "ABCD-1234");

    let pending = auth::device_poll_once(&server.uri(), "client123", &start.device_code).await.unwrap();
    assert_eq!(pending, auth::DevicePoll::Pending);
    let done = auth::device_poll_once(&server.uri(), "client123", &start.device_code).await.unwrap();
    assert_eq!(done, auth::DevicePoll::Token("gho_tok".into()));
}

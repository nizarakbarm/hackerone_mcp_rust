//! Wire-level tests for the HackerOne client against a fake API.

use hackerone_api::{Client, Error, Secret};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::new("user", "token")
        .unwrap()
        .with_base_url(server.uri())
}

#[tokio::test]
async fn sends_exact_basic_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .and(header("authorization", "Basic dXNlcjp0b2tlbg=="))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server).get("/reports", &[], None).await.unwrap();

    assert_eq!(body, json!({ "data": [] }));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].headers["authorization"],
        "Basic dXNlcjp0b2tlbg=="
    );
}

#[tokio::test]
async fn encodes_query_params_including_repeated_keys() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [] })))
        .mount(&server)
        .await;

    let query = vec![
        ("filter[state][]".to_owned(), "new".to_owned()),
        ("filter[state][]".to_owned(), "triaged".to_owned()),
        ("sort".to_owned(), "reported_at:descending".to_owned()),
        ("q".to_owned(), "sql injection".to_owned()),
    ];
    client(&server).get("/reports", &query, None).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let url = &requests[0].url;
    assert_eq!(
        url.query_pairs().into_owned().collect::<Vec<_>>(),
        query,
        "repeated keys stay repeated and values survive percent-encoding: {url}"
    );
}

#[tokio::test]
async fn passes_json_body_through_on_post() {
    let server = MockServer::start().await;
    let payload = json!({ "data": { "type": "report", "attributes": { "title": "x" } } });
    Mock::given(method("POST"))
        .and(path("/reports"))
        .and(body_json(&payload))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "data": { "id": "1" } })))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server)
        .post("/reports", &[], Some(payload.clone()))
        .await
        .unwrap();

    assert_eq!(body, json!({ "data": { "id": "1" } }));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&requests[0].body).unwrap(),
        payload
    );
    assert_eq!(requests[0].headers["content-type"], "application/json");
}

#[tokio::test]
async fn maps_404_to_api_error_carrying_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/1"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({ "errors": [{ "detail": "not found" }] })),
        )
        .mount(&server)
        .await;

    let error = client(&server)
        .get("/reports/1", &[], None)
        .await
        .unwrap_err();

    match error {
        Error::Api {
            status,
            detail,
            body,
        } => {
            assert_eq!(status, 404);
            assert!(detail.contains("not found"), "detail: {detail}");
            assert!(body.contains("\"errors\""), "body: {body}");
        }
        other => panic!("expected Api, got {other:?}"),
    }
}

#[tokio::test]
async fn maps_500_to_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let error = client(&server)
        .get("/reports", &[], None)
        .await
        .unwrap_err();

    assert!(
        matches!(error, Error::Api { status: 500, .. }),
        "got {error:?}"
    );
}

#[tokio::test]
async fn maps_non_json_200_body_to_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>gateway</html>"))
        .mount(&server)
        .await;

    let error = client(&server)
        .get("/reports", &[], None)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Decode(_)), "got {error:?}");
}

#[tokio::test]
async fn empty_2xx_body_decodes_to_null() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/reports/1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let body = client(&server)
        .delete("/reports/1", &[], None)
        .await
        .unwrap();

    assert_eq!(body, serde_json::Value::Null);
}

#[tokio::test]
async fn connection_refused_is_transport_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let client = Client::new("user", "token")
        .unwrap()
        .with_base_url(format!("http://{addr}"));

    let error = client.get("/reports", &[], None).await.unwrap_err();

    assert!(matches!(error, Error::Transport(_)), "got {error:?}");
}

#[tokio::test]
async fn request_that_outlives_the_timeout_is_a_transport_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(1)))
        .mount(&server)
        .await;

    let client = Client::new("user", "token")
        .unwrap()
        .with_base_url(server.uri())
        .with_request_timeout(std::time::Duration::from_millis(50))
        .unwrap();

    let error = client.get("/slow", &[], None).await.unwrap_err();

    assert!(matches!(error, Error::Transport(_)), "got {error:?}");
}

#[tokio::test]
async fn path_without_leading_slash_is_rejected_before_sending() {
    let server = MockServer::start().await;
    let client = client(&server);

    let error = client.get("reports/1", &[], None).await.unwrap_err();

    match error {
        Error::Validation { field, .. } => assert_eq!(field, "path"),
        other => panic!("expected a validation error, got {other:?}"),
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn parent_traversal_path_is_rejected_before_sending() {
    let server = MockServer::start().await;
    let client = client(&server);

    for path in ["../admin", "/reports/../admin", "..", "/.."] {
        let error = client.get(path, &[], None).await.unwrap_err();
        match error {
            Error::Validation { field, .. } => assert_eq!(field, "path", "path {path}"),
            other => panic!("expected a validation error for {path}, got {other:?}"),
        }
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn absolute_path_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "1"}})))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server).get("/reports/1", &[], None).await.unwrap();

    assert_eq!(body, json!({"data": {"id": "1"}}));
}

/// Token-shaped fixture: distinctive, so a leak in a redaction test is obvious.
const SECRET_TOKEN: &str = "tok_live_9f3";

fn assert_redacted(printed: &str, what: &str) {
    assert!(
        printed.contains("<redacted>"),
        "{what} lost the marker: {printed}"
    );
    assert!(
        !printed.contains(SECRET_TOKEN),
        "{what} leaked the value: {printed}"
    );
    assert!(
        !printed.contains(&SECRET_TOKEN.len().to_string()),
        "{what} leaked the length: {printed}"
    );
}

#[test]
fn secret_debug_prints_only_a_marker() {
    let secret = Secret::from(SECRET_TOKEN);
    assert_redacted(&format!("{secret:?}"), "Secret");
}

#[test]
fn client_debug_hides_the_credential_pair() {
    let client = Client::new(SECRET_TOKEN, SECRET_TOKEN).expect("client");
    assert_redacted(&format!("{client:?}"), "Client");
}

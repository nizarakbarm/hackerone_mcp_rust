//! Credential classes (ADR-0008): each tool spends its own token pair, a
//! missing pair fails the call — never startup — and no request ever carries
//! the other class's token.

use hackerone_api::{CredentialClass, Error};
use hackerone_mcp::{
    Clients,
    config::Config,
    tool::{self, dispatch, scope::ProgramScope},
};
use serde_json::{Map, Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// HTTP Basic for `customer-user:customer-token`.
const CUSTOMER_AUTH: &str = "Basic Y3VzdG9tZXItdXNlcjpjdXN0b21lci10b2tlbg==";
/// HTTP Basic for `hacker-user:hacker-token`.
const HACKER_AUTH: &str = "Basic aGFja2VyLXVzZXI6aGFja2VyLXRva2Vu";

fn config(server: &MockServer, customer: bool, hacker: bool) -> Config {
    Config {
        api_username: customer.then(|| "customer-user".to_owned()),
        api_token: customer.then(|| "customer-token".to_owned()),
        hacker_api_username: hacker.then(|| "hacker-user".to_owned()),
        hacker_api_token: hacker.then(|| "hacker-token".to_owned()),
        base_url: Some(server.uri()),
        ..Config::default()
    }
}

fn clients(server: &MockServer, customer: bool, hacker: bool) -> Clients {
    Clients::from_config(&config(server, customer, hacker)).expect("clients")
}

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected an object, got {other}"),
    }
}

/// The `Authorization` header of every request the fake API received, in order.
async fn authorizations(server: &MockServer) -> Vec<Option<String>> {
    server
        .received_requests()
        .await
        .expect("recorded requests")
        .iter()
        .map(|request| {
            request
                .headers
                .get("authorization")
                .map(|value| value.to_str().expect("ascii header").to_owned())
        })
        .collect()
}

async fn mount_me_reports(server: &MockServer, times: u64) {
    Mock::given(method("GET"))
        .and(path("/me/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(times)
        .mount(server)
        .await;
}

#[tokio::test]
async fn customer_tool_sends_only_the_customer_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "7"}})))
        .expect(1)
        .mount(&server)
        .await;

    let clients = clients(&server, true, true);
    dispatch(
        &clients,
        &ProgramScope::new(None),
        "get_report",
        &args(json!({"report_id": 7})),
    )
    .await
    .expect("customer credentials are enough for a customer tool");

    assert_eq!(
        authorizations(&server).await,
        vec![Some(CUSTOMER_AUTH.to_owned())]
    );
}

#[tokio::test]
async fn hacker_class_client_sends_only_the_hacker_token() {
    let server = MockServer::start().await;
    mount_me_reports(&server, 2).await;

    let clients = clients(&server, true, true);
    let hacker = clients
        .for_class(CredentialClass::Hacker)
        .expect("hacker client");
    let customer = clients
        .for_class(CredentialClass::Customer)
        .expect("customer client");
    hacker
        .get("/me/reports", &[], None)
        .await
        .expect("hacker call");
    customer
        .get("/me/reports", &[], None)
        .await
        .expect("customer call");

    assert_eq!(
        authorizations(&server).await,
        vec![Some(HACKER_AUTH.to_owned()), Some(CUSTOMER_AUTH.to_owned()),],
        "each class spends its own token, never the other's"
    );
}

#[tokio::test]
async fn customer_tools_without_customer_credentials_fail_before_any_request() {
    let server = MockServer::start().await;
    // Hacker credentials only: the class clients exist, but not for `customer`.
    let clients = clients(&server, false, true);
    assert!(clients.for_class(CredentialClass::Customer).is_none());
    assert!(clients.for_class(CredentialClass::Hacker).is_some());

    for name in tool::WIRED
        .iter()
        .filter(|(_, _, class)| *class == CredentialClass::Customer)
        .map(|(tool, _, _)| tool.name())
    {
        let error = dispatch(&clients, &ProgramScope::new(None), name, &args(json!({})))
            .await
            .expect_err("a customer tool cannot run without customer credentials");
        let Error::MissingCredential { class } = error else {
            panic!("expected MissingCredential for {name}, got {error:?}");
        };
        assert_eq!(class, CredentialClass::Customer, "{name}");
    }

    let message = dispatch(
        &clients,
        &ProgramScope::new(None),
        "get_report",
        &args(json!({"report_id": 7})),
    )
    .await
    .expect_err("still missing")
    .to_string();
    assert!(message.contains("missing customer credential"), "{message}");
    assert!(message.contains("HACKERONE_API_USERNAME"), "{message}");
    assert!(message.contains("HACKERONE_API_TOKEN"), "{message}");
    assert!(
        authorizations(&server).await.is_empty(),
        "no request may leave the process without credentials"
    );
}

#[tokio::test]
async fn no_credentials_build_no_clients_without_failing() {
    let server = MockServer::start().await;
    let clients = clients(&server, false, false);
    for class in [CredentialClass::Customer, CredentialClass::Hacker] {
        assert!(clients.for_class(class).is_none(), "{class:?}");
    }
    // Half a pair is not a pair.
    let half = Clients::from_config(&Config {
        api_username: Some("customer-user".to_owned()),
        ..Config::default()
    })
    .expect("clients");
    assert!(half.for_class(CredentialClass::Customer).is_none());
}

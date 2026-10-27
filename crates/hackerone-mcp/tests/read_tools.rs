//! Read-tool bodies and the router against a fake HackerOne API: exact
//! method/path/query, body passthrough, upstream normalization, typed failures.

use hackerone_api::{Client, Error};
use hackerone_mcp::{
    Clients,
    config::Config,
    tool::{self, ToolName, dispatch, scope::ProgramScope},
};
use serde_json::{Map, Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn client(server: &MockServer) -> Client {
    Client::new("user", "token")
        .expect("client")
        .with_base_url(server.uri())
}

/// Class clients as the server builds them: customer credentials only.
fn clients(server: &MockServer) -> Clients {
    Clients::from_config(&Config {
        api_username: Some("user".to_owned()),
        api_token: Some("token".to_owned()),
        base_url: Some(server.uri()),
        ..Config::default()
    })
    .expect("clients")
}

/// One configured program, as the server shell holds it.
fn scope(handle: Option<&str>) -> ProgramScope {
    ProgramScope::new(handle.map(str::to_owned))
}

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected an object, got {other}"),
    }
}

async fn received(server: &MockServer) -> Vec<wiremock::Request> {
    server.received_requests().await.expect("recorded requests")
}

fn query_pairs(request: &wiremock::Request) -> Vec<(String, String)> {
    request
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

fn expected(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn decode_message(error: Error) -> String {
    match error {
        Error::Decode(source) => source.to_string(),
        other => panic!("expected a decode error, got {other:?}"),
    }
}

const REPORT: &str = r#"{
  "data": {
    "type": "report",
    "id": "7",
    "attributes": {"title": "SQL injection", "state": "new"}
  }
}"#;

#[tokio::test]
async fn get_report_returns_the_raw_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/7"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(REPORT, "application/json"))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &scope(None),
        "get_report",
        &args(json!({"report_id": 7})),
    )
    .await
    .expect("get_report");

    assert_eq!(value, serde_json::from_str::<Value>(REPORT).expect("json"));
}

#[tokio::test]
async fn get_report_rejects_a_missing_or_non_integer_id() {
    let server = MockServer::start().await;
    for arguments in [json!({}), json!({"report_id": "7"})] {
        let error = dispatch(
            &clients(&server),
            &scope(None),
            "get_report",
            &args(arguments),
        )
        .await
        .expect_err("invalid report_id");
        assert!(matches!(error, Error::Validation { ref field, .. } if field == "report_id"));
    }
    assert!(received(&server).await.is_empty(), "no request may be sent");
}

#[tokio::test]
async fn custom_field_values_are_normalized_like_upstream() {
    let server = MockServer::start().await;
    let body = json!({
        "data": {
            "id": "7",
            "relationships": {
                "custom_field_values": {
                    "data": [
                        {
                            "id": "11",
                            "attributes": {"value": "high", "created_at": "c1", "updated_at": "u1"},
                            "relationships": {
                                "custom_field_attribute": {
                                    "data": {
                                        "id": "5",
                                        "attributes": {
                                            "label": "Severity",
                                            "field_type": "select",
                                            "internal": true,
                                            "required": false,
                                            "helper_text": "pick one",
                                            "configuration": {"options": ["low", "high"]},
                                            "checkbox_text": "check",
                                            "regex": "[a-z]+",
                                            "error_message": "bad",
                                            "created_at": "c2",
                                            "updated_at": "u2",
                                            "archived_at": null
                                        }
                                    }
                                }
                            }
                        },
                        {
                            "id": "12",
                            "attributes": {"value": ""}
                        }
                    ]
                }
            }
        }
    });
    Mock::given(method("GET"))
        .and(path("/reports/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &scope(None),
        "get_report_custom_field_values",
        &args(json!({"report_id": 7})),
    )
    .await
    .expect("custom field values");

    let expected = json!({
        "report_id": 7,
        "custom_fields": [
            {
                "value_id": "11",
                "value": "high",
                "value_created_at": "c1",
                "value_updated_at": "u1",
                "attribute_id": "5",
                "attribute_label": "Severity",
                "field_type": "select",
                "internal": true,
                "required": false,
                "helper_text": "pick one",
                "configuration": {"options": ["low", "high"]},
                "checkbox_text": "check",
                "regex": "[a-z]+",
                "error_message": "bad",
                "attribute_created_at": "c2",
                "attribute_updated_at": "u2",
                "attribute_archived_at": null
            },
            {
                "value_id": "12",
                "value": "",
                "value_created_at": null,
                "value_updated_at": null,
                "attribute_id": null,
                "attribute_label": null,
                "field_type": null,
                "internal": null,
                "required": null,
                "helper_text": null,
                "configuration": null,
                "checkbox_text": null,
                "regex": null,
                "error_message": null,
                "attribute_created_at": null,
                "attribute_updated_at": null,
                "attribute_archived_at": null
            }
        ],
        "values_by_attribute_id": {"5": "high"},
        "values_by_label": {"Severity": "high"}
    });
    assert_eq!(value, expected);
}

#[tokio::test]
async fn malformed_report_payloads_are_decode_errors_naming_the_path() {
    for (body, expected_path) in [
        (json!({"meta": {}}), "data"),
        (json!({"data": []}), "data"),
        (
            json!({"data": {"relationships": {"custom_field_values": []}}}),
            "data.relationships.custom_field_values",
        ),
        (
            json!({"data": {"relationships": {"custom_field_values": {"data": {}}}}}),
            "data.relationships.custom_field_values.data",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/reports/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let error = dispatch(
            &clients(&server),
            &scope(None),
            "get_report_custom_field_values",
            &args(json!({"report_id": 7})),
        )
        .await
        .expect_err("malformed payload");
        let message = decode_message(error);
        assert!(
            message.contains(expected_path),
            "error must name path {expected_path}: {message}"
        );
    }
}

#[tokio::test]
async fn query_reports_sends_the_builder_query_for_the_configured_program() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &scope(Some("acme")),
        "query_reports",
        &args(json!({
            "states": ["new", "triaged"],
            "report_ids": [3],
            "created_after": "2024-01-01T00:00:00Z",
            "sort_by": "created_at",
            "sort_order": "desc",
            "page_number": 2,
            "page_size": 50,
            // Hostile overrides must be ignored: the handle is server-side only
            // (the schema guard is `no_schema_exposes_a_program_override`, and
            // `create_report_posts_the_program_scoped_body` covers the POST side).
            "program": "evil",
            "programs": ["evil"],
            "program_handle": "evil",
            "team_handle": "evil",
            "filter[program][]": "evil"
        })),
    )
    .await
    .expect("query_reports");
    assert_eq!(value, json!({"data": []}));

    let requests = received(&server).await;
    assert_eq!(
        query_pairs(&requests[0]),
        expected(&[
            ("filter[program][]", "acme"),
            ("filter[state][]", "new"),
            ("filter[state][]", "triaged"),
            ("filter[id][]", "3"),
            ("filter[created_at__gt]", "2024-01-01T00:00:00Z"),
            ("page[number]", "2"),
            ("page[size]", "50"),
            ("sort", "-reports.created_at"),
        ])
    );
}

#[tokio::test]
async fn query_reports_without_a_handle_fails_before_any_request() {
    let server = MockServer::start().await;
    let error = dispatch(
        &clients(&server),
        &scope(None),
        "query_reports",
        &args(json!({})),
    )
    .await
    .expect_err("missing program handle");
    assert!(matches!(error, Error::MissingProgramHandle));
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn list_scopes_follows_links_next_and_then_lists_scopes() {
    let server = MockServer::start().await;
    // Page 2's link is absolute and on another host: only its path may be used,
    // and it deliberately is not page 2 of this server's numbering.
    let next = format!("{}/me/programs?page[number]=7&page[size]=100", server.uri());
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .and(query_param("page[number]", "1"))
        .and(query_param("page[size]", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "1", "attributes": {"handle": "other"}}],
            "links": {"next": next}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .and(query_param("page[number]", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "4242", "attributes": {"handle": "acme"}}],
            "links": {"next": null}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/programs/4242/structured_scopes"))
        .and(query_param("filter[id__gt]", "10"))
        .and(query_param("page[size]", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &scope(Some("acme")),
        "list_scopes",
        &args(json!({"scope_id_gt": 10, "page_size": 5})),
    )
    .await
    .expect("list_scopes");
    assert_eq!(value, json!({"data": []}));

    let requests = received(&server).await;
    assert_eq!(requests.len(), 3, "page 1, next page, scopes");
    assert_eq!(
        query_pairs(&requests[1]),
        expected(&[("page[number]", "7"), ("page[size]", "100"),])
    );
}

#[tokio::test]
async fn one_scope_resolves_its_program_id_once() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "4242", "attributes": {"handle": "cached-handle"}}],
            "links": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let scope = ProgramScope::new(Some("cached-handle".to_owned()));
    for _ in 0..2 {
        assert_eq!(
            scope.id(&client(&server)).await.expect("program id"),
            "4242"
        );
    }
}

#[tokio::test]
async fn separate_scopes_do_not_share_a_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "4242", "attributes": {"handle": "shared-handle"}}],
            "links": {}
        })))
        .expect(2)
        .mount(&server)
        .await;

    for _ in 0..2 {
        let scope = ProgramScope::new(Some("shared-handle".to_owned()));
        assert_eq!(
            scope.id(&client(&server)).await.expect("program id"),
            "4242"
        );
    }
}

/// The scope the server holds outlives a `tools/call`, so a second
/// `list_scopes` reuses the id instead of re-reading `/me/programs`.
#[tokio::test]
async fn one_scope_caches_the_program_id_across_tool_calls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "4242", "attributes": {"handle": "acme"}}],
            "links": {}
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/programs/4242/structured_scopes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(2)
        .mount(&server)
        .await;

    let clients = clients(&server);
    let program = scope(Some("acme"));
    for _ in 0..2 {
        dispatch(&clients, &program, "list_scopes", &args(json!({})))
            .await
            .expect("list_scopes");
    }
}

#[tokio::test]
async fn missing_handle_is_reported_before_the_scope_request() {
    let server = MockServer::start().await;
    let error = dispatch(
        &clients(&server),
        &scope(None),
        "list_scopes",
        &args(json!({})),
    )
    .await
    .expect_err("missing program handle");
    assert!(matches!(error, Error::MissingProgramHandle));
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn unknown_handle_ends_in_program_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/programs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "1", "attributes": {"handle": "someone-else"}}],
            "links": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        &clients(&server),
        &scope(Some("nobody")),
        "list_scopes",
        &args(json!({})),
    )
    .await
    .expect_err("program not found");
    assert!(matches!(error, Error::ProgramNotFound { ref handle } if handle == "nobody"));
}

#[tokio::test]
async fn router_rejects_unknown_tool_names_without_a_request() {
    let server = MockServer::start().await;
    let error = dispatch(
        &clients(&server),
        &scope(None),
        "totally_unknown",
        &args(json!({})),
    )
    .await
    .expect_err("unknown tool must not be dispatchable");
    assert!(
        matches!(error, Error::Validation { ref field, .. } if field == "name"),
        "{error:?}"
    );
    assert!(received(&server).await.is_empty());
}

#[test]
fn wired_tools_are_all_advertised_in_registry_order() {
    let names: Vec<_> = tool::WIRED.iter().map(|(tool, _)| tool.name()).collect();
    assert_eq!(
        names,
        ToolName::ALL
            .iter()
            .map(|tool| tool.name())
            .collect::<Vec<_>>()
    );
}

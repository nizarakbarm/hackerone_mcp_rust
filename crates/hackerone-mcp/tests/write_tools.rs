//! Write-tool bodies and their wiring against a fake HackerOne API: exact
//! method, path, JSON:API `data.type` and attribute map, plus the typed
//! failures that must precede any request.

use hackerone_api::{Client, Error};
use hackerone_mcp::{config::Config, tool::dispatch};
use serde_json::{Map, Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

fn client(server: &MockServer) -> Client {
    Client::new("user", "token")
        .expect("client")
        .with_base_url(server.uri())
}

fn config(handle: Option<&str>) -> Config {
    Config {
        program_handle: handle.map(str::to_owned),
        ..Config::default()
    }
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

fn request_body(request: &wiremock::Request) -> Value {
    serde_json::from_slice(&request.body).expect("request body is JSON")
}

fn validation_field(error: Error) -> String {
    match error {
        Error::Validation { field, .. } => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

const REPORT: &str = r#"{"data": {"type": "report", "id": "9", "attributes": {"title": "t"}}}"#;

#[tokio::test]
async fn create_report_posts_the_program_scoped_body() {
    let server = MockServer::start().await;
    let body = json!({
        "data": {
            "type": "report",
            "attributes": {
                "title": "SQL injection",
                "vulnerability_information": "payload",
                "impact": "full read",
                "source": "api",
                "team_handle": "acme",
            },
        },
    });
    Mock::given(method("POST"))
        .and(path("/reports"))
        .and(body_json(body))
        .respond_with(ResponseTemplate::new(201).set_body_raw(REPORT, "application/json"))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(Some("acme")),
        "create_report",
        &args(json!({
            "title": "SQL injection",
            "vulnerability_information": "payload",
            "impact": "full read",
            "team_handle": "evil",
            "program_handle": "evil",
        })),
    )
    .await
    .expect("create_report");

    assert_eq!(value, serde_json::from_str::<Value>(REPORT).expect("json"));
    assert_eq!(
        received(&server).await.len(),
        1,
        "exactly one POST /reports"
    );
}

#[tokio::test]
async fn create_report_without_a_handle_fails_before_any_request() {
    let server = MockServer::start().await;
    let error = dispatch(
        &client(&server),
        &config(None),
        "create_report",
        &args(json!({
            "title": "t",
            "vulnerability_information": "v",
            "impact": "i",
        })),
    )
    .await
    .expect_err("missing program handle");
    assert!(matches!(error, Error::MissingProgramHandle));
    assert!(received(&server).await.is_empty(), "no request may be sent");
}

#[tokio::test]
async fn change_state_posts_the_state_change_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/reports/7/state_changes"))
        .and(body_json(json!({
            "data": {
                "type": "state-change",
                "attributes": {
                    "state": "duplicate",
                    "message": "dup of 1",
                    "original_report_id": 1,
                    "attachment_ids": [4, 5],
                },
            },
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "7"}})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(None),
        "change_state",
        &args(json!({
            "report_id": 7,
            "state": "duplicate",
            "message": "dup of 1",
            "original_report_id": 1,
            "attachment_ids": [4, 5],
        })),
    )
    .await
    .expect("change_state");
    assert_eq!(value, json!({"data": {"id": "7"}}));
}

#[tokio::test]
async fn change_state_rejects_an_unknown_state_before_any_request() {
    let server = MockServer::start().await;
    let error = dispatch(
        &client(&server),
        &config(None),
        "change_state",
        &args(json!({"report_id": 7, "state": "archived"})),
    )
    .await
    .expect_err("unknown state");
    assert_eq!(validation_field(error), "state");
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn create_comment_posts_an_activity_comment_and_omits_false_internal() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/reports/7/activities"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "11"}})))
        .expect(3)
        .mount(&server)
        .await;

    for (arguments, expected_attributes) in [
        (
            json!({"report_id": 7, "message": "hello", "internal": false}),
            json!({"message": "hello"}),
        ),
        (
            json!({"report_id": 7, "message": "plain"}),
            json!({"message": "plain"}),
        ),
        (
            json!({"report_id": 7, "message": "team only", "internal": true}),
            json!({"message": "team only", "internal": true}),
        ),
    ] {
        dispatch(
            &client(&server),
            &config(None),
            "create_comment",
            &args(arguments),
        )
        .await
        .expect("create_comment");
        let requests = received(&server).await;
        let body = request_body(requests.last().expect("request"));
        assert_eq!(body["data"]["type"], "activity-comment", "body: {body}");
        assert_eq!(body["data"]["attributes"], expected_attributes);
    }
}

#[tokio::test]
async fn update_title_patches_only_the_title() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/reports/7"))
        .and(body_json(json!({
            "data": {"type": "report", "attributes": {"title": "New title"}},
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "7"}})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(None),
        "update_title",
        &args(json!({"report_id": 7, "title": "New title", "state": "triaged"})),
    )
    .await
    .expect("update_title");
    assert_eq!(value, json!({"data": {"id": "7"}}));
    let requests = received(&server).await;
    assert_eq!(
        request_body(&requests[0]),
        json!({"data": {"type": "report", "attributes": {"title": "New title"}}})
    );
}

#[tokio::test]
async fn write_tools_reject_wrongly_typed_arguments_before_any_request() {
    let server = MockServer::start().await;
    for (name, arguments, field) in [
        (
            "create_report",
            json!({"title": 5, "vulnerability_information": "v", "impact": "i"}),
            "title",
        ),
        (
            "change_state",
            json!({"report_id": 7, "state": ["triaged"]}),
            "state",
        ),
        (
            "create_comment",
            json!({"report_id": {"id": 7}, "message": "m"}),
            "report_id",
        ),
        (
            "create_comment",
            json!({"report_id": 7, "message": "m", "internal": "yes"}),
            "internal",
        ),
        ("update_title", json!({"report_id": 7}), "title"),
        ("update_reference", json!({"report_id": 7}), "reference"),
        (
            "update_cves",
            json!({"report_id": 7, "cve_ids": [1]}),
            "cve_ids",
        ),
        ("escalate_report", json!({"report_id": 7}), "integration_id"),
        ("remove_escalation", json!({}), "report_id"),
        (
            "update_custom_field_values",
            json!({"report_id": 7, "custom_fields": {"1": 42}}),
            "custom_fields",
        ),
    ] {
        let error = dispatch(
            &client(&server),
            &config(Some("acme")),
            name,
            &args(arguments),
        )
        .await
        .expect_err("arguments must be rejected");
        assert_eq!(validation_field(error), field, "{name}");
    }
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn update_reference_posts_the_reference_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/reports/7/issue_tracker_reference_id"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "r1"}})))
        .expect(2)
        .mount(&server)
        .await;

    for (arguments, attributes) in [
        (
            json!({"report_id": 7, "reference": "JIRA-1"}),
            json!({"reference": "JIRA-1"}),
        ),
        (
            json!({"report_id": 7, "reference": "JIRA-2", "message": "see ticket"}),
            json!({"reference": "JIRA-2", "message": "see ticket"}),
        ),
    ] {
        let value = dispatch(
            &client(&server),
            &config(None),
            "update_reference",
            &args(arguments),
        )
        .await
        .expect("update_reference");
        assert_eq!(value, json!({"data": {"id": "r1"}}));

        let requests = received(&server).await;
        let body = request_body(requests.last().expect("request"));
        assert_eq!(
            body,
            json!({"data": {"type": "issue-tracker-reference-id", "attributes": attributes}}),
            "body: {body}"
        );
    }
}

#[tokio::test]
async fn update_cves_puts_the_ids_in_caller_order() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/reports/7/cves"))
        .and(body_json(json!({
            "data": {
                "type": "report-cves",
                "attributes": {"cve_ids": ["CVE-2024-2", "CVE-2024-1"]},
            },
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "7"}})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(None),
        "update_cves",
        &args(json!({"report_id": 7, "cve_ids": ["CVE-2024-2", "CVE-2024-1"]})),
    )
    .await
    .expect("update_cves");
    assert_eq!(value, json!({"data": {"id": "7"}}));
}

#[tokio::test]
async fn escalate_report_posts_an_untyped_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/reports/7/escalate"))
        .and(body_json(
            json!({"data": {"attributes": {"integration_id": "42"}}}),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "e1"}})))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(None),
        "escalate_report",
        &args(json!({"report_id": 7, "integration_id": "42"})),
    )
    .await
    .expect("escalate_report");
    assert_eq!(value, json!({"data": {"id": "e1"}}));
}

#[tokio::test]
async fn remove_escalation_deletes_without_a_body() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/reports/7/escalate"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &client(&server),
        &config(None),
        "remove_escalation",
        &args(json!({"report_id": 7})),
    )
    .await
    .expect("remove_escalation");
    assert_eq!(value, Value::Null);

    let requests = received(&server).await;
    assert!(requests[0].body.is_empty(), "DELETE must not send a body");
}

#[tokio::test]
async fn custom_field_values_post_each_attribute_in_key_order() {
    let server = MockServer::start().await;
    for (attribute_id, value) in [(2, "two"), (10, "ten")] {
        Mock::given(method("POST"))
            .and(path("/reports/7/custom_field_values"))
            .and(body_json(json!({
                "data": {
                    "attributes": {
                        "custom_field_attribute_id": attribute_id,
                        "value": value,
                    },
                },
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "cf"}})))
            .expect(1)
            .mount(&server)
            .await;
    }

    let value = dispatch(
        &client(&server),
        &config(None),
        "update_custom_field_values",
        &args(json!({"report_id": 7, "custom_fields": {"10": "ten", "2": "two"}})),
    )
    .await
    .expect("update_custom_field_values");
    assert_eq!(
        value,
        json!({"results": [
            {"attribute_id": 2, "ok": true, "error": null},
            {"attribute_id": 10, "ok": true, "error": null},
        ]})
    );

    let requests = received(&server).await;
    let ids: Vec<Value> = requests
        .iter()
        .map(|request| {
            request_body(request)["data"]["attributes"]["custom_field_attribute_id"].clone()
        })
        .collect();
    assert_eq!(ids, vec![json!(2), json!(10)], "ascending attribute ids");
}

#[tokio::test]
async fn custom_field_values_report_a_partial_failure_and_keep_going() {
    let server = MockServer::start().await;
    for (attribute_id, value, status) in [(2, "two", 201), (5, "five", 500), (9, "nine", 201)] {
        let response = if status == 201 {
            ResponseTemplate::new(201).set_body_json(json!({"data": {"id": "cf"}}))
        } else {
            ResponseTemplate::new(500).set_body_string("boom")
        };
        Mock::given(method("POST"))
            .and(path("/reports/7/custom_field_values"))
            .and(body_json(json!({
                "data": {
                    "attributes": {
                        "custom_field_attribute_id": attribute_id,
                        "value": value,
                    },
                },
            })))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
    }

    let value = dispatch(
        &client(&server),
        &config(None),
        "update_custom_field_values",
        &args(json!({
            "report_id": 7,
            "custom_fields": {"5": "five", "9": "nine", "2": "two"},
        })),
    )
    .await
    .expect("all writes attempted");
    let results = value["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3, "every key is attempted: {value}");
    assert_eq!(
        results[0],
        json!({"attribute_id": 2, "ok": true, "error": null})
    );
    assert_eq!(results[1]["attribute_id"], 5);
    assert_eq!(results[1]["ok"], false);
    assert!(
        results[1]["error"]
            .as_str()
            .is_some_and(|text| text.contains("hackerone api error 500") && text.contains("boom")),
        "error: {}",
        results[1]["error"]
    );
    assert_eq!(
        results[2],
        json!({"attribute_id": 9, "ok": true, "error": null})
    );
    assert_eq!(received(&server).await.len(), 3);
}

#[tokio::test]
async fn custom_field_values_edge_cases_never_send_a_request() {
    let server = MockServer::start().await;

    let empty = dispatch(
        &client(&server),
        &config(None),
        "update_custom_field_values",
        &args(json!({"report_id": 7, "custom_fields": {}})),
    )
    .await
    .expect("empty custom fields are a no-op");
    assert_eq!(empty, json!({"results": []}));

    let error = dispatch(
        &client(&server),
        &config(None),
        "update_custom_field_values",
        &args(json!({"report_id": 7, "custom_fields": {"not-a-number": "x"}})),
    )
    .await
    .expect_err("non-numeric attribute id");
    assert_eq!(validation_field(error), "custom_fields");

    assert!(received(&server).await.is_empty(), "no request may be sent");
}

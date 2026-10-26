//! Black-box tests for the `query_reports` live harness: fake HackerOne API,
//! real `hackerone-mcp` child, real JSON-RPC handshake.

use std::process::{Command, Output};

use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

const HARNESS: &str = env!("CARGO_BIN_EXE_hackerone-mcp-query-reports-harness");

fn run_harness(base_url: &str, arguments: &[&str]) -> Output {
    Command::new(HARNESS)
        .env_clear()
        .env("HACKERONE_API_USERNAME", "user")
        .env("HACKERONE_API_TOKEN", "token")
        .env("HACKERONE_PROGRAM_HANDLE", "acme")
        .env("HACKERONE_API_BASE_URL", base_url)
        .args(arguments)
        .output()
        .expect("run query_reports harness")
}

fn reports(count: usize) -> Value {
    let data: Vec<Value> = (1..=count)
        .map(|id| {
            json!({
                "id": id.to_string(),
                "attributes": {
                    "title": "SQL injection",
                    "state": "triaged",
                    "created_at": "2026-01-01T00:00:00Z",
                },
                "relationships": {"severity": {"data": {"attributes": {"rating": "high"}}}},
            })
        })
        .collect();
    json!({"data": data, "links": {"next": "https://example.test/next"}})
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[tokio::test]
async fn summary_run_succeeds_when_min_results_are_met() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .and(query_param("filter[program][]", "acme"))
        .and(query_param("filter[state][]", "triaged"))
        .and(query_param("page[size]", "5"))
        .and(query_param("sort", "-reports.created_at"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reports(2)))
        .expect(1)
        .mount(&api)
        .await;

    let output = run_harness(&api.uri(), &["--state", "triaged", "--min-results", "2"]);
    assert!(
        output.status.success(),
        "harness failed: {}",
        stderr(&output)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Program: acme"), "{stdout}");
    assert!(stdout.contains("\"states\""), "{stdout}");
    assert!(stdout.contains("Returned 2 reports."), "{stdout}");
    assert!(stdout.contains("Pagination links: next"), "{stdout}");
    assert!(
        stdout.contains(
            "- #1 [triaged] SQL injection (created_at=2026-01-01T00:00:00Z, severity=high)"
        ),
        "{stdout}"
    );
}

#[tokio::test]
async fn raw_run_prints_only_the_payload() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reports(1)))
        .expect(1)
        .mount(&api)
        .await;

    let output = run_harness(&api.uri(), &["--raw"]);
    assert!(
        output.status.success(),
        "harness failed: {}",
        stderr(&output)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Program:"), "{stdout}");
    let payload: Value = serde_json::from_str(&stdout).expect("raw stdout is JSON");
    assert_eq!(payload["data"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn too_few_results_fail_with_a_clear_message() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reports(1)))
        .expect(1)
        .mount(&api)
        .await;

    let output = run_harness(&api.uri(), &["--min-results", "3"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("query_reports returned 1 reports, expected at least 3."),
        "{}",
        stderr(&output)
    );
}

#[tokio::test]
async fn api_errors_fail_with_the_api_message() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .expect(1)
        .mount(&api)
        .await;

    let output = run_harness(&api.uri(), &[]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("hackerone api error 500: boom"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn missing_credentials_fail_before_spawning_the_server() {
    let output = Command::new(HARNESS)
        .env_clear()
        .env("HACKERONE_PROGRAM_HANDLE", "acme")
        .output()
        .expect("run query_reports harness");
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains(
            "Missing required HackerOne environment variables: \
             HACKERONE_API_USERNAME, HACKERONE_API_TOKEN"
        ),
        "{stderr}"
    );
    assert_eq!(output.status.code(), Some(2));
}

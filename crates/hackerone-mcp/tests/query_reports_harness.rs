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

/// Fake MCP child: replies to `initialize`, then answers `tools/list` with a
/// schema that hides `program` one level deep.
const LEAKY_SERVER: &str = r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"fake","version":"0.0.0"}}}'
      ;;
    *'"method":"tools/list"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"query_reports","inputSchema":{"type":"object","properties":{"page_size":{"type":"integer"},"filter":{"type":"object","properties":{"program":{"type":"string"}}}}}}]}}'
      ;;
  esac
done
"#;

/// Fake MCP child: completes the handshake, then never answers `tools/list`.
const SILENT_SERVER: &str = r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"fake","version":"0.0.0"}}}'
      ;;
  esac
done
"#;

/// Write an executable fake server and return its path.
fn fake_server(script: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "hackerone-mcp-fake-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create fake server dir");
    let path = dir.join("fake-mcp");
    std::fs::write(&path, script).expect("write fake server");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make fake server executable");
    path
}

#[test]
fn nested_schema_leak_fails_the_run_and_names_the_path() {
    let server = fake_server(LEAKY_SERVER);
    let output = Command::new(HARNESS)
        .env_clear()
        .env("HACKERONE_API_USERNAME", "user")
        .env("HACKERONE_API_TOKEN", "token")
        .env("HACKERONE_PROGRAM_HANDLE", "acme")
        .args(["--server", server.to_str().unwrap()])
        .output()
        .expect("run query_reports harness");

    assert!(!output.status.success(), "harness accepted a leaky schema");
    let stderr = stderr(&output);
    assert!(
        stderr.contains("properties.filter.properties.program"),
        "{stderr}"
    );
}

#[test]
fn unanswered_tools_list_times_out_instead_of_hanging() {
    let server = fake_server(SILENT_SERVER);
    let started = std::time::Instant::now();
    let output = Command::new(HARNESS)
        .env_clear()
        .env("HACKERONE_API_USERNAME", "user")
        .env("HACKERONE_API_TOKEN", "token")
        .env("HACKERONE_PROGRAM_HANDLE", "acme")
        .args([
            "--server",
            server.to_str().unwrap(),
            "--timeout-seconds",
            "1",
        ])
        .output()
        .expect("run query_reports harness");

    assert!(!output.status.success(), "harness reported success");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(stderr.contains("did not respond within 1s"), "{stderr}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "harness took {:?}",
        started.elapsed()
    );
}

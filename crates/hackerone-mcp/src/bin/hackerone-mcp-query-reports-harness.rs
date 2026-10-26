//! Live end-to-end checker for the `query_reports` tool (upstream parity).
//!
//! Starts the sibling `hackerone-mcp` binary over stdio, asserts the advertised
//! schema cannot be steered to another program, calls `query_reports` with the
//! requested filters, and prints either a compact summary or the raw payload.

use std::{
    env,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use clap::Parser;
use hackerone_mcp::tool::{REPORT_STATES, SEVERITIES};
use serde_json::{Map, Value, json};

/// Credentials the server needs before it will serve any tool.
const REQUIRED_ENV: [&str; 3] = [
    "HACKERONE_API_USERNAME",
    "HACKERONE_API_TOKEN",
    "HACKERONE_PROGRAM_HANDLE",
];

/// Schema properties that would let a caller escape the configured program.
const FORBIDDEN_SCHEMA_PROPERTIES: [&str; 4] =
    ["program", "programs", "program_handle", "team_handle"];

/// Sibling server binary name.
const SERVER_BINARY: &str = "hackerone-mcp";

/// MCP protocol version the server speaks (see `tests/stdio_handshake.rs`).
const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Parser)]
#[command(
    name = "hackerone-mcp-query-reports-harness",
    version,
    about = "Start the local hackerone-mcp server and exercise the query_reports tool against a live HackerOne program."
)]
struct Cli {
    /// Number of reports to request. Defaults to 5.
    #[arg(long, value_name = "N", default_value_t = 5)]
    page_size: i64,
    /// Page number to request. Defaults to 1.
    #[arg(long, value_name = "N", default_value_t = 1)]
    page_number: i64,
    /// Sort field to use. Defaults to created_at.
    #[arg(long, value_name = "FIELD", default_value = "created_at")]
    sort_by: String,
    /// Sort in ascending order.
    #[arg(long, conflicts_with = "desc")]
    asc: bool,
    /// Sort in descending order.
    #[arg(long, conflicts_with = "asc")]
    desc: bool,
    /// Filter by report state. May be supplied more than once.
    #[arg(long, value_name = "STATE", value_parser = clap::builder::PossibleValuesParser::new(REPORT_STATES))]
    state: Vec<String>,
    /// Filter by severity. May be supplied more than once.
    #[arg(long, value_name = "SEVERITY", value_parser = clap::builder::PossibleValuesParser::new(SEVERITIES))]
    severity: Vec<String>,
    /// Filter by report ID. May be supplied more than once.
    #[arg(long, value_name = "ID")]
    report_id: Vec<i64>,
    /// Filter by reporter username. May be supplied more than once.
    #[arg(long, value_name = "USER")]
    reporter: Vec<String>,
    /// Filter by assignee username, email, or group name. May be supplied more than once.
    #[arg(long, value_name = "ASSIGNEE")]
    assignee: Vec<String>,
    /// Filter by inbox ID. May be supplied more than once.
    #[arg(long, value_name = "ID")]
    inbox_id: Vec<i64>,
    /// Only include reports created after this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    created_after: Option<String>,
    /// Only include reports created before this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    created_before: Option<String>,
    /// Only include reports submitted after this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    submitted_after: Option<String>,
    /// Only include reports submitted before this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    submitted_before: Option<String>,
    /// Only include reports triaged after this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    triaged_after: Option<String>,
    /// Only include reports triaged before this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    triaged_before: Option<String>,
    /// Only include reports closed after this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    closed_after: Option<String>,
    /// Only include reports closed before this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    closed_before: Option<String>,
    /// Only include reports with activity after this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    last_activity_after: Option<String>,
    /// Only include reports with activity before this RFC3339 timestamp.
    #[arg(long, value_name = "TIMESTAMP")]
    last_activity_before: Option<String>,
    /// Fail if fewer than this many reports are returned. Defaults to 0.
    #[arg(long, value_name = "N", default_value_t = 0)]
    min_results: usize,
    /// Maximum number of reports to show in the summary output. Defaults to 5.
    #[arg(long, value_name = "N", default_value_t = 5)]
    summary_limit: usize,
    /// Print the raw JSON payload instead of a summary.
    #[arg(long)]
    raw: bool,
}

/// A failure carrying the process exit code upstream uses: 2 for bad
/// invocation/configuration, 1 for a failed run.
struct Failure {
    code: i32,
    message: String,
}

fn usage(message: impl Into<String>) -> Failure {
    Failure {
        code: 2,
        message: message.into(),
    }
}

fn failed(message: impl Into<String>) -> Failure {
    Failure {
        code: 1,
        message: message.into(),
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(failure) = run(&cli) {
        eprintln!("{}", failure.message);
        std::process::exit(failure.code);
    }
}

fn run(cli: &Cli) -> Result<(), Failure> {
    let program_handle = collect_required_env(|key| env::var(key).ok()).map_err(usage)?;
    let arguments = build_query_reports_arguments(cli);
    let server = server_binary().map_err(usage)?;
    let mut client = McpClient::spawn(&server).map_err(usage)?;

    client.initialize().map_err(failed)?;
    let tools = client.list_tools().map_err(failed)?;
    let tool = find_query_reports_tool(&tools).map_err(failed)?;
    validate_query_reports_tool(tool).map_err(usage)?;

    let result = client
        .call_tool("query_reports", &arguments)
        .map_err(failed)?;
    let payload = parse_call_tool_payload(&result).map_err(failed)?;

    let reports = payload
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            failed(format!(
                "query_reports payload has no data array:\n{}",
                pretty(&payload)
            ))
        })?;
    if reports.len() < cli.min_results {
        return Err(failed(format!(
            "query_reports returned {} reports, expected at least {}.\n{}",
            reports.len(),
            cli.min_results,
            pretty(&payload)
        )));
    }

    if cli.raw {
        println!("{}", pretty(&payload));
    } else {
        println!("Program: {program_handle}");
        println!("Arguments: {}", pretty(&arguments));
        println!("{}", summarize(&payload, cli.summary_limit));
    }
    Ok(())
}

/// Required env values, checked exactly as upstream: empty or whitespace is
/// missing. Returns the configured program handle used in the summary header.
fn collect_required_env<F>(var: F) -> Result<String, String>
where
    F: Fn(&str) -> Option<String>,
{
    let missing: Vec<&str> = REQUIRED_ENV
        .into_iter()
        .filter(|key| var(key).is_none_or(|value| value.trim().is_empty()))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Missing required HackerOne environment variables: {}",
            missing.join(", ")
        ));
    }
    Ok(var("HACKERONE_PROGRAM_HANDLE")
        .unwrap_or_default()
        .trim()
        .to_owned())
}

/// Flag values in, `query_reports` arguments out. Optional filters are omitted
/// when empty, exactly like upstream `build_query_reports_arguments`.
fn build_query_reports_arguments(cli: &Cli) -> Value {
    let mut arguments = Map::new();
    arguments.insert("page_size".to_owned(), json!(cli.page_size));
    arguments.insert("page_number".to_owned(), json!(cli.page_number));
    arguments.insert("sort_by".to_owned(), json!(cli.sort_by));
    arguments.insert(
        "sort_order".to_owned(),
        json!(if cli.asc { "asc" } else { "desc" }),
    );

    insert_array(&mut arguments, "report_ids", &cli.report_id);
    insert_array(&mut arguments, "inbox_ids", &cli.inbox_id);
    insert_array(&mut arguments, "reporters", &cli.reporter);
    insert_array(&mut arguments, "assignees", &cli.assignee);
    insert_array(&mut arguments, "states", &cli.state);
    insert_array(&mut arguments, "severities", &cli.severity);

    for (key, value) in [
        ("created_after", &cli.created_after),
        ("created_before", &cli.created_before),
        ("submitted_after", &cli.submitted_after),
        ("submitted_before", &cli.submitted_before),
        ("triaged_after", &cli.triaged_after),
        ("triaged_before", &cli.triaged_before),
        ("closed_after", &cli.closed_after),
        ("closed_before", &cli.closed_before),
        ("last_activity_after", &cli.last_activity_after),
        ("last_activity_before", &cli.last_activity_before),
    ] {
        if let Some(value) = value {
            arguments.insert(key.to_owned(), json!(value));
        }
    }

    Value::Object(arguments)
}

fn insert_array<T: serde::Serialize>(arguments: &mut Map<String, Value>, key: &str, values: &[T]) {
    if !values.is_empty() {
        arguments.insert(key.to_owned(), json!(values));
    }
}

/// The advertised `query_reports` entry from a `tools/list` payload.
fn find_query_reports_tool(tools: &Value) -> Result<&Value, String> {
    tools
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "query_reports"))
        .ok_or_else(|| "query_reports is not advertised by the MCP server.".to_owned())
}

/// The advertised `query_reports` schema must not accept caller-controlled
/// program scope (ADR-0001 wart fix 6). Upstream checks only `program`.
fn validate_query_reports_tool(tool: &Value) -> Result<(), String> {
    let properties = tool
        .get("inputSchema")
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
        .ok_or_else(|| "query_reports advertises no inputSchema properties.".to_owned())?;
    let forbidden: Vec<&str> = FORBIDDEN_SCHEMA_PROPERTIES
        .into_iter()
        .filter(|name| properties.contains_key(*name))
        .collect();
    if !forbidden.is_empty() {
        return Err(format!(
            "query_reports unexpectedly exposes caller-controlled program fields: {}",
            forbidden.join(", ")
        ));
    }
    Ok(())
}

/// One parsed JSON-RPC response line: `result` on success, the server's error
/// message on an error frame.
fn parse_response(frame: &Value) -> Result<Value, String> {
    if let Some(error) = frame.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown JSON-RPC error");
        return Err(format!("JSON-RPC error: {message}"));
    }
    frame
        .get("result")
        .cloned()
        .ok_or_else(|| format!("JSON-RPC frame has no result: {frame}"))
}

/// A `tools/call` result: tool-level errors are text, success text is the API
/// JSON (pretty-printed by the server).
fn parse_call_tool_payload(result: &Value) -> Result<Value, String> {
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(format!(
            "query_reports returned an error response: {}",
            content_text(result).unwrap_or_else(|| result.to_string())
        ));
    }
    let text =
        content_text(result).ok_or_else(|| "query_reports returned no text payload.".to_owned())?;
    serde_json::from_str(&text)
        .map_err(|error| format!("query_reports returned invalid JSON: {error}: {text}"))
}

fn content_text(result: &Value) -> Option<String> {
    result
        .get("content")?
        .as_array()?
        .iter()
        .find(|item| item["type"] == "text")
        .and_then(|item| item["text"].as_str())
        .map(str::to_owned)
}

/// Compact summary mirroring upstream `summarize_response` line for line.
fn summarize(payload: &Value, summary_limit: usize) -> String {
    let reports = payload
        .get("data")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let mut lines = vec![format!("Returned {} reports.", reports.len())];

    if let Some(links) = payload.get("links").and_then(Value::as_object) {
        let mut available: Vec<&str> = links
            .iter()
            .filter(|(_, value)| truthy(value))
            .map(|(key, _)| key.as_str())
            .collect();
        available.sort_unstable();
        if !available.is_empty() {
            lines.push(format!("Pagination links: {}", available.join(", ")));
        }
    }

    for report in reports.iter().take(summary_limit) {
        let id = scalar(report.get("id"), "<unknown>");
        let attributes = report.get("attributes").unwrap_or(&Value::Null);
        let title = scalar(attributes.get("title"), "<untitled>");
        let state = scalar(attributes.get("state"), "<unknown>");
        let created_at = scalar(attributes.get("created_at"), "<unknown>");
        let severity = extract_severity(report)
            .map(|severity| format!(", severity={severity}"))
            .unwrap_or_default();
        lines.push(format!(
            "- #{id} [{state}] {title} (created_at={created_at}{severity})"
        ));
    }

    lines.join("\n")
}

fn extract_severity(report: &Value) -> Option<&str> {
    report
        .get("relationships")?
        .get("severity")?
        .get("data")?
        .get("attributes")?
        .get("rating")?
        .as_str()
}

/// Python-style truthiness for pagination link values.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Number(_) => true,
    }
}

fn scalar(value: Option<&Value>, default: &str) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(value) if !value.is_null() => value.to_string(),
        _ => default.to_owned(),
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// `hackerone-mcp` next to this executable (both live in the crate's target
/// directory), falling back to the test-time `CARGO_BIN_EXE_` hint.
fn server_binary() -> Result<PathBuf, String> {
    let name = format!("{SERVER_BINARY}{}", env::consts::EXE_SUFFIX);
    let exe = env::current_exe().map_err(|error| format!("cannot locate harness: {error}"))?;
    let sibling = exe.with_file_name(&name);
    if sibling.is_file() {
        return Ok(sibling);
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_hackerone-mcp") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "cannot find the {name} server binary next to {}; build it with `cargo build --bins`",
        exe.display()
    ))
}

/// Minimal stdio JSON-RPC client: one JSON object per line, no rmcp client
/// feature needed. Requests are sequential, so matching on `id` is enough.
struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn(program: &Path) -> Result<Self, String> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("failed to start {}: {error}", program.display()))?;
        let stdin = child.stdin.take().ok_or("server stdin unavailable")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("server stdout unavailable")?);
        Ok(Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        })
    }

    fn send(&mut self, message: &Value) -> Result<(), String> {
        writeln!(self.stdin, "{message}")
            .and_then(|()| self.stdin.flush())
            .map_err(|error| format!("cannot write to the MCP server: {error}"))
    }

    fn recv(&mut self) -> Result<Value, String> {
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| format!("cannot read from the MCP server: {error}"))?;
        if read == 0 {
            return Err("MCP server closed stdout before responding.".to_owned());
        }
        serde_json::from_str(&line)
            .map_err(|error| format!("MCP server wrote a non-JSON line: {line:?}: {error}"))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))?;
        loop {
            let frame = self.recv()?;
            if frame.get("id").and_then(Value::as_u64) == Some(id) {
                return parse_response(&frame);
            }
            // Notifications and server-initiated requests are not expected;
            // ignore anything that is not this request's response.
        }
    }

    fn notify(&mut self, method: &str) -> Result<(), String> {
        self.send(&json!({"jsonrpc": "2.0", "method": method}))
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "hackerone-mcp-query-reports-harness",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )?;
        self.notify("notifications/initialized")
    }

    fn list_tools(&mut self) -> Result<Value, String> {
        let result = self.request("tools/list", json!({}))?;
        result
            .get("tools")
            .cloned()
            .ok_or_else(|| format!("tools/list returned no tools array: {result}"))
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<Value, String> {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn cli(arguments: &[&str]) -> Cli {
        Cli::parse_from(arguments)
    }

    #[test]
    fn defaults_map_to_the_upstream_arg_defaults() {
        assert_eq!(
            build_query_reports_arguments(&cli(&["harness"])),
            json!({
                "page_size": 5,
                "page_number": 1,
                "sort_by": "created_at",
                "sort_order": "desc",
            })
        );
    }

    #[test]
    fn repeated_flags_and_dates_map_to_filters() {
        let arguments = build_query_reports_arguments(&cli(&[
            "harness",
            "--page-size",
            "2",
            "--page-number",
            "3",
            "--sort-by",
            "triaged_at",
            "--asc",
            "--state",
            "triaged",
            "--state",
            "resolved",
            "--severity",
            "high",
            "--report-id",
            "7",
            "--report-id",
            "9",
            "--reporter",
            "alice",
            "--assignee",
            "team",
            "--inbox-id",
            "4",
            "--created-after",
            "2026-01-01T00:00:00Z",
            "--last-activity-before",
            "2026-02-01T00:00:00Z",
        ]));

        assert_eq!(arguments["page_size"], 2);
        assert_eq!(arguments["page_number"], 3);
        assert_eq!(arguments["sort_by"], "triaged_at");
        assert_eq!(arguments["sort_order"], "asc");
        assert_eq!(arguments["states"], json!(["triaged", "resolved"]));
        assert_eq!(arguments["severities"], json!(["high"]));
        assert_eq!(arguments["report_ids"], json!([7, 9]));
        assert_eq!(arguments["reporters"], json!(["alice"]));
        assert_eq!(arguments["assignees"], json!(["team"]));
        assert_eq!(arguments["inbox_ids"], json!([4]));
        assert_eq!(arguments["created_after"], "2026-01-01T00:00:00Z");
        assert_eq!(arguments["last_activity_before"], "2026-02-01T00:00:00Z");
        assert!(arguments.get("created_before").is_none());
    }

    #[test]
    fn missing_credentials_are_named() {
        let error = collect_required_env(|_| None).unwrap_err();
        assert_eq!(
            error,
            "Missing required HackerOne environment variables: \
             HACKERONE_API_USERNAME, HACKERONE_API_TOKEN, HACKERONE_PROGRAM_HANDLE"
        );
        assert_eq!(
            collect_required_env(|_| Some("  ".to_owned())).unwrap_err(),
            "Missing required HackerOne environment variables: \
             HACKERONE_API_USERNAME, HACKERONE_API_TOKEN, HACKERONE_PROGRAM_HANDLE"
        );
        assert!(
            collect_required_env(
                |key| (key == "HACKERONE_PROGRAM_HANDLE").then(|| "acme".to_owned())
            )
            .unwrap_err()
            .contains("HACKERONE_API_USERNAME, HACKERONE_API_TOKEN")
        );
        assert_eq!(
            collect_required_env(|_| Some("value".to_owned())).unwrap(),
            "value"
        );
    }

    #[test]
    fn response_parsing_surfaces_results_and_errors() {
        assert_eq!(
            parse_response(&json!({"jsonrpc": "2.0", "id": 1, "result": {"ok": true}})).unwrap(),
            json!({"ok": true})
        );
        let error = parse_response(
            &json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32602, "message": "bad params"}}),
        )
        .unwrap_err();
        assert_eq!(error, "JSON-RPC error: bad params");
        assert!(parse_response(&json!({"jsonrpc": "2.0", "id": 1})).is_err());
    }

    #[test]
    fn call_tool_payload_parses_text_and_rejects_tool_errors() {
        let payload = json!({"data": [{"id": "1"}]});
        assert_eq!(
            parse_call_tool_payload(&json!({
                "isError": false,
                "content": [{"type": "text", "text": "{\"data\":[{\"id\":\"1\"}]}"}],
            }))
            .unwrap(),
            payload
        );

        let error = parse_call_tool_payload(&json!({
            "isError": true,
            "content": [{"type": "text", "text": "hackerone api error 500: boom"}],
        }))
        .unwrap_err();
        assert!(error.contains("hackerone api error 500: boom"), "{error}");

        assert!(
            parse_call_tool_payload(&json!({"isError": false, "content": []}))
                .unwrap_err()
                .contains("no text payload")
        );
    }

    #[test]
    fn missing_query_reports_tool_is_named() {
        assert_eq!(
            find_query_reports_tool(&json!([])).unwrap_err(),
            "query_reports is not advertised by the MCP server."
        );
        let tools = json!([{"name": "get_report"}, {"name": "query_reports"}]);
        assert_eq!(
            find_query_reports_tool(&tools).unwrap()["name"],
            "query_reports"
        );
    }

    #[test]
    fn schema_leak_is_rejected() {
        let tool = |properties: Value| json!({"name": "query_reports", "inputSchema": {"type": "object", "properties": properties}});
        validate_query_reports_tool(&tool(json!({"page_size": {"type": "integer"}})))
            .expect("clean schema");
        let error = validate_query_reports_tool(&tool(json!({
            "page_size": {"type": "integer"},
            "program": {"type": "string"},
            "team_handle": {"type": "string"},
        })))
        .unwrap_err();
        assert!(error.contains("program, team_handle"), "{error}");
    }

    #[test]
    fn summary_matches_the_upstream_format() {
        let payload = json!({
            "data": [{
                "id": "42",
                "attributes": {
                    "title": "SQL injection",
                    "state": "triaged",
                    "created_at": "2026-01-01T00:00:00Z",
                },
                "relationships": {"severity": {"data": {"attributes": {"rating": "high"}}}},
            }],
            "links": {"next": "https://example.test/next", "prev": null},
        });
        assert_eq!(
            summarize(&payload, 5),
            "Returned 1 reports.\n\
             Pagination links: next\n\
             - #42 [triaged] SQL injection (created_at=2026-01-01T00:00:00Z, severity=high)"
        );
        assert_eq!(summarize(&json!({"data": []}), 5), "Returned 0 reports.");
    }
}

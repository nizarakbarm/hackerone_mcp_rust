//! Black-box tests over the real stdio transport: handshake, advertised tool
//! registry, a real `tools/call` round trip against a fake API, and the
//! guarantee that stdout carries JSON-RPC frames only.

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
};

use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
    stdout_lines: Vec<String>,
}

impl Server {
    fn start() -> Self {
        Self::start_with(&[])
    }

    fn start_with(environment: &[(&str, &str)]) -> Self {
        Self::spawn(environment, true)
    }

    /// Same binary with no API credentials configured: startup must still
    /// succeed and advertise every tool (ADR-0008).
    fn start_without_credentials(environment: &[(&str, &str)]) -> Self {
        Self::spawn(environment, false)
    }

    fn spawn(environment: &[(&str, &str)], credentials: bool) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hackerone-mcp"));
        // Pin config sources: a developer's `~/.config` file or `HACKERONE_*`
        // env vars must not change what this test observes.
        command
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if credentials {
            command
                .env("HACKERONE_API_USERNAME", "user")
                .env("HACKERONE_API_TOKEN", "token");
        }
        for (key, value) in environment {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("spawn hackerone-mcp");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        let stderr = BufReader::new(child.stderr.take().expect("stderr piped"));
        Self {
            child,
            stdin,
            stdout,
            stderr,
            stdout_lines: Vec::new(),
        }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{message}").expect("write frame");
        stdin.flush().expect("flush frame");
    }

    /// Next stdout line, asserted to be a JSON-RPC frame.
    fn recv(&mut self) -> Value {
        let mut line = String::new();
        let read = self.stdout.read_line(&mut line).expect("read stdout");
        assert!(read > 0, "server closed stdout before answering");
        let value: Value = serde_json::from_str(&line)
            .unwrap_or_else(|err| panic!("stdout line is not JSON: {line:?}: {err}"));
        self.stdout_lines.push(line);
        value
    }

    fn handshake(&mut self) {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "stdio-handshake-test", "version": "0.0.0" }
            }
        }));
        let response = self.recv();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1, "initialize response: {response}");
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(response["result"]["serverInfo"]["name"], "hackerone-mcp");
        assert_eq!(
            response["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            response["result"]["capabilities"]["tools"].is_object(),
            "tools capability missing: {response}"
        );
        self.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    }

    fn list_tools(&mut self) -> Value {
        let response = self.request(2, "tools/list", json!({}));
        response["result"]["tools"].clone()
    }

    /// `tools/call` and the matching response, skipping unrelated frames.
    fn call_tool(&mut self, id: i64, name: &str, arguments: Value) -> Value {
        let response = self.request(
            id,
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        );
        assert!(
            response.get("error").is_none(),
            "tools/call failed: {response}"
        );
        response["result"].clone()
    }

    fn request(&mut self, id: i64, method: &str, params: Value) -> Value {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let response = self.recv();
            if response["id"] == json!(id) {
                return response;
            }
        }
    }

    /// Close stdin, drain both pipes, and return (all stdout lines, all stderr).
    fn finish(self) -> (Vec<String>, String) {
        let (stdout, err, status) = self.finish_with_status();
        assert!(status.success(), "server exited with {status}");
        (stdout, err)
    }

    /// [`Server::finish`] without requiring a clean exit, for startup failures.
    fn finish_with_status(mut self) -> (Vec<String>, String, ExitStatus) {
        drop(self.stdin.take());
        let mut rest = String::new();
        self.stdout
            .read_to_string(&mut rest)
            .expect("drain stdout after exit");
        let mut err = String::new();
        self.stderr.read_to_string(&mut err).expect("drain stderr");
        let status = self.child.wait().expect("wait for server exit");

        self.stdout_lines.extend(rest.lines().map(str::to_owned));
        (self.stdout_lines, err, status)
    }
}

#[test]
fn stdio_handshake_advertises_only_the_wired_tools() {
    let mut server = Server::start();
    server.handshake();
    let tools = server.list_tools();
    let names: Vec<&str> = tools
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "get_report",
            "get_report_custom_field_values",
            "create_report",
            "query_reports",
            "list_scopes",
            "change_state",
            "create_comment",
            "update_title",
            "update_reference",
            "update_cves",
            "escalate_report",
            "remove_escalation",
            "update_custom_field_values",
            "search_programs",
            "get_program",
            "get_program_structured_scopes"
        ],
        "tools: {tools}"
    );
    for tool in tools.as_array().expect("tools array") {
        assert_eq!(tool["inputSchema"]["type"], "object", "tool: {tool}");
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "tool: {tool}"
        );
    }
    let (stdout, _) = server.finish();
    assert_eq!(stdout.len(), 2, "unexpected extra stdout lines: {stdout:?}");
}

#[test]
fn stdout_is_jsonrpc_only_and_logs_go_to_stderr() {
    let mut server = Server::start();
    server.handshake();
    assert_eq!(server.list_tools().as_array().map(Vec::len), Some(16));
    let (stdout, stderr) = server.finish();

    assert!(!stdout.is_empty(), "no stdout frames were produced");
    for line in &stdout {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|err| panic!("stdout line is not JSON: {line:?}: {err}"));
    }
    assert!(
        stderr.contains("starting stdio MCP server"),
        "startup log missing from stderr: {stderr:?}"
    );
}

#[tokio::test]
async fn real_binary_round_trips_tools_call_over_the_configured_api() {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "7"}})))
        .expect(1)
        .mount(&api)
        .await;
    Mock::given(method("GET"))
        .and(path("/reports"))
        .and(query_param("filter[program][]", "acme"))
        .and(query_param("filter[state][]", "triaged"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&api)
        .await;

    Mock::given(method("POST"))
        .and(path("/reports"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "data": {"type": "report", "id": "9"}
        })))
        .expect(1)
        .mount(&api)
        .await;

    let uri = api.uri();
    let mut server = Server::start_with(&[
        ("HACKERONE_API_BASE_URL", uri.as_str()),
        ("HACKERONE_PROGRAM_HANDLE", "acme"),
    ]);
    server.handshake();

    let result = server.call_tool(3, "get_report", json!({"report_id": 7}));
    assert_eq!(result["isError"], false, "result: {result}");
    assert_eq!(
        result["content"][0]["text"],
        serde_json::to_string_pretty(&json!({"data": {"id": "7"}})).expect("pretty")
    );

    let result = server.call_tool(
        4,
        "query_reports",
        json!({"states": ["triaged"], "page_size": 10}),
    );
    assert_eq!(
        result["content"][0]["text"],
        serde_json::to_string_pretty(&json!({"data": []})).expect("pretty")
    );

    let result = server.call_tool(
        5,
        "create_report",
        json!({
            "title": "SQL injection",
            "vulnerability_information": "payload",
            "impact": "full read",
        }),
    );
    assert_eq!(result["isError"], false, "result: {result}");
    assert_eq!(
        result["content"][0]["text"],
        serde_json::to_string_pretty(&json!({"data": {"type": "report", "id": "9"}}))
            .expect("pretty")
    );

    let result = server.call_tool(6, "totally_unknown", json!({}));
    assert_eq!(result["isError"], true, "result: {result}");
    assert!(
        result["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("unknown tool")),
        "result: {result}"
    );

    let (stdout, _) = server.finish();
    for line in &stdout {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|err| panic!("stdout line is not JSON: {line:?}: {err}"));
    }
}

#[test]
fn advertised_schemas_never_expose_program_scope_at_any_depth() {
    // Mutation: adding `program`, `programs`, `program_handle` or `team_handle`
    // anywhere under an advertised `inputSchema` — including nested `properties`
    // and array `items` — fails this walk. The shallow unit-level guard is
    // `tool_vocabulary.rs::no_schema_exposes_a_program_override`; this covers the
    // live wire advertisement and the depth that guard does not.
    fn walk(value: &Value, tool: &str, path: &str) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    assert!(
                        !["program", "programs", "program_handle", "team_handle"]
                            .contains(&key.as_str()),
                        "{tool}: {path}.{key} exposes program scope"
                    );
                    walk(child, tool, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    walk(child, tool, &format!("{path}[{index}]"));
                }
            }
            _ => {}
        }
    }

    let mut server = Server::start();
    server.handshake();
    let tools = server.list_tools();
    let tools = tools.as_array().expect("tools array");
    assert_eq!(tools.len(), 16, "walk the full registry, not a subset");
    for tool in tools {
        walk(
            &tool["inputSchema"],
            tool["name"].as_str().expect("tool name"),
            "$",
        );
    }
    server.finish();
}

#[test]
fn stdout_stays_jsonrpc_only_on_error_paths() {
    // Mutation: a `println!` (or any stdout log) on a validation error, an
    // unknown tool, or a missing-credential call leaves a non-JSON line here
    // and the test fails.
    let mut server = Server::start();
    server.handshake();
    server.list_tools();

    let validation = server.request(
        3,
        "tools/call",
        json!({"name": "get_report", "arguments": {}}),
    );
    assert_eq!(validation["result"]["isError"], true, "{validation}");

    let unknown = server.request(
        4,
        "tools/call",
        json!({"name": "totally_unknown", "arguments": {}}),
    );
    assert_eq!(unknown["result"]["isError"], true, "{unknown}");

    let (stdout, _) = server.finish();
    for line in &stdout {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|err| panic!("stdout line is not JSON: {line:?}: {err}"));
    }

    // Hacker credentials only: the server still starts and advertises every
    // tool; a customer-class call fails as `isError`, never as a startup abort
    // (ADR-0008).
    let mut server = Server::start_without_credentials(&[
        ("HACKERONE_HACKER_API_USERNAME", "hacker-user"),
        ("HACKERONE_HACKER_API_TOKEN", "hacker-token"),
    ]);
    server.handshake();
    assert_eq!(
        server.list_tools().as_array().expect("tools").len(),
        16,
        "every tool stays advertised without any credentials"
    );
    let missing = server.call_tool(3, "get_report", json!({"report_id": 7}));
    assert_eq!(missing["isError"], true, "{missing}");
    assert!(
        missing["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("missing customer credential")),
        "{missing}"
    );

    let (stdout, _) = server.finish();
    for line in &stdout {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|err| panic!("stdout line is not JSON: {line:?}: {err}"));
    }
}

#[test]
fn version_flag_works_without_credentials() {
    let output = Command::new(env!("CARGO_BIN_EXE_hackerone-mcp"))
        .arg("--version")
        .env_clear()
        .output()
        .expect("run --version");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("hackerone-mcp {}", env!("CARGO_PKG_VERSION"))
    );
}

#[tokio::test]
async fn tokens_never_reach_stderr_even_at_trace_level() {
    const API_TOKEN: &str = "super-secret-token-value";
    const HTTP_TOKEN: &str = "bearer-secret-value";

    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/reports/8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "8"}})))
        .mount(&api)
        .await;
    Mock::given(method("GET"))
        .and(path("/reports/7"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"errors": [{"detail": "unauthorized"}]})),
        )
        .mount(&api)
        .await;

    let uri = api.uri();
    let mut server = Server::start_with(&[
        ("HACKERONE_API_BASE_URL", uri.as_str()),
        ("HACKERONE_API_TOKEN", API_TOKEN),
        ("HACKERONE_MCP_HTTP_TOKEN", HTTP_TOKEN),
        ("RUST_LOG", "trace"),
    ]);
    server.handshake();

    let ok = server.call_tool(3, "get_report", json!({"report_id": 8}));
    assert_eq!(ok["isError"], false, "{ok}");
    let denied = server.call_tool(4, "get_report", json!({"report_id": 7}));
    assert_eq!(denied["isError"], true, "{denied}");
    assert!(
        denied["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("401")),
        "the 401 path was not exercised: {denied}"
    );

    let (_, stderr) = server.finish();

    // Mutation: tracing the config/client (or the built request) at trace level
    // puts either secret in stderr and one of these assertions fails.
    assert!(!stderr.contains(API_TOKEN), "API token leaked: {stderr:?}");
    assert!(
        !stderr.contains(HTTP_TOKEN),
        "HTTP token leaked: {stderr:?}"
    );

    let requests = api.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 2, "one success, one 401");
    let authorization: Vec<String> = requests
        .iter()
        .flat_map(|request| {
            request
                .headers
                .get_all("authorization")
                .iter()
                .map(|value| value.to_str().expect("header value").to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        !authorization.is_empty(),
        "the credential must actually be sent to the API"
    );
    for value in authorization {
        assert!(
            !stderr.contains(&value),
            "Authorization header value leaked: {value}"
        );
    }
}

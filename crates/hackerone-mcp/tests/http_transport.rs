//! Black-box tests for the streamable-HTTP transport (ADR-0005): the bearer
//! gate rejects missing/wrong tokens before a session exists, the right token
//! gets the full tool registry, non-loopback binds warn, and a missing token
//! fails closed at startup.

use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, ChildStderr, Command, Stdio},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

struct Server {
    child: Child,
    stderr: BufReader<ChildStderr>,
    base: String,
}

impl Server {
    /// Spawn the real binary on `bind`; `environment` overlays the pinned creds.
    async fn start(bind: &str, environment: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hackerone-mcp"));
        command
            .env_clear()
            .env("HACKERONE_API_USERNAME", "user")
            .env("HACKERONE_API_TOKEN", "token")
            .args(["--transport", "http", "--bind", bind])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in environment {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("spawn hackerone-mcp");
        let stderr = BufReader::new(child.stderr.take().expect("stderr piped"));
        let server = Self {
            child,
            stderr,
            base: format!("http://{bind}/mcp"),
        };
        server.wait_until_listening().await;
        server
    }

    async fn wait_until_listening(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let addr = self
                .base
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap();
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("server never listened on {}", self.base);
    }

    /// Next stderr line, or `None` when the child closed stderr.
    fn next_stderr_line(&mut self) -> Option<String> {
        let mut line = String::new();
        let read = self.stderr.read_line(&mut line).expect("read stderr");
        (read > 0).then_some(line)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port()
}

fn initialize_body() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "http-transport-test", "version": "0.0.0" }
        }
    })
}

async fn post(
    client: &reqwest::Client,
    base: &str,
    token: Option<&str>,
    session: Option<&str>,
    body: &Value,
) -> reqwest::Response {
    let mut request = client
        .post(base)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body(body.to_string());
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    if let Some(session) = session {
        request = request.header("mcp-session-id", session);
    }
    request.send().await.expect("POST /mcp")
}

/// The JSON-RPC frame with `id` from an SSE body (`data: {...}` lines).
fn sse_frame(text: &str, id: i64) -> Value {
    text.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .find(|frame| frame["id"] == json!(id))
        .unwrap_or_else(|| panic!("no frame with id {id} in: {text}"))
}

const TOKEN: &str = "test-http-token";

#[tokio::test]
async fn bearer_gate_rejects_before_any_session_and_admits_the_right_token() {
    let port = free_port();
    let server = Server::start(
        &format!("127.0.0.1:{port}"),
        &[("HACKERONE_MCP_HTTP_TOKEN", TOKEN)],
    )
    .await;
    let client = reqwest::Client::new();

    let missing = post(&client, &server.base, None, None, &initialize_body()).await;
    assert_eq!(missing.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(
        missing.headers().get("mcp-session-id").is_none(),
        "rejected request must not create a session"
    );

    let wrong = post(
        &client,
        &server.base,
        Some("not-the-token"),
        None,
        &initialize_body(),
    )
    .await;
    assert_eq!(wrong.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(
        wrong.headers().get("mcp-session-id").is_none(),
        "rejected request must not create a session"
    );

    let accepted = post(&client, &server.base, Some(TOKEN), None, &initialize_body()).await;
    assert_eq!(accepted.status(), reqwest::StatusCode::OK);
    let session = accepted
        .headers()
        .get("mcp-session-id")
        .expect("session id on accepted initialize")
        .to_str()
        .expect("ascii session id")
        .to_owned();
    let init = sse_frame(&accepted.text().await.expect("initialize body"), 1);
    assert_eq!(init["result"]["serverInfo"]["name"], "hackerone-mcp");

    let listed = post(
        &client,
        &server.base,
        Some(TOKEN),
        Some(&session),
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }),
    )
    .await;
    assert_eq!(listed.status(), reqwest::StatusCode::OK);
    let tools =
        sse_frame(&listed.text().await.expect("tools/list body"), 2)["result"]["tools"].clone();
    assert_eq!(tools.as_array().expect("tools array").len(), 20);
}

#[tokio::test]
async fn non_loopback_bind_is_honoured_and_warns() {
    let port = free_port();
    let mut server = Server::start(
        &format!("0.0.0.0:{port}"),
        &[("HACKERONE_MCP_HTTP_TOKEN", TOKEN)],
    )
    .await;
    assert_eq!(
        server.base,
        format!("http://0.0.0.0:{port}/mcp"),
        "custom --bind must be honoured"
    );

    let warning = (0..10)
        .filter_map(|_| server.next_stderr_line())
        .find(|line| line.contains("non-loopback"))
        .expect("non-loopback bind must log a warning");
    assert!(
        warning.contains("0.0.0.0"),
        "warning names the address: {warning}"
    );

    // The gate is live on the same listener.
    let rejected = post(
        &reqwest::Client::new(),
        &format!("http://127.0.0.1:{port}/mcp"),
        None,
        None,
        &initialize_body(),
    )
    .await;
    assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_transport_without_a_token_fails_closed_at_startup() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hackerone-mcp"))
        .env_clear()
        .env("HACKERONE_API_USERNAME", "user")
        .env("HACKERONE_API_TOKEN", "token")
        .args(["--transport", "http", "--bind", "127.0.0.1:0"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hackerone-mcp");

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for child") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "server did not exit without a token"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr piped")
        .read_to_string(&mut stderr)
        .expect("read stderr");

    assert!(!status.success(), "missing token must exit non-zero");
    assert!(
        stderr.contains("HACKERONE_MCP_HTTP_TOKEN"),
        "startup error must name the variable: {stderr}"
    );
}

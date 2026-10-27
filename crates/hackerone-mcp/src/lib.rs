//! MCP server for the HackerOne API.
//!
//! This crate owns the shared tool vocabulary (names, schemas, parameter
//! builders), dispatch, the stdio handshake, the layered config loader
//! (ADR-0004), the transport flag, and stderr-only logging.

pub mod config;
pub mod tool;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use hackerone_api::{Client, CredentialClass, Error};
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
    transport::{
        io::stdio,
        streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
        },
    },
};
use serde_json::{Map, Value};
use tracing_subscriber::EnvFilter;

/// HTTP listen address when neither `--bind` nor `[http] bind` is set.
pub const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8765";
/// Config key holding the HTTP bearer token (ADR-0005).
pub const HTTP_TOKEN_VAR: &str = "HACKERONE_MCP_HTTP_TOKEN";

use crate::config::Config;
use crate::tool::scope::ProgramScope;

/// One API client per credential class, built once at startup (ADR-0008).
///
/// A class exists only when both halves of its token pair are configured. A
/// missing pair is not a startup failure: a tool of that class fails at call
/// time with [`hackerone_api::Error::MissingCredential`], so the other class
/// keeps working and `tools/list` is unaffected.
#[derive(Debug, Clone, Default)]
pub struct Clients {
    customer: Option<Client>,
    hacker: Option<Client>,
}

impl Clients {
    /// Build the configured class clients. Both use `HACKERONE_API_BASE_URL`
    /// when set, so a proxy or fake API covers both classes; otherwise each
    /// defaults to the HackerOne API root.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if an HTTP client cannot be built.
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let base_url = config.base_url.as_deref();
        Ok(Self {
            customer: build(
                config.api_username.as_deref(),
                config.api_token.as_deref(),
                base_url,
            )?,
            hacker: build(
                config.hacker_api_username.as_deref(),
                config.hacker_api_token.as_deref(),
                base_url,
            )?,
        })
    }

    /// Client for `class`, or `None` when that class was not configured.
    #[must_use]
    pub fn for_class(&self, class: CredentialClass) -> Option<&Client> {
        match class {
            CredentialClass::Customer => self.customer.as_ref(),
            CredentialClass::Hacker => self.hacker.as_ref(),
        }
    }
}

/// A client only for a complete pair: half a pair is the same as none.
fn build(
    username: Option<&str>,
    token: Option<&str>,
    base_url: Option<&str>,
) -> Result<Option<Client>, Error> {
    let (Some(username), Some(token)) = (username, token) else {
        return Ok(None);
    };
    let client = Client::new(username, token)?;
    Ok(Some(match base_url {
        Some(base_url) => client.with_base_url(base_url),
        None => client,
    }))
}

/// MCP server: the class clients plus the configured program, routing
/// `tools/call` through [`tool::dispatch`] (ADR-0007).
#[derive(Clone)]
pub struct HackerOneServer {
    clients: Clients,
    /// Shared by every session the same process serves, so the resolved
    /// program id outlives a connection.
    scope: Arc<ProgramScope>,
}

impl HackerOneServer {
    #[must_use]
    pub fn new(clients: Clients, scope: Arc<ProgramScope>) -> Self {
        Self { clients, scope }
    }
}

impl ServerHandler for HackerOneServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = tool::WIRED
            .iter()
            .map(|(tool, description)| {
                Tool::new(tool.name(), *description, schema_object(tool.schema()))
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        let result = tool::dispatch(&self.clients, &self.scope, &request.name, &args).await;
        Ok(match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(pretty(&value))]),
            // Failed tool call, not a protocol error: the client sees the cause.
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.to_string())]),
        }
        .into())
    }
}

/// Every `inputSchema` is a JSON object; anything else is a schema bug.
fn schema_object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(object) => object,
        other => unreachable!("tool schema is not an object: {other}"),
    }
}

/// Result text: the API JSON, pretty-printed, nothing else (text passthrough).
fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Install stderr-only logging (`RUST_LOG`, default `info`).
///
/// stdout is the JSON-RPC channel in stdio mode and must stay clean.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

/// Serve MCP over stdio until the client closes stdin.
///
/// # Errors
///
/// [`Error::Config`] / [`Error::Transport`] from client construction, or the
/// transport failure that ended the session.
pub async fn serve_stdio(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let clients = Clients::from_config(&config)?;
    let scope = Arc::new(ProgramScope::new(config.program_handle.clone()));
    let service = HackerOneServer::new(clients, scope).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Serve MCP over streamable HTTP (ADR-0005) until the process is killed.
///
/// Every request must carry `Authorization: Bearer <token>` before it reaches
/// the MCP service, so a rejected request never creates a session.
///
/// # Errors
///
/// [`Error::Config`] when the HTTP token or bind address is missing/invalid,
/// [`Error::Transport`] from client construction, or the listener failure.
pub async fn serve_http(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let token = config.http_token.clone().ok_or_else(|| Error::Config {
        field: HTTP_TOKEN_VAR.to_owned(),
        reason: "is required for --transport http".to_owned(),
    })?;
    let addr = bind_addr(&config)?;
    let clients = Clients::from_config(&config)?;
    let sessions = Arc::new(LocalSessionManager::default());

    // Non-loopback binds are reached by a proxy, whose `Host` header is its own.
    let mut allowed_hosts = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];
    if !addr.ip().is_loopback() {
        allowed_hosts.push(addr.ip().to_string());
        allowed_hosts.push(addr.to_string());
    }
    // One scope for the process: every session shares the resolved program id.
    let scope = Arc::new(ProgramScope::new(config.program_handle.clone()));
    let service = StreamableHttpService::new(
        move || Ok(HackerOneServer::new(clients.clone(), Arc::clone(&scope))),
        sessions.clone(),
        StreamableHttpServerConfig::default().with_allowed_hosts(allowed_hosts),
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(address = %listener.local_addr()?, "serving MCP over streamable HTTP");
    axum::serve(listener, router(token, service)).await?;
    Ok(())
}

/// The MCP endpoint plus the bearer gate wrapping it. The middleware is layered
/// outside the nested service, so it runs first and no session is created for
/// a rejected request.
fn router(
    token: String,
    service: StreamableHttpService<HackerOneServer, LocalSessionManager>,
) -> Router {
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(token, require_bearer))
}

/// Reject anything but `Authorization: Bearer <expected>`, compared in
/// constant time. Missing header and wrong token are indistinguishable: both
/// are a bare 401.
async fn require_bearer(State(expected): State<String>, request: Request, next: Next) -> Response {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if presented
        .is_some_and(|presented| constant_time_eq(presented.as_bytes(), expected.as_bytes()))
    {
        return next.run(request).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
    )
        .into_response()
}

/// Byte-wise compare that does not stop at the first difference. Length is not
/// secret: the token length is fixed by the operator.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Listen address from `[http] bind` / `--bind`, warning when it is not loopback
/// (ADR-0005: no in-process TLS, terminate at a proxy).
fn bind_addr(config: &Config) -> Result<SocketAddr, Error> {
    let bind = config.bind.as_deref().unwrap_or(DEFAULT_HTTP_BIND);
    let addr: SocketAddr = bind.parse().map_err(|_| Error::Config {
        field: "bind".to_owned(),
        reason: format!("is not a socket address: {bind}"),
    })?;
    if !addr.ip().is_loopback() {
        tracing::warn!(
            %addr,
            "binding a non-loopback address; this server speaks plain HTTP, terminate TLS at a reverse proxy"
        );
    }
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_comparison_is_exact() {
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"tokeN"));
        assert!(!constant_time_eq(b"token", b"token "));
        assert!(!constant_time_eq(b"token", b""));
    }

    /// In-process evidence for the ordering claim: a 401 leaves the session
    /// map empty, so the middleware ran before the MCP service could create one.
    #[tokio::test]
    async fn rejected_request_creates_no_session() {
        let sessions = Arc::new(LocalSessionManager::default());
        let clients = Clients::from_config(&Config {
            api_username: Some("user".to_owned()),
            api_token: Some("token".to_owned()),
            ..Config::default()
        })
        .expect("build clients");
        let service = StreamableHttpService::new(
            move || {
                Ok(HackerOneServer::new(
                    clients.clone(),
                    Arc::new(ProgramScope::new(None)),
                ))
            },
            sessions.clone(),
            StreamableHttpServerConfig::default(),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let serving = tokio::spawn(async move {
            let _ = axum::serve(listener, router("secret".to_owned(), service)).await;
        });

        let rejected = reqwest::Client::new()
            .post(format!("http://{addr}/mcp"))
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .header("authorization", "Bearer wrong")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .send()
            .await
            .expect("POST /mcp");

        assert_eq!(rejected.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(rejected.headers().get("mcp-session-id").is_none());
        assert!(
            sessions.sessions.read().await.is_empty(),
            "a rejected request must not create a session"
        );
        serving.abort();
    }
}

//! Typed HackerOne REST client.
//!
//! No MCP types live here (see ADR-0002). Every method returns the decoded JSON
//! body as [`serde_json::Value`]; typed models are a later ticket, and decoding into
//! strict structs would turn sparse-but-valid payloads into failures.

use std::time::Duration;

use reqwest::header::ACCEPT;
use reqwest::{Client as HttpClient, Method, Response};
use serde_json::Value;
use thiserror::Error;

/// Default HackerOne API root.
pub const DEFAULT_BASE_URL: &str = "https://api.hackerone.com/v1";

/// Connect budget per request (ADR-0009).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Per-request budget, not per tool call: one tool may issue many requests (ADR-0009).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Cap on a 2xx response body, counted while streaming (spec #35).
///
/// `Content-Length` is not the control: it is absent for chunked or compressed
/// bodies and can lie, so the cap is enforced on bytes actually read.
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Cap on a non-2xx response body, counted while streaming (spec #35).
///
/// Error text is interpolated into an MCP tool result, i.e. into an LLM's context,
/// so it is truncated far lower than the success cap.
pub const MAX_ERROR_BODY_BYTES: usize = 4 * 1024;

/// Which token pair a tool spends (ADR-0008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialClass {
    /// Program/org token: `HACKERONE_API_USERNAME` + `HACKERONE_API_TOKEN`.
    Customer,
    /// Hacker token: `HACKERONE_HACKER_API_USERNAME` + `HACKERONE_HACKER_API_TOKEN`.
    Hacker,
}

impl CredentialClass {
    /// Lowercase class name, as it appears in error messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Customer => "customer",
            Self::Hacker => "hacker",
        }
    }

    /// The username and token environment variables that must both be set.
    #[must_use]
    pub const fn env_vars(self) -> [&'static str; 2] {
        match self {
            Self::Customer => ["HACKERONE_API_USERNAME", "HACKERONE_API_TOKEN"],
            Self::Hacker => [
                "HACKERONE_HACKER_API_USERNAME",
                "HACKERONE_HACKER_API_TOKEN",
            ],
        }
    }
}

/// A credential that never appears in `Debug` output (spec #35).
///
/// `Debug` prints one fixed marker and nothing else, so any struct holding a
/// `Secret` can be logged or `dbg!`ed safely. There is deliberately no
/// `Display`: the only way back to the value is [`Secret::expose`].
///
/// Construction is dumb and `Deserialize` is transparent, so a TOML section
/// deserializes straight into `Option<Secret<String>>`.
///
/// The derived `PartialEq` is not constant-time: it exists so `Config` stays
/// comparable, and it must never be used to check a presented credential.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T>(T);

impl Secret<String> {
    /// The raw value. Every call is a deliberate leak path: headers only.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl From<String> for Secret<String> {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Secret<String> {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// `--flag TOKEN` parses without going through `String` first.
impl std::str::FromStr for Secret<String> {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.to_owned()))
    }
}

/// Transparent: the wrapper exists for `Debug`, not for the wire format.
impl<'de> serde::Deserialize<'de> for Secret<String> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

impl<T> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Failure of a HackerOne API call, one variant per cause (ADR-0003).
#[derive(Debug, Error)]
pub enum Error {
    /// The API answered with a non-2xx status.
    #[error("hackerone api error {status}: {detail}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Trimmed response body, suitable for an MCP error message.
        detail: String,
        /// Raw response body; may be HTML from an intermediary.
        body: String,
    },
    /// A 2xx response body was larger than the read cap.
    #[error("response body exceeds {limit} bytes")]
    ResponseTooLarge {
        /// Cap that was exceeded, in bytes.
        limit: usize,
    },
    /// The request never produced a response (DNS, TLS, connect, read-timeout).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    /// A 2xx response body was not JSON.
    #[error("decoding response: {0}")]
    Decode(#[from] serde_json::Error),
    /// A caller-supplied argument was rejected before the request was sent.
    #[error("validation failed for {field}: {reason}")]
    Validation {
        /// Argument name.
        field: String,
        /// Why it was rejected.
        reason: String,
    },
    /// Required configuration is missing or malformed.
    #[error("invalid config {field}: {reason}")]
    Config {
        /// Configuration key name.
        field: String,
        /// Why it was rejected.
        reason: String,
    },
    /// No program matches the configured handle.
    #[error("program not found: {handle}")]
    ProgramNotFound {
        /// Program handle that was looked up.
        handle: String,
    },
    /// `HACKERONE_PROGRAM_HANDLE` was never set.
    #[error("missing program handle")]
    MissingProgramHandle,
    /// The requested tool's credential class has no complete token pair.
    #[error(
        "missing {} credential: set {} and {}",
        .class.as_str(),
        .class.env_vars()[0],
        .class.env_vars()[1]
    )]
    MissingCredential {
        /// Class the tool needs (ADR-0008).
        class: CredentialClass,
    },
}

/// Authenticated HackerOne REST client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    username: Secret<String>,
    token: Secret<String>,
    body_limit: usize,
}

impl Client {
    /// Build a client for the default API root with HTTP Basic credentials.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the HTTP client (TLS backend) cannot be built.
    pub fn new(
        username: impl Into<Secret<String>>,
        token: impl Into<Secret<String>>,
    ) -> Result<Self, Error> {
        let http = HttpClient::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            base_url: DEFAULT_BASE_URL.to_owned(),
            username: username.into(),
            token: token.into(),
            body_limit: MAX_BODY_BYTES,
        })
    }

    /// Test seam: rebuild the HTTP client with a shorter per-request timeout.
    ///
    /// Production timeouts are constants (ADR-0009); only tests call this.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the HTTP client cannot be rebuilt.
    #[doc(hidden)]
    pub fn with_request_timeout(mut self, timeout: Duration) -> Result<Self, Error> {
        self.http = HttpClient::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()?;
        Ok(self)
    }

    /// Test seam: override the 2xx body cap.
    ///
    /// Production uses [`MAX_BODY_BYTES`]; only tests call this.
    #[doc(hidden)]
    #[must_use]
    pub fn with_body_limit(mut self, limit: usize) -> Self {
        self.body_limit = limit;
        self
    }

    /// Point the client at another API root (tests, proxies, future versions).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// `GET path?query` with an optional JSON body.
    ///
    /// # Errors
    ///
    /// See [`Client::request`].
    pub async fn get(
        &self,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        self.request(Method::GET, path, query, body).await
    }

    /// `POST path?query` with an optional JSON body.
    ///
    /// # Errors
    ///
    /// See [`Client::request`].
    pub async fn post(
        &self,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        self.request(Method::POST, path, query, body).await
    }

    /// `PUT path?query` with an optional JSON body.
    ///
    /// # Errors
    ///
    /// See [`Client::request`].
    pub async fn put(
        &self,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        self.request(Method::PUT, path, query, body).await
    }

    /// `PATCH path?query` with an optional JSON body.
    ///
    /// # Errors
    ///
    /// See [`Client::request`].
    pub async fn patch(
        &self,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        self.request(Method::PATCH, path, query, body).await
    }

    /// `DELETE path?query` with an optional JSON body.
    ///
    /// # Errors
    ///
    /// See [`Client::request`].
    pub async fn delete(
        &self,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        self.request(Method::DELETE, path, query, body).await
    }

    /// Perform one request and decode the response.
    ///
    /// Response mapping: non-2xx → [`Error::Api`] with the body truncated to
    /// [`MAX_ERROR_BODY_BYTES`]; empty 2xx body → [`Value::Null`]
    /// (there is no JSON document to hand back, and `Null` keeps "no body" distinct
    /// from a `{}` document); unparsable 2xx body → [`Error::Decode`]; 2xx body past
    /// the cap → [`Error::ResponseTooLarge`]; no response at all →
    /// [`Error::Transport`].
    ///
    /// # Errors
    ///
    /// Returns any [`Error`] variant produced by transport, status, or decoding.
    async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, Error> {
        if !path.starts_with('/') || path.contains("..") {
            return Err(Error::Validation {
                field: "path".to_owned(),
                reason: format!("must be an absolute path without `..`: {path}"),
            });
        }
        let url = format!("{}{path}", self.base_url.trim_end_matches('/'));
        let mut request = self
            .http
            .request(method, url)
            .header(ACCEPT, "application/json")
            .basic_auth(self.username.expose(), Some(self.token.expose()))
            .query(query);
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let bytes = read_capped(response, MAX_ERROR_BODY_BYTES).await?;
            let body = truncate_utf8(&bytes, MAX_ERROR_BODY_BYTES);
            return Err(Error::Api {
                status: status.as_u16(),
                detail: body.trim().to_owned(),
                body,
            });
        }

        let bytes = read_capped(response, self.body_limit).await?;
        if bytes.len() > self.body_limit {
            return Err(Error::ResponseTooLarge {
                limit: self.body_limit,
            });
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}

/// Stream at most `limit + 1` body bytes, then drop the response (the rest of the
/// body is never read): one byte past the cap proves the body is too large without
/// buffering it. `Content-Length` is ignored — chunked and compressed bodies do not
/// carry a usable one.
///
/// The size check runs *before* each append: a single chunk may be several MiB, so
/// checking afterwards would let the buffer overshoot the cap.
async fn read_capped(mut response: Response, limit: usize) -> Result<Vec<u8>, Error> {
    let cap = limit.saturating_add(1);
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > cap {
            bytes.extend_from_slice(&chunk[..cap - bytes.len()]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Cut `bytes` to at most `limit` bytes on a character boundary, so the result is
/// always valid UTF-8 (a multi-byte character straddling the cut is dropped).
fn truncate_utf8(bytes: &[u8], limit: usize) -> String {
    let head = &bytes[..bytes.len().min(limit)];
    let cut = match std::str::from_utf8(head) {
        Ok(_) => head.len(),
        Err(error) => error.valid_up_to(),
    };
    String::from_utf8_lossy(&head[..cut]).into_owned()
}

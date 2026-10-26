//! Typed HackerOne REST client.
//!
//! No MCP types live here (see ADR-0002). Every method returns the decoded JSON
//! body as [`serde_json::Value`]; typed models are a later ticket, and decoding into
//! strict structs would turn sparse-but-valid payloads into failures.

use reqwest::header::ACCEPT;
use reqwest::{Client as HttpClient, Method};
use serde_json::Value;
use thiserror::Error;

/// Default HackerOne API root.
pub const DEFAULT_BASE_URL: &str = "https://api.hackerone.com/v1";

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
}

/// Authenticated HackerOne REST client.
#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    base_url: String,
    username: String,
    token: String,
}

impl Client {
    /// Build a client for the default API root with HTTP Basic credentials.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the HTTP client (TLS backend) cannot be built.
    pub fn new(username: impl Into<String>, token: impl Into<String>) -> Result<Self, Error> {
        let http = HttpClient::builder().build()?;
        Ok(Self {
            http,
            base_url: DEFAULT_BASE_URL.to_owned(),
            username: username.into(),
            token: token.into(),
        })
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
    /// Response mapping: non-2xx → [`Error::Api`]; empty 2xx body → [`Value::Null`]
    /// (there is no JSON document to hand back, and `Null` keeps "no body" distinct
    /// from a `{}` document); unparsable 2xx body → [`Error::Decode`]; no response at
    /// all → [`Error::Transport`].
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
        let url = format!("{}{path}", self.base_url.trim_end_matches('/'));
        let mut request = self
            .http
            .request(method, url)
            .header(ACCEPT, "application/json")
            .basic_auth(&self.username, Some(&self.token))
            .query(query);
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                detail: text.trim().to_owned(),
                body: text,
            });
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text)?)
    }
}

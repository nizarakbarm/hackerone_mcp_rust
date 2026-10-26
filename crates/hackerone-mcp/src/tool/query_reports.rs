//! `query_reports`: the configured program's report list.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;
use crate::config::Config;

/// `GET /reports` for the configured program; the query comes from the pure
/// builders, so no caller-supplied program filter can slip through.
///
/// # Errors
///
/// [`Error::MissingProgramHandle`] without a configured handle, then any
/// [`Error::Validation`] the builder raises, then any client error.
pub async fn run(
    client: &Client,
    config: &Config,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let handle = config
        .program_handle
        .as_deref()
        .ok_or(Error::MissingProgramHandle)?;
    let params = params::query_reports(handle, args)?;
    client.get("/reports", &params.query, None).await
}

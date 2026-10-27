//! `list_my_reports`: the authenticated hacker's own reports, paginated.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `GET /hackers/me/reports`.
///
/// # Errors
///
/// [`Error::Validation`] for bad pagination, then any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let query = params::pagination(args)?;
    client.get("/hackers/me/reports", &query.query, None).await
}

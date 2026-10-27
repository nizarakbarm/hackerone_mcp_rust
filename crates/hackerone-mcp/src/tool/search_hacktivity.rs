//! `search_hacktivity`: public hacktivity, filtered by a Lucene query.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `GET /hackers/hacktivity`.
///
/// The Lucene `queryString` is forwarded verbatim; only transport URL
/// encoding applies. `sort` defaults to the API's own
/// `-latest_disclosable_activity_at`.
///
/// # Errors
///
/// [`Error::Validation`] for wrongly typed arguments or bad pagination, then
/// any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let query = params::search_hacktivity(args)?;
    client.get("/hackers/hacktivity", &query.query, None).await
}

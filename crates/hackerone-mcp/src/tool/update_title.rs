//! `update_title`: rewrite one report's title.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `PATCH /reports/{report_id}` with only the title attribute.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id` or a
/// missing/non-string `title`, otherwise any error the client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::update_title(args)?;
    client
        .patch(&format!("/reports/{report_id}"), &params.query, params.body)
        .await
}

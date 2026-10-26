//! `get_report`: one `GET /reports/{id}`, body passed through untouched.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use crate::config::Config;

/// `GET /reports/{report_id}`; the raw JSON:API document is the result.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, otherwise any
/// error the client produced.
pub async fn run(
    client: &Client,
    _config: &Config,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    client
        .get(&format!("/reports/{report_id}"), &[], None)
        .await
}

//! `escalate_report`: escalate a report for higher-priority attention.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `POST /reports/{report_id}/escalate`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id` or a
/// missing/non-string `integration_id`, otherwise any error the client
/// produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::escalate_report(args)?;
    client
        .post(
            &format!("/reports/{report_id}/escalate"),
            &params.query,
            params.body,
        )
        .await
}

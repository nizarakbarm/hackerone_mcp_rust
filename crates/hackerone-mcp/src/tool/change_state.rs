//! `change_state`: one audited state transition on a report.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `POST /reports/{report_id}/state_changes`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id` or a bad
/// `state`/optional attribute, otherwise any error the client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::change_state(args)?;
    client
        .post(
            &format!("/reports/{report_id}/state_changes"),
            &params.query,
            params.body,
        )
        .await
}

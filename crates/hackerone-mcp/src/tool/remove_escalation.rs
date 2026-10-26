//! `remove_escalation`: drop a report's escalation.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

/// `DELETE /reports/{report_id}/escalate` with no body.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, otherwise any
/// error the client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    client
        .delete(&format!("/reports/{report_id}/escalate"), &[], None)
        .await
}

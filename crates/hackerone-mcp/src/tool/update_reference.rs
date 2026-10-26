//! `update_reference`: attach an external reference (e.g. a Jira ticket) to a
//! report.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `POST /reports/{report_id}/issue_tracker_reference_id`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, a
/// missing/non-string `reference`, or a non-string `message`, otherwise any
/// error the client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::update_reference(args)?;
    client
        .post(
            &format!("/reports/{report_id}/issue_tracker_reference_id"),
            &params.query,
            params.body,
        )
        .await
}

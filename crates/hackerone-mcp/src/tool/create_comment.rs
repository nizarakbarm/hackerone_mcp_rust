//! `create_comment`: add an internal (team-only) or external comment to a
//! report's activity timeline.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `POST /reports/{report_id}/activities`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, a missing
/// `message`, or a non-boolean `internal`, otherwise any error the client
/// produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::create_comment(args)?;
    client
        .post(
            &format!("/reports/{report_id}/activities"),
            &params.query,
            params.body,
        )
        .await
}

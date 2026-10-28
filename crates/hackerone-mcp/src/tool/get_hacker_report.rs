//! `get_hacker_report`: one report as its own reporter sees it, body passed
//! through untouched.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

/// `GET /hackers/reports/{report_id}`; the raw JSON:API document is the result.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, otherwise any
/// error the client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    client
        .get(&format!("/hackers/reports/{report_id}"), &[], None)
        .await
}

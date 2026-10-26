//! `update_cves`: replace the CVE identifiers associated with a report.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;

/// `PUT /reports/{report_id}/cves`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, a
/// missing/non-array `cve_ids`, or a non-string member, otherwise any error the
/// client produced.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let params = params::update_cves(args)?;
    client
        .put(
            &format!("/reports/{report_id}/cves"),
            &params.query,
            params.body,
        )
        .await
}

//! `update_custom_field_values`: one write per attribute, each reported
//! separately so a single bad attribute cannot hide the rest.
//!
//! ADR-0001 wart fixes 1-2: attribute ids are typed here instead of coerced by
//! the API layer, and a non-numeric key is rejected before any request. An
//! empty `custom_fields` object is a successful no-op: `{"results": []}` and
//! zero requests, which is exactly what upstream's empty loop returns.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value, json};

use super::params;
use crate::config::Config;

/// Sequential `POST /reports/{report_id}/custom_field_values`, ascending by
/// attribute id.
///
/// Returns `{"results": [{"attribute_id", "ok", "error"}, …]}` in the same
/// order. A failed write records its cause in `error` and does not stop the
/// later attributes; `error` is `null` on success. Only argument validation
/// fails the whole call.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id` or invalid
/// `custom_fields` (including a non-numeric key), never for a failed write.
pub async fn run(
    client: &Client,
    _config: &Config,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let writes = params::update_custom_field_values(args)?;

    let mut results = Vec::with_capacity(writes.len());
    for write in writes {
        let attribute_id = write
            .body
            .as_ref()
            .and_then(|body| body.pointer("/data/attributes/custom_field_attribute_id"))
            .cloned()
            .unwrap_or(Value::Null);
        let error = client
            .post(
                &format!("/reports/{report_id}/custom_field_values"),
                &write.query,
                write.body,
            )
            .await
            .err()
            .map(|error| error.to_string());
        results.push(json!({
            "attribute_id": attribute_id,
            "ok": error.is_none(),
            "error": error,
        }));
    }

    Ok(json!({"results": results}))
}

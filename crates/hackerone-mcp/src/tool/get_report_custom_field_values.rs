//! `get_report_custom_field_values`: one report read, reshaped into the flat
//! field list upstream clients expect.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value, json};

use super::decode_error;
use crate::config::Config;

/// `GET /reports/{report_id}` normalized by [`normalize`].
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-integer `report_id`, [`Error::Decode`]
/// for a payload that does not have the expected shape, then any client error.
pub async fn run(
    client: &Client,
    _config: &Config,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let report_id = super::report_id(args)?;
    let response = client
        .get(&format!("/reports/{report_id}"), &[], None)
        .await?;
    normalize(&response, report_id)
}

/// Upstream `_extract_report_custom_field_values`, minus the `{}`/`[]`
/// coercion: a malformed payload is a decode error naming the offending path.
fn normalize(response: &Value, report_id: i64) -> Result<Value, Error> {
    let report = response
        .get("data")
        .ok_or_else(|| decode_error("data", "is missing"))?
        .as_object()
        .ok_or_else(|| decode_error("data", "must be an object"))?;
    let relationships = optional_object(report.get("relationships"), "data.relationships")?;

    let mut custom_fields = Vec::new();
    let mut values_by_attribute_id = Map::new();
    let mut values_by_label = Map::new();

    for (index, raw_value) in custom_field_values(relationships)?.iter().enumerate() {
        let path = format!("data.relationships.custom_field_values.data[{index}]");
        let raw_value = raw_value
            .as_object()
            .ok_or_else(|| decode_error(&path, "must be an object"))?;
        let value_attributes =
            optional_object(raw_value.get("attributes"), &format!("{path}.attributes"))?;
        let attribute = custom_field_attribute(raw_value, &path)?;
        let attribute_attributes = optional_object(
            attribute.and_then(|attribute| attribute.get("attributes")),
            &format!("{path}.relationships.custom_field_attribute.data.attributes"),
        )?;

        custom_fields.push(json!({
            "value_id": field(Some(raw_value), "id"),
            "value": field(value_attributes, "value"),
            "value_created_at": field(value_attributes, "created_at"),
            "value_updated_at": field(value_attributes, "updated_at"),
            "attribute_id": field(attribute, "id"),
            "attribute_label": field(attribute_attributes, "label"),
            "field_type": field(attribute_attributes, "field_type"),
            "internal": field(attribute_attributes, "internal"),
            "required": field(attribute_attributes, "required"),
            "helper_text": field(attribute_attributes, "helper_text"),
            "configuration": field(attribute_attributes, "configuration"),
            "checkbox_text": field(attribute_attributes, "checkbox_text"),
            "regex": field(attribute_attributes, "regex"),
            "error_message": field(attribute_attributes, "error_message"),
            "attribute_created_at": field(attribute_attributes, "created_at"),
            "attribute_updated_at": field(attribute_attributes, "updated_at"),
            "attribute_archived_at": field(attribute_attributes, "archived_at"),
        }));

        if let Some(attribute_id) = attribute
            .and_then(|attribute| attribute.get("id"))
            .filter(|id| !id.is_null())
        {
            values_by_attribute_id.insert(json_key(attribute_id), field(value_attributes, "value"));
        }
        if let Some(Value::String(label)) =
            attribute_attributes.and_then(|attributes| attributes.get("label"))
            && !label.is_empty()
        {
            values_by_label.insert(label.clone(), field(value_attributes, "value"));
        }
    }

    Ok(json!({
        "report_id": report_id,
        "custom_fields": custom_fields,
        "values_by_attribute_id": values_by_attribute_id,
        "values_by_label": values_by_label,
    }))
}

/// `relationships.custom_field_values.data`, empty when absent.
fn custom_field_values(relationships: Option<&Map<String, Value>>) -> Result<&[Value], Error> {
    const PATH: &str = "data.relationships.custom_field_values";
    let Some(container) =
        relationships.and_then(|relationships| relationships.get("custom_field_values"))
    else {
        return Ok(&[]);
    };
    let container = container
        .as_object()
        .ok_or_else(|| decode_error(PATH, "must be an object"))?;
    match container.get("data") {
        None | Some(Value::Null) => Ok(&[]),
        Some(values) => values.as_array().map(Vec::as_slice).ok_or_else(|| {
            decode_error(
                "data.relationships.custom_field_values.data",
                "must be an array",
            )
        }),
    }
}

/// `relationships.custom_field_attribute.data` of one custom field value.
fn custom_field_attribute<'a>(
    raw_value: &'a Map<String, Value>,
    path: &str,
) -> Result<Option<&'a Map<String, Value>>, Error> {
    let Some(relationships) = optional_object(
        raw_value.get("relationships"),
        &format!("{path}.relationships"),
    )?
    else {
        return Ok(None);
    };
    let path = format!("{path}.relationships.custom_field_attribute");
    let Some(wrapper) = optional_object(relationships.get("custom_field_attribute"), &path)? else {
        return Ok(None);
    };
    optional_object(wrapper.get("data"), &format!("{path}.data"))
}

fn optional_object<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<Option<&'a Map<String, Value>>, Error> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or_else(|| decode_error(path, "must be an object")),
    }
}

/// One field of an object, `null` when the object or field is absent.
fn field(attributes: Option<&Map<String, Value>>, key: &str) -> Value {
    attributes
        .and_then(|attributes| attributes.get(key))
        .cloned()
        .unwrap_or(Value::Null)
}

fn json_key(value: &Value) -> String {
    match value {
        Value::String(key) => key.clone(),
        other => other.to_string(),
    }
}

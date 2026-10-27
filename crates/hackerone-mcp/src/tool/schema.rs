//! `inputSchema` for every tool, built from small per-field helpers.
//!
//! Structure, argument names, descriptions, `required` lists, bounds and
//! `additionalProperties` are byte-parity with upstream (ADR-0007); enum
//! members come from the constants in the parent module so the schema and the
//! parameter validators cannot drift.

use serde_json::{Value, json};

use super::{
    ArrayFilterKind, LIST_SCOPES_DATE_FILTERS, QUERY_REPORT_ARRAY_FILTERS,
    QUERY_REPORT_DATE_FILTERS, REPORT_STATES, SEVERITIES, SORT_FIELDS, SORT_ORDERS, ToolName,
    params::MAX_PROGRAM_SCAN_PAGES,
};

/// `inputSchema` for `tool`.
#[must_use]
pub fn schema(tool: ToolName) -> Value {
    match tool {
        ToolName::GetReport => get_report(),
        ToolName::GetReportCustomFieldValues => get_report_custom_field_values(),
        ToolName::CreateReport => create_report(),
        ToolName::QueryReports => query_reports(),
        ToolName::ListScopes => list_scopes(),
        ToolName::ChangeState => change_state(),
        ToolName::CreateComment => create_comment(),
        ToolName::UpdateTitle => update_title(),
        ToolName::UpdateReference => update_reference(),
        ToolName::UpdateCves => update_cves(),
        ToolName::EscalateReport => escalate_report(),
        ToolName::RemoveEscalation => remove_escalation(),
        ToolName::UpdateCustomFieldValues => update_custom_field_values(),
        ToolName::SearchPrograms => search_programs(),
        ToolName::GetProgram => get_program(),
        ToolName::GetProgramStructuredScopes => get_program_structured_scopes(),
    }
}

/// Object schema with `required`, no `additionalProperties` (upstream default).
fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": properties, "required": required})
}

/// Object schema for the two paginated tools, which close their properties.
fn closed_object(properties: Value) -> Value {
    json!({"type": "object", "properties": properties, "additionalProperties": false})
}

/// Integer parameter, optionally bounded.
fn int_param(description: &str, minimum: Option<i64>, maximum: Option<i64>) -> Value {
    let mut param = json!({"type": "integer", "description": description});
    if let Some(minimum) = minimum {
        param["minimum"] = minimum.into();
    }
    if let Some(maximum) = maximum {
        param["maximum"] = maximum.into();
    }
    param
}

/// String parameter.
fn str_param(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

/// `date-time` string parameter.
fn date_time_param(description: &str) -> Value {
    json!({"type": "string", "format": "date-time", "description": description})
}

/// Boolean parameter.
fn bool_param(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

/// One member of `values`.
fn enum_values<'a>(values: impl IntoIterator<Item = &'a str>) -> Value {
    json!({"type": "string", "enum": values.into_iter().collect::<Vec<_>>()})
}

/// String parameter restricted to `values`.
fn enum_param<'a>(values: impl IntoIterator<Item = &'a str>, description: &str) -> Value {
    let mut param = enum_values(values);
    param["description"] = json!(description);
    param
}

/// Array of integers.
fn int_array_param(description: &str) -> Value {
    json!({"type": "array", "items": {"type": "integer"}, "description": description})
}

/// Array of strings.
fn str_array_param(description: &str) -> Value {
    json!({"type": "array", "items": {"type": "string"}, "description": description})
}

/// Array of strings restricted to `values`.
fn enum_array_param<'a>(values: impl IntoIterator<Item = &'a str>, description: &str) -> Value {
    json!({"type": "array", "items": enum_values(values), "description": description})
}

/// Free-form object whose values are strings.
fn object_param(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "additionalProperties": {"type": "string"},
    })
}

fn search_programs() -> Value {
    closed_object(json!({
        "page_size": int_param(
            "Number of programs per directory page (1-100). Defaults 25.",
            Some(1),
            Some(100),
        ),
        "max_pages": int_param(
            "Maximum directory pages to scan (1-20). Defaults 5. The directory has no \
             server-side filters, so filters only ever see the pages scanned.",
            Some(1),
            Some(MAX_PROGRAM_SCAN_PAGES),
        ),
        "offers_bounties": bool_param(
            "Only keep programs whose offers_bounties matches (client-side).",
        ),
        "state": str_param("Only keep programs whose state matches (client-side)."),
        "submission_state": str_param(
            "Only keep programs whose submission_state matches (client-side).",
        ),
    }))
}

fn get_program() -> Value {
    object(
        json!({"handle": str_param("Program handle, e.g. 'security'.")}),
        &["handle"],
    )
}

fn get_program_structured_scopes() -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "handle".to_owned(),
        str_param("Program handle, e.g. 'security'."),
    );
    properties.insert(
        "scope_id_gt".to_owned(),
        int_param(
            "Only scopes with IDs greater than this. The API returns at most 10 000 scopes \
             per query; to read beyond that cap, set this to the last scope ID seen and \
             repeat.",
            Some(0),
            None,
        ),
    );
    for filter in LIST_SCOPES_DATE_FILTERS {
        properties.insert(
            filter.argument.to_owned(),
            date_time_param(filter.description),
        );
    }
    properties.insert(
        "page_number".to_owned(),
        int_param("Page number to fetch.", Some(1), None),
    );
    properties.insert(
        "page_size".to_owned(),
        int_param("Number of scopes per page (1-100).", Some(1), Some(100)),
    );
    closed_object(Value::Object(properties))
}

fn get_report() -> Value {
    object(
        json!({"report_id": int_param("The report ID.", None, None)}),
        &["report_id"],
    )
}

fn get_report_custom_field_values() -> Value {
    object(
        json!({"report_id": int_param("The report ID.", None, None)}),
        &["report_id"],
    )
}

fn create_report() -> Value {
    object(
        json!({
            "title": str_param("Report title."),
            "vulnerability_information": str_param("Detailed vulnerability description (markdown)."),
            "severity_rating": enum_param(SEVERITIES, "Severity rating."),
            "weakness_id": int_param("CWE weakness ID (optional).", None, None),
            "impact": str_param("Impact description."),
            "source": str_param("Source-system label for tracking. Defaults to 'api'."),
            "structured_scope_id": int_param("Structured scope ID (optional).", None, None),
        }),
        &["title", "vulnerability_information", "impact"],
    )
}

fn query_reports() -> Value {
    let mut properties = serde_json::Map::new();
    for filter in QUERY_REPORT_ARRAY_FILTERS {
        let param = match filter.kind {
            ArrayFilterKind::Integers => int_array_param(filter.description),
            ArrayFilterKind::Strings => str_array_param(filter.description),
            ArrayFilterKind::States => enum_array_param(REPORT_STATES, filter.description),
            ArrayFilterKind::Severities => enum_array_param(SEVERITIES, filter.description),
        };
        properties.insert(filter.argument.to_owned(), param);
    }
    for filter in QUERY_REPORT_DATE_FILTERS {
        properties.insert(
            filter.argument.to_owned(),
            date_time_param(filter.description),
        );
    }
    properties.insert(
        "sort_by".to_owned(),
        enum_param(
            SORT_FIELDS.iter().map(|field| field.name),
            "Field to sort the report list by.",
        ),
    );
    properties.insert(
        "sort_order".to_owned(),
        enum_param(SORT_ORDERS, "Sort direction. Defaults to asc."),
    );
    properties.insert(
        "page_number".to_owned(),
        int_param("Page number to fetch.", Some(1), None),
    );
    properties.insert(
        "page_size".to_owned(),
        int_param("Number of reports per page (1-100).", Some(1), Some(100)),
    );

    closed_object(Value::Object(properties))
}

fn list_scopes() -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "scope_id_gt".to_owned(),
        int_param(
            "Only include scopes with IDs greater than this value.",
            Some(0),
            None,
        ),
    );
    for filter in LIST_SCOPES_DATE_FILTERS {
        properties.insert(
            filter.argument.to_owned(),
            date_time_param(filter.description),
        );
    }
    properties.insert(
        "page_number".to_owned(),
        int_param("Page number to fetch.", Some(1), None),
    );
    properties.insert(
        "page_size".to_owned(),
        int_param("Number of scopes per page (1-100).", Some(1), Some(100)),
    );

    closed_object(Value::Object(properties))
}

fn change_state() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "state": enum_param(REPORT_STATES, "Target state."),
            "message": str_param("Message to include with the state change (optional)."),
            "original_report_id": int_param(
                "Original report ID if marking as duplicate (optional).",
                None,
                None,
            ),
            "attachment_ids": int_array_param(
                "Attachment IDs to include with the state change (optional).",
            ),
        }),
        &["report_id", "state"],
    )
}

fn create_comment() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "message": str_param("Comment text (markdown)."),
            "internal": bool_param(
                "If true, comment is only visible to the team. Defaults to false.",
            ),
        }),
        &["report_id", "message"],
    )
}

fn update_title() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "title": str_param("New title."),
        }),
        &["report_id", "title"],
    )
}

fn update_reference() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "reference": str_param("External reference URL or identifier."),
            "message": str_param(
                "Internal message to include with the reference update (optional).",
            ),
        }),
        &["report_id", "reference"],
    )
}

fn update_cves() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "cve_ids": str_array_param("List of CVE IDs (e.g. ['CVE-2024-1234'])."),
        }),
        &["report_id", "cve_ids"],
    )
}

fn escalate_report() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "integration_id": str_param("Integration instance ID to escalate to."),
        }),
        &["report_id", "integration_id"],
    )
}

fn remove_escalation() -> Value {
    object(
        json!({"report_id": int_param("The report ID.", None, None)}),
        &["report_id"],
    )
}

fn update_custom_field_values() -> Value {
    object(
        json!({
            "report_id": int_param("The report ID.", None, None),
            "custom_fields": object_param(
                "Key-value pairs of custom field attribute_id to value.",
            ),
        }),
        &["report_id", "custom_fields"],
    )
}

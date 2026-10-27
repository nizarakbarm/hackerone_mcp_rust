//! Pure parameter builders: tool arguments in, validated query/body out.
//!
//! Nothing here touches the network or the config loader; callers pass the
//! configured program handle explicitly, which is precisely why a caller can
//! never override the program scope (ADR-0001, wart fix 6). Failures are
//! [`Error::Validation`], never ad-hoc error payloads (wart fix 4).

use std::collections::BTreeMap;

use hackerone_api::Error;
use serde_json::{Map, Value, json};

use super::{
    ArrayFilterKind, DateFilter, LIST_SCOPES_DATE_FILTERS, QUERY_REPORT_ARRAY_FILTERS,
    QUERY_REPORT_DATE_FILTERS, REPORT_STATES, SEVERITIES, SORT_FIELDS, SORT_ORDERS,
};

/// Wire parameters for one API call: repeated query pairs and/or a JSON body.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Params {
    /// Query pairs, in emission order. Repeated keys carry array filters.
    pub query: Vec<(String, String)>,
    /// JSON:API request body, when the call writes.
    pub body: Option<Value>,
}

/// `filter[program][]` query value for `query_reports`.
pub const PROGRAM_FILTER: &str = "filter[program][]";

/// `page_size` used by `search_programs` when the caller does not set one:
/// the directory's own default page length.
pub const DEFAULT_PROGRAM_PAGE_SIZE: i64 = 25;

/// `max_pages` used by `search_programs` when the caller does not set one.
pub const DEFAULT_PROGRAM_MAX_PAGES: i64 = 5;

/// Hard ceiling on `search_programs`' page budget.
pub const MAX_PROGRAM_SCAN_PAGES: i64 = 20;

/// A validated `search_programs` scan.
///
/// `/hackers/programs` has no server-side filters, so the filters live here
/// and are applied to each decoded program. Bounds are enforced here so the
/// scan loop can trust them.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramSearch {
    /// Page length requested from the API (1-100).
    pub page_size: i64,
    /// Maximum number of pages to request (1-20).
    pub max_pages: i64,
    /// Client-side `offers_bounties` filter.
    pub offers_bounties: Option<bool>,
    /// Client-side `state` filter.
    pub state: Option<String>,
    /// Client-side `submission_state` filter.
    pub submission_state: Option<String>,
}

impl ProgramSearch {
    /// Whether `program` passes every requested filter. A program missing the
    /// attribute a filter names never matches.
    #[must_use]
    pub fn matches(&self, program: &Value) -> bool {
        let attribute = |name: &str| {
            program
                .get("attributes")
                .and_then(|values| values.get(name))
        };
        self.offers_bounties.is_none_or(|wanted| {
            attribute("offers_bounties").and_then(Value::as_bool) == Some(wanted)
        }) && self
            .state
            .as_deref()
            .is_none_or(|wanted| attribute("state").and_then(Value::as_str) == Some(wanted))
            && self.submission_state.as_deref().is_none_or(|wanted| {
                attribute("submission_state").and_then(Value::as_str) == Some(wanted)
            })
    }
}

/// `GET /hackers/programs` scan parameters.
///
/// # Errors
///
/// [`Error::Validation`] for a `page_size` outside 1-100, a `max_pages`
/// outside 1-20, or a wrongly typed filter.
pub fn search_programs(args: &Map<String, Value>) -> Result<ProgramSearch, Error> {
    let page_size = optional_int(args, "page_size")?.unwrap_or(DEFAULT_PROGRAM_PAGE_SIZE);
    if !(1..=100).contains(&page_size) {
        return Err(validation("page_size", "must be between 1 and 100"));
    }
    let max_pages = optional_int(args, "max_pages")?.unwrap_or(DEFAULT_PROGRAM_MAX_PAGES);
    if !(1..=MAX_PROGRAM_SCAN_PAGES).contains(&max_pages) {
        return Err(validation("max_pages", "must be between 1 and 20"));
    }
    Ok(ProgramSearch {
        page_size,
        max_pages,
        offers_bounties: optional_bool(args, "offers_bounties")?,
        state: optional_str(args, "state")?.map(str::to_owned),
        submission_state: optional_str(args, "submission_state")?.map(str::to_owned),
    })
}

/// Required `handle` argument, restricted to a single path segment so it can
/// never address another endpoint.
///
/// # Errors
///
/// [`Error::Validation`] when `handle` is missing, not a string, empty, or
/// contains anything but ASCII alphanumerics, `-` and `_`.
pub fn program_handle(args: &Map<String, Value>) -> Result<&str, Error> {
    let handle = required_str(args, "handle")?;
    let valid = !handle.is_empty()
        && handle
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if !valid {
        return Err(validation("handle", "must be a program handle"));
    }
    Ok(handle)
}

/// `GET /reports` parameters for the configured program.
///
/// # Errors
///
/// [`Error::Validation`] for unknown enum members, non-integer values, page
/// bounds, or `sort_order` without `sort_by`.
pub fn query_reports(program_handle: &str, args: &Map<String, Value>) -> Result<Params, Error> {
    let mut query = vec![(PROGRAM_FILTER.to_owned(), program_handle.to_owned())];

    for filter in QUERY_REPORT_ARRAY_FILTERS {
        let Some(value) = args.get(filter.argument) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let Value::Array(items) = value else {
            return Err(validation(filter.argument, "must be an array"));
        };
        for item in items {
            let item = match filter.kind {
                ArrayFilterKind::Integers => item
                    .as_i64()
                    .map(|value| value.to_string())
                    .ok_or_else(|| validation(filter.argument, "must contain only integers"))?,
                ArrayFilterKind::Strings => item
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| validation(filter.argument, "must contain only strings"))?,
                ArrayFilterKind::States => enum_member(item, &REPORT_STATES, filter.argument)?,
                ArrayFilterKind::Severities => enum_member(item, &SEVERITIES, filter.argument)?,
            };
            query.push((filter.query.to_owned(), item));
        }
    }

    push_dates(&mut query, args, &QUERY_REPORT_DATE_FILTERS)?;
    push_pagination(&mut query, args)?;

    let sort_by = optional_str(args, "sort_by")?;
    let sort_order = optional_str(args, "sort_order")?;
    if sort_order.is_some() && sort_by.is_none() {
        return Err(validation("sort_order", "requires sort_by"));
    }
    if let Some(sort_by) = sort_by {
        let field = SORT_FIELDS
            .iter()
            .find(|field| field.name == sort_by)
            .ok_or_else(|| {
                validation(
                    "sort_by",
                    format!(
                        "must be one of: {}",
                        SORT_FIELDS
                            .iter()
                            .map(|field| field.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            })?;
        let order = match sort_order {
            None => "asc",
            Some(order) if SORT_ORDERS.contains(&order) => order,
            Some(_) => return Err(validation("sort_order", "must be 'asc' or 'desc'")),
        };
        query.push((
            "sort".to_owned(),
            if order == "desc" {
                format!("-{}", field.query)
            } else {
                field.query.to_owned()
            },
        ));
    }

    Ok(Params { query, body: None })
}

/// `GET /programs/{id}/structured_scopes` parameters.
///
/// # Errors
///
/// [`Error::Validation`] for a negative `scope_id_gt`, non-integer values, or
/// page bounds.
pub fn list_scopes(args: &Map<String, Value>) -> Result<Params, Error> {
    let mut query = Vec::new();

    if let Some(scope_id_gt) = optional_int(args, "scope_id_gt")? {
        if scope_id_gt < 0 {
            return Err(validation(
                "scope_id_gt",
                "must be greater than or equal to 0",
            ));
        }
        query.push(("filter[id__gt]".to_owned(), scope_id_gt.to_string()));
    }

    push_dates(&mut query, args, &LIST_SCOPES_DATE_FILTERS)?;
    push_pagination(&mut query, args)?;

    Ok(Params { query, body: None })
}

/// `POST /reports` body for the configured program.
///
/// `team_handle` is taken from `program_handle` only: any caller-supplied
/// handle is ignored, so a tool call cannot import into another program.
///
/// # Errors
///
/// [`Error::Validation`] for missing required strings, a non-integer id, or an
/// unknown `severity_rating`.
pub fn create_report(program_handle: &str, args: &Map<String, Value>) -> Result<Params, Error> {
    let title = required_str(args, "title")?;
    let vulnerability_information = required_str(args, "vulnerability_information")?;
    let impact = required_str(args, "impact")?;
    let source = optional_str(args, "source")?.unwrap_or("api");

    let mut attributes = Map::new();
    attributes.insert("title".to_owned(), json!(title));
    attributes.insert(
        "vulnerability_information".to_owned(),
        json!(vulnerability_information),
    );
    attributes.insert("impact".to_owned(), json!(impact));
    attributes.insert("source".to_owned(), json!(source));
    attributes.insert("team_handle".to_owned(), json!(program_handle));

    if let Some(severity_rating) = optional_str(args, "severity_rating")? {
        check_enum("severity_rating", severity_rating, &SEVERITIES)?;
        attributes.insert("severity_rating".to_owned(), json!(severity_rating));
    }
    if let Some(weakness_id) = optional_int(args, "weakness_id")? {
        attributes.insert("weakness_id".to_owned(), json!(weakness_id));
    }
    if let Some(structured_scope_id) = optional_int(args, "structured_scope_id")? {
        attributes.insert("structured_scope_id".to_owned(), json!(structured_scope_id));
    }

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "report", "attributes": attributes},
        })),
    })
}

/// `POST /reports/{id}/state_changes` body.
///
/// # Errors
///
/// [`Error::Validation`] for a missing or unknown `state`, or a wrongly typed
/// optional attribute.
pub fn change_state(args: &Map<String, Value>) -> Result<Params, Error> {
    let state = required_str(args, "state")?;
    check_enum("state", state, &REPORT_STATES)?;

    let mut attributes = Map::new();
    attributes.insert("state".to_owned(), json!(state));

    if let Some(message) = optional_str(args, "message")? {
        attributes.insert("message".to_owned(), json!(message));
    }
    if let Some(original_report_id) = optional_int(args, "original_report_id")? {
        attributes.insert("original_report_id".to_owned(), json!(original_report_id));
    }
    if let Some(attachment_ids) = optional_int_array(args, "attachment_ids")? {
        attributes.insert("attachment_ids".to_owned(), json!(attachment_ids));
    }

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "state-change", "attributes": attributes},
        })),
    })
}

/// `POST /reports/{id}/activities` body.
///
/// `internal` is omitted entirely unless it is `true`, matching upstream: an
/// explicit `false` never reaches the API.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-string `message` or a non-boolean
/// `internal`.
pub fn create_comment(args: &Map<String, Value>) -> Result<Params, Error> {
    let message = required_str(args, "message")?;

    let mut attributes = Map::new();
    attributes.insert("message".to_owned(), json!(message));
    if optional_bool(args, "internal")?.unwrap_or(false) {
        attributes.insert("internal".to_owned(), json!(true));
    }

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "activity-comment", "attributes": attributes},
        })),
    })
}

/// `PATCH /reports/{id}` body: the title and nothing else.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-string `title`.
pub fn update_title(args: &Map<String, Value>) -> Result<Params, Error> {
    let title = required_str(args, "title")?;

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "report", "attributes": {"title": title}},
        })),
    })
}

/// `POST /reports/{id}/issue_tracker_reference_id` body.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-string `reference` or a non-string
/// `message`.
pub fn update_reference(args: &Map<String, Value>) -> Result<Params, Error> {
    let reference = required_str(args, "reference")?;

    let mut attributes = Map::new();
    attributes.insert("reference".to_owned(), json!(reference));
    if let Some(message) = optional_str(args, "message")? {
        attributes.insert("message".to_owned(), json!(message));
    }

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "issue-tracker-reference-id", "attributes": attributes},
        })),
    })
}

/// `PUT /reports/{id}/cves` body; `cve_ids` keeps the caller's order.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-array `cve_ids` or a non-string
/// member.
pub fn update_cves(args: &Map<String, Value>) -> Result<Params, Error> {
    let cve_ids = required_str_array(args, "cve_ids")?;

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"type": "report-cves", "attributes": {"cve_ids": cve_ids}},
        })),
    })
}

/// `POST /reports/{id}/escalate` body: upstream sends no `data.type`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing/non-string `integration_id`.
pub fn escalate_report(args: &Map<String, Value>) -> Result<Params, Error> {
    let integration_id = required_str(args, "integration_id")?;

    Ok(Params {
        query: Vec::new(),
        body: Some(json!({
            "data": {"attributes": {"integration_id": integration_id}},
        })),
    })
}

/// One `POST /reports/{id}/custom_field_values` body per attribute, ascending
/// by attribute id (ADR-0001 wart fixes 1-2).
///
/// # Errors
///
/// [`Error::Validation`] when `custom_fields` is missing, not an object, has a
/// key that is not a `u32`, or has a non-string value.
pub fn update_custom_field_values(args: &Map<String, Value>) -> Result<Vec<Params>, Error> {
    let Some(custom_fields) = args.get("custom_fields") else {
        return Err(validation("custom_fields", "is required"));
    };
    if custom_fields.is_null() {
        return Err(validation("custom_fields", "is required"));
    }
    let Value::Object(custom_fields) = custom_fields else {
        return Err(validation("custom_fields", "must be an object"));
    };

    let mut values = BTreeMap::new();
    for (attribute_id, value) in custom_fields {
        let attribute_id: u32 = attribute_id.parse().map_err(|_| {
            validation(
                "custom_fields",
                format!("key {attribute_id:?} is not a non-negative integer"),
            )
        })?;
        let Value::String(value) = value else {
            return Err(validation(
                "custom_fields",
                format!("value for {attribute_id} must be a string"),
            ));
        };
        values.insert(attribute_id, value.clone());
    }

    Ok(values
        .into_iter()
        .map(|(attribute_id, value)| Params {
            query: Vec::new(),
            body: Some(json!({
                "data": {
                    "attributes": {
                        "custom_field_attribute_id": attribute_id,
                        "value": value,
                    }
                },
            })),
        })
        .collect())
}

fn validation(field: &str, reason: impl Into<String>) -> Error {
    Error::Validation {
        field: field.to_owned(),
        reason: reason.into(),
    }
}

fn check_enum(field: &str, value: &str, allowed: &[&str]) -> Result<(), Error> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(validation(
            field,
            format!("must be one of: {}", allowed.join(", ")),
        ))
    }
}

fn enum_member(value: &Value, allowed: &[&str], field: &str) -> Result<String, Error> {
    let value = value
        .as_str()
        .ok_or_else(|| validation(field, "must contain only strings"))?;
    check_enum(field, value, allowed)?;
    Ok(value.to_owned())
}

fn required_str<'a>(args: &'a Map<String, Value>, field: &str) -> Result<&'a str, Error> {
    match args.get(field) {
        Some(Value::String(value)) => Ok(value),
        Some(_) => Err(validation(field, "must be a string")),
        None => Err(validation(field, "is required")),
    }
}

fn optional_str<'a>(args: &'a Map<String, Value>, field: &str) -> Result<Option<&'a str>, Error> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(validation(field, "must be a string")),
    }
}

fn optional_bool(args: &Map<String, Value>, field: &str) -> Result<Option<bool>, Error> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(validation(field, "must be a boolean")),
    }
}

fn optional_int(args: &Map<String, Value>, field: &str) -> Result<Option<i64>, Error> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| validation(field, "must be an integer")),
    }
}

fn required_str_array(args: &Map<String, Value>, field: &str) -> Result<Vec<String>, Error> {
    match args.get(field) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| validation(field, "must contain only strings"))
            })
            .collect(),
        Some(_) => Err(validation(field, "must be an array")),
        None => Err(validation(field, "is required")),
    }
}

fn optional_int_array(args: &Map<String, Value>, field: &str) -> Result<Option<Vec<i64>>, Error> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_i64()
                    .ok_or_else(|| validation(field, "must contain only integers"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(validation(field, "must be an array")),
    }
}

/// `sort` sent by `search_hacktivity` when the caller does not set one: the
/// API's own default, newest disclosable activity first.
pub const DEFAULT_HACKTIVITY_SORT: &str = "-latest_disclosable_activity_at";

/// Query parameters for a pagination-only read (`list_my_reports`,
/// `get_program_weaknesses`).
///
/// # Errors
///
/// [`Error::Validation`] for non-integer or out-of-range pagination.
pub fn pagination(args: &Map<String, Value>) -> Result<Params, Error> {
    let mut query = Vec::new();
    push_pagination(&mut query, args)?;
    Ok(Params { query, body: None })
}

/// `GET /hackers/hacktivity` parameters.
///
/// `queryString` is Apache Lucene and is passed through verbatim: it is never
/// parsed, grammar-checked or re-encoded beyond transport URL encoding.
///
/// # Errors
///
/// [`Error::Validation`] for a non-string `queryString`/`sort` or out-of-range
/// pagination.
pub fn search_hacktivity(args: &Map<String, Value>) -> Result<Params, Error> {
    let mut query = Vec::new();
    if let Some(query_string) = optional_str(args, "queryString")? {
        query.push(("queryString".to_owned(), query_string.to_owned()));
    }
    query.push((
        "sort".to_owned(),
        optional_str(args, "sort")?
            .unwrap_or(DEFAULT_HACKTIVITY_SORT)
            .to_owned(),
    ));
    push_pagination(&mut query, args)?;
    Ok(Params { query, body: None })
}

fn push_dates(
    query: &mut Vec<(String, String)>,
    args: &Map<String, Value>,
    filters: &[DateFilter],
) -> Result<(), Error> {
    for filter in filters {
        match args.get(filter.argument) {
            None | Some(Value::Null) => {}
            Some(Value::String(value)) => {
                query.push((filter.query.to_owned(), value.clone()));
            }
            Some(_) => return Err(validation(filter.argument, "must be a date-time string")),
        }
    }
    Ok(())
}

fn push_pagination(
    query: &mut Vec<(String, String)>,
    args: &Map<String, Value>,
) -> Result<(), Error> {
    if let Some(page_number) = optional_int(args, "page_number")? {
        if page_number < 1 {
            return Err(validation(
                "page_number",
                "must be greater than or equal to 1",
            ));
        }
        query.push(("page[number]".to_owned(), page_number.to_string()));
    }
    if let Some(page_size) = optional_int(args, "page_size")? {
        if !(1..=100).contains(&page_size) {
            return Err(validation("page_size", "must be between 1 and 100"));
        }
        query.push(("page[size]".to_owned(), page_size.to_string()));
    }
    Ok(())
}

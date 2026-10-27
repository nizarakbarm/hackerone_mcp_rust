//! Builder seam: every filter group, both sort orders, page bounds, typed
//! validation errors, schema parity with upstream, and the program-scope
//! guarantee from ADR-0001 (wart fix 6).

use hackerone_api::{CredentialClass, Error};
use hackerone_mcp::tool::{
    self, ArrayFilterKind, LIST_SCOPES_DATE_FILTERS, QUERY_REPORT_ARRAY_FILTERS,
    QUERY_REPORT_DATE_FILTERS, REPORT_STATES, SEVERITIES, SORT_FIELDS, SORT_ORDERS, ToolName,
};
use serde_json::{Map, Value, json};

/// Upstream `tools/list` payload, reduced to names and `inputSchema`.
const UPSTREAM_SCHEMAS: &str = include_str!("fixtures/upstream_tool_schemas.json");

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected an object, got {other}"),
    }
}

fn query(params: &tool::params::Params) -> Vec<(String, String)> {
    params
        .query
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn expected(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn validation_field(error: Error) -> String {
    match error {
        Error::Validation { field, .. } => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn tool_names_are_all_advertised_in_order() {
    let expected = [
        "get_report",
        "get_report_custom_field_values",
        "create_report",
        "query_reports",
        "list_scopes",
        "change_state",
        "create_comment",
        "update_title",
        "update_reference",
        "update_cves",
        "escalate_report",
        "remove_escalation",
        "update_custom_field_values",
        "search_programs",
        "get_program",
        "get_program_structured_scopes",
    ];

    assert_eq!(ToolName::ALL.len(), expected.len());
    for (tool, name) in ToolName::ALL.into_iter().zip(expected) {
        assert_eq!(tool.name(), name);
        assert_eq!(ToolName::from_wire(name), Some(tool));
    }
    assert_eq!(ToolName::from_wire("program_override"), None);
}

#[test]
fn schemas_match_upstream_byte_for_byte() {
    let upstream: Vec<Value> = serde_json::from_str(UPSTREAM_SCHEMAS).expect("fixture parses");
    assert_eq!(upstream.len(), 13, "the fixture is the upstream thirteen");

    for entry in upstream {
        let name = entry["name"].as_str().expect("fixture tool name");
        let tool = ToolName::from_wire(name).expect("fixture tool exists");
        assert_eq!(
            serde_json::to_string(&tool.schema()).expect("serialize schema"),
            serde_json::to_string(&entry["inputSchema"]).expect("serialize fixture"),
            "schema drift for {name}"
        );
    }
}

#[test]
fn no_schema_exposes_a_program_override() {
    for tool in ToolName::ALL {
        let schema = tool.schema();
        let properties = schema["properties"].as_object().expect("properties object");
        for forbidden in [
            "program",
            "program_handle",
            "team_handle",
            "filter[program][]",
        ] {
            assert!(
                !properties.contains_key(forbidden),
                "{} exposes {forbidden}",
                tool.name()
            );
        }
    }
}

#[test]
fn schema_properties_cover_every_filter_constant() {
    let schema = ToolName::QueryReports.schema();
    let properties = schema["properties"].as_object().expect("properties");

    for filter in QUERY_REPORT_ARRAY_FILTERS {
        assert!(
            properties.contains_key(filter.argument),
            "missing array filter {}",
            filter.argument
        );
        assert_eq!(
            properties[filter.argument]["type"], "array",
            "{} is not an array",
            filter.argument
        );
    }
    for filter in QUERY_REPORT_DATE_FILTERS {
        assert!(
            properties.contains_key(filter.argument),
            "missing date filter {}",
            filter.argument
        );
        assert_eq!(properties[filter.argument]["format"], "date-time");
    }

    let scopes = ToolName::ListScopes.schema();
    let scope_properties = scopes["properties"].as_object().expect("properties");
    for filter in LIST_SCOPES_DATE_FILTERS {
        assert!(scope_properties.contains_key(filter.argument));
    }
}

#[test]
fn schema_enums_are_the_validation_constants() {
    let query_reports = ToolName::QueryReports.schema();
    let properties = query_reports["properties"].as_object().expect("properties");
    assert_eq!(properties["states"]["items"]["enum"], json!(REPORT_STATES));
    assert_eq!(properties["severities"]["items"]["enum"], json!(SEVERITIES));
    assert_eq!(
        properties["sort_by"]["enum"],
        json!(
            SORT_FIELDS
                .iter()
                .map(|field| field.name)
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(properties["sort_order"]["enum"], json!(SORT_ORDERS));

    let change_state = ToolName::ChangeState.schema();
    assert_eq!(
        change_state["properties"]["state"]["enum"],
        json!(REPORT_STATES)
    );
}

#[test]
fn query_reports_emits_every_filter_group() {
    let arguments = args(json!({
        "inbox_ids": [1, 2],
        "reporters": ["alice"],
        "assignees": ["bob", "carol"],
        "states": ["new", "triaged"],
        "report_ids": [7],
        "severities": ["high"],
        "created_after": "2026-01-01T00:00:00Z",
        "created_before": "2026-02-01T00:00:00Z",
        "submitted_after": "2026-01-02T00:00:00Z",
        "submitted_before": "2026-02-02T00:00:00Z",
        "triaged_after": "2026-01-03T00:00:00Z",
        "triaged_before": "2026-02-03T00:00:00Z",
        "closed_after": "2026-01-04T00:00:00Z",
        "closed_before": "2026-02-04T00:00:00Z",
        "last_activity_after": "2026-01-05T00:00:00Z",
        "last_activity_before": "2026-02-05T00:00:00Z",
        "sort_by": "created_at",
        "sort_order": "desc",
        "page_number": 2,
        "page_size": 50,
    }));

    let params = tool::params::query_reports("acme", &arguments).expect("valid arguments");

    assert_eq!(
        query(&params),
        expected(&[
            ("filter[program][]", "acme"),
            ("filter[inbox_ids][]", "1"),
            ("filter[inbox_ids][]", "2"),
            ("filter[reporter][]", "alice"),
            ("filter[assignee][]", "bob"),
            ("filter[assignee][]", "carol"),
            ("filter[state][]", "new"),
            ("filter[state][]", "triaged"),
            ("filter[id][]", "7"),
            ("filter[severity][]", "high"),
            ("filter[created_at__gt]", "2026-01-01T00:00:00Z"),
            ("filter[created_at__lt]", "2026-02-01T00:00:00Z"),
            ("filter[submitted_at__gt]", "2026-01-02T00:00:00Z"),
            ("filter[submitted_at__lt]", "2026-02-02T00:00:00Z"),
            ("filter[triaged_at__gt]", "2026-01-03T00:00:00Z"),
            ("filter[triaged_at__lt]", "2026-02-03T00:00:00Z"),
            ("filter[closed_at__gt]", "2026-01-04T00:00:00Z"),
            ("filter[closed_at__lt]", "2026-02-04T00:00:00Z"),
            ("filter[last_activity_at__gt]", "2026-01-05T00:00:00Z"),
            ("filter[last_activity_at__lt]", "2026-02-05T00:00:00Z"),
            ("page[number]", "2"),
            ("page[size]", "50"),
            ("sort", "-reports.created_at"),
        ])
    );
    assert_eq!(params.body, None);
}

#[test]
fn query_reports_array_filter_kinds_cover_all_six() {
    let kinds: Vec<ArrayFilterKind> = QUERY_REPORT_ARRAY_FILTERS
        .iter()
        .map(|filter| filter.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            ArrayFilterKind::Integers,
            ArrayFilterKind::Strings,
            ArrayFilterKind::Strings,
            ArrayFilterKind::States,
            ArrayFilterKind::Integers,
            ArrayFilterKind::Severities,
        ]
    );
}

#[test]
fn sort_maps_every_field_and_both_orders() {
    for field in SORT_FIELDS {
        let ascending = tool::params::query_reports(
            "acme",
            &args(json!({"sort_by": field.name, "sort_order": "asc"})),
        )
        .expect("valid sort");
        assert_eq!(
            query(&ascending),
            expected(&[("filter[program][]", "acme"), ("sort", field.query)])
        );

        let descending = tool::params::query_reports(
            "acme",
            &args(json!({"sort_by": field.name, "sort_order": "desc"})),
        )
        .expect("valid sort");
        assert_eq!(
            query(&descending),
            expected(&[
                ("filter[program][]", "acme"),
                ("sort", &format!("-{}", field.query)),
            ])
        );
    }

    let default_order =
        tool::params::query_reports("acme", &args(json!({"sort_by": "created_at"})))
            .expect("valid sort");
    assert_eq!(
        query(&default_order),
        expected(&[
            ("filter[program][]", "acme"),
            ("sort", "reports.created_at"),
        ])
    );
}

#[test]
fn sort_order_without_sort_by_is_a_validation_error() {
    let error = tool::params::query_reports("acme", &args(json!({"sort_order": "desc"})))
        .expect_err("sort_order alone must fail");
    assert_eq!(validation_field(error), "sort_order");
}

#[test]
fn unknown_sort_field_and_order_are_validation_errors() {
    let field_error = tool::params::query_reports("acme", &args(json!({"sort_by": "nope"})))
        .expect_err("unknown sort field");
    assert_eq!(validation_field(field_error), "sort_by");

    let order_error = tool::params::query_reports(
        "acme",
        &args(json!({"sort_by": "created_at", "sort_order": "sideways"})),
    )
    .expect_err("unknown sort order");
    assert_eq!(validation_field(order_error), "sort_order");
}

#[test]
fn page_bounds_are_one_to_one_hundred() {
    for page_size in [1, 100] {
        let params = tool::params::query_reports("acme", &args(json!({"page_size": page_size})))
            .expect("page size in range");
        assert_eq!(
            query(&params),
            expected(&[
                ("filter[program][]", "acme"),
                ("page[size]", &page_size.to_string()),
            ])
        );
    }
    for page_size in [0, 101] {
        let error = tool::params::query_reports("acme", &args(json!({"page_size": page_size})))
            .expect_err("page size out of range");
        assert_eq!(validation_field(error), "page_size");
    }

    let error = tool::params::query_reports("acme", &args(json!({"page_number": 0})))
        .expect_err("page number below one");
    assert_eq!(validation_field(error), "page_number");
    let params = tool::params::query_reports("acme", &args(json!({"page_number": 1})))
        .expect("page number one");
    assert_eq!(
        query(&params),
        expected(&[("filter[program][]", "acme"), ("page[number]", "1")])
    );
}

#[test]
fn unknown_enum_members_are_validation_errors() {
    let state_error =
        tool::params::change_state(&args(json!({"state": "archived"}))).expect_err("unknown state");
    assert_eq!(validation_field(state_error), "state");

    let filter_error =
        tool::params::query_reports("acme", &args(json!({"states": ["triaged", "archived"]})))
            .expect_err("unknown state filter");
    assert_eq!(validation_field(filter_error), "states");
}

#[test]
fn custom_field_keys_are_typed_and_ordered() {
    let writes = tool::params::update_custom_field_values(&args(json!({
        "custom_fields": {"10": "ten", "2": "two"},
    })))
    .expect("valid custom fields");

    assert_eq!(writes.len(), 2);
    assert_eq!(
        writes[0].body,
        Some(json!({
            "data": {"attributes": {"custom_field_attribute_id": 2, "value": "two"}},
        }))
    );
    assert_eq!(
        writes[1].body,
        Some(json!({
            "data": {"attributes": {"custom_field_attribute_id": 10, "value": "ten"}},
        }))
    );

    let error = tool::params::update_custom_field_values(&args(json!({
        "custom_fields": {"not-a-number": "x"},
    })))
    .expect_err("non-numeric key");
    assert_eq!(validation_field(error), "custom_fields");

    let value_error = tool::params::update_custom_field_values(&args(json!({
        "custom_fields": {"1": 42},
    })))
    .expect_err("non-string value");
    assert_eq!(validation_field(value_error), "custom_fields");
}

#[test]
fn caller_cannot_override_the_program_scope() {
    let arguments = args(json!({
        "filter[program][]": ["evil"],
        "program": "evil",
        "program_handle": "evil",
        "team_handle": "evil",
        "title": "t",
        "vulnerability_information": "v",
        "impact": "i",
        "sort_by": "created_at",
    }));

    let params = tool::params::query_reports("acme", &arguments).expect("valid query");
    let program_pairs: Vec<&(String, String)> = params
        .query
        .iter()
        .filter(|(key, _)| key == tool::params::PROGRAM_FILTER)
        .collect();
    assert_eq!(program_pairs.len(), 1);
    assert_eq!(program_pairs[0].1, "acme");

    let body = tool::params::create_report("acme", &arguments)
        .expect("valid report")
        .body
        .expect("report body");
    assert_eq!(body["data"]["attributes"]["team_handle"], "acme");
}

#[test]
fn create_report_body_matches_upstream_attributes() {
    let params = tool::params::create_report(
        "acme",
        &args(json!({
            "title": "t",
            "vulnerability_information": "v",
            "impact": "i",
        })),
    )
    .expect("minimal report");
    assert_eq!(
        params.body,
        Some(json!({
            "data": {
                "type": "report",
                "attributes": {
                    "title": "t",
                    "vulnerability_information": "v",
                    "impact": "i",
                    "source": "api",
                    "team_handle": "acme",
                },
            },
        }))
    );

    let full = tool::params::create_report(
        "acme",
        &args(json!({
            "title": "t",
            "vulnerability_information": "v",
            "impact": "i",
            "severity_rating": "high",
            "weakness_id": 20,
            "structured_scope_id": 3,
            "source": "triage",
        })),
    )
    .expect("full report");
    assert_eq!(
        full.body.unwrap()["data"]["attributes"],
        json!({
            "title": "t",
            "vulnerability_information": "v",
            "impact": "i",
            "source": "triage",
            "team_handle": "acme",
            "severity_rating": "high",
            "weakness_id": 20,
            "structured_scope_id": 3,
        })
    );

    let missing = tool::params::create_report("acme", &args(json!({"title": "t"})))
        .expect_err("missing required fields");
    assert_eq!(validation_field(missing), "vulnerability_information");
}

#[test]
fn change_state_maps_optional_attributes() {
    let minimal = tool::params::change_state(&args(json!({"state": "triaged"})))
        .expect("minimal state change");
    assert_eq!(
        minimal.body,
        Some(json!({"data": {"type": "state-change", "attributes": {"state": "triaged"}}}))
    );

    let full = tool::params::change_state(&args(json!({
        "state": "duplicate",
        "message": "dup of #1",
        "original_report_id": 1,
        "attachment_ids": [4, 5],
    })))
    .expect("full state change");
    assert_eq!(
        full.body,
        Some(json!({
            "data": {
                "type": "state-change",
                "attributes": {
                    "state": "duplicate",
                    "message": "dup of #1",
                    "original_report_id": 1,
                    "attachment_ids": [4, 5],
                },
            },
        }))
    );
}

#[test]
fn list_scopes_emits_filters_and_rejects_negative_ids() {
    let params = tool::params::list_scopes(&args(json!({
        "scope_id_gt": 5,
        "created_after": "2026-01-01T00:00:00Z",
        "updated_after": "2026-02-01T00:00:00Z",
        "page_number": 1,
        "page_size": 100,
    })))
    .expect("valid scope list");
    assert_eq!(
        query(&params),
        expected(&[
            ("filter[id__gt]", "5"),
            ("filter[created_at__gt]", "2026-01-01T00:00:00Z"),
            ("filter[updated_at__gt]", "2026-02-01T00:00:00Z"),
            ("page[number]", "1"),
            ("page[size]", "100"),
        ])
    );

    let error =
        tool::params::list_scopes(&args(json!({"scope_id_gt": -1}))).expect_err("negative id");
    assert_eq!(validation_field(error), "scope_id_gt");
}

#[test]
fn every_tool_variant_is_wired_exactly_once_in_advertised_order() {
    assert_eq!(tool::WIRED.len(), ToolName::ALL.len());

    for variant in ToolName::ALL {
        let count = tool::WIRED
            .iter()
            .filter(|(wired, _, _)| *wired == variant)
            .count();
        assert_eq!(count, 1, "{} is not wired exactly once", variant.name());
    }

    let wired: Vec<ToolName> = tool::WIRED.iter().map(|(tool, _, _)| *tool).collect();
    assert_eq!(wired, ToolName::ALL.to_vec());
}

#[test]
fn every_shipped_tool_declares_its_credential_class() {
    let customer_tools = 13;
    for (index, tool) in ToolName::ALL.into_iter().enumerate() {
        let expected = if index < customer_tools {
            CredentialClass::Customer
        } else {
            CredentialClass::Hacker
        };
        assert_eq!(tool.class(), expected, "{}", tool.name());
    }
}

#[test]
fn new_write_builders_match_upstream_bodies() {
    let reference = tool::params::update_reference(&args(json!({
        "reference": "JIRA-1",
        "message": "see ticket",
    })))
    .expect("reference update");
    assert_eq!(
        reference.body,
        Some(json!({
            "data": {
                "type": "issue-tracker-reference-id",
                "attributes": {"reference": "JIRA-1", "message": "see ticket"},
            },
        }))
    );

    let cves = tool::params::update_cves(&args(json!({
        "cve_ids": ["CVE-2024-2", "CVE-2024-1"],
    })))
    .expect("cve update");
    assert_eq!(
        cves.body,
        Some(json!({
            "data": {
                "type": "report-cves",
                "attributes": {"cve_ids": ["CVE-2024-2", "CVE-2024-1"]},
            },
        }))
    );

    let escalation =
        tool::params::escalate_report(&args(json!({"integration_id": "42"}))).expect("escalation");
    assert_eq!(
        escalation.body,
        Some(json!({"data": {"attributes": {"integration_id": "42"}}}))
    );
}

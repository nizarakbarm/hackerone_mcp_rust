//! Hacker-side reads (ADR-0008): routing to the hacker token, the capped
//! client-side scan of `/hackers/programs`, exact paths and filter encoding,
//! and 404 → `ProgramNotFound`.

use hackerone_api::Error;
use hackerone_mcp::{
    Clients,
    config::Config,
    tool::{self, dispatch, params, scope::ProgramScope},
};
use serde_json::{Map, Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

/// HTTP Basic for `hacker-user:hacker-token`.
const HACKER_AUTH: &str = "Basic aGFja2VyLXVzZXI6aGFja2VyLXRva2Vu";
/// HTTP Basic for `customer-user:customer-token`.
const CUSTOMER_AUTH: &str = "Basic Y3VzdG9tZXItdXNlcjpjdXN0b21lci10b2tlbg==";

fn clients(server: &MockServer) -> Clients {
    Clients::from_config(&Config {
        api_username: Some("customer-user".to_owned()),
        api_token: Some("customer-token".to_owned()),
        hacker_api_username: Some("hacker-user".to_owned()),
        hacker_api_token: Some("hacker-token".to_owned()),
        base_url: Some(server.uri()),
        ..Config::default()
    })
    .expect("clients")
}

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected an object, got {other}"),
    }
}

async fn received(server: &MockServer) -> Vec<wiremock::Request> {
    server.received_requests().await.expect("recorded requests")
}

fn query_pairs(request: &wiremock::Request) -> Vec<(String, String)> {
    request
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

fn expected(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

async fn authorizations(server: &MockServer) -> Vec<Option<String>> {
    received(server)
        .await
        .iter()
        .map(|request| {
            request
                .headers
                .get("authorization")
                .map(|value| value.to_str().expect("ascii header").to_owned())
        })
        .collect()
}

fn program(id: u32, offers_bounties: bool) -> Value {
    json!({
        "type": "program",
        "id": id.to_string(),
        "attributes": {
            "handle": format!("program-{id}"),
            "offers_bounties": offers_bounties,
            "state": "open",
            "submission_state": "open",
        }
    })
}

fn next_link(server: &MockServer, page: u32) -> String {
    format!("{}/hackers/programs?page[number]={page}", server.uri())
}

/// One directory page of `programs`, with the API's next link when `more`.
fn directory_page(programs: Vec<Value>, next: Option<String>) -> Value {
    let links = match next {
        Some(next) => json!({"next": next}),
        None => json!({}),
    };
    json!({"data": programs, "links": links})
}

async fn mount_directory_page(server: &MockServer, page: u32, size: u32, body: Value) {
    Mock::given(method("GET"))
        .and(path("/hackers/programs"))
        .and(query_param("page[number]", page.to_string()))
        .and(query_param("page[size]", size.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn every_hacker_tool_sends_only_the_hacker_token() {
    let server = MockServer::start().await;
    mount_directory_page(&server, 1, 25, directory_page(Vec::new(), None)).await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"id": "1"}})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme/structured_scopes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;
    for endpoint in [
        "/hackers/programs/acme/scope_exclusions",
        "/hackers/programs/acme/weaknesses",
        "/hackers/me/reports",
        "/hackers/hacktivity",
    ] {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
            .expect(1)
            .mount(&server)
            .await;
    }

    let clients = clients(&server);
    for (name, arguments) in [
        ("search_programs", json!({})),
        ("get_program", json!({"handle": "acme"})),
        ("get_program_structured_scopes", json!({"handle": "acme"})),
        ("get_program_scope_exclusions", json!({"handle": "acme"})),
        ("get_program_weaknesses", json!({"handle": "acme"})),
        ("list_my_reports", json!({})),
        ("search_hacktivity", json!({})),
    ] {
        dispatch(&clients, &ProgramScope::new(None), name, &args(arguments))
            .await
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    }

    let seen = authorizations(&server).await;
    assert_eq!(seen.len(), 7);
    for auth in seen {
        assert_eq!(auth.as_deref(), Some(HACKER_AUTH), "hacker token expected");
        assert_ne!(
            auth.as_deref(),
            Some(CUSTOMER_AUTH),
            "customer token must never be sent"
        );
    }
}

#[tokio::test]
async fn search_programs_truncates_when_the_budget_ends_with_pages_left() {
    let server = MockServer::start().await;
    mount_directory_page(
        &server,
        1,
        1,
        directory_page(vec![program(1, true)], Some(next_link(&server, 2))),
    )
    .await;
    mount_directory_page(
        &server,
        2,
        1,
        directory_page(vec![program(2, true)], Some(next_link(&server, 3))),
    )
    .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({"page_size": 1, "max_pages": 2})),
    )
    .await
    .expect("search");

    assert_eq!(value["truncated"], true, "{value}");
    assert_eq!(value["pages_scanned"], 2, "{value}");
    assert_eq!(value["next_page"], 3, "{value}");
    assert_eq!(
        value["matches"].as_array().expect("matches").len(),
        2,
        "{value}"
    );

    let requests = received(&server).await;
    assert_eq!(requests.len(), 2, "exactly the page budget was spent");
    assert_eq!(
        query_pairs(&requests[0]),
        expected(&[("page[number]", "1"), ("page[size]", "1")])
    );
    assert_eq!(
        query_pairs(&requests[1]),
        expected(&[("page[number]", "2"), ("page[size]", "1")])
    );
}

#[tokio::test]
async fn truncation_is_about_pages_even_when_no_entry_matches() {
    let server = MockServer::start().await;
    for page in [1u32, 2] {
        mount_directory_page(
            &server,
            page,
            1,
            directory_page(
                vec![program(page, false)],
                Some(next_link(&server, page + 1)),
            ),
        )
        .await;
    }

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({"page_size": 1, "max_pages": 2, "offers_bounties": true})),
    )
    .await
    .expect("search");

    assert_eq!(value["matches"], json!([]), "{value}");
    assert_eq!(value["truncated"], true, "{value}");
    assert_eq!(value["pages_scanned"], 2, "{value}");
    assert_eq!(received(&server).await.len(), 2);
}

#[tokio::test]
async fn search_programs_stops_when_the_api_has_no_next_page_and_sends_no_filters() {
    let server = MockServer::start().await;
    mount_directory_page(
        &server,
        1,
        2,
        directory_page(
            vec![program(1, true), program(2, false)],
            Some(next_link(&server, 2)),
        ),
    )
    .await;
    mount_directory_page(&server, 2, 2, directory_page(vec![program(3, true)], None)).await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({
            "page_size": 2,
            "offers_bounties": true,
            "state": "open",
            "submission_state": "open",
        })),
    )
    .await
    .expect("search");

    assert_eq!(value["truncated"], false, "{value}");
    assert_eq!(value["pages_scanned"], 2, "{value}");
    assert!(value.get("next_page").is_none(), "{value}");
    let ids: Vec<&str> = value["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .map(|program| program["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, vec!["1", "3"], "{value}");

    let requests = received(&server).await;
    assert_eq!(requests.len(), 2);
    for (index, request) in requests.iter().enumerate() {
        let page = (index + 1).to_string();
        assert_eq!(
            query_pairs(request),
            expected(&[("page[number]", &page), ("page[size]", "2")]),
            "filters must never reach the API; only pagination may"
        );
    }
}

#[test]
fn search_program_page_size_and_budget_are_bounded() {
    for size in 1..=100 {
        assert_eq!(
            params::search_programs(&args(json!({"page_size": size})))
                .expect("in range")
                .page_size,
            size
        );
    }
    for size in [0, 101] {
        let error =
            params::search_programs(&args(json!({"page_size": size}))).expect_err("out of range");
        assert!(
            matches!(error, Error::Validation { ref field, .. } if field == "page_size"),
            "{error:?}"
        );
    }

    let defaults = params::search_programs(&args(json!({}))).expect("defaults");
    assert_eq!(defaults.page_size, 25);
    assert_eq!(defaults.max_pages, 5);

    for budget in [0, 21] {
        let error =
            params::search_programs(&args(json!({"max_pages": budget}))).expect_err("out of range");
        assert!(
            matches!(error, Error::Validation { ref field, .. } if field == "max_pages"),
            "{error:?}"
        );
    }
}

#[tokio::test]
async fn out_of_range_page_size_is_rejected_before_any_request() {
    let server = MockServer::start().await;
    for size in [0, 101] {
        let error = dispatch(
            &clients(&server),
            &ProgramScope::new(None),
            "search_programs",
            &args(json!({"page_size": size})),
        )
        .await
        .expect_err("out of range");
        assert!(matches!(error, Error::Validation { ref field, .. } if field == "page_size"));
    }
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn get_program_uses_the_exact_path_and_passes_the_body_through() {
    let server = MockServer::start().await;
    let body = json!({
        "data": {
            "type": "program",
            "id": "4242",
            "attributes": {"handle": "acme", "offers_bounties": true}
        }
    });
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program",
        &args(json!({"handle": "acme"})),
    )
    .await
    .expect("get_program");

    assert_eq!(value, body);
    assert!(query_pairs(&received(&server).await[0]).is_empty());
}

#[tokio::test]
async fn get_program_maps_a_404_to_program_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/ghost"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errors": []})))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program",
        &args(json!({"handle": "ghost"})),
    )
    .await
    .expect_err("404");

    assert!(
        matches!(error, Error::ProgramNotFound { ref handle } if handle == "ghost"),
        "{error:?}"
    );
}

#[tokio::test]
async fn structured_scopes_encode_every_filter_and_pass_the_body_through() {
    let server = MockServer::start().await;
    let body = json!({"data": [{"type": "structured_scope", "id": "10"}]});
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme/structured_scopes"))
        .and(query_param("filter[id__gt]", "10"))
        .and(query_param(
            "filter[created_at__gt]",
            "2026-01-01T00:00:00Z",
        ))
        .and(query_param(
            "filter[updated_at__gt]",
            "2026-02-01T00:00:00Z",
        ))
        .and(query_param("page[number]", "2"))
        .and(query_param("page[size]", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_structured_scopes",
        &args(json!({
            "handle": "acme",
            "scope_id_gt": 10,
            "created_after": "2026-01-01T00:00:00Z",
            "updated_after": "2026-02-01T00:00:00Z",
            "page_number": 2,
            "page_size": 5,
        })),
    )
    .await
    .expect("structured scopes");

    assert_eq!(value, body);
    let requests = received(&server).await;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        query_pairs(&requests[0]),
        expected(&[
            ("filter[id__gt]", "10"),
            ("filter[created_at__gt]", "2026-01-01T00:00:00Z"),
            ("filter[updated_at__gt]", "2026-02-01T00:00:00Z"),
            ("page[number]", "2"),
            ("page[size]", "5"),
        ])
    );
}

#[tokio::test]
async fn structured_scopes_map_a_404_to_program_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/ghost/structured_scopes"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errors": []})))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_structured_scopes",
        &args(json!({"handle": "ghost"})),
    )
    .await
    .expect_err("404");

    assert!(
        matches!(error, Error::ProgramNotFound { ref handle } if handle == "ghost"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_handle_can_never_escape_its_path_segment() {
    let server = MockServer::start().await;
    for name in [
        "get_program",
        "get_program_structured_scopes",
        "get_program_scope_exclusions",
        "get_program_weaknesses",
    ] {
        let error = dispatch(
            &clients(&server),
            &ProgramScope::new(None),
            name,
            &args(json!({"handle": "../reports/7"})),
        )
        .await
        .expect_err("path injection");
        assert!(
            matches!(error, Error::Validation { ref field, .. } if field == "handle"),
            "{name}: {error:?}"
        );
    }
    assert!(received(&server).await.is_empty());
}

#[test]
fn hacker_tools_are_in_the_registry_as_hacker_class() {
    for name in [
        "search_programs",
        "get_program",
        "get_program_structured_scopes",
        "get_program_scope_exclusions",
        "get_program_weaknesses",
        "list_my_reports",
        "search_hacktivity",
    ] {
        let tool = tool::ToolName::from_wire(name).expect("tool exists");
        assert_eq!(tool.class(), hackerone_api::CredentialClass::Hacker);
    }
}

#[tokio::test]
async fn scope_exclusions_uses_the_exact_path_and_passes_the_body_through() {
    let server = MockServer::start().await;
    let body = json!({
        "data": [{
            "type": "scope_exclusion",
            "id": "7",
            "attributes": {"name": "Third-party components"}
        }]
    });
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme/scope_exclusions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_scope_exclusions",
        &args(json!({"handle": "acme"})),
    )
    .await
    .expect("scope exclusions");

    assert_eq!(value, body);
    let requests = received(&server).await;
    assert_eq!(requests.len(), 1);
    assert!(query_pairs(&requests[0]).is_empty());
}

#[tokio::test]
async fn scope_exclusions_map_a_404_to_program_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/ghost/scope_exclusions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errors": []})))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_scope_exclusions",
        &args(json!({"handle": "ghost"})),
    )
    .await
    .expect_err("404");

    assert!(
        matches!(error, Error::ProgramNotFound { ref handle } if handle == "ghost"),
        "{error:?}"
    );
}

#[tokio::test]
async fn weaknesses_encode_pagination_and_pass_the_body_through() {
    let server = MockServer::start().await;
    let body = json!({"data": [{"type": "weakness", "id": "79"}]});
    Mock::given(method("GET"))
        .and(path("/hackers/programs/acme/weaknesses"))
        .and(query_param("page[number]", "3"))
        .and(query_param("page[size]", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_weaknesses",
        &args(json!({"handle": "acme", "page_number": 3, "page_size": 100})),
    )
    .await
    .expect("weaknesses");

    assert_eq!(value, body);
    assert_eq!(
        query_pairs(&received(&server).await[0]),
        expected(&[("page[number]", "3"), ("page[size]", "100")])
    );
}

#[tokio::test]
async fn weaknesses_map_a_404_to_program_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/programs/ghost/weaknesses"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errors": []})))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "get_program_weaknesses",
        &args(json!({"handle": "ghost"})),
    )
    .await
    .expect_err("404");

    assert!(
        matches!(error, Error::ProgramNotFound { ref handle } if handle == "ghost"),
        "{error:?}"
    );
}

#[tokio::test]
async fn list_my_reports_uses_the_exact_path_with_pagination() {
    let server = MockServer::start().await;
    let body = json!({"data": [{"type": "report", "id": "1"}]});
    Mock::given(method("GET"))
        .and(path("/hackers/me/reports"))
        .and(query_param("page[number]", "2"))
        .and(query_param("page[size]", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "list_my_reports",
        &args(json!({"page_number": 2, "page_size": 25})),
    )
    .await
    .expect("my reports");

    assert_eq!(value, body);
    assert_eq!(
        query_pairs(&received(&server).await[0]),
        expected(&[("page[number]", "2"), ("page[size]", "25")])
    );
}

#[tokio::test]
async fn list_my_reports_sends_no_parameters_when_pagination_is_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/me/reports"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "list_my_reports",
        &args(json!({})),
    )
    .await
    .expect("my reports");

    assert!(query_pairs(&received(&server).await[0]).is_empty());
}

/// Lucene that exercises spaces, `:`, `>=`, `AND`/`OR`, quotes and parentheses.
const LUCENE: &str = r#"severity_rating:critical AND disclosed_at:>=01-01-1970 AND team:"acme corp" AND (cwe:79 OR cwe:89)"#;

#[tokio::test]
async fn search_hacktivity_passes_the_lucene_query_through_verbatim() {
    let server = MockServer::start().await;
    let body = json!({"data": [{"type": "hacktivity_item", "id": "9"}]});
    Mock::given(method("GET"))
        .and(path("/hackers/hacktivity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_hacktivity",
        &args(json!({
            "queryString": LUCENE,
            "sort": "-disclosed_at",
            "page_number": 2,
            "page_size": 50,
        })),
    )
    .await
    .expect("hacktivity");

    assert_eq!(value, body);
    assert_eq!(
        query_pairs(&received(&server).await[0]),
        expected(&[
            ("queryString", LUCENE),
            ("sort", "-disclosed_at"),
            ("page[number]", "2"),
            ("page[size]", "50"),
        ])
    );
}

#[tokio::test]
async fn search_hacktivity_omits_an_absent_query_string_and_sends_the_default_sort() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/hacktivity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_hacktivity",
        &args(json!({})),
    )
    .await
    .expect("hacktivity");

    assert_eq!(
        query_pairs(&received(&server).await[0]),
        expected(&[("sort", "-latest_disclosable_activity_at")])
    );
}

#[test]
fn hacktivity_page_size_is_bounded_and_query_string_is_optional() {
    let defaults = params::search_hacktivity(&args(json!({}))).expect("defaults");
    assert_eq!(
        defaults.query,
        expected(&[("sort", "-latest_disclosable_activity_at")])
    );

    for size in [1, 100] {
        let query = params::search_hacktivity(&args(json!({"page_size": size})))
            .expect("in range")
            .query;
        assert_eq!(
            query,
            expected(&[
                ("sort", "-latest_disclosable_activity_at"),
                ("page[size]", &size.to_string()),
            ])
        );
    }
    for size in [0, 101] {
        let error =
            params::search_hacktivity(&args(json!({"page_size": size}))).expect_err("out of range");
        assert!(
            matches!(error, Error::Validation { ref field, .. } if field == "page_size"),
            "{error:?}"
        );
    }
}

#[tokio::test]
async fn hacktivity_rejects_an_out_of_range_page_size_before_any_request() {
    let server = MockServer::start().await;
    for size in [0, 101] {
        let error = dispatch(
            &clients(&server),
            &ProgramScope::new(None),
            "search_hacktivity",
            &args(json!({"page_size": size})),
        )
        .await
        .expect_err("out of range");
        assert!(matches!(error, Error::Validation { ref field, .. } if field == "page_size"));
    }
    assert!(received(&server).await.is_empty());
}

#[tokio::test]
async fn a_query_string_is_passed_through_unvalidated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hackers/hacktivity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;

    dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_hacktivity",
        &args(json!({"queryString": "  unbalanced ( quote\" AND "})),
    )
    .await
    .expect("hacktivity");

    assert_eq!(
        query_pairs(&received(&server).await[0]),
        expected(&[
            ("queryString", "  unbalanced ( quote\" AND "),
            ("sort", "-latest_disclosable_activity_at"),
        ])
    );
}

#[tokio::test]
async fn search_programs_resumes_from_start_page() {
    let server = MockServer::start().await;
    mount_directory_page(
        &server,
        3,
        2,
        directory_page(vec![program(9, true)], Some(next_link(&server, 4))),
    )
    .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({"page_size": 2, "start_page": 3, "max_pages": 1})),
    )
    .await
    .expect("search");

    assert_eq!(value["pages_scanned"], 1, "{value}");
    assert_eq!(value["truncated"], true, "{value}");
    assert_eq!(value["next_page"], 4, "{value}");
    let ids: Vec<&str> = value["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .map(|program| program["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, vec!["9"], "{value}");
}

#[tokio::test]
async fn search_programs_reports_a_program_once_when_pages_overlap() {
    let server = MockServer::start().await;
    mount_directory_page(
        &server,
        1,
        2,
        directory_page(
            vec![program(1, true), program(2, true)],
            Some(next_link(&server, 2)),
        ),
    )
    .await;
    // The directory shifted: program 1 shows up again on page 2.
    mount_directory_page(
        &server,
        2,
        2,
        directory_page(vec![program(1, true), program(3, true)], None),
    )
    .await;

    let value = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({"page_size": 2})),
    )
    .await
    .expect("search");

    let ids: Vec<&str> = value["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .map(|program| program["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, vec!["1", "2", "3"], "{value}");
}

#[tokio::test]
async fn search_programs_rejects_a_start_page_below_one() {
    let server = MockServer::start().await;
    let error = dispatch(
        &clients(&server),
        &ProgramScope::new(None),
        "search_programs",
        &args(json!({"start_page": 0})),
    )
    .await
    .expect_err("a page below 1 is invalid");

    let Error::Validation { field, .. } = error else {
        panic!("expected Validation, got {error:?}");
    };
    assert_eq!(field, "start_page");
    assert!(received(&server).await.is_empty(), "no request may be sent");
}

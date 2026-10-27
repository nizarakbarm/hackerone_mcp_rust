//! `search_programs`: the hacker-side program directory, filtered client-side
//! over a capped page scan.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value, json};

use super::{decode_error, params};

/// `GET /hackers/programs`, walking up to `max_pages` pages from `start_page`.
///
/// The API exposes no filters, so matches are selected here, page by page.
/// `truncated` is true only when the page budget ended while the API still
/// advertised a next page: it says pages went unscanned, never that they hold
/// matches. `next_page` is the page number to resume from.
///
/// # Errors
///
/// [`Error::Validation`] from the builder, [`Error::Decode`] when a page has no
/// `data` array, then any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let search = params::search_programs(args)?;
    let mut matches = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut pages_scanned = 0;
    let mut page = search.start_page;
    let mut truncated = false;

    while pages_scanned < search.max_pages {
        let query = vec![
            ("page[number]".to_owned(), page.to_string()),
            ("page[size]".to_owned(), search.page_size.to_string()),
        ];
        let response = client.get("/hackers/programs", &query, None).await?;
        pages_scanned += 1;

        let data = response
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| decode_error("data", "must be an array"))?;
        // Dedupe by id: the directory can shift between page requests, and a
        // program seen twice must not be reported twice.
        matches.extend(
            data.iter()
                .filter(|program| search.matches(program))
                .filter(|program| {
                    let id = program.get("id").and_then(Value::as_str).map(str::to_owned);
                    id.is_none_or(|id| seen.insert(id))
                })
                .cloned(),
        );

        if next_page(&response).is_none() {
            break;
        }
        if pages_scanned == search.max_pages {
            truncated = true;
            break;
        }
        page += 1;
    }

    let mut result = json!({
        "matches": matches,
        "pages_scanned": pages_scanned,
        "truncated": truncated,
    });
    if truncated {
        result["next_page"] = json!(page + 1);
    }
    Ok(result)
}

/// The API's next-page link, when one is advertised.
fn next_page(response: &Value) -> Option<&str> {
    response
        .get("links")
        .and_then(|links| links.get("next"))
        .and_then(Value::as_str)
}

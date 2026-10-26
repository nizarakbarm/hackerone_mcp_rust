//! `list_scopes`: resolve the configured handle to a program id, then read the
//! program's structured scopes.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::{decode_error, params};
use crate::config::Config;

/// Handle → program id, cached for the process lifetime (the id never changes
/// for a handle). Scoped by handle, so a reconfiguration re-resolves.
static CACHE: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(Mutex::default);

/// `GET /programs/{id}/structured_scopes` for the configured program.
///
/// # Errors
///
/// [`Error::MissingProgramHandle`] without a configured handle,
/// [`Error::Validation`] from the builder, [`Error::ProgramNotFound`] when
/// `/me/programs` has no matching handle, then any client error.
pub async fn run(
    client: &Client,
    config: &Config,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let handle = config
        .program_handle
        .as_deref()
        .ok_or(Error::MissingProgramHandle)?;
    let params = params::list_scopes(args)?;
    let program_id = program_id(client, handle).await?;
    client
        .get(
            &format!("/programs/{program_id}/structured_scopes"),
            &params.query,
            None,
        )
        .await
}

/// Program id for `handle`, from cache when already resolved.
///
/// Pages `/me/programs` by following `links.next`; the API's own cursor is the
/// only correct next page (incrementing `page[number]` can skip or repeat).
async fn program_id(client: &Client, handle: &str) -> Result<String, Error> {
    if let Some(program_id) = lock().get(handle) {
        return Ok(program_id.clone());
    }

    let mut path = "/me/programs".to_owned();
    let mut query = vec![
        ("page[number]".to_owned(), "1".to_owned()),
        ("page[size]".to_owned(), "100".to_owned()),
    ];
    loop {
        let response = client.get(&path, &query, None).await?;
        for program in response
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let matched = program
                .get("attributes")
                .and_then(|attributes| attributes.get("handle"))
                .and_then(Value::as_str)
                == Some(handle);
            if !matched {
                continue;
            }
            let program_id = match program.get("id") {
                Some(Value::String(id)) => id.clone(),
                Some(Value::Number(id)) => id.to_string(),
                _ => return Err(decode_error("data[].id", "is missing")),
            };
            lock().insert(handle.to_owned(), program_id.clone());
            return Ok(program_id);
        }

        let Some(next) = response
            .get("links")
            .and_then(|links| links.get("next"))
            .and_then(Value::as_str)
        else {
            break;
        };
        path = relative(next).to_owned();
        query.clear();
    }

    Err(Error::ProgramNotFound {
        handle: handle.to_owned(),
    })
}

fn lock() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Path + query of a pagination link. The scheme and host are dropped so the
/// credentials never follow a link off the configured API host.
fn relative(link: &str) -> &str {
    let Some(scheme) = link.find("://") else {
        return link;
    };
    let rest = &link[scheme + 3..];
    match rest.find('/') {
        Some(slash) => &rest[slash..],
        None => "/",
    }
}

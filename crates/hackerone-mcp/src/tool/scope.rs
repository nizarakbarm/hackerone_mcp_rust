//! The configured program: its handle, and the id `/me/programs` resolves it
//! to (architecture review candidate 1).
//!
//! One concept, one owner. The handle comes from config, the id is a network
//! lookup, and the cache that keeps it to one lookup lives here as instance
//! state — never a process-wide static.

use std::sync::OnceLock;

use hackerone_api::{Client, Error};
use serde_json::Value;

/// A program handle from config plus the id it resolves to, cached on the
/// instance.
///
/// The server holds one behind an `Arc`, so the id survives every session and
/// `tools/call`; a scope built by a test gets its own cache and cannot read the
/// server's program id.
#[derive(Debug)]
pub struct ProgramScope {
    handle: Option<String>,
    program_id: OnceLock<String>,
}

impl ProgramScope {
    /// A scope for `handle`, unresolved.
    #[must_use]
    pub fn new(handle: Option<String>) -> Self {
        Self {
            handle,
            program_id: OnceLock::new(),
        }
    }

    /// The configured handle, if any.
    #[must_use]
    pub fn handle(&self) -> Option<&str> {
        self.handle.as_deref()
    }

    /// The program id for the configured handle, resolving it on first use.
    ///
    /// # Errors
    ///
    /// [`Error::MissingProgramHandle`] without a configured handle,
    /// [`Error::ProgramNotFound`] when `/me/programs` has no matching handle,
    /// otherwise any client error.
    pub async fn id(&self, client: &Client) -> Result<&str, Error> {
        if let Some(program_id) = self.program_id.get() {
            return Ok(program_id);
        }
        let handle = self.handle.as_deref().ok_or(Error::MissingProgramHandle)?;
        // No lock is held across the await, so concurrent callers never block
        // each other; the first resolution to land wins and the rest reuse it.
        let resolved = resolve(client, handle).await?;
        Ok(self.program_id.get_or_init(|| resolved))
    }
}

/// Id of `handle`, paging `/me/programs` by following `links.next`: the API's
/// own cursor is the only correct next page (incrementing `page[number]` can
/// skip or repeat).
async fn resolve(client: &Client, handle: &str) -> Result<String, Error> {
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
            return match program.get("id") {
                Some(Value::String(id)) => Ok(id.clone()),
                Some(Value::Number(id)) => Ok(id.to_string()),
                _ => Err(super::decode_error("data[].id", "is missing")),
            };
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

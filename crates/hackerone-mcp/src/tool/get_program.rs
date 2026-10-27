//! `get_program`: one hacker-side program lookup by handle.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::{not_found, params};

/// `GET /hackers/programs/{handle}`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing or malformed handle,
/// [`Error::ProgramNotFound`] on 404, then any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let handle = params::program_handle(args)?;
    client
        .get(&format!("/hackers/programs/{handle}"), &[], None)
        .await
        .map_err(|error| not_found(handle, error))
}

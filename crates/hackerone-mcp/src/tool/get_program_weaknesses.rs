//! `get_program_weaknesses`: a program's CWE list, paginated.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::{not_found, params};

/// `GET /hackers/programs/{handle}/weaknesses`.
///
/// # Errors
///
/// [`Error::Validation`] for a missing or malformed handle or bad pagination,
/// [`Error::ProgramNotFound`] on 404, then any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let handle = params::program_handle(args)?;
    let query = params::pagination(args)?;
    client
        .get(
            &format!("/hackers/programs/{handle}/weaknesses"),
            &query.query,
            None,
        )
        .await
        .map_err(|error| not_found(handle, error))
}

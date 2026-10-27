//! `get_program_structured_scopes`: a program's scopes from the hacker side.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::{not_found, params};

/// `GET /hackers/programs/{handle}/structured_scopes`.
///
/// Shares the customer-side `list_scopes` query builder: `filter[id__gt]`,
/// `filter[created_at__gt]`, `filter[updated_at__gt]` and pagination.
///
/// # Errors
///
/// [`Error::Validation`] from the builders, [`Error::ProgramNotFound`] on 404,
/// then any client error.
pub async fn run(client: &Client, args: &Map<String, Value>) -> Result<Value, Error> {
    let handle = params::program_handle(args)?;
    let query = params::list_scopes(args)?;
    client
        .get(
            &format!("/hackers/programs/{handle}/structured_scopes"),
            &query.query,
            None,
        )
        .await
        .map_err(|error| not_found(handle, error))
}

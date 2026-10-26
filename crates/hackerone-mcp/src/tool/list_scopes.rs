//! `list_scopes`: read the configured program's structured scopes.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::params;
use super::scope::ProgramScope;

/// `GET /programs/{id}/structured_scopes` for the configured program.
///
/// # Errors
///
/// [`Error::MissingProgramHandle`] without a configured handle,
/// [`Error::Validation`] from the builder, [`Error::ProgramNotFound`] when
/// `/me/programs` has no matching handle, then any client error.
pub async fn run(
    client: &Client,
    scope: &ProgramScope,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    // The config error outranks argument validation, as it always has.
    scope.handle().ok_or(Error::MissingProgramHandle)?;
    let params = params::list_scopes(args)?;
    let program_id = scope.id(client).await?;
    client
        .get(
            &format!("/programs/{program_id}/structured_scopes"),
            &params.query,
            None,
        )
        .await
}

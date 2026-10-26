//! `create_report`: import a known vulnerability report into the configured
//! program. The program handle comes from config only, never from arguments.

use hackerone_api::{Client, Error};
use serde_json::{Map, Value};

use super::scope::ProgramScope;

/// `POST /reports` for the configured program.
///
/// # Errors
///
/// [`Error::MissingProgramHandle`] without a configured handle (checked before
/// the body is built and before any request), [`Error::Validation`] from the
/// builder, otherwise any error the client produced.
pub async fn run(
    client: &Client,
    scope: &ProgramScope,
    args: &Map<String, Value>,
) -> Result<Value, Error> {
    let handle = scope.handle().ok_or(Error::MissingProgramHandle)?;
    let params = super::params::create_report(handle, args)?;
    client.post("/reports", &params.query, params.body).await
}

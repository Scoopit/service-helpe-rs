use axum::response::IntoResponse;
use futures::Future;
use http::StatusCode;
use log::error;

use crate::errors::format_error;

/// Reject the request if a NotFound error is returned by the future. Otherwise, log the error
/// and send  a 500 error.
pub async fn handle_errors<R: IntoResponse, F: Future<Output = anyhow::Result<R>>>(
    f: F,
) -> Result<R, (StatusCode, String)> {
    match f.await {
        Ok(resp) => Ok(resp),
        Err(err) => match err.downcast::<NotFound>() {
            Ok(_not_found) => Err((StatusCode::NOT_FOUND, "404 Not Found".to_string())),
            Err(err) => match err.downcast::<BadRequest>() {
                Ok(bad_request) => Err((StatusCode::BAD_REQUEST, bad_request.0)),
                Err(err) => match err.downcast::<Forbidden>() {
                    Ok(forbidden) => Err((StatusCode::FORBIDDEN, forbidden.0)),
                    Err(err) => {
                        error!("Unable to handle request: {}", format_error(err));
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "500 Internal Server Error".to_string(),
                        ))
                    }
                },
            },
        },
    }
}

/// When returned by a future handled by handle_errors, respond with a 404 not found.
#[derive(Debug, thiserror::Error)]
#[error("Not found")]
pub struct NotFound;

#[derive(Debug, thiserror::Error)]
#[error("Forbidden: {0}")]
pub struct Forbidden(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Bad request: {0}")]
pub struct BadRequest(pub String);

use crate::errors::format_error;
use std::future::Future;

/// Reject the request if a NotFound error is returned by the future. Otherwise, log the error
/// and send  a 500 error.
pub async fn handle_errors<R: warp::Reply + 'static, F: Future<Output = anyhow::Result<R>>>(
    f: F,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match f.await {
        Ok(resp) => Ok(Box::new(resp)),
        Err(err) => match err.downcast::<NotFound>() {
            Ok(_not_found) => Ok(Box::new(warp::http::StatusCode::NOT_FOUND)),
            Err(err) => match err.downcast::<BadRequest>() {
                Ok(bad_request) => Ok(Box::new(
                    warp::http::Response::builder()
                        .status(warp::http::StatusCode::BAD_REQUEST)
                        .body(bad_request.to_string()),
                )),
                Err(err) => match err.downcast::<Forbidden>() {
                    Ok(_forbidden) => Ok(Box::new(warp::http::StatusCode::FORBIDDEN)),
                    Err(err) => {
                        log::error!("Unable to handle request: {}", format_error(err));
                        Ok(Box::new(warp::http::StatusCode::INTERNAL_SERVER_ERROR))
                    }
                },
            },
        },
    }
}

/// When returned by a future handled by handle_errors, respond with a 404 not found.
#[derive(Debug, thiserror::Error)]
#[error("not found")]
pub struct NotFound;

#[derive(Debug, thiserror::Error)]
#[error("forbidden")]
pub struct Forbidden;

#[derive(Debug, thiserror::Error)]
#[error("Bad request: {0}")]
pub struct BadRequest(pub String);

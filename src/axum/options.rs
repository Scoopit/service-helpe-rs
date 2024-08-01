use axum::extract::Request;
use axum::middleware::Next;
use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use http::Method;

/// Unconditionally handle `OPTIONS` requests. Send a `200 OK` response
/// with `Allow` header set to `OPTIONS, GET, HEAD, POST, PUT, DELETE`.
///
/// This middleware is especially useful when a load balancer checks the availability
/// of the service by sending an `OPTIONS` http request (eg. haproxy).
///
pub async fn options_middleware(req: Request, next: Next) -> impl IntoResponse {
    if req.method() == Method::OPTIONS {
        Response::builder()
            .status(200)
            .header(
                axum::http::header::ALLOW,
                "OPTIONS, GET, HEAD, POST, PUT, DELETE",
            )
            .body(Body::empty())
            .unwrap()
    } else {
        next.run(req).await
    }
}

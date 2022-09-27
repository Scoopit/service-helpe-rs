use axum::middleware::Next;
use axum::{
    body::{boxed, Body},
    response::{IntoResponse, Response},
};
use http::{Method, Request};

pub async fn options_middleware<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    if req.method() == Method::OPTIONS {
        Response::builder()
            .status(200)
            .header(axum::http::header::ALLOW, "OPTIONS, GET, HEAD, POST")
            .body(boxed(Body::empty()))
            .unwrap()
    } else {
        next.run(req).await
    }
}

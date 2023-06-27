use std::{net::SocketAddr, time::Instant};

use axum::{extract::ConnectInfo, middleware::Next, response::IntoResponse};
use data_encoding::BASE64URL_NOPAD;
use futures::FutureExt;
use http::Request;
use tracing::{error_span, Instrument};

/// Logs every request to `access_log` target in Info.
///
/// Also setup a tracing span with:
/// - `tx_id` an id for the current request
/// - `method`
/// - `path`
/// - `remote_ip` if the service has a ConnectInfo<RemoteAddr> in a request extention
pub async fn access_log<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    // do not record metrics on /metrics nor /health endpoint
    let path = req.uri().path().to_string();
    let log = path != "/metrics" && path != "/health";
    let start = Instant::now();
    let method = req.method().clone();

    let tx_id = BASE64URL_NOPAD.encode(uuid::Uuid::new_v4().as_bytes());

    let remote_addr = req.extensions().get::<ConnectInfo<SocketAddr>>();

    let span = match remote_addr {
        Some(ConnectInfo(remote_addr)) => error_span!(
            "request",
            tx = tx_id,
            method = method.to_string(),
            path = path,
            remote_ip = format!("{}", remote_addr.ip()),
        ),
        None => error_span!(
            "request",
            tx = tx_id,
            method = method.to_string(),
            path = path,
        ),
    };
    if log {
        let _enter = span.enter();
        log::log!(
            target: "access_log",
            log::Level::Info,
            "{method} {path} Received",
        );
    }

    next.run(req)
        .then(|r| async {
            if log {
                let elapsed = start.elapsed().as_millis();
                let status = r.status().as_u16();
                log::log!(
                    target: "access_log",
                    log::Level::Info,
                    "{method} {path} {status} {elapsed}ms",
                );
            }
            r
        })
        .instrument(span)
        .await
}

use std::time::Instant;

use axum::response::IntoResponse;
use axum::{extract::Request, middleware::Next};
use futures::FutureExt;
use lazy_static::lazy_static;
use prometheus::{Histogram, IntCounterVec, IntGauge};

use crate::metrics::{create_counter_with_labels, create_gauge};

lazy_static! {
    pub static ref REQUEST_DURATION: Histogram = {
        let ret = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP requests duration",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
                25.0, 50.0, 100.0,
            ]),
        )
        .unwrap();
        prometheus::register(Box::new(ret.clone())).unwrap();

        ret
    };
    pub static ref INFLIGHT_REQUESTS: IntGauge = create_gauge(
        "inflight_http_request_total",
        "Number of requests being processed"
    );
    pub static ref REQUEST_TOTAL: IntCounterVec = create_counter_with_labels(
        "http_request_total",
        "HTTP requests handled",
        &["method", "status"]
    );
}

pub async fn metrics_middleware(req: Request, next: Next) -> impl IntoResponse {
    // do not record metrics on /metrics nor /health endpoint
    let path = req.uri().path();
    let record_metrics = path != "/metrics" && path != "/health";
    let start = Instant::now();
    let method = req.method().clone();
    if record_metrics {
        INFLIGHT_REQUESTS.inc();
    }
    next.run(req)
        .then(|r| async {
            if record_metrics {
                REQUEST_DURATION.observe(start.elapsed().as_secs_f64());
                INFLIGHT_REQUESTS.dec();
                REQUEST_TOTAL
                    .with_label_values(&[method.as_str(), r.status().as_str()])
                    .inc();
            }
            r
        })
        .await
}

use std::sync::Arc;
use warp::filters::log::{Info, Log};

use crate::metrics::create_counter_with_labels;

pub fn requests_metrics(report_by_path: bool) -> Log<impl Fn(Info) + Clone> {
    let total = Arc::new(create_counter_with_labels(
        "http_request_total",
        "HTTP requests handled",
        &["status"],
    ));

    let by_path = if report_by_path {
        Some(Arc::new(create_counter_with_labels(
            "http_request_by_path_total",
            "HTTP requests handled",
            &["path", "status"],
        )))
    } else {
        None
    };

    let request_duration = prometheus::Histogram::with_opts(
        prometheus::HistogramOpts::new("http_request_duration_seconds", "HTTP requests duration")
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
                25.0, 50.0, 100.0,
            ]),
    )
    .unwrap();
    prometheus::register(Box::new(request_duration.clone())).unwrap();

    let request_duration_by_path = if report_by_path {
        let request_duration_by_path = prometheus::HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_by_path_seconds",
                "HTTP requests duration",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
                25.0, 50.0, 100.0,
            ]),
            &["path"],
        )
        .unwrap();
        prometheus::register(Box::new(request_duration_by_path.clone())).unwrap();
        Some(request_duration_by_path)
    } else {
        None
    };

    warp::log::custom(move |info| {
        if info.path().starts_with("/metrics") || info.path().starts_with("/health") {
            return;
        }
        total
            .clone()
            .get_metric_with_label_values(&[&format!("{}", info.status().as_u16())])
            .unwrap()
            .inc();
        if let Some(by_path) = by_path.clone() {
            by_path
                .get_metric_with_label_values(&[
                    info.path(),
                    &format!("{}", info.status().as_u16()),
                ])
                .unwrap()
                .inc();
        }

        request_duration.observe(info.elapsed().as_secs_f64());

        if let Some(request_duration_by_path) = request_duration_by_path.clone() {
            request_duration_by_path
                .get_metric_with_label_values(&[info.path()])
                .unwrap()
                .observe(info.elapsed().as_secs_f64());
        }
    })
}

#[cfg(test)]
#[test]
fn test() {
    requests_metrics(true);
}

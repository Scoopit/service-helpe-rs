use std::collections::HashSet;
use warp::filters::log::{Info, Log};

use crate::metrics::create_counter_with_labels;

/// Warp filter to log requests metrics
///
/// If `report_by_path` is true, metrics will be reported by path. `path_allow_list` can be used to filter paths to report,
/// if None is provided, all paths will be reported. Ignored path will be reported as "__other__".
///
pub fn requests_metrics(
    report_by_path: bool,
    path_allow_list: Option<&[&str]>,
) -> Log<impl Fn(Info) + Clone> {
    let path_allow_list =
        path_allow_list.map(|list| list.iter().map(|s| s.to_string()).collect::<HashSet<_>>());

    let total =
        create_counter_with_labels("http_request_total", "HTTP requests handled", &["status"]);

    let by_path = if report_by_path {
        Some(create_counter_with_labels(
            "http_request_by_path_total",
            "HTTP requests handled",
            &["path", "status"],
        ))
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
            .get_metric_with_label_values(&[&format!("{}", info.status().as_u16())])
            .unwrap()
            .inc();

        let path = if let Some(allow_list) = path_allow_list.as_ref() {
            if allow_list.contains(info.path()) {
                info.path()
            } else {
                "__other__"
            }
        } else {
            info.path()
        };

        if let Some(by_path) = by_path.clone() {
            by_path
                .get_metric_with_label_values(&[path, &format!("{}", info.status().as_u16())])
                .unwrap()
                .inc();
        }

        request_duration.observe(info.elapsed().as_secs_f64());

        if let Some(request_duration_by_path) = request_duration_by_path.clone() {
            request_duration_by_path
                .get_metric_with_label_values(&[path])
                .unwrap()
                .observe(info.elapsed().as_secs_f64());
        }
    })
}

#[cfg(test)]
#[test]
fn test() {
    requests_metrics(true, None);
}

use prometheus::{Encoder, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, TextEncoder};
use std::time::Duration;

/// Helper methods used to creates metrics
///

/// Creates a counter and register it.
///
/// It will return an error if the counter is already registered
///
pub fn create_counter(name: &str, help: &str) -> prometheus::Result<IntCounter> {
    let counter = IntCounter::new(name, help)?;
    prometheus::register(Box::new(counter.clone()))?;
    Ok(counter)
}

/// Creates a counter and register it.
///
/// It will return an error if the counter is already registered
///
pub fn create_counter_with_labels(
    name: &str,
    help: &str,
    labels: &[&str],
) -> prometheus::Result<IntCounterVec> {
    let counter = IntCounterVec::new(Opts::new(name, help), labels)?;
    prometheus::register(Box::new(counter.clone()))?;
    Ok(counter)
}

/// Creates a gauge and register it.
///
/// It will return an error if the gauge is already registered
///
pub fn create_gauge(name: &str, help: &str) -> prometheus::Result<IntGauge> {
    let gauge = IntGauge::new(name, help)?;
    prometheus::register(Box::new(gauge.clone()))?;
    Ok(gauge)
}

/// Creates a gauge and register it.
///
/// It will return an error if the gauge is already registered
///
pub fn create_gauge_with_labels(
    name: &str,
    help: &str,
    labels: &[&str],
) -> prometheus::Result<IntGaugeVec> {
    let gauge = IntGaugeVec::new(Opts::new(name, help), labels)?;
    prometheus::register(Box::new(gauge.clone()))?;
    Ok(gauge)
}

/// Generate the content of /metrics prometheus metrics gathering endpoint.
///
pub fn generate_metrics() -> String {
    // Gather the metrics.
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Launch async process collector at specified interval. It requires a running tokio runtime!
#[cfg(all(feature = "tokio"))]
pub fn launch_async_process_collector(interval: Duration) {
    tokio::task::spawn(collect(interval));
}
#[cfg(all(target_os = "linux", feature = "tokio"))]
async fn collect(interval: Duration) {
    use prometheus::core::Collector;
    let process_collector = prometheus::process_collector::ProcessCollector::for_self();
    loop {
        log::debug!("Collecting process info");
        process_collector.collect();
        tokio::time::sleep(interval).await;
    }
}

#[cfg(all(not(target_os = "linux"), feature = "tokio"))]
async fn collect(interval: Duration) {
    loop {
        log::warn!("Collecting process info not available on this platform");
        tokio::time::sleep(interval).await;
    }
}

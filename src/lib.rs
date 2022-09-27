#[cfg(feature = "tracing-gelf")]
pub mod tracing_gelf;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "warp")]
pub mod warp;

#[cfg(feature = "axum")]
pub mod axum;

pub mod errors;

pub mod config;

/// Struct used to describe the service (typically used in logging services)
pub struct ServiceDef<'a> {
    version: &'a str,
    git_hash: &'a str,
    pkg_name: &'a str,
}

impl<'a> ServiceDef<'a> {
    pub const fn new(pkg_name: &'a str, version: &'a str, git_hash: &'a str) -> Self {
        Self {
            version,
            git_hash,
            pkg_name,
        }
    }
}

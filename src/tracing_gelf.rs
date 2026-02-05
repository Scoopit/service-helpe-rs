use std::io::{stdout, IsTerminal};

use serde::{Deserialize, Serialize};
use tracing_gelf::Logger;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::SubscriberBuilder, util::SubscriberInitExt, EnvFilter};

use crate::ServiceDef;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GelfParams {
    pub tcp_address: String,
    pub env: String,
}

pub fn init<'a>(gelf: Option<GelfParams>, service: ServiceDef<'a>) -> anyhow::Result<()> {
    let enable_colors = stdout().is_terminal();
    let stdout = SubscriberBuilder::default()
        // only enable colored output on real terminals
        .with_ansi(enable_colors)
        .with_env_filter(EnvFilter::from_default_env())
        // build but do not install the subscriber.
        .finish();

    match gelf {
        Some(gelf) => {
            println!(
                "Configuring GELF logger env:{}, tcp:{}",
                gelf.env, gelf.tcp_address
            );
            // launch tracing gelf
            let mut conn_handle = Logger::builder()
                .additional_field(
                    "version",
                    format!("{}-{}", service.version, service.git_hash),
                )
                .additional_field("service", service.pkg_name)
                .additional_field("env", gelf.env)
                .init_tcp_with_subscriber(gelf.tcp_address, stdout)?;
            tokio::spawn(async move { conn_handle.connect().await });

            // convert "classic" logs into tracing events
            LogTracer::init()?;
        }
        None => {
            println!("Configuring stdout logger");
            // only install tracing subscriber
            stdout.init();
            // no need to convert class logs with the LogTracer ; this is
            // done automatically by init() method
        }
    }

    Ok(())
}

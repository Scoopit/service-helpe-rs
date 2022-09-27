use std::{fs::File, io::Read, path::PathBuf};

use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::ServiceDef;

/// Load configuration mode
pub enum LoadConfigMode<'a> {
    /// configuration is only read from environment variable.
    ///
    /// Note that using this mode prevents from using nested structure, lists or maps. (see `envy` crate)
    ///
    /// You may want to load dot env files with the `dotenv` crate while using this mode.
    EnvOnly,
    /// Configuration is loaded from filesystem. If the path is not specified,
    /// the config file is loaded from "/etc/{pkg_name}/config.yaml"
    FileOnly(Option<&'a str>),
    /// Configuration is loaded from filesystem. If the path is not specified,
    /// the config file is loaded from "/etc/{pkg_name}/config.yaml"
    ///
    /// If the file does not exists, configuration is loaded from env. (see EnvOnly)
    FileAndEnvFallback(Option<&'a str>),
}

pub fn load_config<C: DeserializeOwned>(
    config_mode: LoadConfigMode,
    service_def: &ServiceDef,
) -> anyhow::Result<C> {
    match config_mode {
        LoadConfigMode::EnvOnly => {
            Ok(envy::from_env().context("Cannot read configuration from environment variables")?)
        }
        LoadConfigMode::FileOnly(file) => {
            Ok(serde_yaml::from_reader(open_config(file, service_def)?)
                .context("Cannot parse configuration file")?)
        }
        LoadConfigMode::FileAndEnvFallback(file) => match open_config(file, service_def) {
            Ok(reader) => {
                Ok(serde_yaml::from_reader(reader).context("Cannot parse configuration file")?)
            }
            Err(_) => Ok(envy::from_env()
                .context("Cannot read configuration from filesystem nor environment variables")?),
        },
    }
}

fn open_config(file: Option<&str>, service_def: &ServiceDef) -> anyhow::Result<impl Read> {
    let path: PathBuf = if let Some(filename) = file {
        filename.into()
    } else {
        format!("/etc/{}/config.yaml", service_def.pkg_name).into()
    };
    File::open(&path)
        .with_context(|| format!("Cannot load configuration file {}", path.to_string_lossy()))
}

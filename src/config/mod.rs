use config::{Config, File, FileFormat};
use errors::*;
use snafu::ResultExt;

pub mod args;
pub mod def;
pub mod errors;

pub fn load_config(file_name: &str) -> Result<def::Config, errors::ConfigError> {
    let settings = Config::builder()
        .add_source(File::new(file_name, FileFormat::Yaml))
        .build()
        .context(ConfigSnafu {
            file_name: file_name.to_string(),
        })?;
    settings.try_deserialize().context(ConfigSnafu {
        file_name: file_name.to_string(),
    })
}

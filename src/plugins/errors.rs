use serde_yaml::Error as YamlError;
use snafu::prelude::*;
use validator::ValidationErrors;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum PluginError {
    #[snafu(display("Unknown plugin: {}", name))]
    UnknownPlugin { name: String },
    #[snafu(display("Failed to validate plugin: {}, error: {:?}", name, source))]
    ValidateErr {
        source: ValidationErrors,
        name: String,
    },
    #[snafu(display("Failed to parse plugin config: {}, error: {:?}", name, source))]
    YamlErr { source: YamlError, name: String },
    #[snafu(display("Lack plugin config: {}", name))]
    LackPluginConfig { name: String },
    #[snafu(display("Specific error: {}, name: {}", source, name))]
    SpecificErr {
        source: Box<dyn std::error::Error>,
        name: String,
    },
}

use config::ConfigError as ExternalConfigError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ConfigError {
    #[snafu(display("Failed to load config: {}", source))]
    Config {
        file_name: String,
        source: ExternalConfigError,
    },
}

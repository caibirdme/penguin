use crate::{
    builder::errors::BuilderError, clusters::errors::ClusterError, config::errors::ConfigError,
};
use pingora::BError;
use snafu::Snafu;
use validator::ValidationErrors;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AppError {
    #[snafu(display("Cluster error: {}", source))]
    Cluster { source: ClusterError },
    #[snafu(display("Config error: {}", source))]
    Config { source: ConfigError },
    #[snafu(display("Builder error: {}", source))]
    Builder { source: BuilderError },
    #[snafu(display("Pingora error: {}", source))]
    Pingora { source: BError },
    #[snafu(display("Validation error: {}", source))]
    Validation { source: ValidationErrors },
}

use matchit::InsertError;
use snafu::Snafu;

use crate::plugins::errors::PluginError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BuilderError {
    #[snafu(display("Lack config for plugin: {}", name))]
    LackConfig { name: String },
    #[snafu(display("Failed to build plugin: {}, error: {:?}", name, source))]
    PluginBuild { source: PluginError, name: String },
    #[snafu(display("Lack uri for route: {}", name))]
    LackUri { name: String },
    #[snafu(display("Failed to compile regex: {}, error: {:?}", re, source))]
    Regexp { source: regex::Error, re: String },
    #[snafu(display("Failed to insert route: {}, error: {:?}", path, source))]
    InsertRoute { source: InsertError, path: String },
}

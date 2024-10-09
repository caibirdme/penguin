use crate::config::def::ResolverType;
use hickory_resolver::error::ResolveError;
use serde_yaml::Error as YamlError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ClusterError {
    #[snafu(display("Invalid static config for {}, reason: {}", name, source))]
    StaticConfig { source: YamlError, name: String },
    #[snafu(display("Lack config for {}", name))]
    LackConfig { name: String },
    #[snafu(display("Failed to build discovery for {} due to {}", name, source))]
    DiscoveryConfig { source: YamlError, name: String },
    #[snafu(display("Invalid port {}", port))]
    InvalidPort { port: String },
    #[snafu(display("Invalid endpoints {}", ep))]
    InvalidEndpoints { ep: String },
    #[snafu(display("Unknown resolver type {:?}", resolver))]
    UnknownResolver { resolver: ResolverType },
    #[snafu(display("Failed to resolve ip for {}", name))]
    ResolveIp { source: ResolveError, name: String },
}

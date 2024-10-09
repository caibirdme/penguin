use crate::clusters::{errors::*, ClusterResult, Resolver};
use async_trait::async_trait;
use hickory_resolver::TokioAsyncResolver;
use once_cell::sync::OnceCell;
use pingora::lb::{discovery::ServiceDiscovery, Backend};
use pingora::prelude::*;
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;
use serde::Deserialize;
use serde_yaml::Value as YamlValue;
use snafu::ResultExt;
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, SocketAddr as StdSocketAddr};
use std::sync::Arc;
use std::vec::IntoIter;

static GLOBAL_RESOLVER: OnceCell<Arc<TokioAsyncResolver>> = OnceCell::new();

fn get_global_resolver() -> Arc<TokioAsyncResolver> {
    GLOBAL_RESOLVER
        .get_or_init(|| Arc::new(TokioAsyncResolver::tokio_from_system_conf().unwrap()))
        .clone()
}

pub struct ResolverWrapper {
    resolver: Arc<TokioAsyncResolver>,
}

impl ResolverWrapper {
    pub fn new() -> Self {
        Self {
            resolver: get_global_resolver(),
        }
    }
}

impl Default for ResolverWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Resolver for ResolverWrapper {
    async fn lookup_ip(&self, name: &str) -> ClusterResult<Vec<IpAddr>> {
        Ok(self
            .resolver
            .lookup_ip(name)
            .await
            .context(ResolveIpSnafu { name })?
            .iter()
            .collect())
    }
}

pub struct DnsDiscovery {
    resolver: Arc<dyn Resolver>,
    name: String,
    port: u16,
}

impl DnsDiscovery {
    pub fn new(name: String, port: u16, resolver: Arc<dyn Resolver>) -> Self {
        Self {
            resolver,
            name,
            port,
        }
    }
}

#[async_trait]
impl ServiceDiscovery for DnsDiscovery {
    async fn discover(&self) -> Result<(BTreeSet<Backend>, HashMap<u64, bool>)> {
        let backends = self
            .resolver
            .lookup_ip(self.name.as_str())
            .await
            .unwrap()
            .iter()
            .map(|ip| Backend {
                addr: PingoraSocketAddr::Inet(StdSocketAddr::new(*ip, self.port)),
                weight: 1,
            })
            .collect();
        Ok((backends, HashMap::new()))
    }
}

#[derive(Debug, Deserialize)]
struct StaticConfig {
    endpoints: Vec<StdSocketAddr>,
}

pub struct StaticDiscovery {
    pub backends: Vec<StdSocketAddr>,
}

impl StaticDiscovery {
    pub fn new(cfg: Option<YamlValue>) -> ClusterResult<Self> {
        let cfg = cfg.ok_or(ClusterError::LackConfig {
            name: "static".to_string(),
        })?;
        let config: StaticConfig =
            serde_yaml::from_value(cfg).context(StaticConfigSnafu { name: "static" })?;
        Ok(Self {
            backends: config.endpoints,
        })
    }
}

impl IntoIterator for StaticDiscovery {
    type Item = StdSocketAddr;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.backends.into_iter()
    }
}

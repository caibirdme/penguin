use std::{collections::HashMap, net::IpAddr, sync::Arc};

use crate::{
    clusters::{
        discovery::{DnsDiscovery, StaticDiscovery},
        errors::*,
    },
    config::def::{Cluster as ClusterConfig, ResolverType},
    core::lb::LB,
};
use async_trait::async_trait;
use pingora::lb::{selection::Random, Backends, LoadBalancer};
use serde::Deserialize;
use snafu::ResultExt;

pub mod discovery;
pub mod errors;

pub type ClusterResult<T> = Result<T, errors::ClusterError>;

#[async_trait]
pub trait Resolver: Send + Sync {
    async fn lookup_ip(&self, name: &str) -> ClusterResult<Vec<IpAddr>>;
}

pub struct ClusterManager {
    clusters: HashMap<String, Arc<dyn LB>>,
}

impl ClusterManager {
    pub fn new(
        cfgs: Vec<ClusterConfig>,
        resolvers: &HashMap<ResolverType, Arc<dyn Resolver>>,
    ) -> ClusterResult<Self> {
        let mut clusters: HashMap<String, Arc<dyn LB>> = HashMap::new();
        for cfg in cfgs {
            match cfg.resolver {
                ResolverType::DNS => {
                    let resolver = resolvers.get(&ResolverType::DNS).cloned().ok_or(
                        ClusterError::UnknownResolver {
                            resolver: cfg.resolver,
                        },
                    )?;
                    let c: DNSConfig = serde_yaml::from_value(cfg.config.unwrap()).context(
                        errors::DiscoveryConfigSnafu {
                            name: cfg.name.clone(),
                        },
                    )?;
                    let discovery = DnsDiscovery::new(c.host, c.port, resolver);
                    let backends = Backends::new(Box::new(discovery));
                    let lb = LoadBalancer::<Random>::from_backends(backends);
                    clusters.insert(cfg.name, Arc::new(lb));
                }
                ResolverType::Static => {
                    let discovery = StaticDiscovery::new(cfg.config)?;
                    let lb = LoadBalancer::<Random>::try_from_iter(discovery).unwrap();
                    clusters.insert(cfg.name, Arc::new(lb));
                }
            }
        }
        Ok(Self { clusters })
    }
    pub fn get_cluster(&self, name: &str) -> Option<Arc<dyn LB>> {
        self.clusters.get(name).cloned()
    }
}

#[derive(Debug, Deserialize)]
struct DNSConfig {
    pub host: String,
    pub port: u16,
}

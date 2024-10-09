use std::{collections::HashMap, net::SocketAddr, time::Duration};

use pingora::server::configuration::ServerConf;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use validator::{Validate, ValidationError};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Config {
    #[serde(default)]
    pub identities: Vec<Identity>,
    #[validate(length(min = 1))]
    #[validate(nested)]
    pub services: Vec<Service>,
    #[serde(rename = "resolvers", default)]
    pub discovery_providers: Vec<DiscoveryProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub basic_auth: Option<BasicAuth>,
    pub hmac_auth: Option<HmacAuth>,
    pub jwt_auth: Option<JwtAuth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmacAuth {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtAuth {
    pub issuer: String,
    pub secret: String,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Service {
    pub name: String,
    pub server_conf: Option<ServerConf>,
    #[validate(length(min = 1))]
    #[validate(nested)]
    pub listeners: Vec<Listener>,
    pub plugins: Option<Vec<Plugin>>,
    #[validate(length(min = 1))]
    pub routes: Vec<Route>,
    pub clusters: Vec<Cluster>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_listener"))]
pub struct Listener {
    pub name: String,
    pub address: SocketAddr,
    #[serde(default)]
    pub protocol: Protocol,
    pub ssl_config: Option<SslConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum Protocol {
    #[default]
    HTTP,
    HTTPS,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SslConfig {
    #[serde(rename = "cert")]
    pub cert_path: String,
    #[serde(rename = "key")]
    pub key_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Route {
    pub name: String,
    #[serde(rename = "match")]
    pub matcher: Matcher,
    pub auth: Option<Auth>,
    pub plugins: Option<Vec<Plugin>>,
    pub cluster: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Matcher {
    pub uri: Option<StrMatch>,
    pub headers: Option<HashMap<String, StrMatch>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrMatch {
    Regexp(String),
    Prefix(String),
    Exact(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub allowed_identities: Option<Vec<String>>,
    pub config: Option<ForwardAuthConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForwardAuthConfig {
    pub cluster: String,
    pub path: String,
    pub headers_to_forward: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub config: Option<YamlValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub resolver: ResolverType,
    pub lb_policy: LbPolicy,
    pub config: Option<YamlValue>,
    pub health_checks: Option<Vec<HealthCheck>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterType {
    StrictDns,
    Static,
    #[serde(other)]
    Unsupported,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LbPolicy {
    RoundRobin,
    LeastConn,
    Random,
    #[serde(other)]
    Unsupported,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub interval: Duration,
    pub unhealthy_threshold: u32,
    pub healthy_threshold: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryProvider {
    pub name: String,
    #[serde(rename = "type")]
    pub resolver_type: ResolverType,
    pub config: Option<YamlValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResolverType {
    DNS,
    Static,
}

fn validate_listener(listener: &Listener) -> Result<(), ValidationError> {
    if matches!(listener.protocol, Protocol::HTTPS) && listener.ssl_config.is_none() {
        return Err(ValidationError::new(
            "ssl_config is required for HTTPS listener",
        ));
    }
    Ok(())
}

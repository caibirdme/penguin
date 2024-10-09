use std::{collections::HashMap, sync::Arc};

use crate::core::plugin::Plugin;
use once_cell::sync::Lazy;
use serde_yaml::Value as YamlValue;

pub mod cms_rate;
pub mod echo;
pub mod errors;

use errors::*;

pub type PluginResult<T> = std::result::Result<T, PluginError>;

/// Type alias for plugin initialization functions
pub type PluginInitFn =
    Arc<dyn Fn(Option<YamlValue>) -> PluginResult<Box<dyn Plugin>> + Send + Sync>;

/// Registry of plugin builders
static PLUGIN_BUILDER_REGISTRY: Lazy<HashMap<&'static str, PluginInitFn>> = Lazy::new(|| {
    let arr: Vec<(&str, PluginInitFn)> = vec![
        (echo::ECHO_PLUGIN_NAME, Arc::new(echo::create_echo_plugin)),
        (
            cms_rate::CMS_RATE_PLUGIN_NAME,
            Arc::new(cms_rate::create_cms_rate_limiter),
        ),
    ];
    arr.into_iter().collect()
});

pub fn create_plugin_builder(name: &str, cfg: Option<YamlValue>) -> PluginResult<Box<dyn Plugin>> {
    let builder = PLUGIN_BUILDER_REGISTRY
        .get(name)
        .ok_or(PluginError::UnknownPlugin {
            name: name.to_string(),
        })?;
    builder(cfg)
}

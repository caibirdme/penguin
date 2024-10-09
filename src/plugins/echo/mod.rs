use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use pingora::prelude::*;
use serde::Deserialize;
use serde_yaml::Value as YamlValue;
use snafu::prelude::*;

use crate::{
    core::plugin::{Plugin, PluginCtx},
    plugins::{errors::*, PluginResult},
    utils::send_response,
};

pub const ECHO_PLUGIN_NAME: &str = "echo";

pub fn create_echo_plugin(cfg: Option<YamlValue>) -> PluginResult<Box<dyn Plugin>> {
    let cfg = cfg.ok_or(PluginError::LackPluginConfig {
        name: ECHO_PLUGIN_NAME.to_string(),
    })?;
    let mut config: EchoConfigRaw = serde_yaml::from_value(cfg).context(YamlErrSnafu {
        name: ECHO_PLUGIN_NAME.to_string(),
    })?;
    // 把config.headers中的key转换为小写
    if let Some(headers) = config.headers.as_mut() {
        *headers = std::mem::take(headers)
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
    }
    let config = EchoConfig {
        body: Bytes::from(config.body),
        status_code: StatusCode::from_u16(config.status_code.unwrap_or(200))
            .map_err(|e| e.into())
            .context(SpecificErrSnafu {
                name: ECHO_PLUGIN_NAME.to_string(),
            })?,
        headers: config.headers,
    };
    Ok(Box::new(EchoPlugin {
        config: Arc::new(config),
    }))
}

#[derive(Clone)]
pub struct EchoPlugin {
    config: Arc<EchoConfig>,
}

#[derive(Debug)]
struct EchoConfig {
    body: Bytes,
    status_code: StatusCode,
    headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct EchoConfigRaw {
    body: String,
    status_code: Option<u16>,
    headers: Option<HashMap<String, String>>,
}

#[async_trait]
impl Plugin for EchoPlugin {
    async fn request_filter(&self, session: &mut Session, _ctx: &mut PluginCtx) -> Result<bool> {
        send_response(
            session,
            self.config.status_code,
            None,
            Some(self.config.body.clone()),
            self.config.headers.clone(),
        )
        .await?;
        Ok(true)
    }
}

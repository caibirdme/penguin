use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use http::StatusCode;
use pingora::prelude::*;
use pingora_limits::rate::Rate;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use snafu::ResultExt;
use validator::{Validate, ValidationError};

use crate::{
    core::plugin::{Plugin, PluginCtx},
    plugins::{errors::*, PluginResult},
    utils::send_response,
};

pub const CMS_RATE_PLUGIN_NAME: &str = "cms_rate";

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CmsRateConf {
    #[validate(range(min = 1))]
    pub total: isize,
    #[serde(with = "humantime_serde")]
    #[validate(custom(function = "atleast_1_second"))]
    pub interval: Duration,
}

fn atleast_1_second(v: &Duration) -> Result<(), ValidationError> {
    if v.as_secs() == 0 {
        return Err(ValidationError::new("interval must be at least 1 second"));
    }
    Ok(())
}

pub fn create_cms_rate_limiter(config: Option<YamlValue>) -> PluginResult<Box<dyn Plugin>> {
    let config = config.ok_or(PluginError::LackPluginConfig {
        name: CMS_RATE_PLUGIN_NAME.to_string(),
    })?;
    let cfg: CmsRateConf = serde_yaml::from_value(config).context(YamlErrSnafu {
        name: CMS_RATE_PLUGIN_NAME.to_string(),
    })?;
    cfg.validate().context(ValidateErrSnafu {
        name: CMS_RATE_PLUGIN_NAME.to_string(),
    })?;
    let r = Arc::new(Rate::new(cfg.interval));
    Ok(Box::new(PerRoutePlugin {
        total: cfg.total,
        r,
    }))
}

#[derive(Clone)]
pub struct PerRoutePlugin {
    total: isize,
    r: Arc<Rate>,
}

#[async_trait]
impl Plugin for PerRoutePlugin {
    async fn request_filter(&self, session: &mut Session, _ctx: &mut PluginCtx) -> Result<bool> {
        let path = session.req_header().uri.path();
        let nc = self.r.observe(&path, 1);
        if nc > self.total {
            send_response(session, StatusCode::TOO_MANY_REQUESTS, None, None, None).await?;
            return Ok(true);
        }
        Ok(false)
    }
}

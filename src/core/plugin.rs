use async_trait::async_trait;
use bytes::Bytes;
use matchit::Params;
use pingora::{http::ResponseHeader, prelude::*};
use regex::Captures;

/// Context for plugin execution
#[derive(Default)]
pub struct PluginCtx {
    pub route_params: Option<RouteParams>,
}

/// Main trait for plugins, defining various filter methods
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Handle the incoming request.
    ///
    /// In this phase, users can parse, validate, rate limit, perform access control and/or
    /// return a response for this request.
    ///
    /// # Arguments
    ///
    /// * `_session` - Mutable reference to the current session
    /// * `_ctx` - Mutable reference to the plugin context
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if a response was sent and the proxy should exit
    /// * `Ok(false)` if the proxy should continue to the next phase
    async fn request_filter(&self, _session: &mut Session, _ctx: &mut PluginCtx) -> Result<bool> {
        Ok(false)
    }

    /// Handle the incoming request body.
    ///
    /// This function will be called every time a piece of request body is received.
    ///
    /// # Arguments
    ///
    /// * `_session` - Mutable reference to the current session
    /// * `_body` - Mutable reference to an optional Bytes containing the body chunk
    /// * `_end_of_stream` - Boolean indicating if this is the last chunk
    /// * `_ctx` - Mutable reference to the plugin context
    async fn request_body_filter(
        &self,
        _session: &mut Session,
        _body: &mut Option<Bytes>,
        _end_of_stream: bool,
        _ctx: &mut PluginCtx,
    ) -> Result<()> {
        Ok(())
    }

    /// Modify the request before it is sent to the upstream
    ///
    /// # Arguments
    ///
    /// * `_session` - Mutable reference to the current session
    /// * `_upstream_request` - Mutable reference to the upstream request header
    /// * `_ctx` - Mutable reference to the plugin context
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        _upstream_request: &mut RequestHeader,
        _ctx: &mut PluginCtx,
    ) -> Result<()> {
        Ok(())
    }

    /// Modify the response header before it is sent to the downstream
    ///
    /// # Arguments
    ///
    /// * `_session` - Mutable reference to the current session
    /// * `_upstream_response` - Mutable reference to the upstream response header
    /// * `_ctx` - Mutable reference to the plugin context
    async fn response_filter(
        &self,
        _session: &mut Session,
        _upstream_response: &mut ResponseHeader,
        _ctx: &mut PluginCtx,
    ) -> Result<()> {
        Ok(())
    }

    /// Handle the response body chunks
    ///
    /// # Arguments
    ///
    /// * `_session` - Mutable reference to the current session
    /// * `_body` - Mutable reference to an optional Bytes containing the body chunk
    /// * `_end_of_stream` - Boolean indicating if this is the last chunk
    /// * `_ctx` - Mutable reference to the plugin context
    fn response_body_filter(
        &self,
        _session: &mut Session,
        _body: &mut Option<Bytes>,
        _end_of_stream: bool,
        _ctx: &mut PluginCtx,
    ) -> Result<()> {
        Ok(())
    }
}

/// Represents the parameters extracted from a route match
///
/// This struct encapsulates the parameters that are extracted when a route is matched,
/// either by a regex pattern or a path matching algorithm. It provides a unified
/// interface to access these parameters, regardless of their source.
#[derive(Debug, Default)]
pub struct RouteParams {
    /// A vector of strings containing the captured parameters
    ///
    /// For regex matches, this includes all captured groups.
    /// For path matches, this includes all path segments that were matched as parameters.
    params: Vec<String>,
}

impl RouteParams {
    pub fn new_caps(caps: &Captures) -> Self {
        Self {
            params: caps
                .iter()
                .filter_map(|s| s.map(|ss| ss.as_str().to_string()))
                .collect(),
        }
    }

    pub fn new_params(params: &Params) -> Self {
        Self {
            params: params.iter().map(|(_, v)| v.to_string()).collect(),
        }
    }

    pub fn get(&self, idx: usize) -> Option<&str> {
        self.params.get(idx).map(|s| s.as_str())
    }
}

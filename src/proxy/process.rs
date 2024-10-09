use std::borrow::Cow;
use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use http::StatusCode;
use log::{error, info, log_enabled, Level};
use matchit::{InsertError, Router};
use once_cell::sync::Lazy;
use pingora::{http::ResponseHeader, prelude::*, proxy::ProxyHttp};
use regex::Regex;

use crate::{
    clusters::ClusterManager,
    core::plugin::{Plugin, PluginCtx, RouteParams},
    utils::send_response,
};

/// Represents the main proxy structure
pub struct Proxy {
    plugins: Vec<Box<dyn Plugin>>,
    /// Router for matching requests to pipelines
    matcher: MatchEntry,
    /// Manager for handling clusters of backends
    cluster_manager: ClusterManager,
}

impl Proxy {
    /// Creates a new Proxy instance
    ///
    /// # Arguments
    ///
    /// * `matcher` - The MatchEntry for routing requests
    /// * `cluster_manager` - The ClusterManager for handling backend clusters
    pub fn new(
        matcher: MatchEntry,
        cluster_manager: ClusterManager,
        plugins: Vec<Box<dyn Plugin>>,
    ) -> Self {
        Self {
            matcher,
            cluster_manager,
            plugins,
        }
    }
}

static NOT_FOUND: Lazy<Bytes> = Lazy::new(|| Bytes::from("not found"));

/// Context for the proxy, holding plugins and other request-specific data
#[derive(Default)]
pub struct ProxyCtx {
    /// List of plugins to be applied
    plugins: Arc<Vec<Box<dyn Plugin>>>,
    /// The selected cluster for the request
    cluster: Option<String>,
    /// Context for plugin execution
    plugin_ctx: PluginCtx,
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = ProxyCtx;

    /// Creates a new context for each request
    fn new_ctx(&self) -> Self::CTX {
        Self::CTX::default()
    }

    /// Filters incoming requests
    ///
    /// This method matches the request to a pipeline, initializes plugins,
    /// and applies request filters from each plugin.
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // global plugins
        for plugin in &self.plugins {
            let stop = plugin.request_filter(session, &mut ctx.plugin_ctx).await?;
            if stop {
                return Ok(true);
            }
        }

        // Match request to pipeline
        if let Some((route_params, ppl)) = self.matcher.match_request(session) {
            ctx.cluster = Some(ppl.cluster.clone());

            // Initialize plugins
            ctx.plugins = ppl.plugins.clone();
            ctx.plugin_ctx.route_params = Some(route_params);

            // Apply request filters from each plugin
            for plugin in ctx.plugins.iter() {
                let should_stop = plugin.request_filter(session, &mut ctx.plugin_ctx).await?;
                if should_stop {
                    return Ok(true);
                }
            }
        } else {
            send_response(
                session,
                StatusCode::NOT_FOUND,
                None,
                Some(NOT_FOUND.clone()),
                None,
            )
            .await?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Filters the request body
    ///
    /// Applies request body filters from each plugin.
    async fn request_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // global plugins
        for plugin in &self.plugins {
            plugin
                .request_body_filter(session, body, end_of_stream, &mut ctx.plugin_ctx)
                .await?;
        }
        for plugin in ctx.plugins.iter() {
            plugin
                .request_body_filter(session, body, end_of_stream, &mut ctx.plugin_ctx)
                .await?;
        }
        Ok(())
    }

    /// Filters the upstream request
    ///
    /// Applies upstream request filters from each plugin.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // global plugins
        for plugin in &self.plugins {
            plugin
                .upstream_request_filter(session, upstream_request, &mut ctx.plugin_ctx)
                .await?;
        }
        for plugin in ctx.plugins.iter() {
            plugin
                .upstream_request_filter(session, upstream_request, &mut ctx.plugin_ctx)
                .await?;
        }
        Ok(())
    }

    /// Filters the response
    ///
    /// Applies response filters from each plugin.
    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // global plugins
        for plugin in self.plugins.iter() {
            plugin
                .response_filter(session, upstream_response, &mut ctx.plugin_ctx)
                .await?;
        }
        for plugin in ctx.plugins.iter() {
            plugin
                .response_filter(session, upstream_response, &mut ctx.plugin_ctx)
                .await?;
        }
        Ok(())
    }

    /// Filters the response body
    ///
    /// Applies response body filters from each plugin.
    fn response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>> {
        // global plugins
        for plugin in self.plugins.iter() {
            plugin.response_body_filter(session, body, end_of_stream, &mut ctx.plugin_ctx)?;
        }
        for plugin in ctx.plugins.iter() {
            plugin.response_body_filter(session, body, end_of_stream, &mut ctx.plugin_ctx)?;
        }
        Ok(None)
    }

    /// This filter is called when the entire response is sent to the downstream successfully or
    /// there is a fatal error that terminate the request.
    ///
    /// An error log is already emitted if there is any error. This phase is used for collecting
    /// metrics and sending access logs.
    async fn logging(&self, session: &mut Session, e: Option<&Error>, _ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if log_enabled!(Level::Info) {
            let req = session.req_header();
            let resp = session.response_written();

            let status = resp
                .map_or(StatusCode::INTERNAL_SERVER_ERROR, |r| r.status)
                .as_u16();
            let body_bytes_sent = session.body_bytes_sent();
            let remote_addr = session
                .client_addr()
                .map_or(Cow::Borrowed("-"), |ip| Cow::Owned(ip.to_string()));
            // 使用类似nginx 的格式打日志
            info!(
                "{} \"{} {}\" {} {}",
                remote_addr, req.method, req.uri, status, body_bytes_sent
            );
        }
        if let Some(e) = e {
            error!(error:? = e; "Error occurred");
        }
    }

    /// Selects an upstream peer for the request
    ///
    /// This method selects a backend from the appropriate cluster for the request.
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let cluster = ctx
            .cluster
            .as_ref()
            .ok_or(Error::new(ErrorType::Custom("no cluster")))?;
        let lb = self
            .cluster_manager
            .get_cluster(cluster)
            .ok_or(Error::new(ErrorType::ConnectNoRoute))?;
        let backend = lb.select_backend(session.req_header()).ok_or(
            Error::new(ErrorType::Custom("no backend"))
                .more_context(format!("cluster: {}", cluster)),
        )?;
        Ok(Box::new(HttpPeer::new(backend, false, "a.b.c".to_string())))
    }
}

/// Represents a pipeline of plugins for a specific route
pub struct Pipeline {
    /// List of plugin builders for this pipeline
    plugins: Arc<Vec<Box<dyn Plugin>>>,
    /// The cluster associated with this pipeline
    cluster: String,
}

impl Pipeline {
    /// Creates a new Pipeline instance
    pub fn new(plugins: Arc<Vec<Box<dyn Plugin>>>, cluster: String) -> Self {
        Self { plugins, cluster }
    }
}

/// Struct for matching requests to pipelines
pub struct MatchEntry {
    /// Router for non-regex URI matching
    non_reg_uri: Router<Arc<Pipeline>>,
    /// Vector of regex patterns and associated pipelines
    regex_uris: Vec<(Regex, Arc<Pipeline>)>,
}

impl MatchEntry {
    /// Creates a new MatchEntry instance
    pub fn new() -> Self {
        Self {
            non_reg_uri: Router::new(),
            regex_uris: vec![],
        }
    }

    /// Inserts a new route into the non-regex router
    pub fn insert_route(&mut self, path: &str, ppl: Arc<Pipeline>) -> Result<(), InsertError> {
        if self.non_reg_uri.at(path).is_ok() {
            return Ok(());
        }
        self.non_reg_uri.insert(path, ppl)
    }

    /// Adds a new regex route
    pub fn add_regex_route(&mut self, re: Regex, ppl: Arc<Pipeline>) {
        self.regex_uris.push((re, ppl));
    }

    /// Matches a request to a pipeline
    fn match_request(&self, session: &mut Session) -> Option<(RouteParams, Arc<Pipeline>)> {
        let uri = session.req_header().uri.path();
        if let Ok(ppl) = self.non_reg_uri.at(uri) {
            return Some((RouteParams::new_params(&ppl.params), ppl.value.clone()));
        }

        for (re, ppl) in self.regex_uris.iter() {
            if let Some(caps) = re.captures(uri) {
                return Some((RouteParams::new_caps(&caps), ppl.clone()));
            }
        }
        None
    }
}

impl Default for MatchEntry {
    fn default() -> Self {
        Self::new()
    }
}

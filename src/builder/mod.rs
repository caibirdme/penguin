use regex::Regex;
use snafu::ResultExt;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    clusters::{discovery::ResolverWrapper, Resolver},
    config::def::{DiscoveryProvider, Plugin, ResolverType, Route, StrMatch},
    core::plugin::Plugin as PluginTrait,
    plugins::create_plugin_builder,
    proxy::process::{MatchEntry, Pipeline},
};
use errors::*;

pub mod errors;

pub type BuilderResult<T> = Result<T, errors::BuilderError>;

pub fn init_discovery_providers(
    cfg: &[DiscoveryProvider],
) -> BuilderResult<HashMap<ResolverType, Arc<dyn Resolver>>> {
    let mut providers: HashMap<ResolverType, Arc<dyn Resolver>> = HashMap::new();
    for provider in cfg {
        if provider.resolver_type == ResolverType::DNS {
            let resolver = ResolverWrapper::new();
            providers.insert(ResolverType::DNS, Arc::new(resolver));
        }
    }
    Ok(providers)
}

pub fn init_routes(cfg: Vec<Route>) -> BuilderResult<MatchEntry> {
    let mut matcher = MatchEntry::new();
    for one_route in cfg {
        // build plugins
        let ppl = build_pipleline(one_route.plugins, &one_route.cluster)?;

        // build matcher
        if let Some(uri) = one_route.matcher.uri {
            match uri {
                StrMatch::Regexp(re) => {
                    let re = Regex::new(&re).context(RegexpSnafu { re })?;
                    matcher.add_regex_route(re, ppl);
                }
                StrMatch::Prefix(prefix) => {
                    matcher
                        .insert_route(revise_prefix(&prefix).as_str(), ppl)
                        .context(InsertRouteSnafu { path: prefix })?;
                }
                StrMatch::Exact(exact) => {
                    matcher
                        .insert_route(&exact, ppl)
                        .context(InsertRouteSnafu { path: exact })?;
                }
            }
        } else {
            unimplemented!("uri is required for now");
        }
    }
    Ok(matcher)
}

/// revise prefix to be a valid path for matchit
/// if it suffix of *, remove * and append {*rest}
/// else append {*rest}
fn revise_prefix(prefix: &str) -> String {
    if prefix.ends_with("*") {
        prefix.to_string().replace("*", r"{*rest}")
    } else {
        format!("{}{}", prefix, r"{*rest}")
    }
}

fn build_pipleline(cfg: Option<Vec<Plugin>>, cluster: &str) -> BuilderResult<Arc<Pipeline>> {
    let plugin_builder = build_plugin_list(cfg)?;
    Ok(Arc::new(Pipeline::new(
        Arc::new(plugin_builder),
        cluster.to_string(),
    )))
}

pub fn build_plugin_list(cfg: Option<Vec<Plugin>>) -> BuilderResult<Vec<Box<dyn PluginTrait>>> {
    let mut plugin_builder = vec![];
    if let Some(plugins) = cfg {
        for pl in plugins {
            let builder = create_plugin_builder(&pl.name, pl.config)
                .context(PluginBuildSnafu { name: pl.name })?;
            plugin_builder.push(builder);
        }
    }
    Ok(plugin_builder)
}

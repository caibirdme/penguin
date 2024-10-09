use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use penguin::{
    builder::{build_plugin_list, init_discovery_providers, init_routes},
    clusters::ClusterManager,
    config::{
        args::{Args, Command},
        def::{Config, Listener, Service as ServiceConf},
        load_config,
    },
    errors::*,
    proxy::Proxy,
};
use pingora::{
    prelude::*, proxy::http_proxy_service_with_name, server::configuration::ServerConf,
    services::Service as PingoraServiceTrait,
};
use snafu::ResultExt;
use validator::Validate;

fn main() -> Result<(), AppError> {
    let args = Args::parse();
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();
    match args.command {
        Command::ValidateConfig => {
            load_and_validate_config(args.config)?;
            Ok(())
        }
        Command::Run => {
            let config = load_and_validate_config(args.config)?;
            // init discovery providers
            let resolvers = init_discovery_providers(&config.discovery_providers).unwrap();

            // init pingora server
            let mut server = Server::new(None).unwrap();

            // for each service in config, init its routes, clusters
            // combine them into a Proxy object(which is an implementation of Pingora ProxyHttp Trait)
            // create a pingora service based on the Proxy object
            // add the service to the pingora server
            let mut svcs = vec![];
            for ServiceConf {
                name,
                server_conf,
                plugins,
                listeners,
                routes,
                clusters,
            } in config.services
            {
                let routes = init_routes(routes).context(BuilderSnafu)?;
                let clusters = ClusterManager::new(clusters, &resolvers).context(ClusterSnafu)?;
                let global_plugins = build_plugin_list(plugins).context(BuilderSnafu)?;
                let proxy = Proxy::new(routes, clusters, global_plugins);
                let svc =
                    create_service(name, server_conf, listeners, proxy).context(PingoraSnafu)?;
                svcs.push(svc);
            }
            server.add_services(svcs);
            // run the server
            server.run_forever();
        }
    }
}

fn create_service(
    name: String,
    server_conf: Option<ServerConf>,
    listeners: Vec<Listener>,
    proxy: Proxy,
) -> Result<Box<dyn PingoraServiceTrait>> {
    let mut svc =
        http_proxy_service_with_name(&Arc::new(server_conf.unwrap_or_default()), proxy, &name);
    for listener in listeners {
        let addr = listener.address.to_string();
        match listener.ssl_config {
            Some(ssl_config) => {
                svc.add_tls(&addr, &ssl_config.cert_path, &ssl_config.key_path)?;
            }
            None => {
                svc.add_tcp(&addr);
            }
        }
    }
    Ok(Box::new(svc))
}

fn load_and_validate_config(path: PathBuf) -> Result<Config, AppError> {
    let config = load_config(path.as_path().to_str().unwrap()).context(ConfigSnafu)?;
    config.validate().context(ValidationSnafu)?;
    Ok(config)
}

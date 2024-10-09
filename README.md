# Penguin API Gateway 

## Introduction

In a word: **Penguin is to Pingora what Kong or APISIX is to openresty.**

Penguin is an API gateway built on top of [Pingora](https://github.com/cloudflare/pingora).

Pingora is very similar to openresty, it's a **framework** for building applications. It's **not** an out of box solution that end-user can use. This is where penguin comes in. 

Penguin is high performance, extensible, and easy to use.


Key features include:
- Concise configuration based on YAML
- Flexible routing matching rules
- composable plugins
- **Easy to extend and customize through plugins**

## Quick View

Here's an example of the configuration file:

```yaml
services:
  - name: service1
    listeners:
      - name: listener1
        address: 0.0.0.0:8080
        protocol: http
    routes:
      - name: public_route
        match:
          uri: 
            regexp: "/public/api/v\\d+.*"
        plugins:
          - name: cms_rate
            config:
              count: 3
          - name: echo
            config:
              body: "hello, world"
              status_code: 200
              headers:
                x-custom-header: "123"
                foo-bar: "baz"
        cluster: public_backend_cluster
      - name: qq
        match:
          uri: 
            exact: "/param"
        cluster: public_backend_cluster

    clusters:
      - name: public_backend_cluster
        resolver: static
        lb_policy: round_robin
        config:
          endpoints:
            - 127.0.0.1:9933
            - 127.0.0.1:9934
```

## Configuration Explanation

Here's a detailed explanation of the configuration file:

1. pingora supports multiple seperated services, each service has its own listeners and logic. Similarity, our configuration is consists of multiple services.
```yaml
services:
  - name: service_1
    # ...
  - name: service_2
    # ...
```

2. Service Configuration:
```yaml
services:
  - name: service1 # service name, just for user's convenience
    listeners:
      - name: listener1
        address: 0.0.0.0:8443 # address to listen
        protocol: https # protocol to listen
        ssl_config: # tls config if protocol is https
          cert: /path/to/cert
          key: /path/to/key
    plugins: # plugins that will be applied to all routes in this service
      - name: cms_rate # name of the plugin, must match the name in the plugin registry. cms_rate is `count-min-sketch` based rate limiter
        config: # plugin specific configuration
          total: 100 # 100 requests per ${interval}
          interval: 1m
    routes:
      - name: route_1
        match: # match rule is to define how to match incoming requests
          uri: # match by request uri
            regexp: "/public/api/v\\d+.*" # {regexp, prefix, exact} are supported
        plugins: # plugins apply for this route only, order matters, first configured plugin will be applied first
          - name: cms_rate # name of the plugin, must match the name of the plugin in the plugin registry. cms_rate is `count-min-sketch` based rate limiter
            config: # plugin specific configuration
              total: 3 # 3 requests per ${interval}
              interval: 5s
        cluster: cluster_aa # backend cluster to forward to, use this name to refer to the cluster
      - name: route_hello
        match: # match rule is to define how to match incoming requests
          uri: # match by request uri
            prefix: "/hello" # {regexp, prefix, exact} are supported
        cluster: cluster_bb # backend cluster to forward to, use this name to refer to the cluster
      # other routes
    clusters: # set of backend clusters
      - name: cluster_aa # name of the cluster
        resolver: static # how to resolve ip address of the cluster, currently supported: static, dns (consul, k8s, nacos... are on the roadmap)
        lb_policy: round_robin # load balancing policy, currently supported: round_robin, random
        config: # cluster specific configuration
          endpoints: # for static resolver, just list all backend addresses
            - 127.0.0.1:9933
            - 127.0.0.1:9934
      - name: cluster_bb # name of the cluster
        resolver: dns # use dns resolver
        lb_policy: random # load balancing policy, currently supported: round_robin, random
        config: # cluster specific configuration
          host: foo.svc.bar
          port: 8500
```


## Plugin Development

Penguin's plugin system is designed to be highly extensible and customizable. You can easily add new plugins or modify existing ones to suit your needs.

To develop a new plugin, you need to implement the `Plugin` trait and register it in the plugin registry. Here's a simple example of how to create a custom plugin:

```rust
use std::{collections::HashMap, sync::Arc};
use async_trait::async_trait;
// and more others ...
use crate::{
    core::plugin::{Plugin, PluginCtx},
    plugins::{PluginResult,errors::*},
};

pub const ECHO_PLUGIN_NAME: &str = "echo";

#[derive(Clone)]
pub struct EchoPlugin {
    config: Arc<EchoConfig>,
}

#[derive(Debug, Deserialize)]
struct EchoConfig {
    body: Bytes,
    status_code: StatusCode,
    headers: Option<HashMap<String, String>>,
}

// constructor of the plugin
pub fn create_echo_plugin(cfg: Option<YamlValue>) -> PluginResult<Box<dyn PluginBuilder>> {
    // unmarshal config from yaml
    let config: EchoConfig = serde_yaml::from_value(cfg)?;
    // ...

    // create plugin instance
    Ok(Box::new(EchoPlugin { config: Arc::new(config) }))
}

#[async_trait]
impl Plugin for EchoPlugin {
    async fn request_filter(&self, session: &mut Session, _ctx: &mut PluginCtx) -> Result<bool> {
        // your logic here
        // you can store your state in _ctx
        // you can use session to write response directly and return Ok(true) to stop proxying
        // return Ok(false) to continue proxying
        Ok(false)
    }
}
```

Then you need to register the plugin in the plugin registry `src/plugins/mod.rs`:

```rust
static PLUGIN_BUILDER_REGISTRY: Lazy<HashMap<&'static str, PluginInitFn>> = Lazy::new(|| {
    let arr: Vec<(&str, PluginInitFn)> = vec![
        (echo::ECHO_PLUGIN_NAME, Arc::new(echo::create_echo_plugin)),
        (cms_rate::CMS_RATE_PLUGIN_NAME, Arc::new(cms_rate::create_cms_rate_limiter)),
        // (your_plugin_name, Arc::new(your_plugin_create_func),
        // and more others ...
    ];
    arr.into_iter().collect()
});
```

That's it! Your plugin is now registered and can be used in the configuration file. ✿✿ヽ(°▽°)ノ✿

examples:
- [cms_rate](./src/plugins/cms_rate/mod.rs)
- [echo](./src/plugins/echo/mod.rs)

### Plugin trait

Plugin trait is a subset of Pingora's ProxyHttp trait, you can refer to Pingora's [official documentation](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/phase.md) for more information.

```rust
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
```

## TODO

- resolver:
  - [ ] polarismesh
  - [ ] consul
  - [ ] k8s
  - [ ] nacos
- lb_policy:
  - [ ] least_conn
- plugin:
  - [ ] cors
  - [ ] fault injection
  - [ ] better rate limiter
- auth system
- better error handling
- nginx like logging
- self monitoring


## Contributing

We welcome contributions to Penguin! If you have any ideas, suggestions, or bug reports, please feel free to open an issue or submit a pull request.

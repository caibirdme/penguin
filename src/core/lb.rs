use pingora::{
    http::RequestHeader,
    lb::{
        selection::{BackendIter, BackendSelection},
        Backend, LoadBalancer,
    },
};

/// Trait defining the interface for load balancers
///
/// This trait should be implemented by types that provide load balancing functionality.
pub trait LB: Send + Sync {
    /// Selects a backend based on the given request header
    ///
    /// # Arguments
    ///
    /// * `header` - The request header to use for backend selection
    ///
    /// # Returns
    ///
    /// An `Option<Backend>` representing the selected backend, or `None` if no backend is available
    fn select_backend(&self, header: &RequestHeader) -> Option<Backend>;
}

/// Implementation of the `LB` trait for `LoadBalancer<S>`
///
/// This implementation allows the Pingora `LoadBalancer` to be used with our `LB` trait.
impl<S> LB for LoadBalancer<S>
where
    S: BackendSelection + Send + Sync + 'static,
    S::Iter: BackendIter,
{
    /// Selects a backend using the Pingora `LoadBalancer`
    ///
    /// This implementation ignores the request header and uses a default key and TTL.
    ///
    /// # Arguments
    ///
    /// * `_header` - The request header (ignored in this implementation)
    ///
    /// # Returns
    ///
    /// An `Option<Backend>` representing the selected backend, or `None` if no backend is available
    fn select_backend(&self, _header: &RequestHeader) -> Option<Backend> {
        self.select(b"", 256)
    }
}

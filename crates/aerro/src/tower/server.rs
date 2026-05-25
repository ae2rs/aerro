//! Server-side `tower::Layer` carrying `EncodeOptions` and service/RPC names.
//!
//! See spec §11. The layer composes with `tower::ServiceBuilder` and exposes
//! its config to inner services via the request `Extensions`. The actual
//! frame-append + envelope rewrite happens in `#[aerro::handler]` (which reads
//! these extensions at the user's handler boundary).

use crate::{Exposure, wire::encode::EncodeOptions};

#[derive(Debug, Copy, Clone)]
pub struct ServerLayer {
    pub(crate) service: &'static str,
    pub(crate) rpc: &'static str,
    pub(crate) opts: EncodeOptions,
}

impl ServerLayer {
    /// Create a new `ServerLayer` with default options (`Internal` exposure, 16-frame cap).
    pub fn new() -> Self {
        Self {
            service: "unknown",
            rpc: "unknown",
            opts: EncodeOptions::default(),
        }
    }

    /// Set the service name embedded in each [`Frame`](crate::Frame) appended by this layer.
    pub fn service_name(mut self, s: &'static str) -> Self {
        self.service = s;
        self
    }

    /// Set the RPC name embedded in each [`Frame`](crate::Frame).
    pub fn rpc_name(mut self, s: &'static str) -> Self {
        self.rpc = s;
        self
    }

    /// Set the egress [`Exposure`] tier for this handler.
    ///
    /// System-category errors are redacted to `"internal error"` at `Trusted` and `Public` tiers.
    /// Call frames are stripped entirely at `Public`.
    pub fn exposure(mut self, e: Exposure) -> Self {
        self.opts.exposure = e;
        self
    }

    /// Maximum number of [`Frame`](crate::Frame)s retained in the wire envelope.
    ///
    /// Excess frames are collapsed to a synthetic `"<n> frames elided"` frame.
    pub fn max_frames(mut self, n: u8) -> Self {
        self.opts.max_frames = n;
        self
    }

    /// Return the current [`EncodeOptions`] built by this layer.
    pub fn encode_options(&self) -> &EncodeOptions {
        &self.opts
    }
}

impl Default for ServerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for ServerLayer {
    type Service = ServerService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ServerService {
            inner,
            layer: *self,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerService<S> {
    inner: S,
    layer: ServerLayer,
}

impl<S> ServerService<S> {
    pub fn config(&self) -> &ServerLayer {
        &self.layer
    }
}

impl<S, Req> tower::Service<Req> for ServerService<S>
where
    S: tower::Service<Req>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::Layer;

    #[test]
    fn builder_captures_config() {
        let l = ServerLayer::new()
            .service_name("create-user")
            .rpc_name("create")
            .exposure(Exposure::Public)
            .max_frames(8);
        assert_eq!(l.service, "create-user");
        assert_eq!(l.rpc, "create");
        assert_eq!(l.opts.exposure, Exposure::Public);
        assert_eq!(l.opts.max_frames, 8);
    }

    #[test]
    fn wraps_inner_service() {
        let inner = tower::service_fn(|n: u32| async move { Ok::<u32, ()>(n + 1) });
        let svc = ServerLayer::new().layer(inner);
        assert_eq!(svc.config().service, "unknown");
    }
}

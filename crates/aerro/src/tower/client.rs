//! Client-side `tower::Layer` carrying the caller service name.
//!
//! See spec §11. The layer holds the caller identity. Automatic response-side
//! frame appending will land in v1.1 once a body-trailer-rewriting interceptor
//! is in place; for v1, callers append frames manually via
//! [`StatusIntoResultExt::into_aerro`] or rely on the `#[aerro::handler]` macro
//! on the calling side.

#[derive(Debug, Copy, Clone)]
pub struct ClientLayer {
    pub(crate) caller_service: &'static str,
}

impl ClientLayer {
    pub fn new() -> Self {
        Self {
            caller_service: "unknown",
        }
    }

    pub fn caller_service(mut self, s: &'static str) -> Self {
        self.caller_service = s;
        self
    }
}

impl Default for ClientLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for ClientLayer {
    type Service = ClientService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        ClientService {
            inner,
            layer: *self,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientService<S> {
    inner: S,
    layer: ClientLayer,
}

impl<S> ClientService<S> {
    pub fn caller_service(&self) -> &'static str {
        self.layer.caller_service
    }
}

impl<S, Req> tower::Service<Req> for ClientService<S>
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
    fn builder_captures_caller() {
        let l = ClientLayer::new().caller_service("api-gateway");
        assert_eq!(l.caller_service, "api-gateway");
    }

    #[test]
    fn wraps_inner_service() {
        let inner = tower::service_fn(|n: u32| async move { Ok::<u32, ()>(n) });
        let svc = ClientLayer::new().caller_service("api").layer(inner);
        assert_eq!(svc.caller_service(), "api");
    }
}

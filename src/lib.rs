use std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
};

use hyper::{
    client::connect::dns::{GaiFuture, GaiResolver, Name},
    service::Service,
};

#[derive(Debug)]
pub struct CachedFuture {
    inner: GaiFuture,
    cached: Option<std::vec::IntoIter<SocketAddr>>,
    cache: Arc<Mutex<Option<std::vec::IntoIter<SocketAddr>>>>,
}

impl Future for CachedFuture {
    type Output = Result<std::vec::IntoIter<SocketAddr>, std::io::Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        tracing::debug!(?self, "Polling the inner future");

        let pinned = Pin::new(&mut self.inner);
        let inner = futures::ready!(pinned.poll(cx));

        let res = inner.map(|v| v.into_iter().collect::<Vec<_>>());

        if let Ok(addrs) = res.as_ref() {
            tracing::info!(?addrs, "Setting the cached value");
            self.cached = Some(addrs.clone().into_iter());

            let mut resolver = self.cache.lock().unwrap();
            *resolver = Some(addrs.clone().into_iter());
        }

        tracing::info!(?self, "Future has resolved");

        std::task::Poll::Ready(res.map(|v| v.into_iter()))
    }
}

#[derive(Clone, Debug)]
pub struct CachedResolver {
    inner: GaiResolver,
    cached: Arc<Mutex<Option<std::vec::IntoIter<SocketAddr>>>>,
}

impl CachedResolver {
    pub fn new() -> Self {
        Self {
            inner: GaiResolver::new(),
            cached: Arc::new(Mutex::new(None)),
        }
    }
}

impl Service<Name> for CachedResolver {
    type Response = std::vec::IntoIter<SocketAddr>;
    type Error = std::io::Error;
    type Future = CachedFuture;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Name) -> Self::Future {
        tracing::debug!(?req, "Processing a DNS request");

        CachedFuture {
            inner: self.inner.call(req),
            cached: None,
            cache: Arc::clone(&self.cached),
        }
    }
}

use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use hyper::{
    client::connect::dns::{GaiFuture, GaiResolver, Name},
    service::Service,
};

type Addrs = std::vec::IntoIter<SocketAddr>;
type Cache = Arc<Mutex<Option<Addrs>>>;

#[derive(Debug)]
pub struct CachedFuture {
    inner: GaiFuture,
    cache: Cache,
}

impl Future for CachedFuture {
    type Output = Result<Addrs, io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        tracing::debug!("Polling the inner future");

        // Check if we have a cached value
        if let Some(v) = &*self.cache.lock().unwrap() {
            tracing::info!("Using the cached value from a previous query");
            return Poll::Ready(Ok(v.clone()));
        }

        let pinned = Pin::new(&mut self.inner);
        let inner = futures::ready!(pinned.poll(cx));

        let res = inner.map(|v| v.into_iter().collect::<Vec<_>>());

        if let Ok(addrs) = res.as_ref() {
            tracing::info!(?addrs, "Setting the cached value");
            *self.cache.lock().unwrap() = Some(addrs.clone().into_iter());
        }

        Poll::Ready(res.map(|v| v.into_iter()))
    }
}

#[derive(Clone, Debug)]
pub struct CachedResolver {
    inner: GaiResolver,
    cache: Cache,
}

impl CachedResolver {
    pub fn new() -> Self {
        Self {
            inner: GaiResolver::new(),
            cache: Arc::new(Mutex::new(None)),
        }
    }
}

impl Service<Name> for CachedResolver {
    type Response = Addrs;
    type Error = io::Error;
    type Future = CachedFuture;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Name) -> Self::Future {
        tracing::debug!(?req, "Processing a DNS request");

        CachedFuture {
            inner: self.inner.call(req),
            cache: Arc::clone(&self.cache),
        }
    }
}

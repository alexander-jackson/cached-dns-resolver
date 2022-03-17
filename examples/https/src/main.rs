use std::str::FromStr;

use cached_dns_resolver::CachedResolver;
use hyper::client::connect::dns::Name;
use hyper::client::HttpConnector;
use hyper::service::Service;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut resolver = CachedResolver::new();
    let response = resolver
        .call(Name::from_str("starlingbank.com")?)
        .await?
        .into_iter()
        .collect::<Vec<_>>();

    tracing::info!(?response, "Got a response from the DNS resolver");

    let mut connector = HttpConnector::new_with_resolver(resolver);
    connector.enforce_http(false);

    let secure_connector = HttpsConnector::new_with_connector(connector);
    let client = Client::builder().build::<_, Body>(secure_connector);

    let request = Request::builder()
        .method(Method::POST)
        .uri("https://starlingbank.com")
        .body(Body::empty())?;

    let res = client.request(request).await?;

    tracing::info!(?res, "Got a response from the request");

    Ok(())
}

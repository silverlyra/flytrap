use std::net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6};

use metrics_exporter_prometheus::PrometheusBuilder;
use tracing::Level;

mod app;
mod backend;

#[tokio::main]
async fn main() {
    app::start(setup().await, listen_address()).await;
}

async fn setup() -> backend::Backend {
    setup_tracing();
    setup_metrics();

    backend::Backend::new().await
}

fn listen_address() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);

    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}

#[cfg(debug_assertions)]
fn setup_tracing() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(Level::TRACE)
        .init();
}

#[cfg(not(debug_assertions))]
fn setup_tracing() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

fn setup_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 9091, 0, 0))
        .set_buckets(&[
            0.001, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.75, 1.0,
            1.25, 1.5, 2.0, 2.5, 5.0,
        ])
        .expect("failed to initialize Prometheus histogram buckets")
        .install()
        .expect("failed to set up metrics endpoint");
}

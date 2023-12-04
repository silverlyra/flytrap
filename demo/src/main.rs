use std::net::{IpAddr, Ipv6Addr, SocketAddr};

mod app;
mod backend;

#[tokio::main]
async fn main() {
    let backend = backend::Backend::new().await;
    app::start(backend, listen_address()).await;
}

fn listen_address() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);

    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}

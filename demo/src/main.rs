use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html, routing::get, Json, Router};
use axum_extra::TypedHeader;
use flytrap::{
    http::{FlyClientIp, FlyRegion},
    Error, Peer, Placement, RegionDetails, Resolver,
};
use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Template, Clone, Debug)]
#[template(path = "index.html")]
struct IndexResponse {
    client: IpAddr,
    placement: Placement,
    host: RegionDetails<'static>,
    edge: RegionDetails<'static>,
    peers: Vec<Peer>,
}

#[tokio::main]
async fn main() {
    let resolver = Resolver::new().expect("failed to configure DNS resolver");

    let app = Router::new()
        .route("/", get(index))
        .route("/ip", get(ip))
        .route("/peers", get(peers))
        .route("/regions", get(regions))
        .route("/up", get(up))
        .with_state(resolver);

    let listen = listen_address();
    let listener = TcpListener::bind(&listen)
        .await
        .expect("failed to listen for requests");

    println!("Listening on {listen}");
    axum::serve(listener, app).await.unwrap();
}

async fn index(
    State(resolver): State<Resolver>,
    TypedHeader(ip): TypedHeader<FlyClientIp>,
    TypedHeader(edge): TypedHeader<FlyRegion>,
) -> Result<IndexResponse, StatusCode> {
    let placement = Placement::current().map_err(error)?;
    let host_region = placement.region().ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let edge_region = edge.region().ok_or(StatusCode::NOT_IMPLEMENTED)?;

    let app = resolver.current().map_err(error)?;
    let mut peers = app.peers().await.map_err(error)?;
    peers.sort();

    Ok(IndexResponse {
        client: ip.into_inner(),
        placement,
        host: host_region.details(),
        edge: edge_region.details(),
        peers,
    })
}

async fn ip(
    TypedHeader(ip): TypedHeader<FlyClientIp>,
    TypedHeader(edge): TypedHeader<FlyRegion>,
) -> Html<String> {
    Html(format!(
        "<b>Your IP:</b> <code>{ip}</code> <i>(via <abbr style=\"font-variant: small-caps;\">{edge}</abbr>)</i>"
    ))
}

#[derive(Serialize, Clone, Debug)]
struct PeersResponse {
    peers: Vec<Peer>,
}

async fn peers(State(resolver): State<Resolver>) -> Result<Json<PeersResponse>, StatusCode> {
    let app = resolver.current().map_err(error)?;

    let peers = app.peers().await.map_err(error)?;

    Ok(Json(PeersResponse { peers }))
}

#[derive(Serialize, Clone, Debug)]
struct RegionsResponse {
    regions: Vec<RegionDetails<'static>>,
}

async fn regions(State(resolver): State<Resolver>) -> Result<Json<RegionsResponse>, StatusCode> {
    let app = resolver.current().map_err(error)?;
    let regions = app.regions().await.map_err(error)?;

    Ok(Json(RegionsResponse {
        regions: regions.into_iter().map(|region| region.details()).collect(),
    }))
}

async fn up() -> Result<Html<String>, StatusCode> {
    let Placement {
        app,
        allocation,
        private_ip,
        ..
    } = Placement::current().map_err(error)?;

    Ok(Html(format!(
        "Fly.io app <b>{app}</b> machine <code>{allocation}</code> running at {private_ip}."
    )))
}

fn error(err: Error) -> StatusCode {
    eprintln!("error: {err}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn listen_address() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);

    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}

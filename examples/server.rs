use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use axum::{
    extract::State, http::StatusCode, response::Html, routing::get, Json, Router, TypedHeader,
};
use flytrap::{
    http::{FlyClientIp, FlyRegion},
    Peer, Placement, Region, RegionDetails, Resolver,
};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
struct PlacementResponse {
    now: chrono::DateTime<chrono::Utc>,
    client: IpAddr,
    #[serde(flatten)]
    placement: Placement,
    host: Option<RegionDetails<'static>>,
    edge: Option<RegionDetails<'static>>,
}

#[tokio::main]
async fn main() {
    let resolver = Resolver::new().expect("failed to configure DNS resolver");

    let app = Router::new()
        .route("/", get(placement))
        .route("/ip", get(ip))
        .route("/peers", get(peers))
        .route("/regions", get(regions))
        .with_state(resolver);

    let listen = listen_address();
    println!("Listening on {listen}");

    // run it with hyper on localhost:3000
    axum::Server::bind(&listen)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn placement(
    TypedHeader(ip): TypedHeader<FlyClientIp>,
    TypedHeader(edge): TypedHeader<FlyRegion>,
) -> Result<Json<PlacementResponse>, StatusCode> {
    let here = Placement::current().map_err(|err| {
        println!("error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let region = here.region().as_ref().map(Region::details);

    Ok(Json(PlacementResponse {
        now: chrono::Utc::now(),
        client: ip.into_inner(),
        placement: here,
        host: region,
        edge: edge.region().as_ref().map(Region::details),
    }))
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
    let app = resolver.current().map_err(|err| {
        println!("error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let peers = app.peers().await.map_err(|err| {
        println!("error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(PeersResponse { peers }))
}

#[derive(Serialize, Clone, Debug)]
struct RegionsResponse {
    regions: Vec<RegionDetails<'static>>,
}

async fn regions(State(resolver): State<Resolver>) -> Result<Json<RegionsResponse>, StatusCode> {
    let app = resolver.current().map_err(|err| {
        println!("error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let regions = app.regions().await.map_err(|err| {
        println!("error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(RegionsResponse {
        regions: regions.into_iter().map(|region| region.details()).collect(),
    }))
}

fn listen_address() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);

    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html, routing::get, Json, Router};
use axum_extra::TypedHeader;
use flytrap::{
    http::{FlyClientIp, FlyRegion},
    ping::PeerStatus,
    Placement, RegionDetails,
};
use serde::Serialize;
use tokio::{net::TcpListener, sync::oneshot};

use super::backend::Backend;

pub async fn start(backend: Backend, listen: SocketAddr) {
    let (stop_tx, stop_rx) = oneshot::channel();
    backend.poll_status(Duration::from_millis(125), stop_rx);

    let app = Router::new()
        .route("/", get(index))
        .route("/ip", get(ip))
        .route("/peers", get(peers))
        .route("/up", get(up))
        .with_state(backend);

    let listener = TcpListener::bind(listen)
        .await
        .expect("failed to listen for HTTP requests");
    println!("listening on {listen}");

    axum::serve(listener, app).await.unwrap();
    let _ = stop_tx.send(());
}

#[derive(Template, Clone, Debug)]
#[template(path = "index.html")]
struct IndexResponse {
    client: IpAddr,
    placement: Placement,
    host: RegionDetails<'static>,
    edge: RegionDetails<'static>,
    peers: Vec<PeerStatus>,
}

async fn index(
    State(backend): State<Backend>,
    TypedHeader(ip): TypedHeader<FlyClientIp>,
    TypedHeader(edge): TypedHeader<FlyRegion>,
) -> Result<IndexResponse, StatusCode> {
    let host_region = backend
        .placement
        .region()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let edge_region = edge.region().ok_or(StatusCode::NOT_IMPLEMENTED)?;

    let peers = backend.status.read().await.clone();

    Ok(IndexResponse {
        client: ip.into_inner(),
        placement: backend.placement.clone(),
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
#[serde(rename_all = "camelCase")]
struct PeersResponse {
    peers: Vec<PeerStatus>,
}

async fn peers(State(backend): State<Backend>) -> Result<Json<PeersResponse>, StatusCode> {
    let peers = backend.status.read().await.clone();

    Ok(Json(PeersResponse { peers }))
}

async fn up(State(backend): State<Backend>) -> Result<Html<String>, StatusCode> {
    let Placement {
        app,
        allocation,
        private_ip,
        ..
    } = &backend.placement;

    Ok(Html(format!(
        "Fly.io app <b>{app}</b> machine <code>{allocation}</code> running at {private_ip}."
    )))
}

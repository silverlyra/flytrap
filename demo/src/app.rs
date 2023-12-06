use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html, routing::get, Json, Router};
use axum_extra::TypedHeader;
use flytrap::{
    http::{FlyClientIp, FlyRegion},
    ping, Peer, Placement, RegionDetails,
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
        .route("/status", get(status))
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
    peers: Vec<ping::PeerStatus>,
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
struct StatusResponse {
    #[serde(flatten)]
    placement: Placement,
    region: RegionDetails<'static>,
    peers: Vec<PeerStatus>,
}

#[derive(Serialize, Clone, Debug)]
struct PeerStatus {
    #[serde(flatten)]
    peer: Peer,
    available: bool,
    seen: Option<ping::Timestamp>,
    replied: Option<ping::Timestamp>,
    latency: Option<PeerLatency>,
}

impl From<&ping::PeerStatus> for PeerStatus {
    fn from(status: &ping::PeerStatus) -> Self {
        Self {
            peer: status.peer.clone(),
            available: status.available,
            seen: status.last_seen,
            replied: status.last_replied,
            latency: status.latency.map(PeerLatency::from),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
struct PeerLatency {
    average: f32,
    latest: f32,
}

impl From<ping::Latency> for PeerLatency {
    fn from(lag: ping::Latency) -> Self {
        Self {
            average: (lag.average.as_secs_f64() * 1000.0) as f32,
            latest: (lag.latest.as_secs_f64() * 1000.0) as f32,
        }
    }
}

async fn status(State(backend): State<Backend>) -> Result<Json<StatusResponse>, StatusCode> {
    let placement = backend.placement.clone();
    let region = placement
        .region()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?
        .details();

    let peers = backend
        .status
        .read()
        .await
        .iter()
        .map(PeerStatus::from)
        .collect();

    Ok(Json(StatusResponse {
        placement,
        region,
        peers,
    }))
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

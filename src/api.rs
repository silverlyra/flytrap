//! [Access][Client] the Fly.io [Machines API][].
//!
//! [Machines API]: https://fly.io/docs/machines/api/

use std::net::Ipv6Addr;

use http::header;
use reqwest::{IntoUrl, Method, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

#[cfg(feature = "environment")]
use crate::placement::{private_address, Placement};
#[cfg(feature = "regions")]
use crate::Region;
use crate::{Error, Location};

/// A client for the Fly.io [Machines API][].
///
/// [Machines API]: https://fly.io/docs/machines/api/
///
/// ```no_run
/// use flytrap::api::Client;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let token = std::env::var("FLY_API_TOKEN")?;
/// let client = Client::new(token);
///
/// for app in client.apps("personal").await? {
///     println!("{}: {} machine(s)", app.name, app.machine_count);
/// }
///
/// for machine in client.peers().await? {
///     println!("{} in {}: {:?}", machine.name, machine.location, machine.state);
/// }
///
/// Ok(())
/// # }
/// ```
pub struct Client {
    http: reqwest::Client,
    origin: Url,
    token: String,
}

impl Client {
    pub const PUBLIC_ORIGIN: &'static str = "https://api.machines.dev";
    pub const PRIVATE_ORIGIN: &'static str = "http://_api.internal:4280";

    pub const USER_AGENT: &'static str =
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    /// Create a [Client] for the Machines API. An [authentication token][] is
    /// required.
    ///
    /// When called under a Fly.io hosted environment (or when the _detect_
    /// feature is active and a Fly.io Wireguard connection is present), uses
    /// the `http://_api.internal` origin. Otherwise, uses the
    /// `https://api.machines.dev` origin.
    ///
    /// [authentication token]: https://fly.io/docs/machines/api/working-with-machines-api/#authentication
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_origin(Self::default_origin(), token)
    }

    /// Create a [Client] which sends API Requests to the given `origin`.
    pub fn with_origin(origin: impl IntoUrl, token: impl Into<String>) -> Self {
        Self::with_client(Default::default(), origin, token)
    }

    /// Create a [Client] wrapping an explicit [`reqwest::Client`].
    pub fn with_client(
        http_client: reqwest::Client,
        origin: impl IntoUrl,
        token: impl Into<String>,
    ) -> Self {
        Self {
            http: http_client,
            origin: origin
                .into_url()
                .expect("invalid Fly.io Machines API base URL"),
            token: token.into(),
        }
    }

    /// List the Fly.io [apps][AppEntry] under the given `organization`.
    pub async fn apps(&self, organization: impl AsRef<str>) -> Result<OrganizationApps, Error> {
        self.request(Method::GET, "/v1/apps")
            .query(&OrganizationAppsQuery {
                organization: organization.as_ref(),
            })
            .send()
            .await
            .map_err(Error::from)?
            .error_for_status()
            .map_err(Error::from)?
            .json()
            .await
            .map_err(Error::from)
    }

    /// List Fly.io [machines][Machine] for the given `app`.
    pub async fn machines(&self, app: impl AsRef<str>) -> Result<Vec<Machine>, Error> {
        let app = app.as_ref();

        self.request(Method::GET, format!("/v1/apps/{app}/machines"))
            .send()
            .await
            .map_err(Error::from)?
            .error_for_status()
            .map_err(Error::from)?
            .json()
            .await
            .map_err(Error::from)
    }

    /// List Fly.io [machines][Machine] for the current app, excluding the
    /// current machine.
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub async fn peers(&self) -> Result<Vec<Machine>, Error> {
        let placement = Placement::current()?;
        let id = match placement.machine {
            Some(ref machine) => machine.id.as_str(),
            None => return Err(Error::Unavailable),
        };

        let mut machines = self.machines(&placement.app).await?;
        machines.retain(move |m| m.id != id);

        Ok(machines)
    }

    fn request(&self, method: Method, url: impl AsRef<str>) -> RequestBuilder {
        let url = self
            .origin
            .join(url.as_ref())
            .expect("invalid Machines API request URL");

        self.http
            .request(method, url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.token.as_str()),
            )
            .header(header::USER_AGENT, Self::USER_AGENT)
    }

    #[cfg(feature = "environment")]
    fn default_origin() -> Url {
        let origin = match private_address() {
            Some(_) => Self::PRIVATE_ORIGIN,
            None => Self::PUBLIC_ORIGIN,
        };

        Url::parse(origin).unwrap()
    }
}

#[cfg(feature = "environment")]
#[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
impl Default for Client {
    fn default() -> Self {
        let token = std::env::var("FLY_API_TOKEN").expect("$FLY_API_TOKEN not set");

        Self {
            http: Default::default(),
            origin: Self::default_origin(),
            token,
        }
    }
}

/// The response structure returned by [`Client::apps`].
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OrganizationApps {
    #[serde(rename = "total_apps")]
    pub total: usize,
    pub apps: Vec<AppEntry>,
}

impl OrganizationApps {
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, AppEntry> {
        self.apps.iter()
    }
}

impl IntoIterator for OrganizationApps {
    type Item = AppEntry;
    type IntoIter = std::vec::IntoIter<AppEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.apps.into_iter()
    }
}

/// A Fly.io application, as made available by [`Client::apps`].
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub machine_count: usize,
    #[serde(rename = "network")]
    pub network_name: String,
}

#[derive(Serialize, Debug)]
struct OrganizationAppsQuery<'a> {
    #[serde(rename = "org_slug")]
    pub organization: &'a str,
}

/// A Fly.io [machine][].
///
/// [machine]: https://docs.machines.dev/#model/machine
#[derive(Deserialize, Serialize, Clone, Debug)]
#[non_exhaustive]
pub struct Machine {
    /// A stable identifier for the Machine.
    pub id: String,
    /// A unique name for the Machine.
    pub name: String,
    pub state: MachineState,
    #[serde(rename = "region")]
    pub location: Location,
    /// An identifier for the current running/ready version of the Machine.
    /// Every Update request potentially changes the `instance_id`.
    pub instance_id: String,
    pub private_ip: Ipv6Addr,
    // TODO: created_at, updated_At
    #[serde(default)]
    pub checks: Vec<MachineCheckState>,
    #[serde(default)]
    pub host_status: HostStatus,
}

impl Machine {
    /// Checks if this machine’s `state` is `started`.
    pub fn is_running(&self) -> bool {
        self.state.is_ready()
    }

    /// Checks if this machine is running, and its [health
    /// checks][MachineCheckState] (if any) are passing.
    pub fn is_ready(&self) -> bool {
        self.is_running()
            && self.host_status.is_ready()
            && self.checks.iter().all(MachineCheckState::is_ready)
    }

    /// Return this machine’s [`Region`], if its region code was recognized.
    #[cfg(feature = "regions")]
    #[cfg_attr(docsrs, doc(cfg(feature = "regions")))]
    pub const fn region(&self) -> Option<Region> {
        self.location.region()
    }
}

/// The [state] of a Fly.io [machine][Machine].
///
/// [state]: https://fly.io/docs/machines/machine-states/
#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum MachineState {
    /// The initial status of a machine
    Created,
    /// Transitioning from `Stopped` to `Started`
    Starting,
    /// Running and network-accessible
    Started,
    /// Transitioning from `Started` to `Stopped`
    Stopping,
    /// Exited, either on its own or explicitly stopped
    Stopped,
    /// User-initiated configuration change (image, VM size, etc.) in progress
    Replacing,
    /// User asked for the Machine to be completely removed
    Destroying,
    /// No longer exists
    Destroyed,
}

impl MachineState {
    /// Check if the state indicates the machine is ready to handle requests.
    #[inline]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Started)
    }

    /// If this state is a transition, the state the machine will be in if the
    /// transition completes.
    ///
    /// ```
    /// # use flytrap::api::MachineState;
    /// let state = MachineState::Starting;
    /// assert_eq!(state.target(), Some(MachineState::Started));
    ///
    /// let state = MachineState::Started;
    /// assert_eq!(state.target(), None);
    /// ```
    pub const fn target(&self) -> Option<Self> {
        match self {
            Self::Starting => Some(Self::Started),
            Self::Stopping => Some(Self::Stopped),
            Self::Replacing => Some(Self::Stopped), // TODO: verify
            Self::Destroying => Some(Self::Destroyed),
            _ => None,
        }
    }

    /// Check if the state is an in-progress transition to
    /// [another state][Self::target()].
    #[inline]
    pub const fn is_transition(&self) -> bool {
        self.target().is_some()
    }
}

/// The status of the hardware underlying a Fly.io machine.
#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Default, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum HostStatus {
    #[default]
    Ok,
    Unreachable,
    Unknown,
}

impl HostStatus {
    /// Check if the status indicates the machine is ready to handle requests.
    #[inline]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// The last-observed state of a service health check on a [Machine].
#[derive(Deserialize, Serialize, PartialEq, Eq, Clone, Debug)]
pub struct MachineCheckState {
    pub name: String,
    pub status: CheckStatus,
    pub output: Option<String>,
    // TODO: updated_at
}

impl MachineCheckState {
    /// Check if the status indicates the service is ready to handle requests.
    #[inline]
    pub const fn is_ready(&self) -> bool {
        self.status.is_ready()
    }
}

/// The status of a [health check][MachineCheckState].
#[derive(Deserialize, Serialize, PartialEq, Eq, Copy, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Passing,
    Warning,
    Critical,
}

impl CheckStatus {
    /// Check if the status indicates the service is ready to handle requests.
    #[inline]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Passing)
    }
}

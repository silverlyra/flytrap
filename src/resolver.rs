use std::{
    net::{Ipv6Addr, SocketAddr},
    ops::Deref,
    str::FromStr,
};

use hickory_resolver::{
    config::{NameServerConfig, NameServerConfigGroup, ResolverConfig, ResolverOpts},
    IntoName, Name, TokioAsyncResolver,
};

use crate::{error::Error, placement::private_address, AppResolver, Location, Region};

/// Query the Fly.io [internal DNS][] records.
///
/// [internal DNS]: https://fly.io/docs/reference/private-networking/#fly-internal-addresses
#[derive(Clone)]
pub struct Resolver(pub(crate) TokioAsyncResolver);

impl Resolver {
    /// Create a [`Resolver`] which configures itself based on the host's
    /// [detected][private_address] Fly.io [private network][] address.
    ///
    /// If the host does not appear to be running under Fly.io or connected to
    /// the Wireguard VPN, an `Unavailable` [error][Error] will be returned.
    ///
    /// [private network]: https://fly.io/docs/reference/private-networking/
    #[cfg(any(feature = "detect", feature = "environment"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "detect", feature = "environment"))))]
    pub fn new() -> Result<Self, Error> {
        use std::net::IpAddr;

        let local = private_address().ok_or(Error::Unavailable)?;

        #[cfg(feature = "environment")]
        let hosted = crate::placement::hosted();
        #[cfg(not(feature = "environment"))]
        let hosted = true;

        Ok(Self::with_source(
            SocketAddr::new(IpAddr::V6(dns_server_address(local, hosted)), 53),
            Some(SocketAddr::new(IpAddr::V6(local), 0)),
        ))
    }

    /// Create a [`Resolver`] which will send DNS queries to the given `source`
    /// server to determine your Fly.io application topology.
    ///
    /// If `local` is specified, sockets used to send DNS queries will be bound
    /// to the given address.
    pub fn with_source(source: SocketAddr, local: Option<SocketAddr>) -> Self {
        Self::with_sources(std::iter::once(source), local)
    }

    /// Create a [`Resolver`] which will send DNS queries to the given `sources`
    /// to determine your Fly.io application topology.
    ///
    /// To set `sources` automatically based on the operating systems' DNS
    /// resolution configuration, see [`Resolver::system`].
    ///
    /// If `local` is specified, sockets used to send DNS queries will be bound
    /// to the given address.
    pub fn with_sources(
        sources: impl IntoIterator<Item = SocketAddr>,
        local: Option<SocketAddr>,
    ) -> Self {
        Self(TokioAsyncResolver::tokio(
            Self::config(sources.into_iter(), local),
            Self::options(),
        ))
    }

    /// Create a [`Resolver`] which will send DNS queries to the nameservers
    /// configured in the host operating system (i.e., in `/etc/resolv.conf`).
    #[cfg(feature = "system-resolver")]
    #[cfg_attr(docsrs, doc(cfg(feature = "system-resolver")))]
    pub fn system() -> Result<Self, Error> {
        let resolver = TokioAsyncResolver::tokio_from_system_conf().map_err(Error::from)?;
        Ok(Self(resolver))
    }

    /// Create an [`AppResolver`] for querying the named app.
    pub fn app(&self, name: impl Into<String>) -> AppResolver<'_> {
        AppResolver::new(name, self)
    }

    /// Create an [`AppResolver`] for querying the running app (as set by `$FLY_APP_NAME`).
    ///
    /// If `$FLY_APP_NAME` is unset, an `Unavailable` error is returned.
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn current(&self) -> Result<AppResolver<'_>, Error> {
        match std::env::var("FLY_APP_NAME") {
            Ok(app) => Ok(self.app(app)),
            _ => Err(Error::Unavailable),
        }
    }

    /// Find all apps in the current Fly.io organization.
    pub async fn apps(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .txt("_apps")
            .await?
            .split(',')
            .filter(|app| !app.starts_with("fly-builder-"))
            .map(ToOwned::to_owned)
            .collect())
    }

    /// Find all running instances in the current Fly.io organization, across
    /// all apps.
    pub async fn instances(&self) -> Result<Vec<Instance>, Error> {
        self.txt("_instances")
            .await?
            .split(';')
            .map(|instance| instance.parse())
            .collect()
    }

    /// Perform an arbitrary `TXT` record query on the `.internal` domain.
    pub async fn txt(&self, name: impl IntoName) -> Result<String, Error> {
        let query = name
            .into_name()
            .expect("invalid name")
            .append_domain(&Name::from_ascii("internal").unwrap())
            .expect("invalid query");

        lookup_txt(&self.0, query).await
    }

    fn config(
        sources: impl Iterator<Item = SocketAddr>,
        local: Option<SocketAddr>,
    ) -> ResolverConfig {
        use hickory_resolver::config::Protocol;

        let capacity = 2 * match sources.size_hint() {
            (_, Some(len)) => len,
            (min, _) if min > 0 => min,
            _ => 1,
        };
        let mut servers = NameServerConfigGroup::with_capacity(capacity);
        let domain = Name::from_ascii("internal.").expect("fly.io internal domain");

        for source in sources {
            servers.push(NameServerConfig::new(source, Protocol::Udp));
            servers.push(NameServerConfig::new(source, Protocol::Tcp));
        }

        ResolverConfig::from_parts(Some(domain), vec![], servers.with_bind_addr(local))
    }

    fn options() -> ResolverOpts {
        let mut opts = ResolverOpts::default();
        opts.edns0 = true;

        opts
    }
}

impl From<TokioAsyncResolver> for Resolver {
    fn from(value: TokioAsyncResolver) -> Self {
        Self(value)
    }
}

pub(crate) async fn lookup_txt(
    resolver: &TokioAsyncResolver,
    query: Name,
) -> Result<String, Error> {
    let results = resolver.txt_lookup(query).await.map_err(Error::from)?;

    let length: usize = results
        .iter()
        .flat_map(|r| r.iter().map(|item| item.len()))
        .sum();

    let mut value = String::with_capacity(length);

    for result in results {
        for item in result.iter() {
            if let Ok(text) = std::str::from_utf8(item) {
                value.push_str(text);
            }
        }
    }

    Ok(value)
}

/// A Fly.io [machine][] with an ID and [region][Region].
///
/// `Node` represents a result returned from querying the [`vms.<app>.internal`][dns]
/// TXT record, as [`AppResolver::nodes`] does.
///
/// [machine]: https://fly.io/docs/machines/
/// [dns]: https://fly.io/docs/reference/private-networking/#fly-internal-addresses
///
/// ```
/// use flytrap::{Node, Region};
///
/// let result = "148e21dad76789 sea,4d89699c030518 ams,6e82de14c35038 sin";
/// let nodes: Vec<Node> = result.split(',').filter_map(|n| n.parse().ok()).collect();
///
/// assert_eq!(
///     &nodes,
///     &[
///         Node::new(Region::Seattle, "148e21dad76789"),
///         Node::new(Region::Amsterdam, "4d89699c030518"),
///         Node::new(Region::Singapore, "6e82de14c35038")
///     ]
/// );
/// ```
#[derive(PartialOrd, Ord, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Node {
    #[cfg_attr(feature = "serde", serde(rename = "region"))]
    pub location: Location,
    pub id: String,
}

impl Node {
    pub fn new(region: impl Into<Location>, id: impl Into<String>) -> Self {
        Self {
            location: region.into(),
            id: id.into(),
        }
    }

    /// Create a [`Peer`] from this [`Node`] and an [IP address][Ipv6Addr].
    pub const fn into_peer(self, private_ip: Ipv6Addr) -> Peer {
        Peer {
            node: self,
            private_ip,
        }
    }

    /// Return this node’s [`Region`], if its region code was recognized.
    #[cfg(feature = "regions")]
    #[cfg_attr(docsrs, doc(cfg(feature = "regions")))]
    pub const fn region(&self) -> Option<Region> {
        self.location.region()
    }
}

impl FromStr for Node {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((id, region)) = s.split_once(' ') {
            #[cfg(feature = "regions")]
            let location: Location = region.parse().map_err(Error::from)?;
            #[cfg(not(feature = "regions"))]
            let location: Location = region.to_owned();

            Ok(Self {
                location,
                id: id.to_owned(),
            })
        } else {
            Err(Error::Parse)
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Node {}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// A fully-resolved [`Node`] whose private IP address is known.
///
/// Returned from [`AppResolver::peers`][crate::AppResolver::peers].
#[derive(PartialOrd, Ord, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Peer {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub node: Node,
    pub private_ip: Ipv6Addr,
}

impl Peer {
    pub fn new(region: impl Into<Location>, id: impl Into<String>, private_ip: Ipv6Addr) -> Self {
        Self {
            node: Node::new(region, id),
            private_ip,
        }
    }
}

impl Deref for Peer {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Peer {}

impl std::hash::Hash for Peer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// The result type of `_instances.internal` TXT [queries][].
///
/// [queries]: Resolver::instances
///
/// Querying `_instances.internal` returns every [`Peer`] across all your
/// organization's apps.
///
/// ```
/// use flytrap::{Instance, Region};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = "instance=148e21dad76789,app=flytrap,ip=fdaa:2:224b:a7b:2dbb:3e15:aaea:2,region=sea";
/// let instance: Instance = result.parse()?;
///
/// assert_eq!(
///   instance,
///   Instance::new(
///     "flytrap",
///     "148e21dad76789",
///     Region::Seattle,
///     "fdaa:2:224b:a7b:2dbb:3e15:aaea:2".parse()?,
///   )
/// );
/// # Ok(())
/// # }
/// ```
#[derive(PartialOrd, Ord, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Instance {
    pub app: String,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub peer: Peer,
}

impl Instance {
    pub fn new(
        app: impl Into<String>,
        id: impl Into<String>,
        region: impl Into<Location>,
        private_ip: Ipv6Addr,
    ) -> Self {
        Self {
            app: app.into(),
            peer: Peer::new(region, id, private_ip),
        }
    }
}

impl Deref for Instance {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

impl FromStr for Instance {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut app: Option<&str> = None;
        let mut id: Option<&str> = None;
        let mut ip: Option<Ipv6Addr> = None;
        let mut location: Option<Location> = None;

        for field in s.split(',') {
            match field.split_once('=') {
                Some(("instance", value)) => id = Some(value),
                Some(("app", value)) => app = Some(value),
                Some(("ip", value)) => match value.parse::<Ipv6Addr>() {
                    Ok(value) => ip = Some(value),
                    Err(_) => return Err(Error::Parse),
                },
                Some(("region", value)) => match value.parse::<Location>() {
                    Ok(value) => location = Some(value),
                    Err(_) => return Err(Error::Parse),
                },
                _ => continue,
            }
        }

        if let (Some(app), Some(id), Some(ip), Some(location)) = (app, id, ip, location) {
            Ok(Self {
                app: app.to_owned(),
                peer: Peer {
                    node: Node {
                        location,
                        id: id.to_owned(),
                    },
                    private_ip: ip,
                },
            })
        } else {
            Err(Error::Parse)
        }
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Instance {}

impl std::hash::Hash for Instance {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Return the Fly.io DNS server address which serves a given `local` Fly.io
/// Wireguard address.
///
/// Fly.io uses a standard address within its datacenters (`fdaa::3`) but an
/// organization-specific address when connected to the WireGuard VPN.
///
/// # Panics
///
/// If `local` does not appear to be a Fly.io private networking address (i.e.,
/// its first 16 bits are not `fdaa`).
///
/// ```
/// # use std::net::Ipv6Addr;
/// # fn main() -> Result<(), std::net::AddrParseError> {
/// let address: Ipv6Addr = "fdaa:0:18:a7b:d6b:0:a:2".parse()?;
/// let dns = flytrap::dns_server_address(address, false);
/// assert_eq!("fdaa:0:18::3", dns.to_string());
/// # Ok(())
/// # }
/// ```
pub fn dns_server_address(local: impl Into<Ipv6Addr>, hosted: bool) -> Ipv6Addr {
    let [a, b, c, _, _, _, _, _] = local.into().segments();
    assert_eq!(a, 0xfdaa);

    let (b, c) = if hosted { (0, 0) } else { (b, c) };

    Ipv6Addr::new(a, b, c, 0, 0, 0, 0, 3)
}

#[cfg(test)]
mod test {
    use std::net::Ipv6Addr;

    use super::{dns_server_address, Node};

    #[test]
    fn test_dns_server_address() {
        let address = "fdaa:0:18:a7b:d6b:0:a:2".parse::<Ipv6Addr>().unwrap();
        let external = "fdaa:0:18::3".parse::<Ipv6Addr>().unwrap();
        let hosted = "fdaa::3".parse::<Ipv6Addr>().unwrap();

        assert_eq!(external, dns_server_address(address, false));
        assert_eq!(hosted, dns_server_address(address, true));
    }

    #[test]
    #[cfg(feature = "regions")]
    fn test_parse_node() {
        use crate::Region;

        let result = "148e21dad76789 sea,4d89699c030518 ams,6e82de14c35038 sin";
        let nodes: Vec<Node> = result.split(',').filter_map(|n| n.parse().ok()).collect();
        assert_eq!(
            &nodes,
            &[
                Node::new(Region::Seattle, "148e21dad76789"),
                Node::new(Region::Amsterdam, "4d89699c030518"),
                Node::new(Region::Singapore, "6e82de14c35038")
            ]
        );
    }
}

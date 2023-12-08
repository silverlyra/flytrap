use std::net::Ipv6Addr;

use futures::future::join_all;
use hickory_resolver::{
    error::ResolveError, proto::rr::rdata::AAAA, IntoName, Name, TokioAsyncResolver,
};
#[cfg(feature = "tracing")]
use tracing::{instrument, Level};

use crate::{
    error::Error,
    resolver::{lookup_aaaa, lookup_txt, Resolver},
    Node, Peer, Region,
};

/// Query the Fly.io [internal DNS][] records for a particular app.
///
/// [internal DNS]: https://fly.io/docs/reference/private-networking/#fly-internal-addresses
pub struct AppResolver<'r> {
    domain: Name,
    resolver: &'r TokioAsyncResolver,
}

impl<'r> AppResolver<'r> {
    pub(crate) fn new(app: impl Into<String>, resolver: &'r Resolver) -> Self {
        let app: String = app.into();
        let name = Name::from_ascii(app).expect("invalid app name");
        let domain = name.append_label("internal").unwrap();

        Self {
            domain,
            resolver: &resolver.0,
        }
    }

    /// Find the Fly.io regions where this app is deployed.
    #[cfg(feature = "regions")]
    #[cfg_attr(docsrs, doc(cfg(feature = "regions")))]
    #[cfg_attr(feature = "tracing", instrument(level = Level::TRACE, err(level = Level::WARN), skip_all, fields(domain = %self.domain)))]
    pub async fn regions(&self) -> Result<Vec<Region>, Error> {
        let value = self.txt("regions").await?;

        Ok(value
            .split(',')
            .filter_map(|code| code.parse::<Region>().ok())
            .collect())
    }

    /// Find all running instances of this Fly.io app.
    #[cfg_attr(feature = "tracing", instrument(level = Level::TRACE, err(level = Level::WARN), skip_all, fields(domain = %self.domain)))]
    pub async fn nodes(&self) -> Result<Vec<Node>, Error> {
        let value = self.txt("vms").await?;

        value.split(',').map(|peer| peer.parse::<Node>()).collect()
    }

    /// Find all running [instances][AppResolver::nodes] of this Fly.io app, and
    /// resolve all their instance ID’s to private IP addresses.
    #[cfg_attr(feature = "tracing", instrument(level = Level::TRACE, err(level = Level::WARN), skip_all, fields(domain = %self.domain)))]
    pub async fn peers(&self) -> Result<Vec<Peer>, Error> {
        let nodes = self.nodes().await?;

        let addrs = join_all(nodes.iter().map(|node| {
            let name = Name::from_ascii(&node.id)
                .expect("invalid node ID")
                .append_label("vm")
                .unwrap()
                .append_domain(&self.domain)
                .expect("invalid query");

            self.resolver.ipv6_lookup(name)
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, ResolveError>>()
        .map_err(Error::from)?;

        Ok(nodes
            .into_iter()
            .zip(addrs.into_iter())
            .filter_map(|(node, addrs)| {
                if let Some(AAAA(addr)) = addrs.into_iter().next() {
                    Some(node.into_peer(addr))
                } else {
                    None
                }
            })
            .collect())
    }

    /// Find the geographically-nearest _n_ instances of this Fly.io app.
    #[cfg_attr(feature = "tracing", instrument(level = Level::TRACE, err(level = Level::WARN), skip_all, fields(count = n, domain = %self.domain)))]
    pub async fn nearest_peer_addresses(&self, n: usize) -> Result<Vec<Ipv6Addr>, Error> {
        let top = Name::from_ascii(format!("top{n}"))
            .expect("invalid top n")
            .append_label("nearest")
            .unwrap()
            .append_label("of")
            .unwrap()
            .append_domain(&self.domain)
            .expect("invalid query");

        let results = self.resolver.ipv6_lookup(top).await.map_err(Error::from)?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    /// Perform an arbitrary `AAAA` record query on the `<app>.internal` domain.
    pub async fn ip(&self, name: impl IntoName) -> Result<Vec<Ipv6Addr>, Error> {
        let query = name
            .into_name()
            .expect("invalid name")
            .append_domain(&self.domain)
            .expect("invalid app domain");

        lookup_aaaa(self.resolver, query).await
    }

    /// Perform an arbitrary `TXT` record query on the `<app>.internal` domain.
    pub async fn txt(&self, name: impl IntoName) -> Result<String, Error> {
        let query = name
            .into_name()
            .expect("invalid name")
            .append_domain(&self.domain)
            .expect("invalid app domain");

        lookup_txt(self.resolver, query).await
    }
}

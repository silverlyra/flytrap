#![cfg_attr(docsrs, feature(doc_cfg), deny(rustdoc::broken_intra_doc_links))]
#![cfg_attr(not(feature = "regions"), allow(unused_imports))]

//! Flytrap is a crate for reading the [Fly.io][] runtime [environment][].
//!
//! [Fly.io]: https://fly.io/
//! [environment]: https://fly.io/docs/reference/runtime-environment/
//!
//! - Read Fly.io [environment variables][env-vars] like `$FLY_PUBLIC_IP` into a `struct`
//! - Query Fly.io [internal DNS][dns] addresses like `top3.nearest.of.<app>.internal`
//! - Parse Fly.io [request headers][] like `Fly-Client-IP` (into an [`IpAddr`][std::net::IpAddr])
//! - Turn Fly.io [region][regions] codes like `ord` into names like “Chicago” and lat/long coordinates
//!
//! [env-vars]: https://fly.io/docs/reference/runtime-environment/#environment-variables
//! [dns]: https://fly.io/docs/reference/private-networking/#fly-internal-addresses
//! [request headers]: https://fly.io/docs/reference/runtime-environment/#request-headers
//! [regions]: https://fly.io/docs/reference/regions/
//!
//! A [demo app][] is available at [**flytrap.fly.dev**](https://flytrap.fly.dev) which shows this crate’s capabilities.
//!
//! [demo app]: https://github.com/silverlyra/flytrap/blob/main/examples/server.rs
//!
//! ## Usage
//!
//! ### Placement
//!
//! The [`Placement`] type gives access to Fly.io runtime [environment
//! variables][env-vars] like `$FLY_PUBLIC_IP` and `$FLY_REGION`.
//!
//! ```no_run
//! use flytrap::{Placement, Machine};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = Placement::current()?;
//!
//!     println!("Fly.io app: {}", runtime.app);
//!     println!("    region: {}", runtime.location);
//!
//!     if let Some(Machine{ id, memory: Some(memory), image: Some(image), .. }) = runtime.machine {
//!         println!("   machine: {id} ({memory} MB) running {image}");
//!     }
//!
//!     if let Some(public_ip) = runtime.public_ip {
//!         println!(" public IP: {}", public_ip);
//!     }
//!     println!("private IP: {}", runtime.private_ip);
//!
//!     Ok(())
//! }
//! ```
//!
//! #### Regions
//!
//! Flytrap models Fly.io [regions][] as an `enum`:
//!
//! ```no_run
//! use flytrap::{City, Placement, Region};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = Placement::current()?;
//!     let region = runtime.region().unwrap_or(Region::Guadalajara);
//!
//!     show(region);
//!     Ok(())
//! }
//!
//! fn show(region: Region) {
//!     let City { name, country, geo } = region.city;
//!     println!("Running in {name} ({country}) @ {}, {}", geo.x(), geo.y());
//! }
//! ```
//!
//! Regions implement [`Ord`], and sort geographically:
//!
//! ```rust
//! # fn main() {
//! use flytrap::Region::*;
//!
//! let mut regions = [Bucharest, Chicago, HongKong, Johannesburg,
//!                    LosAngeles, Madrid, Santiago, Tokyo];
//! regions.sort();
//!
//! assert_eq!(regions, [LosAngeles, Chicago, Santiago, Madrid,
//!                      Bucharest, Johannesburg, HongKong, Tokyo]);
//! # }
//! ```
//!
//! ### DNS queries
//!
//! Create a [`Resolver`] in order to query the Fly.io [`.internal` DNS zone][dns].
//!
//! ```no_run
//! use flytrap::Resolver;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let resolver = Resolver::new()?;
//!
//!     // Discover all instances of the currently-running app
//!     let peers = resolver.current()?.peers().await?;
//!
//!     for peer in peers {
//!         println!("peer {} in {} @ {}", peer.id, peer.location, peer.private_ip);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! Flytrap’s compilation can be controlled through a number of [Cargo features][].
//!
//! [Cargo features]: https://doc.rust-lang.org/cargo/reference/features.html
//!
//! - **`dns`**: Enable [`Resolver`] for querying Fly.io [internal DNS][dns], via [`hickory-dns`][hickory] ⭐
//! - **`detect`**: Enable automatic [`Resolver`] setup for Wireguard VPN clients, via [`if-addrs`][if-addrs] ⭐️
//! - **`environment`**: Enable code which reads Fly.io environment variables like `$FLY_PUBLIC_IP` ⭐️
//! - **`http`**: Enable types for HTTP [`headers`][headers] like [`Fly-Client-IP`][http::FlyClientIp] ⭐️
//! - **`nightly`**: Enable code which is only accepted by nightly Rust toolchains
//! - **`regions`**: Enable the [`Region`] type and [`RegionDetails`] structures ⭐️
//! - **`serde`**: Enable [Serde][serde] `#[derive(Deserialize, Serialize)]` on this crate’s types
//! - **`system-resolver`**: Enable the [`Resolver::system()`][Resolver::system] constructor, which reads `/etc/resolv.conf`
//!
//! _(Features marked with a ⭐️ are enabled by default.)_
//!
//! [headers]: https://docs.rs/headers/latest/headers/trait.Header.html
//! [hickory]: https://lib.rs/crates/hickory-resolver
//! [if-addrs]: https://lib.rs/crates/if-addrs
//! [serde]: https://serde.rs/

#[cfg(feature = "dns")]
mod app;
mod error;
#[cfg(feature = "http")]
#[cfg_attr(docsrs, doc(cfg(feature = "http")))]
pub mod http;
mod placement;
#[cfg(feature = "regions")]
mod region;
#[cfg(feature = "dns")]
mod resolver;

#[cfg(feature = "dns")]
#[cfg_attr(docsrs, doc(cfg(feature = "dns")))]
pub use app::AppResolver;
pub use error::Error;
pub use placement::{hosted, private_address, Machine, Placement};
#[cfg(feature = "regions")]
#[cfg_attr(docsrs, doc(cfg(feature = "regions")))]
pub use region::{City, Location, Region, RegionCode, RegionDetails, RegionError};
#[cfg(feature = "dns")]
#[cfg_attr(docsrs, doc(cfg(feature = "dns")))]
pub use resolver::{dns_server_address, Instance, Node, Peer, Resolver};

#[cfg(not(feature = "regions"))]
pub type Location = String;
#[cfg(not(feature = "regions"))]
pub type Region = String;

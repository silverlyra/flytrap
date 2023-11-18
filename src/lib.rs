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
pub use region::{Location, Region, RegionCode, RegionDetails, RegionError};
#[cfg(feature = "dns")]
#[cfg_attr(docsrs, doc(cfg(feature = "dns")))]
pub use resolver::{dns_server_address, Instance, Node, Peer, Resolver};

#[cfg(not(feature = "regions"))]
pub type Location = String;
#[cfg(not(feature = "regions"))]
pub type Region = String;

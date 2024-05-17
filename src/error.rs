#[cfg(feature = "dns")]
use hickory_resolver::error::ResolveError;

/// An error in a [`Resolver`][crate::Resolver] operation.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "api")]
    #[error(transparent)]
    Api(#[from] reqwest::Error),
    #[cfg(feature = "dns")]
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error("no Fly.io private networking detected")]
    Unavailable,
    #[error("failed to parse Fly.io TXT record")]
    Parse,
}

#[cfg(feature = "regions")]
impl From<crate::region::RegionError> for Error {
    fn from(_value: crate::region::RegionError) -> Self {
        Self::Parse
    }
}

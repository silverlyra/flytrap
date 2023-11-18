use hickory_resolver::error::ResolveError;

/// An error in a [`Resolver`] operation.
#[derive(thiserror::Error, Clone, Debug)]
#[non_exhaustive]
pub enum Error {
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

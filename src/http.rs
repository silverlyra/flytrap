use std::{fmt, net::IpAddr};

use headers::{Header, HeaderName, HeaderValue};

use crate::{Location, Region};

/// The [`Fly-Client-IP`][def] header: the IP address that Fly.io accepted the
/// incoming connection from.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-client-ip
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[doc(alias = "Fly-Client-IP")]
pub struct FlyClientIp(pub IpAddr);

impl FlyClientIp {
    pub const fn new(address: IpAddr) -> Self {
        Self(address)
    }

    pub const fn into_inner(self) -> IpAddr {
        self.0
    }
}

impl Header for FlyClientIp {
    fn name() -> &'static HeaderName {
        &FLY_CLIENT_IP
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        let value = value.to_str().map_err(|_| headers::Error::invalid())?;

        let ip: IpAddr = value.parse().map_err(|_| headers::Error::invalid())?;

        Ok(Self(ip))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(HeaderValue::from(*self)))
    }
}

impl fmt::Display for FlyClientIp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<FlyClientIp> for HeaderValue {
    fn from(value: FlyClientIp) -> Self {
        HeaderValue::from_str(&value.0.to_string())
            .expect("IP address not serializable as HeaderValue")
    }
}

/// The [`Fly-Forwarded-Port`][def] header: the port that the client connected
/// to the Fly.io edge.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-forwarded-port
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[doc(alias = "Fly-Forwarded-Port")]
pub struct FlyForwardedPort(pub u16);

impl FlyForwardedPort {
    pub const fn new(port: u16) -> Self {
        Self(port)
    }

    pub const fn into_inner(self) -> u16 {
        self.0
    }
}

impl Header for FlyForwardedPort {
    fn name() -> &'static HeaderName {
        &FLY_FORWARDED_PORT
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        let value = value.to_str().map_err(|_| headers::Error::invalid())?;

        let port: u16 = value.parse().map_err(|_| headers::Error::invalid())?;

        Ok(Self(port))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(HeaderValue::from(*self)))
    }
}

impl fmt::Display for FlyForwardedPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<FlyForwardedPort> for HeaderValue {
    fn from(value: FlyForwardedPort) -> Self {
        HeaderValue::from_str(&value.0.to_string())
            .expect("port number not serializable as HeaderValue")
    }
}

/// The [`Fly-Region`][def] header: the Fly.io edge [region][Location] where the
/// client's connection was accepted from the Internet.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-region
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "regions", derive(Copy))]
#[doc(alias = "Fly-Region")]
pub struct FlyRegion(pub Location);

impl FlyRegion {
    pub const fn new(region: Location) -> Self {
        Self(region)
    }

    pub const fn region(&self) -> Option<Region> {
        match self.0 {
            Location::Region(region) => Some(region),
            Location::Unknown(_) => None,
        }
    }

    pub const fn into_inner(self) -> Location {
        self.0
    }
}

impl Header for FlyRegion {
    fn name() -> &'static HeaderName {
        &FLY_REGION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(headers::Error::invalid)?;
        let value = value.to_str().map_err(|_| headers::Error::invalid())?;

        #[cfg(feature = "regions")]
        let region: Location = value.parse().map_err(|_| headers::Error::invalid())?;

        #[cfg(not(feature = "regions"))]
        let region = value.to_owned();

        Ok(Self(region))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(HeaderValue::from(self)))
    }
}

impl fmt::Display for FlyRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "regions")]
impl From<FlyRegion> for HeaderValue {
    fn from(region: FlyRegion) -> Self {
        match region.0 {
            Location::Region(region) => HeaderValue::from_static(region.code),
            Location::Unknown(ref code) => {
                HeaderValue::from_bytes(code.as_ref()).expect("invalid Fly.io region code")
            }
        }
    }
}

impl From<&FlyRegion> for HeaderValue {
    fn from(region: &FlyRegion) -> Self {
        #[cfg(feature = "regions")]
        let value = match region.0 {
            Location::Region(region) => HeaderValue::from_static(region.code),
            Location::Unknown(ref code) => {
                HeaderValue::from_bytes(code.as_ref()).expect("invalid Fly.io region code")
            }
        };

        #[cfg(not(feature = "regions"))]
        let value =
            HeaderValue::from_str(&value.0).expect("Fly-Region not serializable as HeaderValue");

        value
    }
}

/// The [name][HeaderName] for the [`Fly-Client-IP`][def] HTTP header.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-client-ip
pub static FLY_CLIENT_IP: HeaderName = HeaderName::from_static("fly-client-ip");

/// The [name][HeaderName] for the [`Fly-Forwarded-Port`][def] HTTP header.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-forwarded-port
pub static FLY_FORWARDED_PORT: HeaderName = HeaderName::from_static("fly-forwarded-port");

/// The [name][HeaderName] for the [`Fly-Region`][def] HTTP header.
///
/// [def]: https://fly.io/docs/reference/runtime-environment/#fly-region
pub static FLY_REGION: HeaderName = HeaderName::from_static("fly-region");

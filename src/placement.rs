use std::net::{IpAddr, Ipv6Addr};

use crate::{error::Error, Location, Region};

/// Details how the current process is running in the Fly.io [runtime environment][].
///
/// [runtime environment]: https://fly.io/docs/reference/runtime-environment/
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Placement {
    /// The Fly.io application name ([`$FLY_APP_NAME`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_app_name
    #[doc(alias = "FLY_APP_NAME")]
    pub app: String,

    /// The name of the [process group][] associated with this Fly.io machine
    /// ([`$FLY_PROCESS_GROUP`][def]).
    ///
    /// [process group]: https://fly.io/docs/apps/processes/#run-multiple-processes
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_process_group
    #[doc(alias = "FLY_PROCESS_GROUP")]
    pub process_group: Option<String>,

    /// The [public][] IPv6 address for this Fly.io machine ([`$FLY_PUBLIC_IP`][def]).
    ///
    /// [public]: https://fly.io/docs/reference/services/#ipv6
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_public_ip
    #[doc(alias = "FLY_PUBLIC_IP")]
    pub public_ip: Option<Ipv6Addr>,

    /// The [private][] IPv6 address for this Fly.io machine ([`$FLY_PRIVATE_IP`][def]).
    ///
    /// [private]: https://fly.io/docs/reference/private-networking/
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_private_ip
    #[doc(alias = "FLY_PRIVATE_IP")]
    pub private_ip: Ipv6Addr,

    /// The [machine][Machine] ID for Fly.io “Apps v2”, or the Nomad allocation
    /// ID for legacy apps ([`$FLY_ALLOC_ID`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_alloc_id
    #[doc(alias = "FLY_ALLOC_ID")]
    pub allocation: String,

    /// Details of the Fly.io [machine][] running this process.
    ///
    /// [machine]: https://fly.io/docs/machines/
    pub machine: Option<Machine>,

    /// The Fly.io [region][Location] where the process is being run
    /// ([`$FLY_REGION`][def]).
    ///
    /// See the [`region()`][Placement::region] method for more convenient access.
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_region
    #[doc(alias = "FLY_REGION")]
    pub location: Location,
}

impl Placement {
    /// Get the current process's [`Placement`], based on `$FLY_` environment variables.
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn current() -> Result<Self, Error> {
        let app = var("FLY_APP_NAME")?;
        let process_group = std::env::var("FLY_PROCESS_GROUP").ok();
        let public_ip = public_address();
        let private_ip = environment_address().ok_or(Error::Unavailable)?;
        let allocation = var("FLY_ALLOC_ID")?;
        let machine = Machine::current().ok();
        let region_code = var("FLY_REGION")?;

        #[cfg(feature = "regions")]
        let location: Location = region_code.parse().expect("invalid $FLY_REGION");
        #[cfg(not(feature = "regions"))]
        let location = region_code;

        Ok(Self {
            app,
            process_group,
            public_ip,
            private_ip,
            allocation,
            machine,
            location,
        })
    }

    /// The Fly.io runtime [region][], as a [`Region`][] enum.
    ///
    /// If `$FLY_REGION` could not be parsed as a `Region`, returns `None`; use
    /// the `.location` field to access the bare [`RegionCode`].
    ///
    /// [region]: https://fly.io/docs/reference/regions/
    #[cfg(feature = "regions")]
    pub fn region(&self) -> Option<Region> {
        match self.location {
            Location::Region(region) => Some(region),
            _ => None,
        }
    }
}

/// Information about the [Fly.io Machine][machine] on which the current process
/// is running.
///
/// [machine]: https://fly.io/docs/machines/
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Machine {
    /// The unique [ID][] of this Fly.io Machine.
    ///
    /// Fly.io exposes this value to VM’s as `$FLY_MACHINE_ID`.
    ///
    /// [ID]: https://fly.io/docs/reference/runtime-environment/#fly_machine_id
    #[doc(alias = "FLY_MACHINE_ID")]
    pub id: String,

    /// The name of the Docker [image][] running this container.
    ///
    /// Fly.io exposes this value to VM’s as `$FLY_IMAGE_REF`.
    ///
    /// [image]: https://fly.io/docs/reference/runtime-environment/#fly_image_ref
    #[doc(alias = "FLY_IMAGE_REF")]
    pub image: Option<String>,

    /// The [version][] assigned to a specific Fly.io Machine configuration.
    ///
    /// Fly.io exposes this value to VM’s as `$FLY_MACHINE_VERSION`.
    ///
    /// [version]: https://fly.io/docs/reference/runtime-environment/#fly_machine_version
    #[doc(alias = "FLY_MACHINE_VERSION")]
    pub version: String,

    /// The [memory][] allocated to the Fly.io Machine, in MB.
    ///
    /// Fly.io exposes this value to VM’s as `$FLY_VM_MEMORY_MB`.
    ///
    /// [memory]: https://fly.io/docs/reference/runtime-environment/#fly_vm_memory_mb
    #[doc(alias = "FLY_VM_MEMORY_MB")]
    pub memory: Option<usize>,
}

impl Machine {
    /// Populates a [`Machine`] based on `$FLY_` environment variables.
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn current() -> Result<Self, Error> {
        let id = var("FLY_MACHINE_ID")?;
        let image = std::env::var("FLY_IMAGE_REF").ok();
        let version = var("FLY_MACHINE_VERSION")?;
        let memory = std::env::var("FLY_VM_MEMORY_MB")
            .ok()
            .and_then(|value| value.parse::<usize>().ok());

        Ok(Self {
            id,
            image,
            version,
            memory,
        })
    }
}

/// Checks to see if the current process appears to be running in the Fly.io
/// [runtime environment][], based on the presence of certain `$FLY_`
/// environment variables.
///
/// [runtime environment]: https://fly.io/docs/reference/runtime-environment/
#[cfg(feature = "environment")]
#[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
pub fn hosted() -> bool {
    use std::env::var;
    matches!((var("FLY_APP_NAME"), var("FLY_PRIVATE_IP")), (Ok(_), Ok(_)))
}

/// Read the [`$FLY_PRIVATE_IP`][private-ip] [environment variable][std::env::var],
/// if set to a valid IPv6 address.
///
/// If the `detect` crate feature is enabled (default) and `$FLY_PRIVATE_IP` is
/// unset, looks for a local network interface with a Fly.io private IPv6
/// address (starting with `fdaa:`), and returns that address.
///
/// Returns `None` if `$FLY_PRIVATE_IP` is unset, empty, or [`Ipv6Addr`] cannot
/// [parse][std::str::FromStr] it.
///
/// [private-ip]: https://fly.io/docs/reference/runtime-environment/#fly_private_ip
#[cfg(any(feature = "detect", feature = "environment"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "detect", feature = "environment"))))]
pub fn private_address() -> Option<Ipv6Addr> {
    #[cfg(feature = "environment")]
    let ip = environment_address();
    #[cfg(not(feature = "environment"))]
    let ip: Option<Ipv6Addr> = None;

    #[cfg(feature = "detect")]
    let ip = ip.or_else(detect_address);

    ip
}

/// Read the [`$FLY_PUBLIC_IP`][ip] [environment variable][std::env::var], if
/// set to a valid IPv6 address.
///
/// [ip]: https://fly.io/docs/reference/services/#ipv6
#[cfg(feature = "environment")]
#[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
pub fn public_address() -> Option<Ipv6Addr> {
    std::env::var("FLY_PUBLIC_IP")
        .ok()
        .and_then(|value| value.parse::<Ipv6Addr>().ok())
}

/// Read the `$FLY_PRIVATE_IP` environment variable, if set to a valid
/// IPv6 address.
#[cfg(feature = "environment")]
fn environment_address() -> Option<Ipv6Addr> {
    let ip = std::env::var("FLY_PRIVATE_IP").ok();

    match ip {
        Some(ip) if !ip.is_empty() => ip.parse::<Ipv6Addr>().ok(),
        _ => None,
    }
}

/// Find the first host IPv6 address starting with `fdaa:`.
#[cfg(feature = "detect")]
fn detect_address() -> Option<Ipv6Addr> {
    let interfaces = if_addrs::get_if_addrs().ok()?;

    interfaces
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            IpAddr::V6(ip) if ip.segments()[0] == 0xfdaa => Some(ip),
            _ => None,
        })
        .next()
}

#[cfg(feature = "environment")]
fn var(name: &'static str) -> Result<String, Error> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        _ => Err(Error::Unavailable),
    }
}

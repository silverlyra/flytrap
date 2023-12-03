//! [Node][NodeId] and [client][ClientId] ID's, as used in the server-server and
//! server-client wire formats.

use std::{
    env, fmt,
    num::{NonZeroU64, ParseIntError},
    str::FromStr,
};

use crate::error::Error;

/// Information about the [Fly.io Machine][machine] on which the current process
/// is running.
///
/// [machine]: https://fly.io/docs/machines/
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Machine {
    /// The unique [ID][MachineId] of this Fly.io Machine ([`$FLY_MACHINE_ID`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_machine_id
    #[doc(alias = "FLY_MACHINE_ID")]
    pub id: MachineId,

    /// The name of the Docker image running this container on `registry.fly.io`
    /// ([`$FLY_IMAGE_REF`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_image_ref
    #[doc(alias = "FLY_IMAGE_REF")]
    pub image: Option<String>,

    /// The version assigned to a specific Fly.io Machine configuration
    /// ([`$FLY_MACHINE_VERSION`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_machine_version
    #[doc(alias = "FLY_MACHINE_VERSION")]
    pub version: String,

    /// The memory allocated to the Fly.io Machine, in MB
    /// ([`$FLY_VM_MEMORY_MB`][def]).
    ///
    /// [def]: https://fly.io/docs/reference/runtime-environment/#fly_vm_memory_mb
    #[doc(alias = "FLY_VM_MEMORY_MB")]
    pub memory: Option<usize>,
}

impl Machine {
    /// Populates a [`Machine`] based on `$FLY_` environment variables.
    #[cfg(feature = "environment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "environment")))]
    pub fn current() -> Result<Self, Error> {
        let id = env::var("FLY_MACHINE_ID").map_err(Error::from)?;
        let image = env::var("FLY_IMAGE_REF").ok();
        let version = env::var("FLY_MACHINE_VERSION").map_err(Error::from)?;
        let memory = env::var("FLY_VM_MEMORY_MB")
            .ok()
            .and_then(|value| value.parse::<usize>().ok());

        Ok(Self {
            id: MachineId::new(id),
            image,
            version,
            memory,
        })
    }
}

/// A Fly.io [machine][] ID, encoded as a `u64`.
///
/// [machine]: https://fly.io/docs/machines/
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[cfg_attr(feature = "binrw", derive(binrw::BinRead, binrw::BinWrite))]
#[repr(transparent)]
pub struct MachineId(NonZeroU64);

impl MachineId {
    pub fn new(id: impl AsRef<str>) -> Self {
        id.as_ref().parse().expect("invalid machine ID")
    }
}

impl FromStr for MachineId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16).and_then(|id| match NonZeroU64::new(id) {
            Some(id) => Ok(Self(id)),
            None => Err("0".parse::<NonZeroU64>().unwrap_err()), // hacky :\
        })
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:014x}", self.0)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MachineId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MachineId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let string = String::deserialize(d)?;
        string.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU64;

    use super::MachineId;

    #[test]
    fn machine_id() {
        assert_eq!(
            MachineId::new("148eddd3b36068"),
            MachineId(NonZeroU64::new(0x148eddd3b36068).unwrap()),
        );

        assert_eq!(
            format!("{}", MachineId::new("148eddd3b36068")),
            "148eddd3b36068"
        );
    }
}

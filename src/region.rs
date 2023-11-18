use std::{fmt, ops::Deref, str::FromStr};

use enum_map::{enum_map, Enum, EnumMap};
use geo_types::Point;
use lazy_static::lazy_static;
use noisy_float::types::R32;
use tinyvec::ArrayVec;

/// A Fly.io point of presence.
///
/// For region codes recognized by this package (e.g., `ams`, `nrt`, `ord`), the
/// value will be a [`Region`] with known [details][RegionDetails]. For
/// unrecognized codes, the value will be a bare [`RegionCode`].
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Location {
    Region(Region),
    Unknown(RegionCode),
}

impl Location {
    #[inline]
    fn key(&self) -> RegionKey<'_> {
        match self {
            Location::Region(region) => region.key(),
            Location::Unknown(code) => code.key(),
        }
    }
}

impl FromStr for Location {
    type Err = RegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(region) = s.parse::<Region>() {
            Ok(Self::Region(region))
        } else if let Ok(code) = s.parse::<RegionCode>() {
            Ok(Self::Unknown(code))
        } else {
            Err(RegionError::Invalid)
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Region(region) => write!(f, "{region}"),
            Location::Unknown(code) => write!(f, "{code}"),
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Region> for Location {
    fn from(value: Region) -> Self {
        Self::Region(value)
    }
}

impl From<RegionCode> for Location {
    fn from(value: RegionCode) -> Self {
        Self::Unknown(value)
    }
}

/// A [Fly.io region][regions].
///
/// Information about the region is available through the associated
/// [`RegionDetails`], including the [`City`] where the region is located.
///
/// [regions]: https://fly.io/docs/reference/regions/
#[derive(Enum, PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[repr(u32)]
pub enum Region {
    /// The _Amsterdam, Netherlands_ Fly.io region (`ams`).
    #[cfg_attr(feature = "serde", serde(rename = "ams"))]
    Amsterdam = 0x616d7300,
    /// The _Ashburn, Virginia (US)_ Fly.io region (`iad`).
    #[cfg_attr(feature = "serde", serde(rename = "iad"))]
    Ashburn = 0x69616400,
    /// The _Atlanta, Georgia (US)_ Fly.io region (`atl`).
    #[cfg_attr(feature = "serde", serde(rename = "atl"))]
    Atlanta = 0x61746c00,
    /// The _Bogotá, Colombia_ Fly.io region (`bog`).
    #[cfg_attr(feature = "serde", serde(rename = "bog"))]
    Bogota = 0x626f6700,
    /// The _Boston, Massachusetts (US)_ Fly.io region (`bos`).
    #[cfg_attr(feature = "serde", serde(rename = "bos"))]
    Boston = 0x626f7300,
    /// The _Bucharest, Romania_ Fly.io region (`otp`).
    #[cfg_attr(feature = "serde", serde(rename = "otp"))]
    Bucharest = 0x6f747000,
    /// The _Chennai (Madras), India_ Fly.io region (`maa`).
    #[cfg_attr(feature = "serde", serde(rename = "maa"))]
    Chennai = 0x6d616100,
    /// The _Chicago, Illinois (US)_ Fly.io region (`ord`).
    #[cfg_attr(feature = "serde", serde(rename = "ord"))]
    Chicago = 0x6f726400,
    /// The _Dallas, Texas (US)_ Fly.io region (`dfw`).
    #[cfg_attr(feature = "serde", serde(rename = "dfw"))]
    Dallas = 0x64667700,
    /// The _Denver, Colorado (US)_ Fly.io region (`den`).
    #[cfg_attr(feature = "serde", serde(rename = "den"))]
    Denver = 0x64656e00,
    /// The _Ezeiza, Argentina_ Fly.io region (`eze`).
    #[cfg_attr(feature = "serde", serde(rename = "eze"))]
    Ezeiza = 0x657a6500,
    /// The _Frankfurt, Germany_ Fly.io region (`fra`).
    #[cfg_attr(feature = "serde", serde(rename = "fra"))]
    Frankfurt = 0x66726100,
    /// The _Guadalajara, Mexico_ Fly.io region (`gdl`).
    #[cfg_attr(feature = "serde", serde(rename = "gdl"))]
    Guadalajara = 0x67646c00,
    /// The _Hong Kong, Hong Kong_ Fly.io region (`hkg`).
    #[cfg_attr(feature = "serde", serde(rename = "hkg"))]
    HongKong = 0x686b6700,
    /// The _Johannesburg, South Africa_ Fly.io region (`jnb`).
    #[cfg_attr(feature = "serde", serde(rename = "jnb"))]
    Johannesburg = 0x6a6e6200,
    /// The _London, United Kingdom_ Fly.io region (`lhr`).
    #[cfg_attr(feature = "serde", serde(rename = "lhr"))]
    London = 0x6c687200,
    /// The _Los Angeles, California (US)_ Fly.io region (`lax`).
    #[cfg_attr(feature = "serde", serde(rename = "lax"))]
    LosAngeles = 0x6c617800,
    /// The _Madrid, Spain_ Fly.io region (`mad`).
    #[cfg_attr(feature = "serde", serde(rename = "mad"))]
    Madrid = 0x6d616400,
    /// The _Miami, Florida (US)_ Fly.io region (`mia`).
    #[cfg_attr(feature = "serde", serde(rename = "mia"))]
    Miami = 0x6d696100,
    /// The _Montreal, Canada_ Fly.io region (`yul`).
    #[cfg_attr(feature = "serde", serde(rename = "yul"))]
    Montreal = 0x79756c00,
    /// The _Mumbai, India_ Fly.io region (`bom`).
    #[cfg_attr(feature = "serde", serde(rename = "bom"))]
    Mumbai = 0x626f6d00,
    /// The _Paris, France_ Fly.io region (`cdg`).
    #[cfg_attr(feature = "serde", serde(rename = "cdg"))]
    Paris = 0x63646700,
    /// The _Phoenix, Arizona (US)_ Fly.io region (`phx`).
    #[cfg_attr(feature = "serde", serde(rename = "phx"))]
    Phoenix = 0x70687800,
    /// The _Querétaro, Mexico_ Fly.io region (`qro`).
    #[cfg_attr(feature = "serde", serde(rename = "qro"))]
    Queretaro = 0x71726f00,
    /// The _Rio de Janeiro, Brazil_ Fly.io region (`gig`).
    #[cfg_attr(feature = "serde", serde(rename = "gig"))]
    RioDeJaneiro = 0x67696700,
    /// The _San Jose, California (US)_ Fly.io region (`sjc`).
    #[cfg_attr(feature = "serde", serde(rename = "sjc"))]
    SanJose = 0x736a6300,
    /// The _Santiago, Chile_ Fly.io region (`scl`).
    #[cfg_attr(feature = "serde", serde(rename = "scl"))]
    Santiago = 0x73636c00,
    /// The _Sao Paulo, Brazil_ Fly.io region (`gru`).
    #[cfg_attr(feature = "serde", serde(rename = "gru"))]
    SaoPaulo = 0x67727500,
    /// The _Seattle, Washington (US)_ Fly.io region (`sea`).
    #[cfg_attr(feature = "serde", serde(rename = "sea"))]
    Seattle = 0x73656100,
    /// The _Secaucus, NJ (US)_ Fly.io region (`ewr`).
    #[cfg_attr(feature = "serde", serde(rename = "ewr"))]
    Secaucus = 0x65777200,
    /// The _Singapore, Singapore_ Fly.io region (`sin`).
    #[cfg_attr(feature = "serde", serde(rename = "sin"))]
    Singapore = 0x73696e00,
    /// The _Stockholm, Sweden_ Fly.io region (`arn`).
    #[cfg_attr(feature = "serde", serde(rename = "arn"))]
    Stockholm = 0x61726e00,
    /// The _Sydney, Australia_ Fly.io region (`syd`).
    #[cfg_attr(feature = "serde", serde(rename = "syd"))]
    Sydney = 0x73796400,
    /// The _Tokyo, Japan_ Fly.io region (`nrt`).
    #[cfg_attr(feature = "serde", serde(rename = "nrt"))]
    Tokyo = 0x6e727400,
    /// The _Toronto, Canada_ Fly.io region (`yyz`).
    #[cfg_attr(feature = "serde", serde(rename = "yyz"))]
    Toronto = 0x79797a00,
    /// The _Warsaw, Poland_ Fly.io region (`waw`).
    #[cfg_attr(feature = "serde", serde(rename = "waw"))]
    Warsaw = 0x77617700,
}

impl Region {
    /// The known [details][RegionDetails] of the region.
    pub fn details(&self) -> RegionDetails<'static> {
        DETAILS[*self]
    }

    fn key(&self) -> RegionKey<'_> {
        (self.city.geo.x(), self.city.geo.y(), &self.code)
    }
}

/// The [sort][Ord] comparison key for a [`Region`] or [`RegionCode`].
type RegionKey<'a> = (R32, R32, &'a str);

impl Ord for Region {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for Region {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Deref for Region {
    type Target = RegionDetails<'static>;

    fn deref(&self) -> &Self::Target {
        &DETAILS[*self]
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl FromStr for Region {
    type Err = RegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ams" => Ok(Self::Amsterdam),
            "arn" => Ok(Self::Stockholm),
            "atl" => Ok(Self::Atlanta),
            "bog" => Ok(Self::Bogota),
            "bom" => Ok(Self::Mumbai),
            "bos" => Ok(Self::Boston),
            "cdg" => Ok(Self::Paris),
            "den" => Ok(Self::Denver),
            "dfw" => Ok(Self::Dallas),
            "ewr" => Ok(Self::Secaucus),
            "eze" => Ok(Self::Ezeiza),
            "fra" => Ok(Self::Frankfurt),
            "gdl" => Ok(Self::Guadalajara),
            "gig" => Ok(Self::RioDeJaneiro),
            "gru" => Ok(Self::SaoPaulo),
            "hkg" => Ok(Self::HongKong),
            "iad" => Ok(Self::Ashburn),
            "jnb" => Ok(Self::Johannesburg),
            "lax" => Ok(Self::LosAngeles),
            "lhr" => Ok(Self::London),
            "maa" => Ok(Self::Chennai),
            "mad" => Ok(Self::Madrid),
            "mia" => Ok(Self::Miami),
            "nrt" => Ok(Self::Tokyo),
            "ord" => Ok(Self::Chicago),
            "otp" => Ok(Self::Bucharest),
            "phx" => Ok(Self::Phoenix),
            "qro" => Ok(Self::Queretaro),
            "scl" => Ok(Self::Santiago),
            "sea" => Ok(Self::Seattle),
            "sin" => Ok(Self::Singapore),
            "sjc" => Ok(Self::SanJose),
            "syd" => Ok(Self::Sydney),
            "waw" => Ok(Self::Warsaw),
            "yul" => Ok(Self::Montreal),
            "yyz" => Ok(Self::Toronto),
            _ => Err(RegionError::Unrecognized),
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RegionDetails<'l> {
    pub code: &'l str,
    pub name: &'l str,
    pub city: City<'l>,
}

impl RegionDetails<'static> {
    pub(crate) const fn new(
        code: &'static str,
        name: &'static str,
        city: &'static str,
        country: &'static str,
        geo: [f32; 2],
    ) -> Self {
        Self {
            code,
            name,
            city: City {
                name: city,
                country,
                geo: point(geo[0], geo[1]),
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct City<'l> {
    pub name: &'l str,
    pub country: &'l str,
    pub geo: Point<R32>,
}

lazy_static! {
    static ref DETAILS: EnumMap<Region, RegionDetails<'static>> = enum_map! {
        Region::Amsterdam => RegionDetails::new("ams", "Amsterdam, Netherlands", "Amsterdam", "NL", [52.374342, 4.895439]),
        Region::Stockholm => RegionDetails::new("arn", "Stockholm, Sweden", "Stockholm", "SE", [59.6512, 17.9178]),
        Region::Atlanta => RegionDetails::new("atl", "Atlanta, Georgia (US)", "Atlanta", "US", [33.6407, -84.4277]),
        Region::Bogota => RegionDetails::new("bog", "Bogotá, Colombia", "Bogotá", "CO", [4.70159, -74.1469]),
        Region::Mumbai => RegionDetails::new("bom", "Mumbai, India", "Mumbai", "IN", [19.097403, 72.874245]),
        Region::Boston => RegionDetails::new("bos", "Boston, Massachusetts (US)", "Boston", "US", [42.366978, -71.022_36]),
        Region::Paris => RegionDetails::new("cdg", "Paris, France", "Paris", "FR", [48.860875, 2.353477]),
        Region::Denver => RegionDetails::new("den", "Denver, Colorado (US)", "Denver", "US", [39.7392, -104.9847]),
        Region::Dallas => RegionDetails::new("dfw", "Dallas, Texas (US)", "Dallas", "US", [32.778287, -96.7984]),
        Region::Secaucus => RegionDetails::new("ewr", "Secaucus, NJ (US)", "Secaucus", "US", [40.789543, -74.056_53]),
        Region::Ezeiza => RegionDetails::new("eze", "Ezeiza, Argentina", "Ezeiza", "AR", [-34.8222, -58.5358]),
        Region::Frankfurt => RegionDetails::new("fra", "Frankfurt, Germany", "Frankfurt", "DE", [50.1167, 8.6833]),
        Region::Guadalajara => RegionDetails::new("gdl", "Guadalajara, Mexico", "Guadalajara", "MX", [20.5217, -103.3109]),
        Region::RioDeJaneiro => RegionDetails::new("gig", "Rio de Janeiro, Brazil", "Rio de Janeiro", "BR", [-22.8099, -43.2505]),
        Region::SaoPaulo => RegionDetails::new("gru", "Sao Paulo, Brazil", "Sao Paulo", "BR", [-23.549664, -46.654_35]),
        Region::HongKong => RegionDetails::new("hkg", "Hong Kong, Hong Kong", "Hong Kong", "HK", [22.250_97, 114.203224]),
        Region::Ashburn => RegionDetails::new("iad", "Ashburn, Virginia (US)", "Ashburn", "US", [39.02214, -77.462556]),
        Region::Johannesburg => RegionDetails::new("jnb", "Johannesburg, South Africa", "Johannesburg", "ZA", [-26.13629, 28.20298]),
        Region::LosAngeles => RegionDetails::new("lax", "Los Angeles, California (US)", "Los Angeles", "US", [33.9416, -118.4085]),
        Region::London => RegionDetails::new("lhr", "London, United Kingdom", "London", "GB", [51.516434, -0.125656]),
        Region::Chennai => RegionDetails::new("maa", "Chennai (Madras), India", "Chennai", "IN", [13.064429, 80.253_07]),
        Region::Madrid => RegionDetails::new("mad", "Madrid, Spain", "Madrid", "ES", [40.4381, -3.82]),
        Region::Miami => RegionDetails::new("mia", "Miami, Florida (US)", "Miami", "US", [25.7877, -80.2241]),
        Region::Tokyo => RegionDetails::new("nrt", "Tokyo, Japan", "Tokyo", "JP", [35.621_61, 139.741_85]),
        Region::Chicago => RegionDetails::new("ord", "Chicago, Illinois (US)", "Chicago", "US", [41.891544, -87.630_39]),
        Region::Bucharest => RegionDetails::new("otp", "Bucharest, Romania", "Bucharest", "RO", [44.4325, 26.1039]),
        Region::Phoenix => RegionDetails::new("phx", "Phoenix, Arizona (US)", "Phoenix", "US", [33.416084, -112.009_48]),
        Region::Queretaro => RegionDetails::new("qro", "Querétaro, Mexico", "Querétaro", "MX", [20.62, -100.1863]),
        Region::Santiago => RegionDetails::new("scl", "Santiago, Chile", "Santiago", "CL", [-33.36572, -70.64292]),
        Region::Seattle => RegionDetails::new("sea", "Seattle, Washington (US)", "Seattle", "US", [47.6097, -122.3331]),
        Region::Singapore => RegionDetails::new("sin", "Singapore, Singapore", "Singapore", "SG", [1.3, 103.8]),
        Region::SanJose => RegionDetails::new("sjc", "San Jose, California (US)", "San Jose", "US", [37.351_6, -121.896_74]),
        Region::Sydney => RegionDetails::new("syd", "Sydney, Australia", "Sydney", "AU", [-33.866_03, 151.20693]),
        Region::Warsaw => RegionDetails::new("waw", "Warsaw, Poland", "Warsaw", "PL", [52.1657, 20.9671]),
        Region::Montreal => RegionDetails::new("yul", "Montreal, Canada", "Montreal", "CA", [45.48647, -73.75549]),
        Region::Toronto => RegionDetails::new("yyz", "Toronto, Canada", "Toronto", "CA", [43.644_63, -79.384_23]),
    };
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RegionCode(ArrayVec<[u8; 4]>);

impl RegionCode {
    pub(crate) fn new(data: [u8; 4]) -> Self {
        let mut data = ArrayVec::from(data);
        data.set_len(3);

        Self(data)
    }

    /// Checks if the `input` passes for a Fly.io region code – `/^[a-z]{3}$/`.
    pub fn valid(input: &str) -> bool {
        input.len() == 3 && input.chars().all(|c| c.is_ascii_lowercase())
    }

    fn key(&self) -> RegionKey<'_> {
        let zero = R32::unchecked_new(0.0);
        (zero, zero, self.as_ref())
    }
}

impl AsRef<[u8]> for RegionCode {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<str> for RegionCode {
    fn as_ref(&self) -> &str {
        std::str::from_utf8(&self.0).expect("invalid region code")
    }
}

impl FromStr for RegionCode {
    type Err = RegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Self::valid(s) {
            let b = s.as_bytes();
            Ok(Self::new([b[0], b[1], b[2], 0]))
        } else {
            Err(RegionError::Invalid)
        }
    }
}

impl fmt::Display for RegionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match std::str::from_utf8(&self.0) {
            Ok(code) => write!(f, "{code}"),
            Err(_) => write!(f, "---"),
        }
    }
}

impl Ord for RegionCode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for RegionCode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RegionError {
    #[error("invalid Fly.io region code")]
    Invalid,
    #[error("unknown Fly.io region code")]
    Unrecognized,
}

#[inline(always)]
const fn point(lat: f32, lon: f32) -> Point<R32> {
    Point(geo_types::Coord {
        x: R32::unchecked_new(lon),
        y: R32::unchecked_new(lat),
    })
}

#[cfg(test)]
mod test {
    use super::{City, Region};

    #[test]
    fn region_details() {
        let ord = Region::Chicago;

        assert_eq!("Chicago, Illinois (US)", ord.name);
    }

    #[test]
    fn unpack() {
        let cdg = Region::Paris;
        let City { name, country, .. } = cdg.city;

        assert_eq!("Paris", name);
        assert_eq!("FR", country);
    }

    #[test]
    fn ordering() {
        use Region::*;

        let mut regions = [
            Bucharest,
            Chicago,
            HongKong,
            Johannesburg,
            LosAngeles,
            Madrid,
            Santiago,
            Tokyo,
        ];
        regions.sort();

        assert_eq!(
            [
                LosAngeles,
                Chicago,
                Santiago,
                Madrid,
                Bucharest,
                Johannesburg,
                HongKong,
                Tokyo,
            ],
            regions
        );
    }
}

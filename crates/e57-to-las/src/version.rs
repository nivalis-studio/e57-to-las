use std::fmt;
use std::str::FromStr;

/// Version of the LAS format to write.
///
/// The point record format is chosen from the version and from the attributes
/// being written. Colors need LAS 1.2 or newer; a WKT coordinate reference
/// system needs LAS 1.4.
///
/// ```
/// use e57_to_las::LasVersion;
///
/// let version: LasVersion = "1.2".parse()?;
/// assert_eq!(version, LasVersion::V1_2);
/// assert_eq!(version.to_string(), "1.2");
/// # Ok::<(), e57_to_las::ParseLasVersionError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum LasVersion {
    /// LAS 1.0: no colors, no GPS time types.
    #[cfg_attr(feature = "serde", serde(rename = "1.0"))]
    V1_0,
    /// LAS 1.1: no colors.
    #[cfg_attr(feature = "serde", serde(rename = "1.1"))]
    V1_1,
    /// LAS 1.2: the first version with colors.
    #[cfg_attr(feature = "serde", serde(rename = "1.2"))]
    V1_2,
    /// LAS 1.3.
    #[cfg_attr(feature = "serde", serde(rename = "1.3"))]
    V1_3,
    /// LAS 1.4, the default. The only version that stores a WKT coordinate system.
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "1.4"))]
    V1_4,
}

impl LasVersion {
    /// Every supported version, oldest first.
    pub const ALL: [LasVersion; 5] = [
        LasVersion::V1_0,
        LasVersion::V1_1,
        LasVersion::V1_2,
        LasVersion::V1_3,
        LasVersion::V1_4,
    ];

    /// The minor version number (`4` for LAS 1.4).
    pub const fn minor(self) -> u8 {
        match self {
            LasVersion::V1_0 => 0,
            LasVersion::V1_1 => 1,
            LasVersion::V1_2 => 2,
            LasVersion::V1_3 => 3,
            LasVersion::V1_4 => 4,
        }
    }

    /// Whether this version has a point format that stores colors.
    pub const fn supports_color(self) -> bool {
        self.minor() >= 2
    }

    /// Whether this version can store a WKT coordinate reference system.
    pub const fn supports_wkt(self) -> bool {
        self.minor() >= 4
    }

    /// Whether this version can flag GPS times as adjusted standard time.
    pub(crate) const fn supports_standard_gps_time(self) -> bool {
        self.minor() >= 2
    }

    pub(crate) fn to_las(self) -> las::Version {
        las::Version::new(1, self.minor())
    }
}

impl fmt::Display for LasVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "1.{}", self.minor())
    }
}

/// The text given to [`LasVersion::from_str`] is not a LAS version from 1.0 to 1.4.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid LAS version {input:?}: expected one of 1.0, 1.1, 1.2, 1.3, 1.4")]
pub struct ParseLasVersionError {
    input: String,
}

impl FromStr for LasVersion {
    type Err = ParseLasVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LasVersion::ALL
            .into_iter()
            .find(|version| version.to_string() == s.trim())
            .ok_or_else(|| ParseLasVersionError {
                input: s.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays_every_version() {
        for version in LasVersion::ALL {
            assert_eq!(version.to_string().parse::<LasVersion>(), Ok(version));
        }
    }

    #[test]
    fn rejects_unknown_versions() {
        for bad in ["1.5", "2.0", "1", "1.4.0", "", "b.4"] {
            assert!(bad.parse::<LasVersion>().is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn capabilities() {
        assert!(!LasVersion::V1_1.supports_color());
        assert!(LasVersion::V1_2.supports_color());
        assert!(!LasVersion::V1_3.supports_wkt());
        assert!(LasVersion::V1_4.supports_wkt());
        assert_eq!(LasVersion::default(), LasVersion::V1_4);
    }
}

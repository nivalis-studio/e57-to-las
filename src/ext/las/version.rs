use std::ops::Deref;

use crate::{Error, Result};

/// LAS file format version.
///
/// Represents a valid LAS version with major and minor version numbers.
/// Currently supports LAS 1.0 through 1.4 (major version must be 1,
/// minor version must be 0-4).
///
/// # Version Capabilities
///
/// Different LAS versions support different features:
/// - **1.0**: Basic point formats 0-1
/// - **1.1**: Adds point formats 2-3 with color
/// - **1.2**: Adds GPS time support, improved header
/// - **1.3**: Adds waveform data support
/// - **1.4**: Adds extended point formats 6-10 with 64-bit counters
///
/// # Examples
///
/// Creating valid versions:
/// ```
/// use e57_to_las::LasVersion;
///
/// let v12 = LasVersion::new(1, 2)?;
/// let v13 = LasVersion::new(1, 3)?;
/// ```
///
/// Invalid versions return an error:
/// ```should_panic
/// use e57_to_las::LasVersion;
///
/// let invalid = LasVersion::new(2, 0).unwrap(); // panics: major != 1
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LasVersion((u8, u8));

impl LasVersion {
    /// Create a new LAS version.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidLasVersion`](crate::Error::InvalidLasVersion) if:
    /// - Major version is not 1
    /// - Minor version is not in the range 0-4
    ///
    /// # Examples
    ///
    /// ```
    /// use e57_to_las::LasVersion;
    ///
    /// let version = LasVersion::new(1, 2)?;
    /// ```
    pub fn new(major: u8, minor: u8) -> Result<Self> {
        Self::try_from((major, minor))
    }
}

impl TryFrom<(u8, u8)> for LasVersion {
    type Error = Error;
    fn try_from(value: (u8, u8)) -> std::result::Result<Self, Self::Error> {
        let (major, minor) = value;

        if major != 1 {
            return Err(Error::InvalidLasVersion(
                "major should be equals to 1".into(),
            ));
        }

        match minor {
            0..4 => Ok(Self((major, minor))),
            _ => Err(Error::InvalidLasVersion(
                "minor should be between 0 and 4".into(),
            )),
        }
    }
}

impl Default for LasVersion {
    fn default() -> Self {
        Self((1, 2))
    }
}

impl From<LasVersion> for las::Version {
    fn from(value: LasVersion) -> Self {
        let (major, minor) = value.0;
        las::Version::new(major, minor)
    }
}

impl Deref for LasVersion {
    type Target = (u8, u8);

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use crate::{Error, Result};

const ALLOWED_VERSIONS: [(u8, u8); 5] = [(1, 0), (1, 1), (1, 2), (1, 3), (1, 4)];

/// An extension trait for `las::Version` providing additional constructors for creating and validating LAS versions.
pub trait LasVersionExt
where
    Self: Sized,
{
    /// Creates a new version if the provided major and minor numbers are valid according to the allowed LAS versions.
    ///
    /// This method takes `major` and `minor` version numbers and returns a valid instance of `las::Version` if
    /// the version is supported. If the version is invalid, it returns an error.
    ///
    /// # Returns
    /// - `Ok(Self)` with the valid version.
    /// - `Err` if the version is not allowed.
    ///
    /// # Errors
    /// Returns an error if the version is outside of the allowed LAS versions.
    ///
    /// # Example
    /// ```
    /// use e57_to_las::las::LasVersionExt;
    /// use las::Version;
    ///
    /// let version = Version::x_new_valid(1, 4);
    /// assert!(version.is_ok());
    ///
    /// let invalid_version = Version::x_new_valid(3, 0);
    /// assert!(invalid_version.is_err());
    /// ```
    fn x_new_valid(major: u8, minor: u8) -> Result<Self>;

    /// Attempts to create a new version from a string.
    ///
    /// This method takes a string in the format `major.minor` (e.g. "1.4") and attempts to parse it into a
    /// valid version. If the string is improperly formatted or the version is not allowed, an error is returned.
    ///
    /// # Returns
    /// - `Ok(Self)` if the string is successfully parsed and represents a valid version.
    /// - `Err` if the string is incorrectly formatted or the version is not allowed.
    ///
    /// # Errors
    /// Returns an error if the string is not in the correct format or if the version is invalid.
    ///
    /// # Example
    /// ```
    /// use e57_to_las::las::LasVersionExt;
    /// use las::Version;
    ///
    /// let version = Version::x_try_from_str("1.4");
    /// assert!(version.is_ok());
    ///
    /// let invalid_version = Version::x_try_from_str("3.0");
    /// assert!(invalid_version.is_err());
    ///
    /// let malformed_version = Version::x_try_from_str("1");
    /// assert!(malformed_version.is_err());
    /// ```
    fn x_try_from_str(value: &str) -> Result<Self>;
}

impl LasVersionExt for las::Version {
    /// Creates a new `las::Version` if the major and minor version numbers are valid.
    ///
    /// This implementation checks whether the provided version matches one of the allowed LAS versions
    /// before constructing the `las::Version`. Returns an error if the version is not supported.
    fn x_new_valid(major: u8, minor: u8) -> Result<Self> {
        if !ALLOWED_VERSIONS.contains(&(major, minor)) {
            return Err(Error::InvalidLasVersion(format!(
                "must be between {} and {}",
                format_version(ALLOWED_VERSIONS[0]),
                format_version(ALLOWED_VERSIONS[4]),
            )));
        }

        Ok(las::Version { major, minor })
    }

    /// Tries to create a `las::Version` from a string formatted as `major.minor`.
    ///
    /// This implementation attempts to parse a string to extract the major and minor version numbers,
    /// then checks if they form a valid LAS version.
    fn x_try_from_str(value: &str) -> Result<Self> {
        let parts: Vec<&str> = value.split('.').collect();

        if parts.len() != 2 {
            return Err(Error::InvalidLasVersion(
                "expected format `major.minor` (e.g. 1.4)".into(),
            ));
        }

        let major = parts[0]
            .parse::<u8>()
            .map_err(|_| Error::InvalidLasVersion("invalid major number".into()))?;
        let minor = parts[1]
            .parse::<u8>()
            .map_err(|_| Error::InvalidLasVersion("invalid minor number".into()))?;

        Self::x_new_valid(major, minor)
    }
}

fn format_version((major, minor): (u8, u8)) -> String {
    format!("{}.{}", major, minor)
}

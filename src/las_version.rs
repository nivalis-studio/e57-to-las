use crate::{Error, Result};

const ALLOWED_VERSIONS: [(u8, u8); 5] = [(1, 0), (1, 1), (1, 2), (1, 3), (1, 4)];

pub struct Version {
    major: u8,
    minor: u8,
}

impl Version {
    pub fn new(major: u8, minor: u8) -> Result<Self> {
        if !ALLOWED_VERSIONS.contains(&(major, minor)) {
            return Err(Error::InvalidLasVersion(format!(
                "must be between {} and {}",
                format_version(ALLOWED_VERSIONS[0]),
                format_version(ALLOWED_VERSIONS[4]),
            )));
        }

        Ok(Version { major, minor })
    }
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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

        Self::new(major, minor)
    }
}

impl From<&Version> for las::Version {
    fn from(value: &Version) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
        }
    }
}

fn format_version((major, minor): (u8, u8)) -> String {
    format!("{}.{}", major, minor)
}

#[cfg(test)]
mod test {
    use crate::las_version;

    #[test]
    fn test_unsupported_las_version() {
        let las_version = las_version::Version::new(2, 3);

        assert!(las_version.is_err())
    }

    #[test]
    fn test_invalid_las_version_major() {
        let las_version = las_version::Version::try_from("b.4");

        assert!(las_version.is_err())
    }

    #[test]
    fn test_invalid_las_version_minor() {
        let las_version = las_version::Version::try_from("2.c");

        assert!(las_version.is_err())
    }
}

use crate::{Error, Result};

const ALLOWED_VERSIONS: [(u8, u8); 5] = [(1, 0), (1, 1), (1, 2), (1, 3), (1, 4)];

pub trait LasVersionExt
where
    Self: Sized,
{
    fn x_new_valid(major: u8, minor: u8) -> Result<Self>;

    fn x_try_from_str(value: &str) -> Result<Self>;
}

impl LasVersionExt for las::Version {
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

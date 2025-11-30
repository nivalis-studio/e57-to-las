use std::ops::Deref;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct LasVersion((u8, u8));

impl LasVersion {
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

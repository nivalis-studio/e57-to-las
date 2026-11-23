//! LAS header customization: identifiers, coordinate reference system, VLRs.

use crate::{Error, Result};

/// The coordinate reference system written to the LAS header.
///
/// LAS stores it as a WKT string, which needs LAS 1.4.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Crs {
    /// Copy the E57 `coordinateMetadata` when it is WKT and the output is
    /// LAS 1.4. Otherwise nothing is written and the summary carries a
    /// [`Warning::CrsNotWritten`](crate::Warning::CrsNotWritten). The default.
    #[default]
    FromE57,
    /// Write this WKT. Fails with [`Error::CrsUnsupported`] below LAS 1.4.
    Wkt(String),
    /// Write no coordinate reference system.
    None,
}

/// Whether `text` looks like WKT (version 1 or 2) rather than an EPSG code or prose.
pub(crate) fn looks_like_wkt(text: &str) -> bool {
    const KEYWORDS: [&str; 14] = [
        "PROJCS[",
        "GEOGCS[",
        "GEOCCS[",
        "COMPD_CS[",
        "VERT_CS[",
        "LOCAL_CS[",
        "PROJCRS[",
        "GEOGCRS[",
        "GEODCRS[",
        "COMPOUNDCRS[",
        "VERTCRS[",
        "BOUNDCRS[",
        "ENGCRS[",
        "ENGINEERINGCRS[",
    ];
    let text = text.trim_start().to_ascii_uppercase();
    KEYWORDS.iter().any(|keyword| text.starts_with(keyword))
}

/// A LAS variable length record to add to the header.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vlr {
    /// Who defines the record, at most 16 ASCII bytes.
    pub user_id: String,
    /// Record type, defined by `user_id`.
    pub record_id: u16,
    /// At most 32 ASCII bytes.
    pub description: String,
    /// Payload. Above 65,535 bytes it is written as an extended VLR (LAS 1.4).
    pub data: Vec<u8>,
}

impl Vlr {
    /// A new VLR.
    pub fn new(
        user_id: impl Into<String>,
        record_id: u16,
        description: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        Vlr {
            user_id: user_id.into(),
            record_id,
            description: description.into(),
            data: data.into(),
        }
    }

    fn validate(&self) -> Result<()> {
        check_ascii("VLR user_id", &self.user_id, 16)?;
        check_ascii("VLR description", &self.description, 32)
    }

    pub(crate) fn to_las(&self) -> las::Vlr {
        las::Vlr {
            user_id: self.user_id.clone(),
            record_id: self.record_id,
            description: self.description.clone(),
            data: self.data.clone(),
        }
    }
}

/// Header fields callers may change for each output, through
/// [`Converter::header`](crate::Converter::header).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HeaderOptions {
    /// What produced the points, at most 32 ASCII bytes. Defaults to the
    /// scanner model when all point clouds of the output share one.
    pub system_identifier: String,
    /// Software that wrote the file, at most 32 ASCII bytes.
    pub generating_software: String,
    /// Extra VLRs.
    pub vlrs: Vec<Vlr>,
}

impl HeaderOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        check_ascii("system_identifier", &self.system_identifier, 32)?;
        check_ascii("generating_software", &self.generating_software, 32)?;
        self.vlrs.iter().try_for_each(Vlr::validate)
    }
}

/// The default `generating_software`.
pub(crate) fn generating_software() -> String {
    format!("e57-to-las {}", env!("CARGO_PKG_VERSION"))
        .chars()
        .take(32)
        .collect()
}

fn check_ascii(what: &str, value: &str, max_len: usize) -> Result<()> {
    if value.is_ascii() && value.len() <= max_len {
        Ok(())
    } else {
        Err(Error::InvalidOption(format!(
            "{what} must be ASCII and at most {max_len} bytes, got {value:?}"
        )))
    }
}

/// Keeps the ASCII characters of `value`, up to `max_len` bytes.
pub(crate) fn ascii_prefix(value: &str, max_len: usize) -> String {
    value.chars().filter(char::is_ascii).take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkt_detection() {
        assert!(looks_like_wkt(
            r#"PROJCS["RGF93 / Lambert-93",GEOGCS["RGF93"]]"#
        ));
        assert!(looks_like_wkt("  projcrs[\"x\"]"));
        assert!(looks_like_wkt("COMPOUNDCRS[\"x\"]"));
        assert!(!looks_like_wkt("EPSG:2154"));
        assert!(!looks_like_wkt(""));
        assert!(!looks_like_wkt("Lambert 93"));
    }

    #[test]
    fn vlr_validation() {
        assert!(Vlr::new("MYAPP", 1, "ok", b"x".to_vec()).validate().is_ok());
        assert!(
            Vlr::new("A_USER_ID_THAT_IS_TOO_LONG", 1, "", vec![])
                .validate()
                .is_err()
        );
        assert!(Vlr::new("é", 1, "", vec![]).validate().is_err());
    }
}

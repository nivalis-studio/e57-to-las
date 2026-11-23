use std::path::PathBuf;

use crate::output::Format;
use crate::{GroupKey, LasVersion, Warning};

/// What a successful conversion wrote.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Summary {
    /// One entry per output, in the order of their first point cloud.
    pub outputs: Vec<OutputSummary>,
    /// Things worth knowing, in the order they happened.
    pub warnings: Vec<Warning>,
}

impl Summary {
    /// Points written to all outputs.
    pub fn points_written(&self) -> u64 {
        self.outputs.iter().map(|o| o.points_written()).sum()
    }

    /// Points skipped for invalid coordinates (missing, flagged invalid, NaN or infinite).
    pub fn points_skipped(&self) -> u64 {
        self.clouds().map(|c| c.points_skipped).sum()
    }

    /// Points dropped by [`Converter::map_points`](crate::Converter::map_points).
    pub fn points_filtered(&self) -> u64 {
        self.clouds().map(|c| c.points_filtered).sum()
    }

    /// Every converted point cloud, in output order.
    pub fn clouds(&self) -> impl Iterator<Item = &CloudSummary> {
        self.outputs.iter().flat_map(|o| &o.clouds)
    }
}

/// One LAS file written.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct OutputSummary {
    /// The group key.
    pub key: GroupKey,
    /// Where it was written, when the output has a location (files).
    pub location: Option<PathBuf>,
    /// LAS or LAZ.
    pub format: Format,
    /// LAS version.
    pub version: LasVersion,
    /// LAS point record format number.
    pub point_format: u8,
    /// Scale per axis.
    pub scale: [f64; 3],
    /// Offset per axis.
    pub offset: [f64; 3],
    /// The point clouds written to it, in order.
    pub clouds: Vec<CloudSummary>,
}

impl OutputSummary {
    /// Points written to this output.
    pub fn points_written(&self) -> u64 {
        self.clouds.iter().map(|c| c.points_written).sum()
    }
}

/// What happened to the points of one point cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct CloudSummary {
    /// Index of the point cloud in the E57 file.
    pub index: usize,
    /// Points written.
    pub points_written: u64,
    /// Points skipped for invalid coordinates.
    pub points_skipped: u64,
    /// Points dropped by [`Converter::map_points`](crate::Converter::map_points).
    pub points_filtered: u64,
}

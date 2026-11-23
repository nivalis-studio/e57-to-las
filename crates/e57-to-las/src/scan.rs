use std::ops::ControlFlow;

use crate::{Attributes, CloudInfo, Error, Input, Point, Result, decode};

/// An opened E57 file: its metadata, read once.
///
/// Use it to look at the point clouds before converting, then pass it to
/// [`Converter::convert_scan`](crate::Converter::convert_scan) to avoid reading
/// the metadata twice. It also reads points without writing LAS.
///
/// ```no_run
/// use e57_to_las::Scan;
///
/// let scan = Scan::open("scan.e57")?;
/// for cloud in scan.clouds() {
///     println!("{} {:?}: {} points at {:?}", cloud.index, cloud.name, cloud.point_count, cloud.pose.translation);
/// }
/// # Ok::<(), e57_to_las::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Scan {
    pub(crate) input: Input,
    guid: String,
    coordinate_metadata: Option<String>,
    clouds: Vec<CloudInfo>,
    pub(crate) pointclouds: Vec<e57::PointCloud>,
}

impl Scan {
    /// Opens `input` and reads its metadata.
    pub fn open(input: impl Into<Input>) -> Result<Self> {
        let input = input.into();
        let reader = input.open()?;
        let pointclouds = reader.pointclouds();
        let clouds = pointclouds
            .iter()
            .enumerate()
            .map(|(index, pc)| CloudInfo::from_e57(index, pc))
            .collect();
        Ok(Scan {
            guid: reader.guid().to_owned(),
            coordinate_metadata: reader
                .coordinate_metadata()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            clouds,
            pointclouds,
            input,
        })
    }

    /// The input this scan was opened from.
    pub fn input(&self) -> &Input {
        &self.input
    }

    /// The point clouds, in file order.
    pub fn clouds(&self) -> &[CloudInfo] {
        &self.clouds
    }

    /// GUID of the E57 file.
    pub fn guid(&self) -> &str {
        &self.guid
    }

    /// The E57 `coordinateMetadata`: a description of the coordinate reference
    /// system, often WKT or an EPSG code. `None` when absent or empty.
    pub fn coordinate_metadata(&self) -> Option<&str> {
        self.coordinate_metadata.as_deref()
    }

    /// Total number of point records.
    pub fn point_count(&self) -> u64 {
        self.clouds.iter().map(|c| c.point_count).sum()
    }

    /// Reads the points of point cloud `cloud`, with the pose applied, and
    /// calls `on_point` for each point that has valid coordinates.
    /// `on_point` can stop the read early by returning [`ControlFlow::Break`].
    ///
    /// Returns how many records were skipped for invalid coordinates.
    pub fn for_each_point(
        &self,
        cloud: usize,
        attributes: Attributes,
        mut on_point: impl FnMut(Point) -> ControlFlow<()>,
    ) -> Result<u64> {
        let (info, pc) = self
            .clouds
            .get(cloud)
            .zip(self.pointclouds.get(cloud))
            .ok_or_else(|| {
                Error::InvalidOption(format!(
                    "point cloud {cloud} does not exist, the file has {}",
                    self.clouds.len()
                ))
            })?;
        let mut reader = self.input.open()?;
        let stats = decode::for_each_point(&mut reader, pc, info, &attributes, |point| {
            Ok(on_point(point))
        })?;
        Ok(stats.skipped)
    }
}

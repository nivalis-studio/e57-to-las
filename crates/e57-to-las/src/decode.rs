//! Reads E57 point records into [`Point`]s.
//!
//! Two paths produce the same points:
//! - the `e57` crate's simple reader, fast and batched, for coordinates, color,
//!   intensity and row/column;
//! - its raw reader, decoded here, when timestamps, returns or normals are
//!   wanted, since the simple reader does not expose them.

use std::io::{Read, Seek};
use std::ops::ControlFlow;

use e57::{CartesianCoordinate, E57Reader, RecordDataType, RecordName, RecordValue};

use crate::cloud::NORMAL_NAMES;
use crate::point::Point;
use crate::{Attributes, CloudInfo, Error, Result, Rgb};

/// What a read of one point cloud saw, besides the points it returned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DecodeStats {
    /// Records without valid, finite Cartesian coordinates.
    pub(crate) skipped: u64,
    /// Points whose return number or count was above 7 and was clamped.
    pub(crate) returns_clamped: u64,
}

/// Calls `on_point` for every point of `pc` that has valid, finite coordinates.
///
/// Stops early when `on_point` breaks or fails.
pub(crate) fn for_each_point<R: Read + Seek>(
    reader: &mut E57Reader<R>,
    pc: &e57::PointCloud,
    info: &CloudInfo,
    attributes: &Attributes,
    mut on_point: impl FnMut(Point) -> Result<ControlFlow<()>>,
) -> Result<DecodeStats> {
    let cloud = info.index;
    let read_error = |err: e57::Error| Error::Read {
        cloud,
        source: Box::new(err),
    };
    let mut stats = DecodeStats::default();

    let needs_raw = (attributes.time && info.has_time)
        || (attributes.returns && info.has_returns)
        || (attributes.normals && info.has_normals);

    if needs_raw {
        let decoder = RawDecoder::new(pc, info, attributes).map_err(read_error)?;
        for values in reader.pointcloud_raw(pc).map_err(read_error)? {
            let values = values.map_err(read_error)?;
            match decoder.decode(&values, &mut stats).map_err(read_error)? {
                Some(point) => {
                    if on_point(point)?.is_break() {
                        break;
                    }
                }
                None => stats.skipped += 1,
            }
        }
    } else {
        let mut points = reader.pointcloud_simple(pc).map_err(read_error)?;
        // The simple reader turns intensity into gray colors by default, which
        // would invent colors for clouds that have none.
        points.intensity_to_color(false);
        for point in points {
            let point = point.map_err(read_error)?;
            match from_simple(point, cloud, attributes) {
                Some(point) => {
                    if on_point(point)?.is_break() {
                        break;
                    }
                }
                None => stats.skipped += 1,
            }
        }
    }
    Ok(stats)
}

fn finite(xyz: [f64; 3]) -> Option<[f64; 3]> {
    xyz.iter().all(|v| v.is_finite()).then_some(xyz)
}

fn from_simple(point: e57::Point, cloud: usize, attributes: &Attributes) -> Option<Point> {
    let CartesianCoordinate::Valid { x, y, z } = point.cartesian else {
        return None;
    };
    let mut out = Point::at(cloud, finite([x, y, z])?);
    if attributes.color {
        out.color = point.color.map(|c| Rgb {
            red: unit_to_u16(f64::from(c.red)),
            green: unit_to_u16(f64::from(c.green)),
            blue: unit_to_u16(f64::from(c.blue)),
        });
    }
    if attributes.intensity {
        out.intensity = point.intensity.map(|i| unit_to_u16(f64::from(i)));
    }
    if attributes.row_column {
        out.row = (point.row >= 0).then_some(point.row);
        out.column = (point.column >= 0).then_some(point.column);
    }
    Some(out)
}

/// Maps a value normalized to 0..=1 onto 0..=65535, rounding to nearest.
/// Out-of-range values are clamped, non-finite values become 0.
pub(crate) fn unit_to_u16(value: f64) -> u16 {
    if !value.is_finite() {
        return 0;
    }
    // In range by construction: the clamp bounds the product to 0..=65535.
    (value.clamp(0.0, 1.0) * f64::from(u16::MAX)).round() as u16
}

/// A value range used to normalize intensities and colors to 0..=1, built the
/// same way as in the `e57` crate's simple reader: from the point cloud limits
/// when present, otherwise from the record's data type.
#[derive(Debug, Clone, Copy)]
struct Range {
    min: f64,
    span: f64,
}

impl Range {
    fn from_limits(min: Option<&RecordValue>, max: Option<&RecordValue>) -> Option<Self> {
        let value = |v: &RecordValue| match v {
            RecordValue::Double(v) => Some(*v),
            RecordValue::Single(v) => Some(f64::from(*v)),
            RecordValue::Integer(v) => Some(*v as f64),
            RecordValue::ScaledInteger(_) => None,
        };
        Self::new(value(min?)?, value(max?)?)
    }

    fn from_data_type(data_type: &RecordDataType) -> Option<Self> {
        match data_type {
            RecordDataType::Single { min, max } => Self::new(
                f64::from(min.unwrap_or(f32::MIN)),
                f64::from(max.unwrap_or(f32::MAX)),
            ),
            RecordDataType::Double { min, max } => {
                Self::new(min.unwrap_or(f64::MIN), max.unwrap_or(f64::MAX))
            }
            RecordDataType::ScaledInteger {
                min,
                max,
                scale,
                offset,
            } => Self::new(*min as f64 * scale + offset, *max as f64 * scale + offset),
            RecordDataType::Integer { min, max } => Self::new(*min as f64, *max as f64),
        }
    }

    fn new(min: f64, max: f64) -> Option<Self> {
        (max >= min).then_some(Range {
            min,
            span: max - min,
        })
    }

    fn normalize(&self, value: f64) -> f64 {
        (value - self.min) / self.span
    }
}

/// Positions of the attributes in a point cloud's raw records.
#[derive(Debug, Default)]
struct RawIndices {
    cartesian: Option<[usize; 3]>,
    cartesian_invalid: Option<usize>,
    spherical: Option<[usize; 3]>,
    spherical_invalid: Option<usize>,
    color: Option<[usize; 3]>,
    color_invalid: Option<usize>,
    intensity: Option<usize>,
    intensity_invalid: Option<usize>,
    time: Option<usize>,
    time_invalid: Option<usize>,
    return_index: Option<usize>,
    return_count: Option<usize>,
    normal: Option<[usize; 3]>,
    row: Option<usize>,
    column: Option<usize>,
}

/// Decodes raw E57 records into [`Point`]s.
struct RawDecoder<'a> {
    prototype: &'a [e57::Record],
    at: RawIndices,
    cloud: usize,
    rotation: [f64; 9],
    translation: [f64; 3],
    time_origin: f64,
    intensity_range: Option<Range>,
    color_ranges: Option<[Range; 3]>,
}

impl<'a> RawDecoder<'a> {
    fn new(
        pc: &'a e57::PointCloud,
        info: &CloudInfo,
        attributes: &Attributes,
    ) -> e57::Result<Self> {
        let prototype = pc.prototype.as_slice();
        let find = |name: RecordName| prototype.iter().position(|r| r.name == name);
        let find3 = |names: [RecordName; 3]| -> Option<[usize; 3]> {
            let [a, b, c] = names.map(&find);
            Some([a?, b?, c?])
        };
        let find_extension = |wanted: &str| {
            prototype
                .iter()
                .position(|r| matches!(&r.name, RecordName::Unknown { name, .. } if name == wanted))
        };
        let want = |on: bool, index: Option<usize>| index.filter(|_| on);

        let at = RawIndices {
            cartesian: find3([
                RecordName::CartesianX,
                RecordName::CartesianY,
                RecordName::CartesianZ,
            ]),
            cartesian_invalid: find(RecordName::CartesianInvalidState),
            spherical: find3([
                RecordName::SphericalRange,
                RecordName::SphericalAzimuth,
                RecordName::SphericalElevation,
            ]),
            spherical_invalid: find(RecordName::SphericalInvalidState),
            color: find3([
                RecordName::ColorRed,
                RecordName::ColorGreen,
                RecordName::ColorBlue,
            ])
            .filter(|_| attributes.color),
            color_invalid: find(RecordName::IsColorInvalid),
            intensity: want(attributes.intensity, find(RecordName::Intensity)),
            intensity_invalid: find(RecordName::IsIntensityInvalid),
            time: want(attributes.time, find(RecordName::TimeStamp)),
            time_invalid: find(RecordName::IsTimeStampInvalid),
            return_index: want(attributes.returns, find(RecordName::ReturnIndex)),
            return_count: want(attributes.returns, find(RecordName::ReturnCount)),
            normal: {
                let [x, y, z] = NORMAL_NAMES.map(find_extension);
                (|| Some([x?, y?, z?]))().filter(|_| attributes.normals)
            },
            row: want(attributes.row_column, find(RecordName::RowIndex)),
            column: want(attributes.row_column, find(RecordName::ColumnIndex)),
        };

        let range_of =
            |index: Option<usize>, limits: Option<(Option<&RecordValue>, Option<&RecordValue>)>| {
                index.and_then(|i| {
                    limits
                        .and_then(|(min, max)| Range::from_limits(min, max))
                        .or_else(|| Range::from_data_type(&prototype[i].data_type))
                })
            };
        let intensity_range = range_of(
            at.intensity,
            pc.intensity_limits
                .as_ref()
                .map(|l| (l.intensity_min.as_ref(), l.intensity_max.as_ref())),
        );
        let color_ranges = at.color.and_then(|[r, g, b]| {
            let limits = pc.color_limits.as_ref();
            Some([
                range_of(
                    Some(r),
                    limits.map(|l| (l.red_min.as_ref(), l.red_max.as_ref())),
                )?,
                range_of(
                    Some(g),
                    limits.map(|l| (l.green_min.as_ref(), l.green_max.as_ref())),
                )?,
                range_of(
                    Some(b),
                    limits.map(|l| (l.blue_min.as_ref(), l.blue_max.as_ref())),
                )?,
            ])
        });

        Ok(RawDecoder {
            prototype,
            at,
            cloud: info.index,
            rotation: info.pose.matrix(),
            translation: info.pose.translation,
            time_origin: info.acquisition_start.unwrap_or(0.0),
            intensity_range,
            color_ranges,
        })
    }

    fn f64_at(&self, values: &[RecordValue], index: usize) -> e57::Result<f64> {
        values[index].to_f64(&self.prototype[index].data_type)
    }

    fn i64_at(&self, values: &[RecordValue], index: usize) -> e57::Result<i64> {
        values[index].to_i64(&self.prototype[index].data_type)
    }

    /// The value of an `is...Invalid` flag, `0` (valid) when the record has none.
    fn state(&self, values: &[RecordValue], index: Option<usize>) -> e57::Result<i64> {
        index.map_or(Ok(0), |i| self.i64_at(values, i))
    }

    fn rotate(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        let m = &self.rotation;
        [
            m[0] * x + m[3] * y + m[6] * z,
            m[1] * x + m[4] * y + m[7] * z,
            m[2] * x + m[5] * y + m[8] * z,
        ]
    }

    fn position(&self, values: &[RecordValue]) -> e57::Result<Option<[f64; 3]>> {
        let local = if let Some(i) = self.at.cartesian.filter(|_| {
            // Unknown states are treated as invalid rather than failing the read.
            matches!(self.state(values, self.at.cartesian_invalid), Ok(0))
        }) {
            [
                self.f64_at(values, i[0])?,
                self.f64_at(values, i[1])?,
                self.f64_at(values, i[2])?,
            ]
        } else if let Some(i) = self
            .at
            .spherical
            .filter(|_| matches!(self.state(values, self.at.spherical_invalid), Ok(0)))
        {
            let range = self.f64_at(values, i[0])?;
            let azimuth = self.f64_at(values, i[1])?;
            let elevation = self.f64_at(values, i[2])?;
            let cos_elevation = elevation.cos();
            [
                range * cos_elevation * azimuth.cos(),
                range * cos_elevation * azimuth.sin(),
                range * elevation.sin(),
            ]
        } else {
            return Ok(None);
        };
        let [x, y, z] = self.rotate(local);
        let [tx, ty, tz] = self.translation;
        Ok(finite([x + tx, y + ty, z + tz]))
    }

    fn decode(
        &self,
        values: &[RecordValue],
        stats: &mut DecodeStats,
    ) -> e57::Result<Option<Point>> {
        let Some(xyz) = self.position(values)? else {
            return Ok(None);
        };
        let mut point = Point::at(self.cloud, xyz);

        if let (Some([r, g, b]), Some(ranges)) = (self.at.color, &self.color_ranges)
            && self.state(values, self.at.color_invalid)? == 0
        {
            let channel = |i: usize, range: &Range| -> e57::Result<u16> {
                Ok(unit_to_u16(range.normalize(self.f64_at(values, i)?)))
            };
            point.color = Some(Rgb {
                red: channel(r, &ranges[0])?,
                green: channel(g, &ranges[1])?,
                blue: channel(b, &ranges[2])?,
            });
        }
        if let (Some(i), Some(range)) = (self.at.intensity, &self.intensity_range)
            && self.state(values, self.at.intensity_invalid)? == 0
        {
            point.intensity = Some(unit_to_u16(range.normalize(self.f64_at(values, i)?)));
        }
        if let Some(i) = self.at.time
            && self.state(values, self.at.time_invalid)? == 0
        {
            point.time = Some(self.time_origin + self.f64_at(values, i)?);
        }
        if let (Some(index), Some(count)) = (self.at.return_index, self.at.return_count) {
            let index = self.i64_at(values, index)?;
            let count = self.i64_at(values, count)?;
            if index >= 0 && count >= 1 {
                if index + 1 > 7 || count > 7 {
                    stats.returns_clamped += 1;
                }
                // Clamped to 1..=7, the range of legacy LAS point formats.
                point.return_number = Some((index + 1).clamp(1, 7) as u8);
                point.number_of_returns = Some(count.clamp(1, 7) as u8);
            }
        }
        if let Some([x, y, z]) = self.at.normal {
            let normal = self.rotate([
                self.f64_at(values, x)?,
                self.f64_at(values, y)?,
                self.f64_at(values, z)?,
            ]);
            point.normal = Some(normal.map(|v| v as f32));
        }
        if let Some(i) = self.at.row {
            point.row = Some(self.i64_at(values, i)?).filter(|v| *v >= 0);
        }
        if let Some(i) = self.at.column {
            point.column = Some(self.i64_at(values, i)?).filter(|v| *v >= 0);
        }
        Ok(Some(point))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_to_u16_rounds_and_clamps() {
        assert_eq!(unit_to_u16(0.0), 0);
        assert_eq!(unit_to_u16(1.0), 65535);
        assert_eq!(unit_to_u16(0.5), 32768);
        assert_eq!(unit_to_u16(-1.0), 0);
        assert_eq!(unit_to_u16(2.0), 65535);
        assert_eq!(unit_to_u16(f64::NAN), 0);
        // An 8-bit channel maps exactly: 255 -> 65535, 128 -> 128 * 257.
        assert_eq!(unit_to_u16(128.0 / 255.0), 128 * 257);
    }

    #[test]
    fn range_from_integer_data_type() {
        let range = Range::from_data_type(&RecordDataType::Integer { min: 0, max: 255 })
            .expect("valid range");
        assert_eq!(range.normalize(255.0), 1.0);
        assert_eq!(range.normalize(0.0), 0.0);
    }
}

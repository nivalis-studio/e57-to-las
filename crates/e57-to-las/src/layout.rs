//! LAS point records: which point format to use and how to encode a [`Point`].
//!
//! Records are encoded here, on the decoding threads, into raw little-endian
//! bytes handed to `las::Writer::write_points`. The writer then only copies
//! bytes (or compresses them for LAZ).

use crate::Point;
use crate::precision::Transforms;

/// How point times are stored in the LAS GPS time field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeEncoding {
    /// Adjusted standard GPS time: GPS seconds minus 1e9.
    AdjustedStandard,
    /// GPS seconds of the week.
    WeekSeconds,
    /// The time as is: seconds since the scan start, for clouds without an
    /// acquisition start.
    AsIs,
}

/// Everything needed to encode a point record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Layout {
    pub(crate) has_time: bool,
    pub(crate) has_color: bool,
    pub(crate) normals: bool,
    pub(crate) row_column: bool,
    pub(crate) point_source_id: bool,
    pub(crate) time: TimeEncoding,
}

const BASE_LEN: usize = 20;
const NORMALS_LEN: usize = 12;
const ROW_COLUMN_LEN: usize = 8;

impl Layout {
    /// The legacy LAS point format (0 to 3). Legacy formats are read by every
    /// LAS tool, and are smaller than the 1.4 extended formats.
    pub(crate) fn format_number(&self) -> u8 {
        match (self.has_time, self.has_color) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        }
    }

    pub(crate) fn extra_bytes(&self) -> u16 {
        let mut n = 0;
        if self.normals {
            n += NORMALS_LEN;
        }
        if self.row_column {
            n += ROW_COLUMN_LEN;
        }
        n as u16
    }

    pub(crate) fn las_format(&self, compressed: bool) -> las::point::Format {
        // Formats 0 to 3 always exist.
        let mut format = las::point::Format::new(self.format_number()).unwrap_or_default();
        format.extra_bytes = self.extra_bytes();
        format.is_compressed = compressed;
        format
    }

    pub(crate) fn record_len(&self) -> usize {
        BASE_LEN
            + if self.has_time { 8 } else { 0 }
            + if self.has_color { 6 } else { 0 }
            + usize::from(self.extra_bytes())
    }

    /// Appends the record of `point` to `out`. Returns `false`, writing
    /// nothing, if the point does not fit `transforms`.
    #[inline]
    pub(crate) fn encode(&self, point: &Point, transforms: &Transforms, out: &mut Vec<u8>) -> bool {
        let Some([x, y, z]) = transforms.quantize(point.xyz()) else {
            return false;
        };
        out.extend_from_slice(&x.to_le_bytes());
        out.extend_from_slice(&y.to_le_bytes());
        out.extend_from_slice(&z.to_le_bytes());
        out.extend_from_slice(&point.intensity.unwrap_or(0).to_le_bytes());
        let return_number = point.return_number.unwrap_or(1).clamp(1, 7);
        let number_of_returns = point.number_of_returns.unwrap_or(1).clamp(1, 7);
        // Return number (bits 0-2), number of returns (bits 3-5), scan direction, edge of line.
        out.push(return_number | (number_of_returns << 3));
        // Classification 0 (never classified), scan angle rank 0, user data 0.
        out.extend_from_slice(&[0, 0, 0]);
        let source_id = if self.point_source_id {
            u16::try_from(point.cloud).unwrap_or(u16::MAX)
        } else {
            0
        };
        out.extend_from_slice(&source_id.to_le_bytes());
        if self.has_time {
            let time = point.time.map_or(0.0, |t| match self.time {
                TimeEncoding::AdjustedStandard => t - 1e9,
                TimeEncoding::WeekSeconds => t.rem_euclid(604_800.0),
                TimeEncoding::AsIs => t,
            });
            out.extend_from_slice(&time.to_le_bytes());
        }
        if self.has_color {
            let color = point.color.unwrap_or_default();
            out.extend_from_slice(&color.red.to_le_bytes());
            out.extend_from_slice(&color.green.to_le_bytes());
            out.extend_from_slice(&color.blue.to_le_bytes());
        }
        if self.normals {
            for v in point.normal.unwrap_or([0.0; 3]) {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        if self.row_column {
            for v in [point.row, point.column] {
                let v = v.map_or(-1, |v| i32::try_from(v).unwrap_or(i32::MAX));
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        true
    }

    /// The "Extra Bytes" VLR describing the extra bytes of each record, if any.
    pub(crate) fn extra_bytes_vlr(&self) -> Option<las::Vlr> {
        const F32: u8 = 9;
        const I32: u8 = 6;
        let mut fields: Vec<(&str, u8, &str)> = Vec::new();
        if self.normals {
            fields.extend([
                ("normal_x", F32, "Surface normal X"),
                ("normal_y", F32, "Surface normal Y"),
                ("normal_z", F32, "Surface normal Z"),
            ]);
        }
        if self.row_column {
            fields.extend([
                ("row", I32, "Scan grid row (-1: none)"),
                ("column", I32, "Scan grid column (-1: none)"),
            ]);
        }
        if fields.is_empty() {
            return None;
        }
        let mut data = Vec::with_capacity(fields.len() * 192);
        for (name, data_type, description) in fields {
            data.extend_from_slice(&extra_bytes_descriptor(name, data_type, description));
        }
        Some(las::Vlr {
            user_id: "LASF_Spec".to_owned(),
            record_id: 4,
            description: "Extra bytes".to_owned(),
            data,
        })
    }
}

/// One 192-byte "Extra Bytes" descriptor (LAS 1.4 R15, section 2.5.2.4).
fn extra_bytes_descriptor(name: &str, data_type: u8, description: &str) -> [u8; 192] {
    let mut out = [0u8; 192];
    // reserved[2], data_type, options (no no_data/min/max/scale/offset)
    out[2] = data_type;
    out[4..4 + name.len().min(32)].copy_from_slice(&name.as_bytes()[..name.len().min(32)]);
    // unused[4], no_data[24], min[24], max[24], scale[24], offset[24] stay zero.
    out[160..160 + description.len().min(32)]
        .copy_from_slice(&description.as_bytes()[..description.len().min(32)]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(has_time: bool, has_color: bool, normals: bool, row_column: bool) -> Layout {
        Layout {
            has_time,
            has_color,
            normals,
            row_column,
            point_source_id: true,
            time: TimeEncoding::AdjustedStandard,
        }
    }

    #[test]
    fn record_len_matches_las() {
        for has_time in [false, true] {
            for has_color in [false, true] {
                for normals in [false, true] {
                    for row_column in [false, true] {
                        let l = layout(has_time, has_color, normals, row_column);
                        assert_eq!(
                            l.record_len(),
                            usize::from(l.las_format(false).len()),
                            "{l:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn encoded_record_has_the_record_length() {
        let l = layout(true, true, true, true);
        let mut out = Vec::new();
        let mut point = Point::at(3, [1.0, 2.0, 3.0]);
        point.time = Some(1.4e9);
        assert!(l.encode(&point, &Transforms::around([0.0; 3], 0.001), &mut out));
        assert_eq!(out.len(), l.record_len());
    }

    #[test]
    fn far_points_are_rejected_without_writing() {
        let mut out = Vec::new();
        let far = Point::at(0, [1e7, 0.0, 0.0]);
        assert!(!layout(false, false, false, false).encode(
            &far,
            &Transforms::around([0.0; 3], 0.001),
            &mut out
        ));
        assert!(out.is_empty());
    }
}

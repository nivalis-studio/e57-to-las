//! The point type handed to [`Converter::map_points`](crate::Converter::map_points)
//! and [`Scan::for_each_point`](crate::Scan::for_each_point).

/// A 16-bit RGB color, as stored in LAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// Red, 0 to 65535.
    pub red: u16,
    /// Green, 0 to 65535.
    pub green: u16,
    /// Blue, 0 to 65535.
    pub blue: u16,
}

/// One point read from an E57 file, with the pose of its point cloud applied.
///
/// Fields the E57 file does not have, or that the converter was told not to
/// keep (see [`Attributes`](crate::Attributes)), are `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
    /// Intensity scaled to 0..=65535 from the point cloud's intensity limits.
    pub intensity: Option<u16>,
    /// Color scaled to 0..=65535 from the point cloud's color limits.
    pub color: Option<Rgb>,
    /// GPS time in seconds since 1980-01-06, or seconds since the start of the
    /// scan when the point cloud has no acquisition start.
    pub time: Option<f64>,
    /// Return number, from 1.
    pub return_number: Option<u8>,
    /// Number of returns of the pulse, from 1.
    pub number_of_returns: Option<u8>,
    /// Surface normal, rotated by the pose.
    pub normal: Option<[f32; 3]>,
    /// Row of the point in the scan grid.
    pub row: Option<i64>,
    /// Column of the point in the scan grid.
    pub column: Option<i64>,
    /// Index of the point cloud the point comes from.
    pub cloud: usize,
}

impl Point {
    /// The coordinates as `[x, y, z]`.
    pub fn xyz(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    pub(crate) fn at(cloud: usize, [x, y, z]: [f64; 3]) -> Self {
        Point {
            x,
            y,
            z,
            intensity: None,
            color: None,
            time: None,
            return_number: None,
            number_of_returns: None,
            normal: None,
            row: None,
            column: None,
            cloud,
        }
    }
}

/// Which point attributes the converter reads from the E57 file and writes to LAS.
///
/// Coordinates are always written. Attributes missing from the input are
/// simply absent from the output; turning one off drops it even when present.
///
/// ```
/// use e57_to_las::Attributes;
///
/// let attributes = Attributes::default().normals(true).time(false);
/// assert!(attributes.normals && attributes.color && !attributes.time);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Attributes {
    /// RGB colors. On by default.
    pub color: bool,
    /// Intensity. On by default.
    pub intensity: bool,
    /// GPS time. On by default.
    pub time: bool,
    /// Return number and number of returns. On by default.
    pub returns: bool,
    /// Index of the source point cloud, written as the LAS point source ID. On by default.
    pub point_source_id: bool,
    /// Surface normals, written as three `f32` LAS extra bytes
    /// (`normal_x`, `normal_y`, `normal_z`). Off by default.
    pub normals: bool,
    /// Scan grid row and column, written as two `i32` LAS extra bytes
    /// (`row`, `column`). Off by default.
    pub row_column: bool,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            color: true,
            intensity: true,
            time: true,
            returns: true,
            point_source_id: true,
            normals: false,
            row_column: false,
        }
    }
}

macro_rules! setters {
    ($($field:ident),*) => {$(
        #[doc = concat!("Sets [`Attributes::", stringify!($field), "`].")]
        #[must_use]
        pub const fn $field(mut self, on: bool) -> Self {
            self.$field = on;
            self
        }
    )*};
}

impl Attributes {
    /// Every attribute, extra bytes included.
    pub const ALL: Attributes = Attributes {
        color: true,
        intensity: true,
        time: true,
        returns: true,
        point_source_id: true,
        normals: true,
        row_column: true,
    };

    /// Coordinates only.
    pub const NONE: Attributes = Attributes {
        color: false,
        intensity: false,
        time: false,
        returns: false,
        point_source_id: false,
        normals: false,
        row_column: false,
    };

    setters!(
        color,
        intensity,
        time,
        returns,
        point_source_id,
        normals,
        row_column
    );
}

use std::fmt;
use std::sync::Arc;

use crate::output::{Format, Output, Target};
use crate::{
    Attributes, CancelToken, CloudInfo, Crs, Event, GroupKey, HeaderOptions, Input, LasVersion,
    Point, Precision, Result, Scan, Summary, engine,
};

pub(crate) type SelectFn = dyn Fn(&CloudInfo) -> bool + Send + Sync;
pub(crate) type MapFn = dyn Fn(&mut Point) -> bool + Send + Sync;
pub(crate) type HeaderFn = dyn Fn(&Target<'_>, &mut HeaderOptions) + Send + Sync;
pub(crate) type ObserverFn = dyn Fn(&Event<'_>) + Send + Sync;
type GroupFn = dyn Fn(&CloudInfo) -> GroupKey + Send + Sync;

/// How point clouds are distributed among outputs.
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum Grouping {
    /// Every point cloud in one output, with key `"merged"`. The default.
    #[default]
    Merged,
    /// One output per point cloud, keyed by its index.
    PerCloud,
    /// Point clouds returning the same key share an output. See [`Grouping::by`].
    Custom(Arc<GroupFn>),
}

impl Grouping {
    /// Groups point clouds by the key `key` returns.
    ///
    /// ```
    /// use e57_to_las::Grouping;
    ///
    /// // "floor1_scan03" and "floor1_scan04" go to output "floor1".
    /// let by_floor = Grouping::by(|cloud| {
    ///     let name = cloud.name.as_deref().unwrap_or_default();
    ///     name.split('_').next().unwrap_or_default().to_owned()
    /// });
    /// ```
    pub fn by<F, K>(key: F) -> Self
    where
        F: Fn(&CloudInfo) -> K + Send + Sync + 'static,
        K: Into<GroupKey>,
    {
        Grouping::Custom(Arc::new(move |cloud| key(cloud).into()))
    }

    pub(crate) fn key(&self, cloud: &CloudInfo) -> GroupKey {
        match self {
            Grouping::Merged => GroupKey::from("merged"),
            Grouping::PerCloud => GroupKey::from(cloud.index),
            Grouping::Custom(key) => key(cloud),
        }
    }
}

impl fmt::Debug for Grouping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Grouping::Merged => f.write_str("Merged"),
            Grouping::PerCloud => f.write_str("PerCloud"),
            Grouping::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}

/// Converts E57 files to LAS or LAZ.
///
/// Configure it with the builder methods, then call [`convert`](Self::convert).
/// A converter is cheap to clone and can be shared between threads.
///
/// ```no_run
/// use e57_to_las::{Converter, Grouping, LasVersion, output::FileOutput};
///
/// let summary = Converter::new()
///     .las_version(LasVersion::V1_2)
///     .grouping(Grouping::PerCloud)
///     .convert("scan.e57", FileOutput::new("out/{index}.las"))?;
/// println!("{} points in {} files", summary.points_written(), summary.outputs.len());
/// # Ok::<(), e57_to_las::Error>(())
/// ```
#[derive(Clone, Default)]
#[must_use]
pub struct Converter {
    pub(crate) las_version: LasVersion,
    pub(crate) precision: Precision,
    pub(crate) format: Option<Format>,
    pub(crate) attributes: Attributes,
    pub(crate) crs: Crs,
    pub(crate) grouping: Grouping,
    pub(crate) threads: usize,
    pub(crate) stable_order: bool,
    pub(crate) select: Option<Arc<SelectFn>>,
    pub(crate) map_points: Option<Arc<MapFn>>,
    pub(crate) header: Option<Arc<HeaderFn>>,
    pub(crate) observer: Option<Arc<ObserverFn>>,
    pub(crate) cancel: Option<CancelToken>,
}

impl Converter {
    /// A converter with the defaults: LAS 1.4, 1 mm precision, every point
    /// cloud merged into one output, all CPU cores.
    pub fn new() -> Self {
        Self::default()
    }

    /// The LAS version to write. Default: [`LasVersion::V1_4`].
    pub fn las_version(mut self, version: LasVersion) -> Self {
        self.las_version = version;
        self
    }

    /// How scale and offset are chosen. Default: [`Precision::Auto`] (1 mm).
    pub fn precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    /// Writes LAS or LAZ. By default the format comes from the output's file
    /// extension (`.laz` → LAZ), and is LAS for outputs that are not files.
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Which point attributes to write. Default: [`Attributes::default`].
    pub fn attributes(mut self, attributes: Attributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// The coordinate reference system to write. Default: [`Crs::FromE57`].
    pub fn crs(mut self, crs: Crs) -> Self {
        self.crs = crs;
        self
    }

    /// How point clouds are distributed among outputs. Default: [`Grouping::Merged`].
    pub fn grouping(mut self, grouping: Grouping) -> Self {
        self.grouping = grouping;
        self
    }

    /// Number of threads. `0`, the default, uses one per logical CPU; `1`
    /// runs on the calling thread. Without the `parallel` feature, everything
    /// runs on the calling thread.
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    /// Writes the points of each output in point cloud order, so the output is
    /// the same on every run.
    ///
    /// By default, point clouds sharing an output are read in parallel and
    /// their points interleave in an order that changes between runs. With a
    /// stable order, the point clouds of one output are read one after the
    /// other; different outputs are still converted in parallel.
    pub fn stable_order(mut self, stable: bool) -> Self {
        self.stable_order = stable;
        self
    }

    /// Converts only the point clouds for which `select` returns `true`.
    ///
    /// ```
    /// use e57_to_las::Converter;
    ///
    /// let converter = Converter::new().select(|cloud| cloud.point_count > 0 && cloud.index != 3);
    /// ```
    pub fn select<F>(mut self, select: F) -> Self
    where
        F: Fn(&CloudInfo) -> bool + Send + Sync + 'static,
    {
        self.select = Some(Arc::new(select));
        self
    }

    /// Changes or drops points before they are written: `map` gets each point
    /// with valid coordinates and returns `false` to drop it.
    ///
    /// Called from several threads at once.
    ///
    /// ```
    /// use e57_to_las::Converter;
    ///
    /// let converter = Converter::new().map_points(|point| {
    ///     point.z -= 47.3; // shift heights
    ///     point.intensity.is_none_or(|i| i > 1000) // drop dim points
    /// });
    /// ```
    pub fn map_points<F>(mut self, map: F) -> Self
    where
        F: Fn(&mut Point) -> bool + Send + Sync + 'static,
    {
        self.map_points = Some(Arc::new(map));
        self
    }

    /// Edits the header of each output before it is written.
    ///
    /// ```
    /// use e57_to_las::{Converter, Vlr};
    ///
    /// let converter = Converter::new().header(|target, header| {
    ///     header.system_identifier = "RTC360".into();
    ///     header.vlrs.push(Vlr::new("MYAPP", 1, "job", target.key().to_string()));
    /// });
    /// ```
    pub fn header<F>(mut self, edit: F) -> Self
    where
        F: Fn(&Target<'_>, &mut HeaderOptions) + Send + Sync + 'static,
    {
        self.header = Some(Arc::new(edit));
        self
    }

    /// Receives progress [`Event`]s, from the threads doing the work.
    ///
    /// ```
    /// use e57_to_las::{Converter, Event};
    ///
    /// let converter = Converter::new().observer(|event| {
    ///     if let Event::Progress { points_done, points_total } = event {
    ///         eprintln!("{points_done}/{points_total}");
    ///     }
    /// });
    /// ```
    pub fn observer<F>(mut self, observer: F) -> Self
    where
        F: Fn(&Event<'_>) + Send + Sync + 'static,
    {
        self.observer = Some(Arc::new(observer));
        self
    }

    /// Stops the conversion when `token` is cancelled.
    pub fn cancel_token(mut self, token: CancelToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// Converts `input` to `output`.
    ///
    /// `output` can be passed by value or by reference (to read a
    /// [`MemoryOutput`](crate::output::MemoryOutput) afterwards).
    pub fn convert(&self, input: impl Into<Input>, output: impl Output) -> Result<Summary> {
        let scan = Scan::open(input)?;
        self.convert_scan(&scan, output)
    }

    /// Converts an already opened [`Scan`].
    pub fn convert_scan(&self, scan: &Scan, output: impl Output) -> Result<Summary> {
        engine::run(self, scan, &output)
    }
}

impl fmt::Debug for Converter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let set = |hook: bool| if hook { "Some(..)" } else { "None" };
        f.debug_struct("Converter")
            .field("las_version", &self.las_version)
            .field("precision", &self.precision)
            .field("format", &self.format)
            .field("attributes", &self.attributes)
            .field("crs", &self.crs)
            .field("grouping", &self.grouping)
            .field("threads", &self.threads)
            .field("stable_order", &self.stable_order)
            .field("select", &set(self.select.is_some()))
            .field("map_points", &set(self.map_points.is_some()))
            .field("header", &set(self.header.is_some()))
            .field("observer", &set(self.observer.is_some()))
            .field("cancel", &self.cancel)
            .finish()
    }
}

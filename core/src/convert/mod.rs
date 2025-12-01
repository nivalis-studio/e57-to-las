//! Core conversion functions and configuration.
//!
//! This module provides the main entry points for converting E57 sources to LAS format,
//! along with configuration options and event handling.

mod event;
mod options;
#[cfg(feature = "parallel")]
pub mod parallel;

use crate::{
    Result,
    ext::{
        e57::{E57PointExt, PointMeta},
        las::LasHeaderExt,
    },
    io::{ReaderOnce, WritePointCloudCtx, WriterFactory, WriterOnce},
};
use event::EventHandler;
pub use event::{Event, EventCallback};
pub use options::{ConvertOptions, HeaderHook, LasVersion, Scale};

/// Convert an E57 source to a single LAS output, merging all point clouds.
///
/// This function reads all point clouds from the E57 source and writes them sequentially
/// to a single LAS output. The LAS header is configured based on the combined
/// characteristics of all point clouds (global bounds, color support, timestamps).
///
/// # Point Cloud Merging
///
/// When multiple point clouds are present in the E57 source:
/// - They are processed sequentially in order
/// - All points are written to the same LAS output
/// - The LAS header uses global bounds encompassing all clouds
/// - Point format is chosen to support all attributes (color, time) present in any cloud
/// - Each cloud's transformation (rotation/translation) is already applied by the E57 library
///
/// # Return Value
///
/// Returns the underlying writer after the LAS output has been **finalized and closed**.
/// For most use cases, you can safely ignore or drop this return value - the conversion
/// is complete and the output is ready to use.
///
/// Advanced users may want to keep the writer for additional operations:
/// - Reusing the same file handle for other I/O
/// - Extracting written data from in-memory sinks (`Vec<u8>`, `Cursor`)
/// - Accessing metadata about the write operation
///
/// **Important**: The writer has already been flushed and the LAS output is complete.
/// Do not attempt to write additional LAS points to this writer.
///
/// # Examples
///
/// Basic conversion:
///
/// ```no_run
/// use e57_to_las::{convert, ConvertOptions};
///
/// convert("input.e57", "output.las", &ConvertOptions::default())?;
/// ```
///
/// With custom scale and version:
///
/// ```no_run
/// use e57_to_las::{convert, ConvertOptions, Scale, LasVersion};
///
/// let opts = ConvertOptions {
///     scale: Scale::MilliMeter,
///     las_version: LasVersion::new(1, 4)?,
///     ..Default::default()
/// };
/// convert("input.e57", "output.las", &opts)?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The E57 source cannot be opened or parsed
/// - The LAS output cannot be created or written
/// - Point data conversion fails
/// - Invalid configuration in `opts` (e.g., invalid LAS version)
///
/// # See also
///
/// - [`convert_split`] - Convert each point cloud to a separate LAS output
/// - [`parallel::convert`] - Parallel version for improved performance
pub fn convert<I, O>(source: I, sink: O, opts: &ConvertOptions) -> Result<O::Writer>
where
    I: ReaderOnce,
    O: WriterOnce,
{
    let mut e57_reader = e57::E57Reader::new(source.try_into_reader()?)?;
    let pointclouds = e57_reader.pointclouds();

    let (header, point_format) = las::Header::from_pointclouds(&pointclouds, opts)?;

    let mut writer = las::Writer::new(sink.try_into_writer()?, header)?;

    let event_handler = EventHandler::new(opts.on_event.as_ref());

    for (i, pc) in pointclouds.iter().enumerate() {
        let meta = PointMeta::new(point_format, pc);
        let points = e57_reader.pointcloud_simple(pc)?;

        if let Some(ref handler) = event_handler {
            handler.send(Event::pointcloud_started(i, pc));
        }

        for p in points {
            let las_point = p?.to_las_point(&meta);
            writer.write_point(las_point)?;
        }

        if let Some(ref handler) = event_handler {
            handler.send(Event::pointcloud_ended(i));
        }
    }

    Ok(writer.into_inner()?)
}

/// Convert an E57 source to multiple LAS outputs, one per point cloud.
///
/// This function creates a separate LAS output for each point cloud in the E57 source.
/// Each LAS output is independently configured based on that point cloud's specific
/// characteristics (bounds, attributes).
///
/// # Output Naming
///
/// Outputs are named based on the sink path and point cloud metadata:
/// - If the point cloud has a `name` in the E57 source: `base_name.las` → `base_name_{cloud_name}.las`
/// - Otherwise, uses the point cloud index: `base_0.las`, `base_1.las`, etc.
/// - If sink is a directory, outputs are created within it using the cloud name or index
///
/// # Return Value
///
/// Returns a vector of writers, one for each point cloud. Each writer has been **finalized
/// and closed** - the LAS outputs are complete and ready to use.
///
/// For file-based outputs, you can typically ignore these writers. For in-memory sinks
/// (`Vec<u8>`, `Cursor`), you may want to extract the written data from the returned writers.
///
/// # Examples
///
/// ```no_run
/// use e57_to_las::{convert_split, ConvertOptions};
///
/// let writers = convert_split("scan.e57", &"output.las", &ConvertOptions::default())?;
/// println!("Created {} LAS outputs", writers.len());
/// ```
///
/// # When to Use
///
/// Use `convert_split` instead of [`convert`] when:
/// - Point clouds represent distinct scans that should remain separate
/// - Different clouds have incompatible coordinate systems or attributes
/// - You want to process individual clouds independently in downstream tools
/// - Memory is limited and merging would create excessively large outputs
///
/// # Errors
///
/// Returns an error if:
/// - The E57 source cannot be opened or parsed
/// - Any LAS output cannot be created or written
/// - Point data conversion fails
/// - Invalid configuration in `opts`
///
/// # See also
///
/// - [`convert`] - Merge all point clouds into a single LAS output
/// - [`parallel::convert_split`] - Parallel version for improved performance
pub fn convert_split<I, O>(source: I, sink: &O, opts: &ConvertOptions) -> Result<Vec<O::Writer>>
where
    I: ReaderOnce,
    O: WriterFactory,
{
    let mut e57_reader = e57::E57Reader::new(source.try_into_reader()?)?;
    let pointclouds = e57_reader.pointclouds();

    let mut sinks = Vec::with_capacity(pointclouds.len());

    let event_handler = EventHandler::new(opts.on_event.as_ref());

    for (i, pc) in pointclouds.iter().enumerate() {
        let (header, point_format) = las::Header::from_pointcloud(pc, opts)?;

        let pc_name = pc.name.as_ref();
        let ctx = WritePointCloudCtx {
            idx: i,
            name: pc_name,
        };

        let mut writer = las::Writer::new(sink.create_writer(&ctx)?, header)?;

        let meta = PointMeta::new(point_format, pc);
        let points = e57_reader.pointcloud_simple(pc)?;

        if let Some(ref handler) = event_handler {
            handler.send(Event::pointcloud_started(i, pc));
        }

        for p in points {
            let las_point = p?.to_las_point(&meta);
            writer.write_point(las_point)?;
        }

        if let Some(ref handler) = event_handler {
            handler.send(Event::pointcloud_ended(i));
        }
        sinks.push(writer.into_inner()?);
    }

    Ok(sinks)
}

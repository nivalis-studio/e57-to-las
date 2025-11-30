//! Parallel conversion functions for improved performance.
//!
//! This module provides multi-threaded versions of the conversion functions that
//! can significantly improve performance on multi-core systems, especially for
//! large E57 sources with multiple point clouds.
//!
//! # Performance Characteristics
//!
//! The parallel implementation uses a producer-consumer architecture:
//! - Multiple worker threads read and convert point clouds concurrently
//! - Points are batched to reduce synchronization overhead
//! - A dedicated writer thread serializes LAS output to maintain file format consistency
//!
//! Typical speedup: 2-4x on 4-core systems, scaling with available cores and number
//! of point clouds.
//!
//! # Configuration
//!
//! Tune performance using [`ConvertOptions`](crate::ConvertOptions):
//! - `workers`: Number of concurrent reader threads (default: number of CPU cores)
//! - `batch_size`: Points per batch (default: 128,000)
//! - `queue_size`: Maximum batches in flight (default: 8)
//!
//! # Requirements
//!
//! Parallel functions require:
//! - The `parallel` feature (enabled by default)
//! - Source implementing [`ReaderFactory`](crate::io::ReaderFactory) (use `&path` instead of `path`)
//! - Sufficient memory for batching and queuing
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "parallel")]
//! # fn example() -> e57_to_las::Result<()> {
//! use e57_to_las::{parallel, ConvertOptions};
//!
//! let opts = ConvertOptions {
//!     workers: 4,
//!     batch_size: 100_000,
//!     ..Default::default()
//! };
//!
//! // Note: Use &"input.e57" (reference) for ReaderFactory
//! parallel::convert(&"large_scan.e57", "output.las", &opts)?;
//! # Ok(())
//! # }
//! ```

use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::{
    ConvertOptions, Error, Event, Result,
    convert::event::EventHandler,
    ext::{
        e57::{E57PointExt, PointMeta},
        las::LasHeaderExt,
    },
    io::{ReaderFactory, WritePointCloudCtx, WriterFactory, WriterOnce},
};

/// Convert an E57 source to a single LAS output using parallel processing.
///
/// This is the parallel version of [`convert`](crate::convert). It uses multiple
/// worker threads to read and convert point clouds concurrently, providing
/// significant performance improvements for large sources.
///
/// # Architecture
///
/// - Worker threads (configurable via `opts.workers`) read point clouds in parallel
/// - Each worker converts points in batches (size controlled by `opts.batch_size`)
/// - Batches are sent through a bounded queue (size `opts.queue_size`) to the writer
/// - A single writer thread serializes output to maintain LAS file format consistency
///
/// # Performance Tuning
///
/// - **Fewer, larger point clouds**: Increase `workers` to match cloud count
/// - **Many small point clouds**: Reduce `batch_size` to avoid memory overhead
/// - **Memory constrained**: Reduce `queue_size` and `batch_size`
/// - **I/O bound**: Increasing parallelism won't help; optimize disk access instead
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "parallel")]
/// # fn example() -> e57_to_las::Result<()> {
/// use e57_to_las::{parallel, ConvertOptions};
///
/// let opts = ConvertOptions {
///     workers: 4,
///     batch_size: 128_000,
///     queue_size: 8,
///     ..Default::default()
/// };
///
/// // Use reference (&) for ReaderFactory
/// parallel::convert(&"input.e57", "output.las", &opts)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The E57 source cannot be opened or parsed
/// - The LAS output cannot be created or written
/// - Any worker thread panics (reported as [`Error::Internal`](crate::Error::Internal))
/// - Point conversion fails
///
/// # See also
///
/// - [`convert`](crate::convert) - Sequential version requiring less memory
/// - [`convert_split`] - Parallel split conversion
pub fn convert<I, O>(source: &I, sink: O, opts: &ConvertOptions) -> Result<O::Writer>
where
    I: ReaderFactory,
    O: WriterOnce + Send + 'static,
{
    let e57_reader = e57::E57Reader::new(source.create_reader()?)?;
    let pointclouds = e57_reader.pointclouds();
    let n = pointclouds.len();

    let (header, point_format) = las::Header::from_pointclouds(&pointclouds, opts)?;

    let (tx_writer, rx_writer) = flume::bounded::<Vec<las::Point>>(opts.queue_size);

    let writer_handle = thread::spawn(move || -> Result<O::Writer> {
        let mut writer = las::Writer::new(sink.try_into_writer()?, header)?;

        while let Ok(buf) = rx_writer.recv() {
            for p in buf {
                writer.write_point(p)?;
            }
        }

        Ok(writer.into_inner()?)
    });

    let event_handler = EventHandler::new(opts.on_event.as_ref());

    let (rx_jobs, mut readers_handles) = setup_readers_workers(n, opts);

    for _ in 0..readers_handles.capacity() {
        let rx_jobs = rx_jobs.clone();
        let tx_writer = tx_writer.clone();
        let event_sender = event_handler.as_ref().map(|h| h.sender());
        let reader = source.create_reader()?;
        let opts = opts.clone();

        readers_handles.push(thread::spawn(move || -> Result<()> {
            let mut e57_reader = e57::E57Reader::new(reader)?;
            let pointclouds = e57_reader.pointclouds();

            while let Ok(idx) = rx_jobs.recv() {
                let pc = &pointclouds[idx];
                let meta = PointMeta::new(point_format, pc);
                let points = e57_reader.pointcloud_simple(pc)?;

                if let Some(ref sender) = event_sender {
                    sender.send(Event::pointcloud_started(idx, pc));
                }

                let mut buf: Vec<las::Point> = Vec::with_capacity(opts.batch_size);
                for p in points {
                    let las_point = p?.to_las_point(&meta);

                    buf.push(las_point);

                    if buf.len() == buf.capacity() {
                        let full = std::mem::take(&mut buf);
                        tx_writer
                            .send(full)
                            .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                        buf.reserve_exact(opts.batch_size);
                    }
                }

                if !buf.is_empty() {
                    tx_writer
                        .send(buf)
                        .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                }

                if let Some(ref sender) = event_sender {
                    sender.send(Event::pointcloud_ended(idx));
                }
            }

            Ok(())
        }));
    }

    drop(tx_writer);

    join_all_workers(readers_handles, writer_handle)
}

/// Convert an E57 source to multiple LAS outputs using parallel processing.
///
/// This is the parallel version of [`convert_split`](crate::convert_split). Each point
/// cloud is written to a separate LAS output, with multiple point clouds processed
/// concurrently by worker threads.
///
/// # Architecture
///
/// - Worker threads read and convert point clouds in parallel
/// - Each point cloud gets its own LAS output with independent header configuration
/// - A single writer thread multiplexes batches to the appropriate outputs
/// - Output naming follows the same rules as [`convert_split`](crate::convert_split)
///
/// # Performance Considerations
///
/// Split conversion can be faster than merged conversion because:
/// - No global bounds calculation required upfront
/// - Each output can be independently optimized
/// - Writer thread has less work (no coordinate merging)
///
/// However, it creates more outputs which may not be desirable for all workflows.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "parallel")]
/// # fn example() -> e57_to_las::Result<()> {
/// use e57_to_las::{parallel, ConvertOptions};
///
/// let opts = ConvertOptions {
///     workers: 4,
///     ..Default::default()
/// };
///
/// // Creates output_0.las, output_1.las, etc.
/// let writers = parallel::convert_split("input.e57", "output.las", &opts)?;
/// println!("Created {} LAS outputs", writers.len());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The E57 source cannot be opened or parsed
/// - Any LAS output cannot be created or written
/// - Any worker thread panics
/// - Point conversion fails
///
/// # See also
///
/// - [`convert_split`](crate::convert_split) - Sequential version
/// - [`convert`] - Parallel merged conversion
pub fn convert_split<I, O>(source: I, sink: O, opts: &ConvertOptions) -> Result<Vec<O::Writer>>
where
    I: ReaderFactory,
    O: WriterFactory + Send + Sync + 'static,
{
    let e57_reader = e57::E57Reader::new(source.create_reader()?)?;
    let pointclouds = e57_reader.pointclouds();
    let n = pointclouds.len();

    let (tx_msg, rx_msg) = flume::bounded::<(usize, Vec<las::Point>)>(opts.queue_size);

    let mut writers = Vec::with_capacity(n);
    let mut metas = Vec::with_capacity(n);

    for (i, pc) in pointclouds.iter().enumerate() {
        let (header, point_format) = las::Header::from_pointcloud(pc, opts)?;

        metas.push(PointMeta::new(point_format, pc));

        let ctx = WritePointCloudCtx {
            idx: i,
            name: pc.name.as_ref(),
        };

        let writer = sink.create_writer(&ctx)?;
        let writer = las::Writer::new(writer, header)?;

        writers.push(writer);
    }

    let writer_handle = thread::spawn(move || -> Result<Vec<O::Writer>> {
        while let Ok((i, batch)) = rx_msg.recv() {
            let writer = &mut writers[i];

            for p in batch {
                writer.write_point(p)?;
            }
        }

        let mut raws = Vec::with_capacity(n);

        for w in writers {
            raws.push(w.into_inner()?);
        }

        Ok(raws)
    });

    let event_handler = EventHandler::new(opts.on_event.as_ref());

    let (rx_jobs, mut readers_handles) = setup_readers_workers(n, opts);

    let metas = Arc::new(metas);

    for _ in 0..readers_handles.capacity() {
        let rx_jobs = rx_jobs.clone();
        let tx_msg = tx_msg.clone();
        let event_sender = event_handler.as_ref().map(|h| h.sender());
        let reader = source.create_reader()?;
        let opts = opts.clone();
        let metas = Arc::clone(&metas);

        readers_handles.push(thread::spawn(move || -> Result<()> {
            let mut e57_reader = e57::E57Reader::new(reader)?;
            let pointclouds = e57_reader.pointclouds();

            while let Ok(i) = rx_jobs.recv() {
                let pc = &pointclouds[i];
                let meta = metas[i];
                let points = e57_reader.pointcloud_simple(pc)?;

                if let Some(ref sender) = event_sender {
                    sender.send(Event::pointcloud_started(i, pc));
                }

                let mut buf: Vec<las::Point> = Vec::with_capacity(opts.batch_size);

                for p in points {
                    let las_point = p?.to_las_point(&meta);

                    buf.push(las_point);

                    if buf.len() == buf.capacity() {
                        let full = std::mem::take(&mut buf);
                        tx_msg
                            .send((i, full))
                            .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                        buf.reserve_exact(opts.batch_size);
                    }
                }

                if !buf.is_empty() {
                    tx_msg
                        .send((i, buf))
                        .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                }

                if let Some(ref sender) = event_sender {
                    sender.send(Event::pointcloud_ended(i));
                }
            }

            Ok(())
        }));
    }

    drop(tx_msg);

    join_all_workers(readers_handles, writer_handle)
}

#[inline]
fn join_all_workers<T>(
    readers_handles: Vec<JoinHandle<Result<()>>>,
    writer_handle: JoinHandle<Result<T>>,
) -> Result<T> {
    for h in readers_handles {
        h.join()
            .map_err(|_| Error::Internal("worker panicked".into()))??;
    }

    writer_handle
        .join()
        .map_err(|_| Error::Internal("writer panicked".into()))?
}

#[inline]
fn setup_readers_workers(
    n_jobs: usize,
    opts: &ConvertOptions,
) -> (flume::Receiver<usize>, Vec<JoinHandle<Result<()>>>) {
    let (tx_jobs, rx_jobs) = flume::unbounded();

    for i in 0..n_jobs {
        tx_jobs
            .send(i)
            .expect("job sender should not fail - receiver still alive");
    }

    drop(tx_jobs);

    let workers = (n_jobs.min(opts.workers)).max(1);
    let readers_handles = Vec::with_capacity(workers);

    (rx_jobs, readers_handles)
}

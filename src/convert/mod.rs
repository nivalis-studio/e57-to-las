mod options;

use crate::{
    Result,
    ext::{
        e57::{E57PointExt, PointMeta},
        las::LasHeaderExt,
    },
    io::{ReaderOnce, WritePointCloudCtx, WriterFactory, WriterOnce},
};
pub use options::ConvertOptions;

pub fn convert<I, O>(source: I, sink: O, opts: &ConvertOptions) -> Result<O::Writer>
where
    I: ReaderOnce,
    O: WriterOnce,
{
    let mut e57_reader = e57::E57Reader::new(source.try_into_reader()?)?;
    let pointclouds = e57_reader.pointclouds();

    let (header, point_format) = las::Header::from_pointclouds(&pointclouds, opts)?;

    let mut writer = las::Writer::new(sink.try_into_writer()?, header)?;

    for pc in pointclouds {
        let meta = PointMeta::new(point_format, &pc);
        let points = e57_reader.pointcloud_simple(&pc)?;

        for p in points {
            let las_point = p?.to_las_point(&meta);
            writer.write_point(las_point)?;
        }
    }

    Ok(writer.into_inner()?)
}

pub fn convert_split<I, O>(source: I, sink: &O, opts: &ConvertOptions) -> Result<Vec<O::Writer>>
where
    I: ReaderOnce,
    O: WriterFactory,
{
    let mut e57_reader = e57::E57Reader::new(source.try_into_reader()?)?;
    let pointclouds = e57_reader.pointclouds();

    let mut sinks = Vec::with_capacity(pointclouds.len());

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

        for p in points {
            let las_point = p?.to_las_point(&meta);
            writer.write_point(las_point)?;
        }

        sinks.push(writer.into_inner()?);
    }

    Ok(sinks)
}

#[cfg(feature = "parallel")]
pub mod parallel {
    use std::{
        sync::Arc,
        thread::{self, JoinHandle},
    };

    use crate::{
        ConvertOptions, Error, Result,
        ext::{
            e57::{E57PointExt, PointMeta},
            las::LasHeaderExt,
        },
        io::{ReaderFactory, WritePointCloudCtx, WriterFactory, WriterOnce},
    };

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

        let (rx_jobs, mut readers_handles) = setup_readers_workers(n, opts);

        for _ in 0..readers_handles.capacity() {
            let rx_jobs = rx_jobs.clone();
            let tx_writer = tx_writer.clone();
            let reader = source.create_reader()?;
            let opts = opts.clone();

            readers_handles.push(thread::spawn(move || -> Result<()> {
                let mut e57_reader = e57::E57Reader::new(reader)?;
                let pointclouds = e57_reader.pointclouds();

                while let Ok(idx) = rx_jobs.recv() {
                    let pc = &pointclouds[idx];
                    let meta = PointMeta::new(point_format, pc);
                    let points = e57_reader.pointcloud_simple(pc)?;

                    let mut buf: Vec<las::Point> = Vec::with_capacity(opts.batch_size);
                    for p in points {
                        let las_point = p?.to_las_point(&meta);

                        buf.push(las_point);

                        if buf.len() == opts.batch_size {
                            let full = std::mem::take(&mut buf);
                            tx_writer
                                .send(full)
                                .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                            buf.reserve(opts.batch_size);
                        }
                    }

                    if !buf.is_empty() {
                        tx_writer
                            .send(buf)
                            .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                    }
                }

                Ok(())
            }));
        }

        drop(tx_writer);

        join_all_workers(readers_handles, writer_handle)
    }

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

        let (rx_jobs, mut readers_handles) = setup_readers_workers(n, opts);

        let metas = Arc::new(metas);

        for _ in 0..readers_handles.capacity() {
            let rx_jobs = rx_jobs.clone();
            let tx_msg = tx_msg.clone();
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

                    let mut buf: Vec<las::Point> = Vec::with_capacity(opts.batch_size);

                    for p in points {
                        let las_point = p?.to_las_point(&meta);

                        buf.push(las_point);

                        if buf.len() == opts.batch_size {
                            let full = std::mem::take(&mut buf);
                            tx_msg
                                .send((i, full))
                                .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                            buf.reserve(opts.batch_size);
                        }
                    }

                    if !buf.is_empty() {
                        tx_msg
                            .send((i, buf))
                            .map_err(|_| Error::Internal("writer thread dropped".into()))?;
                    }
                }

                Ok(())
            }));
        }

        drop(tx_msg);

        join_all_workers(readers_handles, writer_handle)
    }

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
}

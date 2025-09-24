mod builder;
mod options;

use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, mpsc},
    thread,
};

use crate::{
    Error, MakeReader, MakeWriter, Result, TryIntoReader, TryIntoWriter, e57::E57PointExt,
    las::LasHeaderExt,
};

pub use builder::ConverterBuilder;
pub use options::{ConversionOptions, ParallelOptions};

pub type ConversionResult = Result<()>;

pub struct Sequential;
pub struct Parallel;

pub struct Merged;
pub struct Split;

#[derive(Debug, Clone)]
pub struct Converter<T = Sequential, S = Merged> {
    opts: ConversionOptions,
    threading_opts: Option<ParallelOptions>,
    _threading: PhantomData<T>,
    _strategy: PhantomData<S>,
}

impl Converter {
    pub fn builder() -> ConverterBuilder {
        ConverterBuilder::default()
    }
}

impl Converter<Sequential, Merged> {
    pub fn convert<I, O>(&self, input: I, output: O) -> ConversionResult
    where
        I: TryIntoReader,
        O: TryIntoWriter,
    {
        let reader = input.try_into_reader()?;
        let mut e57_reader = e57::E57Reader::new(reader)?;

        let pointclouds = e57_reader.pointclouds();
        let header = las::Header::from_pointclouds(&pointclouds, &self.opts)?;
        let point_format = *header.point_format();

        let writer = output.try_into_writer()?;
        let mut writer = las::Writer::new(writer, header)?;

        for pointcloud in pointclouds {
            let points = e57_reader.pointcloud_simple(&pointcloud)?;

            for p in points {
                let p = p?;

                let gps_time = point_format.has_gps_time.then_some(
                    pointcloud
                        .acquisition_start
                        .as_ref()
                        .map(|d| d.gps_time)
                        .unwrap_or(0.0),
                );

                let mut las_point = p.to_las_point(point_format);

                las_point.gps_time = gps_time;

                writer.write_point(las_point)?;
            }
        }

        Ok(())
    }
}

impl Converter<Sequential, Split> {
    pub fn convert<I, O>(&self, input: I, output: O) -> ConversionResult
    where
        I: TryIntoReader,
        O: MakeWriter,
    {
        let reader = input.try_into_reader()?;
        let mut e57_reader = e57::E57Reader::new(reader)?;

        let pointclouds = e57_reader.pointclouds();

        for (i, pointcloud) in pointclouds.iter().enumerate() {
            let header = las::Header::from_pointcloud(pointcloud, &self.opts)?;
            let point_format = *header.point_format();

            let pointcloud_id = pointcloud.name.as_ref();
            let writer = output.make_writer(pointcloud_id.unwrap_or(&i.to_string()))?;
            let mut writer = las::Writer::new(writer, header)?;

            let points = e57_reader.pointcloud_simple(pointcloud)?;

            for p in points {
                let p = p?;

                let gps_time = point_format.has_gps_time.then_some(
                    pointcloud
                        .acquisition_start
                        .as_ref()
                        .map(|d| d.gps_time)
                        .unwrap_or(0.0),
                );

                let mut las_point = p.to_las_point(point_format);

                las_point.gps_time = gps_time;

                writer.write_point(las_point)?;
            }
        }

        Ok(())
    }
}

impl Converter<Parallel, Merged> {
    const CHUNK_SIZE: usize = 200_000;
    const QUEUE: usize = 8;

    pub fn convert<I, O>(&self, input: I, output: O) -> ConversionResult
    where
        I: MakeReader,
        O: TryIntoWriter,
    {
        let reader = input.clone().make_reader()?;
        let e57_reader = e57::E57Reader::new(reader)?;

        let pointclouds = e57_reader.pointclouds();
        let header = las::Header::from_pointclouds(&pointclouds, &self.opts)?;
        let point_format = *header.point_format();

        let (tx_fill, rx_fill) = mpsc::sync_channel::<Vec<las::Point>>(Self::QUEUE);

        let writer = output.try_into_writer()?;
        let writer_handle = thread::spawn(move || -> Result<()> {
            let mut w = las::Writer::new(writer, header)?;

            while let Ok(buf) = rx_fill.recv() {
                for p in buf {
                    w.write_point(p)?;
                }
            }

            Ok(())
        });

        let (tx_jobs, rx_jobs) = mpsc::channel::<usize>();
        for i in 0..pointclouds.len() {
            tx_jobs
                .send(i)
                .expect("job sender should not fail - receiver still alive");
        }
        drop(tx_jobs);

        let rx_jobs = Arc::new(Mutex::new(rx_jobs));

        let ParallelOptions { workers } = self.threading_opts.clone().unwrap_or_default();

        let max_workers = (pointclouds.len().min(workers)).max(1);

        let mut workers = Vec::with_capacity(max_workers);

        for _ in 0..max_workers {
            let rx_jobs = Arc::clone(&rx_jobs);
            let tx_fill = tx_fill.clone();
            let rf = input.clone();

            workers.push(thread::spawn(move || -> Result<()> {
                let reader = rf.make_reader()?;
                let mut e57_reader = e57::E57Reader::new(reader)?;
                let pointclouds = e57_reader.pointclouds();
                loop {
                    let idx = match rx_jobs
                        .lock()
                        .expect("job queue mutex poisoned - another worker panicked")
                        .recv()
                    {
                        Ok(i) => i,
                        Err(_) => return Ok(()),
                    };

                    let pc = &pointclouds[idx];

                    let gps_time = point_format.has_gps_time.then_some(
                        pc.acquisition_start
                            .as_ref()
                            .map(|d| d.gps_time)
                            .unwrap_or(0.0),
                    );

                    let points = e57_reader.pointcloud_simple(pc)?;
                    let mut buf: Vec<las::Point> = Vec::with_capacity(Self::CHUNK_SIZE);
                    for p in points {
                        let p = p?;

                        let mut las_point = p.to_las_point(point_format);

                        las_point.gps_time = gps_time;

                        buf.push(las_point);

                        if buf.len() == Self::CHUNK_SIZE {
                            tx_fill.send(buf).expect(
                                "filled-chunk sender should not fail - receiver still alive",
                            );
                            buf = Vec::with_capacity(Self::CHUNK_SIZE);
                        }
                    }
                    if !buf.is_empty() {
                        tx_fill
                            .send(buf)
                            .expect("filled-chunk sender should not fail - receiver still alive");
                    }
                }
            }));
        }

        drop(tx_fill);

        for worker in workers {
            match worker.join() {
                Ok(res) => res?,
                Err(_) => return Err(Error::Internal("Worker thread panicked".into())),
            }
        }

        match writer_handle.join() {
            Ok(res) => res?,
            Err(_) => return Err(Error::Internal("Writer thread panicked".into())),
        }

        Ok(())
    }
}

impl Converter<Parallel, Split> {
    const CHUNK_SIZE: usize = 200_000;
    const QUEUE: usize = 8;

    pub fn convert<I, O>(&self, input: I, output: O) -> ConversionResult
    where
        I: MakeReader,
        O: MakeWriter,
    {
        todo!()
    }
}

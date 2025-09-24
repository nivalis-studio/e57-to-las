use crate::e57::E57PointExt;
use crate::las::LasHeaderExt;
use crate::{ConversionOptions, Error, MakeWriter};
use crate::{MakeReader, TryIntoReader, TryIntoWriter};
use crate::{Result, las::Scale};
use std::sync::{Arc, Mutex};
use std::{sync::mpsc, thread};

pub type ConversionResult = Result<()>;

#[derive(Debug, Default, Clone)]
pub struct ConverterBuilder {
    opts: Option<ConversionOptions>,
    #[cfg(feature = "parallel")]
    workers: Option<usize>,
}

pub struct Converter {
    opts: ConversionOptions,
    #[cfg(feature = "parallel")]
    workers: usize,
}

impl ConverterBuilder {
    pub fn new() -> Self {
        ConverterBuilder::default()
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.opts.get_or_insert_default().scale = scale;
        self
    }

    pub fn las_version(mut self, las_version: (u8, u8)) -> Self {
        self.opts.get_or_insert_default().las_version = las_version;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn workers(mut self, n: usize) -> Self {
        self.workers = Some(n);
        self
    }

    pub fn build(self) -> Converter {
        Converter {
            opts: self.opts.unwrap_or_default(),
            #[cfg(feature = "parallel")]
            workers: self.workers.unwrap_or(num_cpus::get()),
        }
    }
}

impl Converter {
    #[cfg(feature = "parallel")]
    const CHUNK_SIZE: usize = 200_000;
    const QUEUE: usize = 8;

    pub fn builder() -> ConverterBuilder {
        ConverterBuilder::new()
    }

    #[cfg(not(feature = "parallel"))]
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

    #[cfg(not(feature = "parallel"))]
    pub fn convert_split_pointclouds<I, O>(&self, input: I, output: O) -> ConversionResult
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

    #[cfg(feature = "parallel")]
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

        let max_workers = (pointclouds.len().min(self.workers)).max(1);

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
                        use crate::e57::E57PointExt;

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
            use crate::Error;

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

    pub fn convert_split_pointclouds<I, O>(&self, input: I, output: O) -> ConversionResult
    where
        I: MakeReader,
        O: MakeWriter,
    {
        todo!()
    }
}

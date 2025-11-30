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
pub use options::{ConvertOptions, LasVersion, Scale};

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

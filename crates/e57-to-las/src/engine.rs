//! Runs a conversion: plans the outputs, schedules the point clouds on the
//! thread pool, streams encoded points to the LAS writers and settles each
//! output.
//!
//! # Scheduling
//!
//! The unit of work is a point cloud (or, with a stable order, all the point
//! clouds of one output). Each worker thread keeps its own E57 reader, so the
//! file metadata is parsed once per thread. Workers decode, convert and encode
//! points into raw LAS records in batches, then append each batch to the
//! output's LAS writer under a lock: only byte copies (or LAZ compression)
//! happen while the lock is held.
//!
//! # Scale and offset
//!
//! When every point cloud of an output has metadata bounds, the offset is
//! their middle; otherwise it is the first point written. If a point then does
//! not fit (more than ~2,147 km away at 1 mm), the output is discarded and
//! written again after a pass that measures the exact bounds.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use crate::cloud::Bounds;
use crate::header::{ascii_prefix, generating_software, looks_like_wkt};
use crate::input::E57Reader;
use crate::layout::{Layout, TimeEncoding};
use crate::output::{Format, Output, Target};
use crate::precision::{DEFAULT_SCALE, Transforms};
use crate::{
    CloudInfo, CloudSummary, Converter, Crs, Error, Event, GroupKey, HeaderOptions, OutputSummary,
    Precision, Result, Scan, Summary, Warning, decode,
};

/// Points per batch handed to a LAS writer.
const BATCH_POINTS: usize = 65_536;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Every critical section leaves the data consistent, poisoned or not.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn run<O: Output>(converter: &Converter, scan: &Scan, output: &O) -> Result<Summary> {
    Engine::new(converter, scan, output)?.run()
}

/// How the scale and offset of an output are decided.
#[derive(Debug, Clone, Copy)]
enum PlanPrecision {
    /// Given by the caller; points that do not fit are an error.
    Fixed(Transforms),
    /// Chosen here: from metadata bounds when known, else from the first
    /// point; measured in a second pass when a point does not fit.
    Auto {
        scale: f64,
        from_metadata: Option<Transforms>,
    },
}

impl PlanPrecision {
    fn initial(&self) -> Option<Transforms> {
        match self {
            PlanPrecision::Fixed(t) => Some(*t),
            PlanPrecision::Auto { from_metadata, .. } => *from_metadata,
        }
    }

    fn scale(&self) -> f64 {
        match self {
            PlanPrecision::Fixed(t) => t.scale[0],
            PlanPrecision::Auto { scale, .. } => *scale,
        }
    }
}

/// Everything decided about an output before reading points.
struct Plan {
    key: GroupKey,
    clouds: Vec<CloudInfo>,
    location: Option<std::path::PathBuf>,
    format: Format,
    layout: Layout,
    header: HeaderOptions,
    guid: uuid::Uuid,
    gps_time_type: las::GpsTimeType,
    precision: PlanPrecision,
}

/// Per-cloud counters of a running output.
#[derive(Debug, Default)]
struct Counters {
    written: AtomicU64,
    skipped: AtomicU64,
    filtered: AtomicU64,
}

enum Sink<W: std::io::Write + std::io::Seek + Send + Sync + 'static> {
    /// No writer yet: created at the first batch.
    Pending,
    Open(las::Writer<W>),
    Finished(OutputSummary),
    /// Aborted, or abandoned for a restart.
    Closed,
}

/// The state of one output being written.
struct GroupRun<W: std::io::Write + std::io::Seek + Send + Sync + 'static> {
    transforms: OnceLock<Transforms>,
    sink: Mutex<Sink<W>>,
    remaining: AtomicUsize,
    /// A point did not fit the transforms: the output must be written again.
    restart: AtomicBool,
    counters: Vec<Counters>,
}

impl<W: std::io::Write + std::io::Seek + Send + Sync + 'static> GroupRun<W> {
    fn new(plan: &Plan, transforms: Option<Transforms>) -> Self {
        let cell = OnceLock::new();
        if let Some(t) = transforms {
            let _ = cell.set(t);
        }
        GroupRun {
            transforms: cell,
            sink: Mutex::new(Sink::Pending),
            remaining: AtomicUsize::new(plan.clouds.len()),
            restart: AtomicBool::new(false),
            counters: plan.clouds.iter().map(|_| Counters::default()).collect(),
        }
    }

    fn take_summary(&self) -> Option<OutputSummary> {
        match std::mem::replace(&mut *lock(&self.sink), Sink::Closed) {
            Sink::Finished(summary) => Some(summary),
            _ => None,
        }
    }
}

/// A unit of work: one point cloud of an output, or every cloud of an output
/// in order (stable order). Outputs are positions in the `runs` slice.
#[derive(Debug, Clone, Copy)]
enum Job {
    Cloud { run: usize, cloud: usize },
    Output { run: usize },
}

struct Engine<'a, O: Output> {
    converter: &'a Converter,
    scan: &'a Scan,
    output: &'a O,
    input_stem: Option<String>,
    selected: Vec<CloudInfo>,
    plans: Vec<Plan>,
    wkt: Option<String>,
    /// Whether jobs run on a thread pool (feature `parallel`, more than one thread).
    #[cfg_attr(not(feature = "parallel"), allow(dead_code))]
    parallel: bool,
    warnings: Mutex<Vec<Warning>>,
    /// Set when a job failed: the others stop at their next batch.
    stop: AtomicBool,
    points_done: AtomicU64,
    points_total: AtomicU64,
}

impl<'a, O: Output> Engine<'a, O> {
    fn new(converter: &'a Converter, scan: &'a Scan, output: &'a O) -> Result<Self> {
        converter.precision.validate()?;
        let input_stem = scan.input().stem();
        let selected: Vec<CloudInfo> = scan
            .clouds()
            .iter()
            .filter(|cloud| converter.select.as_ref().is_none_or(|select| select(cloud)))
            .cloned()
            .collect();

        let mut groups: Vec<(GroupKey, Vec<CloudInfo>)> = Vec::new();
        let mut positions: HashMap<GroupKey, usize> = HashMap::new();
        for cloud in &selected {
            let key = converter.grouping.key(cloud);
            let position = *positions.entry(key.clone()).or_insert_with(|| {
                groups.push((key, Vec::new()));
                groups.len() - 1
            });
            groups[position].1.push(cloud.clone());
        }

        let mut warnings = Vec::new();
        let wkt = crs_wkt(converter, scan, &mut warnings)?;
        let plans = groups
            .into_iter()
            .map(|(key, clouds)| {
                plan(
                    converter,
                    scan,
                    output,
                    input_stem.as_deref(),
                    wkt.as_deref(),
                    key,
                    clouds,
                    &mut warnings,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let mut seen: HashMap<&std::path::Path, &GroupKey> = HashMap::new();
        for plan in &plans {
            if let Some(location) = &plan.location
                && let Some(first) = seen.insert(location, &plan.key)
            {
                return Err(Error::OutputCollision {
                    location: location.clone(),
                    first: first.clone(),
                    second: plan.key.clone(),
                });
            }
        }

        Ok(Engine {
            converter,
            scan,
            output,
            input_stem,
            selected,
            plans,
            wkt,
            parallel: cfg!(feature = "parallel") && converter.threads != 1,
            warnings: Mutex::new(warnings),
            stop: AtomicBool::new(false),
            points_done: AtomicU64::new(0),
            points_total: AtomicU64::new(0),
        })
    }

    fn emit(&self, event: Event<'_>) {
        if let Some(observer) = &self.converter.observer {
            observer(&event);
        }
    }

    fn warn(&self, warning: Warning) {
        self.emit(Event::Warning(&warning));
        lock(&self.warnings).push(warning);
    }

    fn progress(&self, points: u64) {
        if points == 0 {
            return;
        }
        let done = self.points_done.fetch_add(points, Relaxed) + points;
        self.emit(Event::Progress {
            points_done: done,
            points_total: self.points_total.load(Relaxed),
        });
    }

    /// `Err(Cancelled)` if cancelled; `true` if the current job should stop.
    fn halt(&self, restart: &AtomicBool) -> Result<bool> {
        if self
            .converter
            .cancel
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            return Err(Error::Cancelled);
        }
        Ok(self.stop.load(Relaxed) || restart.load(Relaxed))
    }

    fn target<'p>(&'p self, plan: &'p Plan) -> Target<'p> {
        Target {
            key: &plan.key,
            clouds: &plan.clouds,
            format: plan.format,
            input_stem: self.input_stem.as_deref(),
        }
    }

    fn write_error(&self, plan: &Plan, source: impl Into<crate::error::BoxError>) -> Error {
        Error::Write {
            output: plan
                .location
                .as_ref()
                .map_or_else(|| plan.key.to_string(), |p| p.display().to_string()),
            source: source.into(),
        }
    }

    fn run(self) -> Result<Summary> {
        let total = self.selected.iter().map(|c| c.point_count).sum();
        self.points_total.store(total, Relaxed);
        self.emit(Event::Started {
            clouds: &self.selected,
            outputs: self.plans.len(),
            points_total: total,
        });
        for warning in lock(&self.warnings).iter() {
            self.emit(Event::Warning(warning));
        }

        let outputs = self.with_pool(|| self.write_all())??;
        Ok(Summary {
            outputs,
            warnings: self
                .warnings
                .into_inner()
                .unwrap_or_else(PoisonError::into_inner),
        })
    }

    fn with_pool<T: Send>(&self, work: impl FnOnce() -> T + Send) -> Result<T> {
        #[cfg(feature = "parallel")]
        if self.parallel {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(self.converter.threads)
                .thread_name(|i| format!("e57-to-las-{i}"))
                .build()
                .map_err(|err| Error::InvalidOption(format!("cannot start threads: {err}")))?;
            return Ok(pool.install(work));
        }
        Ok(work())
    }

    fn write_all(&self) -> Result<Vec<OutputSummary>> {
        let all: Vec<usize> = (0..self.plans.len()).collect();
        let runs: Vec<GroupRun<O::Writer>> = self
            .plans
            .iter()
            .map(|plan| GroupRun::new(plan, plan.precision.initial()))
            .collect();
        self.write_groups(&all, &runs)?;

        let mut summaries: Vec<Option<OutputSummary>> =
            runs.iter().map(GroupRun::take_summary).collect();
        let restart: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&g| runs[g].restart.load(Relaxed))
            .collect();
        if !restart.is_empty() {
            for &g in &restart {
                self.emit(Event::OutputRestarted {
                    output: &self.plans[g].key,
                });
                let points: u64 = self.plans[g].clouds.iter().map(|c| c.point_count).sum();
                // Measured, then written again.
                self.points_total.fetch_add(2 * points, Relaxed);
            }
            let bounds = self.measure(&restart)?;
            let reruns: Vec<GroupRun<O::Writer>> = restart
                .iter()
                .zip(&bounds)
                .map(|(&g, bounds)| {
                    let plan = &self.plans[g];
                    GroupRun::new(
                        plan,
                        Some(Transforms::fitting(bounds, plan.precision.scale())),
                    )
                })
                .collect();
            self.write_groups(&restart, &reruns)?;
            for (&g, run) in restart.iter().zip(&reruns) {
                if run.restart.load(Relaxed) {
                    // Only possible if `map_points` moved points differently in the second pass.
                    return Err(Error::InvalidOption(
                        "map_points must give the same result each time it sees a point".into(),
                    ));
                }
                summaries[g] = run.take_summary();
            }
        }
        Ok(summaries.into_iter().flatten().collect())
    }

    /// Writes the outputs `groups` (plan indices), `runs` holding their state
    /// in the same order. On failure, aborts every output not yet finished.
    fn write_groups(&self, groups: &[usize], runs: &[GroupRun<O::Writer>]) -> Result<()> {
        let jobs: Vec<Job> = if self.converter.stable_order {
            (0..groups.len()).map(|run| Job::Output { run }).collect()
        } else {
            let mut jobs: Vec<(usize, usize)> = groups
                .iter()
                .enumerate()
                .flat_map(|(run, &g)| {
                    (0..self.plans[g].clouds.len()).map(move |cloud| (run, cloud))
                })
                .collect();
            // Largest clouds first, so a big one does not start last.
            jobs.sort_by_key(|&(run, cloud)| {
                Reverse(self.plans[groups[run]].clouds[cloud].point_count)
            });
            jobs.into_iter()
                .map(|(run, cloud)| Job::Cloud { run, cloud })
                .collect()
        };

        let result = self.for_each_job(&jobs, |job, reader| match *job {
            Job::Cloud { run, cloud } => self.write_cloud(groups[run], &runs[run], cloud, reader),
            Job::Output { run } => (0..self.plans[groups[run]].clouds.len())
                .try_for_each(|cloud| self.write_cloud(groups[run], &runs[run], cloud, reader)),
        });
        if result.is_err() {
            for (&g, run) in groups.iter().zip(runs) {
                self.abort(&self.plans[g], run);
            }
        }
        result
    }

    /// Runs `work` for each job, on the pool when parallel. Each thread opens
    /// its own E57 reader on first use. Stops at the first error.
    fn for_each_job<J: Sync>(
        &self,
        jobs: &[J],
        work: impl Fn(&J, &mut E57Reader) -> Result<()> + Sync,
    ) -> Result<()> {
        let run_one = |job: &J, reader: &mut E57Reader| -> Result<()> {
            if self.stop.load(Relaxed) {
                return Ok(());
            }
            work(job, reader).inspect_err(|_| self.stop.store(true, Relaxed))
        };

        #[cfg(feature = "parallel")]
        if self.parallel {
            use rayon::prelude::*;
            let slots: Vec<Mutex<Option<E57Reader>>> = (0..=rayon::current_num_threads())
                .map(|_| Mutex::new(None))
                .collect();
            return jobs.par_iter().with_max_len(1).try_for_each(|job| {
                let slot = rayon::current_thread_index()
                    .map_or(slots.len() - 1, |i| i.min(slots.len() - 1));
                let mut slot = lock(&slots[slot]);
                let reader = match &mut *slot {
                    Some(reader) => reader,
                    empty => empty.insert(self.scan.input.open()?),
                };
                run_one(job, reader)
            });
        }

        let mut reader: Option<E57Reader> = None;
        for job in jobs {
            if self.stop.load(Relaxed) {
                break;
            }
            let reader = match &mut reader {
                Some(reader) => reader,
                empty => empty.insert(self.scan.input.open()?),
            };
            run_one(job, reader)?;
        }
        Ok(())
    }

    /// Reads point cloud `cloud` of output `g` and writes its points.
    fn write_cloud(
        &self,
        g: usize,
        run: &GroupRun<O::Writer>,
        cloud: usize,
        reader: &mut E57Reader,
    ) -> Result<()> {
        let plan = &self.plans[g];
        let info = &plan.clouds[cloud];
        let pc = &self.scan.pointclouds[info.index];
        if self.halt(&run.restart)? {
            return Ok(());
        }
        self.emit(Event::CloudStarted { cloud: info });

        let batch_bytes = BATCH_POINTS * plan.layout.record_len();
        let mut batch = Vec::with_capacity(batch_bytes);
        let (mut read, mut written, mut filtered) = (0u64, 0u64, 0u64);
        let mut halted = false;
        let stats =
            decode::for_each_point(reader, pc, info, &self.converter.attributes, |mut point| {
                read += 1;
                if let Some(map) = &self.converter.map_points
                    && !map(&mut point)
                {
                    filtered += 1;
                    return Ok(ControlFlow::Continue(()));
                }
                let transforms = run
                    .transforms
                    .get_or_init(|| Transforms::around(point.xyz(), plan.precision.scale()));
                if !plan.layout.encode(&point, transforms, &mut batch) {
                    if let PlanPrecision::Fixed(_) = plan.precision {
                        return Err(Error::PointOutOfRange {
                            cloud: info.index,
                            x: point.x,
                            y: point.y,
                            z: point.z,
                        });
                    }
                    run.restart.store(true, Relaxed);
                    halted = true;
                    return Ok(ControlFlow::Break(()));
                }
                written += 1;
                if batch.len() >= batch_bytes {
                    if self.halt(&run.restart)? {
                        halted = true;
                        return Ok(ControlFlow::Break(()));
                    }
                    let full = std::mem::replace(&mut batch, Vec::with_capacity(batch_bytes));
                    self.write_batch(plan, run, full)?;
                    self.progress(std::mem::take(&mut read));
                }
                Ok(ControlFlow::Continue(()))
            })?;

        if halted || self.halt(&run.restart)? {
            return Ok(());
        }
        if !batch.is_empty() {
            self.write_batch(plan, run, batch)?;
        }
        self.progress(read + stats.skipped);
        let counters = &run.counters[cloud];
        counters.written.store(written, Relaxed);
        counters.skipped.store(stats.skipped, Relaxed);
        counters.filtered.store(filtered, Relaxed);
        if stats.returns_clamped > 0 {
            self.warn(Warning::ReturnsClamped {
                cloud: info.index,
                count: stats.returns_clamped,
            });
        }
        self.emit(Event::CloudFinished {
            cloud: info,
            points_written: written,
            points_skipped: stats.skipped,
        });

        if run.remaining.fetch_sub(1, Relaxed) == 1 {
            self.finish(plan, run)?;
        }
        Ok(())
    }

    fn open_writer(
        &self,
        plan: &Plan,
        run: &GroupRun<O::Writer>,
    ) -> Result<las::Writer<O::Writer>> {
        let transforms = *run
            .transforms
            .get_or_init(|| Transforms::around([0.0; 3], plan.precision.scale()));
        let header = self
            .build_header(plan, &transforms)
            .map_err(|err| self.write_error(plan, err))?;
        let target = self.target(plan);
        let writer = self
            .output
            .create(&target)
            .map_err(|err| self.write_error(plan, err))?;
        las::Writer::new(writer, header).map_err(|err| {
            self.output.abort(&target);
            self.write_error(plan, err)
        })
    }

    fn build_header(&self, plan: &Plan, transforms: &Transforms) -> las::Result<las::Header> {
        let mut builder = las::Builder::from(self.converter.las_version.to_las());
        builder.point_format = plan.layout.las_format(plan.format == Format::Laz);
        builder.transforms = transforms.to_las();
        builder.guid = plan.guid;
        builder.system_identifier = plan.header.system_identifier.clone();
        builder.generating_software = plan.header.generating_software.clone();
        builder.gps_time_type = plan.gps_time_type;
        builder.vlrs = plan.header.vlrs.iter().map(crate::Vlr::to_las).collect();
        builder.vlrs.extend(plan.layout.extra_bytes_vlr());
        let mut header = builder.into_header()?;
        if let Some(wkt) = &self.wkt {
            header.set_wkt_crs(wkt.as_bytes().to_vec())?;
        }
        Ok(header)
    }

    fn write_batch(&self, plan: &Plan, run: &GroupRun<O::Writer>, bytes: Vec<u8>) -> Result<()> {
        let mut sink = lock(&run.sink);
        if matches!(*sink, Sink::Pending) {
            *sink = Sink::Open(self.open_writer(plan, run)?);
        }
        let Sink::Open(writer) = &mut *sink else {
            return Ok(());
        };
        let header = writer.header();
        let points = las::PointDataBuilder::new()
            .with_format(*header.point_format())
            .with_transforms(*header.transforms())
            .build_from_bytes(bytes)
            .map_err(|err| self.write_error(plan, err))?;
        writer
            .write_points(&points)
            .map_err(|err| self.write_error(plan, err))
    }

    /// Completes the output once its last point cloud is written.
    fn finish(&self, plan: &Plan, run: &GroupRun<O::Writer>) -> Result<()> {
        if self.stop.load(Relaxed) {
            return Ok(());
        }
        if run.restart.load(Relaxed) {
            self.abort(plan, run);
            return Ok(());
        }
        let mut sink = lock(&run.sink);
        let mut writer = match std::mem::replace(&mut *sink, Sink::Closed) {
            Sink::Pending => self.open_writer(plan, run)?,
            Sink::Open(writer) => writer,
            other => {
                *sink = other;
                return Ok(());
            }
        };
        let target = self.target(plan);
        let transforms = *writer.header().transforms();
        if let Err(err) = writer.close() {
            forget_writer(writer);
            self.output.abort(&target);
            return Err(self.write_error(plan, err));
        }
        let inner = writer.into_inner().map_err(|err| {
            self.output.abort(&target);
            self.write_error(plan, err)
        })?;
        if let Err(err) = self.output.finish(&target, inner) {
            self.output.abort(&target);
            return Err(self.write_error(plan, err));
        }

        let scale = [transforms.x.scale, transforms.y.scale, transforms.z.scale];
        let summary = OutputSummary {
            key: plan.key.clone(),
            location: plan.location.clone(),
            format: plan.format,
            version: self.converter.las_version,
            point_format: plan.layout.format_number(),
            scale,
            offset: [
                transforms.x.offset,
                transforms.y.offset,
                transforms.z.offset,
            ],
            clouds: plan
                .clouds
                .iter()
                .zip(&run.counters)
                .map(|(cloud, counters)| CloudSummary {
                    index: cloud.index,
                    points_written: counters.written.load(Relaxed),
                    points_skipped: counters.skipped.load(Relaxed),
                    points_filtered: counters.filtered.load(Relaxed),
                })
                .collect(),
        };
        if let PlanPrecision::Auto {
            scale: requested, ..
        } = plan.precision
            && scale.iter().any(|s| *s > requested * (1.0 + 1e-9))
        {
            self.warn(Warning::CoarseScale {
                output: plan.key.clone(),
                requested,
                used: scale,
            });
        }
        self.emit(Event::OutputFinished { output: &summary });
        *sink = Sink::Finished(summary);
        Ok(())
    }

    /// Drops the writer of an unfinished output and tells the output.
    fn abort(&self, plan: &Plan, run: &GroupRun<O::Writer>) {
        let mut sink = lock(&run.sink);
        match std::mem::replace(&mut *sink, Sink::Closed) {
            Sink::Open(mut writer) => {
                if writer.close().is_err() {
                    forget_writer(writer);
                }
                self.output.abort(&self.target(plan));
            }
            Sink::Finished(summary) => *sink = Sink::Finished(summary),
            Sink::Pending | Sink::Closed => {}
        }
    }

    /// Reads the outputs `groups` without writing, to measure the bounds of
    /// their points (after `map_points`).
    fn measure(&self, groups: &[usize]) -> Result<Vec<Bounds>> {
        let bounds: Vec<Mutex<Bounds>> = groups.iter().map(|_| Mutex::new(Bounds::EMPTY)).collect();
        let jobs: Vec<(usize, usize)> = groups
            .iter()
            .enumerate()
            .flat_map(|(i, &g)| (0..self.plans[g].clouds.len()).map(move |cloud| (i, cloud)))
            .collect();
        let never = AtomicBool::new(false);
        self.for_each_job(&jobs, |&(i, cloud), reader| {
            let info = &self.plans[groups[i]].clouds[cloud];
            let mut local = Bounds::EMPTY;
            let mut read = 0u64;
            let stats = decode::for_each_point(
                reader,
                &self.scan.pointclouds[info.index],
                info,
                &self.converter.attributes,
                |mut point| {
                    read += 1;
                    if read.is_multiple_of(BATCH_POINTS as u64) {
                        if self.halt(&never)? {
                            return Ok(ControlFlow::Break(()));
                        }
                        self.progress(std::mem::take(&mut read));
                    }
                    if self
                        .converter
                        .map_points
                        .as_ref()
                        .is_none_or(|map| map(&mut point))
                    {
                        local.add(point.xyz());
                    }
                    Ok(ControlFlow::Continue(()))
                },
            )?;
            self.progress(read + stats.skipped);
            lock(&bounds[i]).merge(&local);
            Ok(())
        })?;
        Ok(bounds
            .into_iter()
            .map(|b| b.into_inner().unwrap_or_else(PoisonError::into_inner))
            .collect())
    }
}

/// Leaks a writer whose `close` failed.
///
/// `las::Writer`'s `Drop` closes it again and panics if that fails too, which
/// would hide the error already being reported.
fn forget_writer<W: std::io::Write + std::io::Seek + Send + Sync + 'static>(
    writer: las::Writer<W>,
) {
    std::mem::forget(writer);
}

/// The WKT to write, validated once for the whole conversion.
fn crs_wkt(
    converter: &Converter,
    scan: &Scan,
    warnings: &mut Vec<Warning>,
) -> Result<Option<String>> {
    let version = converter.las_version;
    Ok(match &converter.crs {
        Crs::None => None,
        Crs::Wkt(wkt) => {
            if !version.supports_wkt() {
                return Err(Error::CrsUnsupported { version });
            }
            Some(wkt.clone())
        }
        Crs::FromE57 => match scan.coordinate_metadata() {
            None => None,
            Some(metadata) if !looks_like_wkt(metadata) => {
                let shown: String = metadata.chars().take(60).collect();
                warnings.push(Warning::CrsNotWritten {
                    reason: format!("the E57 coordinate metadata is not WKT: {shown:?}"),
                });
                None
            }
            Some(_) if !version.supports_wkt() => {
                warnings.push(Warning::CrsNotWritten {
                    reason: format!(
                        "LAS {version} cannot store a WKT coordinate system, use LAS 1.4"
                    ),
                });
                None
            }
            Some(metadata) => Some(metadata.to_owned()),
        },
    })
}

/// Parses an E57 GUID, which may be braced or use `_` for `-`.
fn parse_guid(text: &str) -> Option<uuid::Uuid> {
    let text = text.trim();
    uuid::Uuid::parse_str(text)
        .or_else(|_| uuid::Uuid::parse_str(&text.replace('_', "-")))
        .ok()
}

#[allow(clippy::too_many_arguments)]
fn plan<O: Output>(
    converter: &Converter,
    scan: &Scan,
    output: &O,
    input_stem: Option<&str>,
    wkt: Option<&str>,
    key: GroupKey,
    clouds: Vec<CloudInfo>,
    warnings: &mut Vec<Warning>,
) -> Result<Plan> {
    let attributes = converter.attributes;
    let version = converter.las_version;

    let has_color = attributes.color && clouds.iter().any(|c| c.has_color);
    if has_color && !version.supports_color() {
        return Err(Error::ColorUnsupported { version });
    }

    let timed: Vec<&CloudInfo> = clouds
        .iter()
        .filter(|c| attributes.time && c.has_time)
        .collect();
    let absolute = timed.iter().all(|c| c.acquisition_start.is_some());
    for cloud in timed.iter().filter(|c| c.acquisition_start.is_none()) {
        warnings.push(Warning::RelativeTime { cloud: cloud.index });
    }
    let (time, gps_time_type) = if !absolute {
        (TimeEncoding::AsIs, las::GpsTimeType::Week)
    } else if version.supports_standard_gps_time() {
        (TimeEncoding::AdjustedStandard, las::GpsTimeType::Standard)
    } else {
        (TimeEncoding::WeekSeconds, las::GpsTimeType::Week)
    };
    if attributes.point_source_id {
        for cloud in clouds.iter().filter(|c| c.index > usize::from(u16::MAX)) {
            warnings.push(Warning::PointSourceIdClamped { cloud: cloud.index });
        }
    }
    let layout = Layout {
        has_time: !timed.is_empty(),
        has_color,
        normals: attributes.normals && clouds.iter().any(|c| c.has_normals),
        row_column: attributes.row_column && clouds.iter().any(|c| c.has_row_column),
        point_source_id: attributes.point_source_id,
        time,
    };

    let mut target = Target {
        key: &key,
        clouds: &clouds,
        format: converter.format.unwrap_or_default(),
        input_stem,
    };
    let location = output.location(&target);
    let format = converter.format.unwrap_or_else(|| {
        let laz = location
            .as_ref()
            .and_then(|p| p.extension())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("laz"));
        if laz { Format::Laz } else { Format::Las }
    });
    if format == Format::Laz && !cfg!(feature = "laz") {
        return Err(Error::LazNotEnabled);
    }
    target.format = format;

    let guid_text = if clouds.len() == 1 {
        clouds[0].guid.as_deref()
    } else {
        Some(scan.guid())
    };
    let guid = match guid_text {
        None => uuid::Uuid::nil(),
        Some(text) => parse_guid(text).unwrap_or_else(|| {
            warnings.push(Warning::InvalidGuid {
                output: key.clone(),
                guid: text.to_owned(),
            });
            uuid::Uuid::nil()
        }),
    };

    let first_model = clouds[0].sensor_model.as_deref();
    let system_identifier = match first_model {
        Some(model)
            if clouds
                .iter()
                .all(|c| c.sensor_model.as_deref() == Some(model)) =>
        {
            ascii_prefix(model, 32)
        }
        _ => "OTHER".to_owned(),
    };
    let mut header = HeaderOptions {
        system_identifier,
        generating_software: generating_software(),
        vlrs: Vec::new(),
    };
    if let Some(edit) = &converter.header {
        edit(&target, &mut header);
    }
    header.validate()?;
    if wkt.is_some() && header.vlrs.iter().any(|v| v.user_id == "LASF_Projection") {
        return Err(Error::InvalidOption(
            "a LASF_Projection VLR was added while a coordinate reference system is written: use Crs::None".into(),
        ));
    }

    let precision = match converter.precision {
        Precision::Fixed { scale, offset } => PlanPrecision::Fixed(Transforms { scale, offset }),
        other => {
            let scale = other.target_scale().unwrap_or(DEFAULT_SCALE);
            let from_metadata = clouds
                .iter()
                .map(|c| c.bounds)
                .try_fold(Bounds::EMPTY, |mut all, bounds| {
                    all.merge(&bounds?);
                    Some(all)
                })
                .map(|bounds| Transforms::fitting(&bounds, scale));
            PlanPrecision::Auto {
                scale,
                from_metadata,
            }
        }
    };

    Ok(Plan {
        key,
        location,
        format,
        layout,
        header,
        guid,
        gps_time_type,
        precision,
        clouds,
    })
}

//! Progress events and cancellation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{CloudInfo, GroupKey, OutputSummary, Warning};

/// Something that happened during a conversion, sent to the observer set with
/// [`Converter::observer`](crate::Converter::observer).
///
/// The observer is called from the threads doing the work, possibly several at
/// once, so it should return quickly.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Event<'a> {
    /// The conversion starts.
    Started {
        /// The selected point clouds.
        clouds: &'a [CloudInfo],
        /// Number of outputs that will be written.
        outputs: usize,
        /// Number of point records to read.
        points_total: u64,
    },
    /// A point cloud starts being read.
    CloudStarted {
        /// The point cloud.
        cloud: &'a CloudInfo,
    },
    /// More points were read. Sent after each batch of points.
    Progress {
        /// Point records read so far.
        points_done: u64,
        /// Point records to read in total. Grows when an output is restarted.
        points_total: u64,
    },
    /// A point cloud has been read and its points handed to its output.
    CloudFinished {
        /// The point cloud.
        cloud: &'a CloudInfo,
        /// Points written.
        points_written: u64,
        /// Points skipped for invalid coordinates.
        points_skipped: u64,
    },
    /// An output is complete.
    OutputFinished {
        /// What was written.
        output: &'a OutputSummary,
    },
    /// The points of an output span too far for the scale chosen from the
    /// first points; the output is discarded and written again in two passes.
    OutputRestarted {
        /// The output.
        output: &'a GroupKey,
    },
    /// A warning, also listed in the summary.
    Warning(&'a Warning),
}

/// Cancels a running conversion from another thread or task.
///
/// The conversion stops at the next batch of points, removes the outputs it
/// had started (through [`Output::abort`](crate::output::Output::abort)) and
/// returns [`Error::Cancelled`](crate::Error::Cancelled).
///
/// ```no_run
/// use e57_to_las::{CancelToken, Converter, output::FileOutput};
///
/// let token = CancelToken::new();
/// let handle = token.clone();
/// std::thread::spawn(move || {
///     std::thread::sleep(std::time::Duration::from_secs(5));
///     handle.cancel();
/// });
/// let result = Converter::new()
///     .cancel_token(token)
///     .convert("scan.e57", FileOutput::new("scan.las"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks every conversion using this token to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether [`cancel`](Self::cancel) was called.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

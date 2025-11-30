use std::sync::Arc;

use crate::convert::event::EventCallback;

pub use crate::ext::las::{LasVersion, Scale};

/// Type alias for a function that customizes the LAS header before finalization.
///
/// The header hook provides full control over the LAS header configuration. It receives
/// a mutable reference to the [`las::Builder`] after the library has configured it based
/// on the E57 data, allowing you to override or augment the default settings.
///
/// # Safety and Correctness
///
/// The hook can modify any aspect of the header, including values that were computed
/// by this library. Use with caution:
/// - Overwriting `point_format` may cause data loss if the new format lacks required fields
/// - Modifying `transforms` (scale/offset) may cause coordinate precision issues
/// - Changing bounds to be inconsistent with actual data will produce invalid outputs
///
/// # Examples
///
/// Adding a coordinate reference system:
///
/// ```
/// use e57_to_las::ConvertOptions;
/// use std::sync::Arc;
///
/// let opts = ConvertOptions {
///     header_hook: Some(Arc::new(|builder| {
///         // Add custom metadata or CRS information
///         builder.system_identifier = "Custom Scanner".into();
///     })),
///     ..Default::default()
/// };
/// ```
pub type HeaderHook = Arc<dyn Fn(&mut las::Builder) + Send + Sync + 'static>;

const DEFAULT_BATCH_SIZE: usize = 128_000;
const DEFAULT_QUEUE_SIZE: usize = 8;

/// Configuration options for E57 to LAS conversion.
///
/// This struct controls all aspects of the conversion process including output format,
/// coordinate precision, event callbacks, and parallel processing parameters.
#[derive(Clone)]
pub struct ConvertOptions {
    /// Coordinate scale factor for LAS output.
    ///
    /// Determines the precision of stored coordinates. The scale is automatically adjusted
    /// if necessary to fit the data within LAS coordinate limits (±2^31).
    ///
    /// # Scale Selection
    ///
    /// Choose based on:
    /// - **Data precision needs**: Smaller scales provide higher precision
    /// - **Coordinate system**: Use `Degree1e7` or `Degree1e8` for lat/lon data
    /// - **Data extent**: Larger extents may require larger scales to fit within LAS limits
    ///
    /// Default: [`Scale::MilliMeter`]
    pub scale: Scale,

    /// Target LAS file version.
    ///
    /// Different versions support different point formats and features:
    /// - 1.0-1.3: Support point formats 0-3 (no extended attributes)
    /// - 1.4: Supports point formats 0-10 including extended attributes
    ///
    /// The point format is automatically selected based on the E57 data attributes
    /// (color, timestamps) and the specified version.
    ///
    /// Default: 1.2
    pub las_version: LasVersion,

    /// Optional callback to customize the LAS header before finalization.
    ///
    /// This provides full control to modify any header settings after they've been
    /// computed from the E57 data. Common uses include adding coordinate reference
    /// system information, custom metadata, or variable length records.
    ///
    /// **Warning**: Incorrectly modifying computed values (transforms, bounds,
    /// point format) can produce invalid LAS outputs.
    ///
    /// Default: `None`
    pub header_hook: Option<HeaderHook>,

    /// Optional callback to receive progress events during conversion.
    ///
    /// The callback is invoked when point clouds start and finish processing. This is
    /// useful for progress reporting in GUI applications or long-running conversions.
    ///
    /// # Event Delivery and Threading
    ///
    /// Events are delivered asynchronously on a dedicated thread via an unbounded channel.
    /// This means:
    /// - **Callbacks can block** without impacting conversion performance - the conversion
    ///   continues in parallel while events are processed
    /// - **Long-running callbacks may delay function return** - the event handler thread
    ///   is joined in the handler's `Drop` implementation, so if a callback takes a long
    ///   time (seconds), the `convert()` function won't return until it completes
    /// - Callbacks must still be thread-safe (`Send + Sync`)
    ///
    /// Default: `None`
    pub on_event: Option<EventCallback>,

    /// Size of point batches for parallel processing (parallel feature only).
    ///
    /// Points are read and converted in batches of this size before being sent to
    /// the writer thread. Larger batches reduce synchronization overhead but increase
    /// memory usage.
    ///
    /// Recommended: 50,000 - 500,000 depending on available memory
    /// Default: 128,000
    #[cfg(feature = "parallel")]
    pub batch_size: usize,

    /// Size of the batch queue for parallel processing (parallel feature only).
    ///
    /// Controls how many batches can be queued between reader and writer threads.
    /// Larger queues improve throughput but increase memory usage.
    ///
    /// Default: 8
    #[cfg(feature = "parallel")]
    pub queue_size: usize,

    /// Number of worker threads for parallel processing (parallel feature only).
    ///
    /// Determines how many threads read and convert point clouds concurrently.
    /// Setting this higher than the number of point clouds has no benefit.
    ///
    /// Default: Number of CPU cores
    #[cfg(feature = "parallel")]
    pub workers: usize,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            scale: Default::default(),
            las_version: Default::default(),
            header_hook: None,
            on_event: None,

            #[cfg(feature = "parallel")]
            batch_size: DEFAULT_BATCH_SIZE,

            #[cfg(feature = "parallel")]
            queue_size: DEFAULT_QUEUE_SIZE,

            #[cfg(feature = "parallel")]
            workers: num_cpus::get(),
        }
    }
}

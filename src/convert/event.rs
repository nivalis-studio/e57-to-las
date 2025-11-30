use std::{sync::Arc, thread};

/// Events emitted during E57 to LAS conversion.
///
/// These events provide visibility into the conversion process, allowing applications
/// to track progress, display status, or implement custom logic based on point cloud
/// metadata.
///
/// # Event Delivery
///
/// Events are delivered asynchronously on a dedicated thread via the [`EventCallback`](crate::EventCallback)
/// provided in [`ConvertOptions`](crate::ConvertOptions).
#[non_exhaustive]
pub enum Event {
    /// A point cloud has started processing.
    ///
    /// This event is emitted before any points from the cloud are converted. It provides
    /// metadata about the point cloud including its transformation parameters from the E57 source.
    ///
    /// # Fields
    ///
    /// - `idx`: Zero-based index of the point cloud within the E57 source
    /// - `name`: Optional name from the E57 metadata
    /// - `description`: Optional description from the E57 metadata
    /// - `points_count`: Total number of points in this cloud
    /// - `translation`: (x, y, z) translation vector from E57 transform
    /// - `rotation`: (w, x, y, z) quaternion rotation from E57 transform
    ///
    /// Note: The transformations are already applied to point coordinates by the E57
    /// library before LAS conversion.
    #[non_exhaustive]
    PointCloudStarted {
        /// Zero-based index of this point cloud in the E57 source.
        idx: usize,
        /// Optional name of the point cloud from E57 metadata.
        name: Option<String>,
        /// Optional description of the point cloud from E57 metadata.
        description: Option<String>,
        /// Total number of points in this point cloud.
        points_count: u64,
        /// Translation component of the E57 transform (x, y, z).
        translation: (f64, f64, f64),
        /// Rotation component of the E57 transform as a quaternion (w, x, y, z).
        rotation: (f64, f64, f64, f64),
    },

    /// A point cloud has finished processing.
    ///
    /// This event is emitted after all points from the cloud have been converted and
    /// written to the LAS output.
    ///
    /// # Fields
    ///
    /// - `idx`: Zero-based index of the point cloud that finished processing
    PointCloudEnded {
        /// Zero-based index of the point cloud that finished.
        idx: usize,
    },
}

impl Event {
    pub(crate) fn pointcloud_started(idx: usize, pc: &e57::PointCloud) -> Self {
        Self::PointCloudStarted {
            idx,
            name: pc.name.clone(),
            description: pc.description.clone(),
            points_count: pc.records,
            translation: pc
                .transform
                .as_ref()
                .map(|t| (t.translation.x, t.translation.y, t.translation.z))
                .unwrap_or_default(),
            rotation: pc
                .transform
                .as_ref()
                .map(|t| (t.rotation.w, t.rotation.x, t.rotation.y, t.rotation.z))
                .unwrap_or_default(),
        }
    }

    pub(crate) fn pointcloud_ended(idx: usize) -> Self {
        Self::PointCloudEnded { idx }
    }
}

/// Type alias for event callback functions.
///
/// This is a thread-safe function that receives [`Event`]s during conversion.
/// The callback is invoked asynchronously on a dedicated thread, so it must be
/// `Send + Sync`.
///
/// Callbacks can safely block or perform long-running operations without impacting
/// conversion performance, though they may delay the return of the conversion function.
///
/// # Examples
///
/// ```
/// use e57_to_las::{Event, EventCallback};
/// use std::sync::Arc;
///
/// let callback: EventCallback = Arc::new(|event| {
///     match event {
///         Event::PointCloudStarted { idx, .. } => {
///             eprintln!("Processing point cloud {}", idx);
///         },
///         Event::PointCloudEnded { .. } => {},
///         _ => {}
///     }
/// });
/// ```
/// ## Threading Behavior
///
/// - Events are sent through an unbounded channel to a separate handler thread
/// - Callbacks execute on the handler thread, **not** the conversion thread
/// - **Callbacks can block** without impacting conversion performance - the conversion
///   continues in parallel while events are processed
/// - **Long-running callbacks may delay function return** - the event handler thread
///   is joined when the conversion completes, so if a callback takes a long time
///   (seconds), the `convert()` function won't return until it finishes
/// - Callbacks must be thread-safe (`Send + Sync`)
///
/// # Examples
///
/// ```
/// use e57_to_las::{Event, ConvertOptions};
/// use std::sync::Arc;
///
/// let opts = ConvertOptions {
///     on_event: Some(Arc::new(|event| {
///         match event {
///             Event::PointCloudStarted { idx, name, points_count, .. } => {
///                 println!("Starting cloud {}: {:?} with {} points", idx, name, points_count);
///             },
///             Event::PointCloudEnded { idx } => {
///                 println!("Finished cloud {}", idx);
///             },
///             _ => {}
///         }
///     })),
///     ..Default::default()
/// };
/// ```
pub type EventCallback = Arc<dyn Fn(Event) + Send + Sync + 'static>;

#[derive(Clone)]
pub(crate) struct EventSender {
    tx: flume::Sender<Event>,
}

impl EventSender {
    #[inline]
    pub(crate) fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

pub(crate) struct EventHandler {
    tx: flume::Sender<Event>,
    handle: Option<thread::JoinHandle<()>>,
}

impl EventHandler {
    pub(crate) fn new(callback: Option<&EventCallback>) -> Option<Self> {
        let callback = callback?.clone();
        let (tx, rx) = flume::unbounded();

        let handle = thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                callback(event);
            }
        });

        Some(Self {
            tx,
            handle: Some(handle),
        })
    }

    #[inline]
    pub(crate) fn sender(&self) -> EventSender {
        EventSender {
            tx: self.tx.clone(),
        }
    }

    #[inline]
    pub(crate) fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        drop(std::mem::replace(&mut self.tx, flume::unbounded().0));

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

use std::{sync::Arc, thread};

#[non_exhaustive]
pub enum Event {
    #[non_exhaustive]
    PointCloudStarted {
        idx: usize,
        name: Option<String>,
        description: Option<String>,
        points_count: u64,
        translation: (f64, f64, f64),
        rotation: (f64, f64, f64, f64),
    },
    PointCloudEnded {
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

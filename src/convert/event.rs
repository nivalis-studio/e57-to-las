use std::{sync::Arc, thread};

#[non_exhaustive]
pub enum Event {
    PointCloudStarted {
        idx: usize,
        pc: Box<e57::PointCloud>,
    },
    PointCloudEnded {
        idx: usize,
    },
}

impl Event {
    pub(crate) fn pointcloud_started(idx: usize, pc: &e57::PointCloud) -> Self {
        Self::PointCloudStarted {
            idx,
            pc: Box::new(pc.clone()),
        }
    }

    pub(crate) fn pointcloud_ended(idx: usize) -> Self {
        Self::PointCloudEnded { idx }
    }
}

pub type EventCallback = Arc<dyn Fn(Event) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct EventSender {
    tx: flume::Sender<Event>,
}

impl EventSender {
    #[inline]
    pub fn send(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

pub struct EventHandler {
    tx: flume::Sender<Event>,
    handle: Option<thread::JoinHandle<()>>,
}

impl EventHandler {
    pub fn new(callback: Option<&EventCallback>) -> Option<Self> {
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
    pub fn sender(&self) -> EventSender {
        EventSender {
            tx: self.tx.clone(),
        }
    }

    #[inline]
    pub fn send(&self, event: Event) {
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

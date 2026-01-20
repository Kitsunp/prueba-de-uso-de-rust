use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use visual_novel_engine::AssetId;

#[derive(Debug)]
pub struct LoadRequest {
    pub id: AssetId,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct LoadResult {
    pub id: AssetId,
    pub bytes: Vec<u8>,
}

pub struct AsyncLoader {
    sender: Sender<LoadRequest>,
    receiver: Receiver<LoadResult>,
    inflight: Arc<AtomicUsize>,
}

impl AsyncLoader {
    pub fn new() -> Self {
        let (sender, request_rx) = mpsc::channel::<LoadRequest>();
        let (result_tx, receiver) = mpsc::channel::<LoadResult>();
        let inflight = Arc::new(AtomicUsize::new(0));
        let inflight_thread = inflight.clone();
        thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                let data = std::fs::read(&request.path).unwrap_or_default();
                let _ = result_tx.send(LoadResult {
                    id: request.id,
                    bytes: data,
                });
                inflight_thread.fetch_sub(1, Ordering::Release);
            }
        });
        Self {
            sender,
            receiver,
            inflight,
        }
    }

    pub fn enqueue(&self, id: AssetId, path: PathBuf) {
        self.inflight.fetch_add(1, Ordering::Release);
        let _ = self.sender.send(LoadRequest { id, path });
    }

    pub fn try_recv(&self) -> Option<LoadResult> {
        self.receiver.try_recv().ok()
    }

    pub fn is_loading(&self) -> bool {
        self.inflight.load(Ordering::Acquire) > 0
    }
}

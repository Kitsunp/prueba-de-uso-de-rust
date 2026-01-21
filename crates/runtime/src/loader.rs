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
    _thread_handle: Option<thread::JoinHandle<()>>,
}

impl AsyncLoader {
    pub fn new() -> Self {
        let (sender, request_rx) = mpsc::channel::<LoadRequest>();
        let (result_tx, receiver) = mpsc::channel::<LoadResult>();
        let inflight = Arc::new(AtomicUsize::new(0));
        let inflight_thread = inflight.clone();

        let handle = thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                let data = std::fs::read(&request.path).unwrap_or_default();
                inflight_thread.fetch_sub(1, Ordering::Release);
                let _ = result_tx.send(LoadResult {
                    id: request.id,
                    bytes: data,
                });
            }
        });

        Self {
            sender,
            receiver,
            inflight,
            _thread_handle: Some(handle),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_loading_behavior() {
        // Engineer Manifesto: Air Gapped / Concurrency.
        // Ensure loading happens off-thread and doesn't block immediately.

        let loader = AsyncLoader::new();
        let id = AssetId::from_path("test_asset");
        let path = PathBuf::from("Cargo.toml"); // Use a file that exists

        assert!(!loader.is_loading());

        loader.enqueue(id, path);

        // Should register as loading
        assert!(loader.is_loading());

        // Wait for result (in real engine this happens per frame)
        let mut result = None;
        for _ in 0..100 {
            if let Some(res) = loader.try_recv() {
                result = Some(res);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let result = result.expect("Loader should complete");
        assert_eq!(result.id, id);
        assert!(!result.bytes.is_empty(), "Should load file content");
        assert!(!loader.is_loading(), "Should update inflight count");
    }
}

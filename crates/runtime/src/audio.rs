use std::io::Cursor;
use std::sync::Arc;

use rodio::{Decoder, OutputStream, Sink, Source};

use crate::AssetStore;

/// Audio trait for runtime playback backends.
pub trait Audio {
    fn play_music(&mut self, id: &str);
    fn stop_music(&mut self);
    fn play_sfx(&mut self, id: &str);
}

impl<T: Audio + ?Sized> Audio for Box<T> {
    fn play_music(&mut self, id: &str) {
        (**self).play_music(id);
    }
    fn stop_music(&mut self) {
        (**self).stop_music();
    }
    fn play_sfx(&mut self, id: &str) {
        (**self).play_sfx(id);
    }
}

/// Audio backend implementation using `rodio`.
///
/// This backend runs audio on a dedicated thread (managed by rodio's OutputStream).
/// It handles decoding and mixing of multiple audio sources.
pub struct RodioBackend {
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    bgm_sink: Sink,
    assets: Arc<dyn AssetStore + Send + Sync>,
}

impl RodioBackend {
    pub fn new(assets: Arc<dyn AssetStore + Send + Sync>) -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("failed to initialize audio output stream: {}", e))?;

        let bgm_sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("failed to create BGM sink: {}", e))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            bgm_sink,
            assets,
        })
    }

    fn play_source(&self, source: Box<dyn Source<Item = f32> + Send>, is_bgm: bool) {
        if is_bgm {
            if !self.bgm_sink.empty() {
                self.bgm_sink.stop();
                // Re-create sink to clear queue or just append new one?
                // For BGM, we typically want swap.
                // rodio's Sink is a queue. To swap, we clear it.
                // But Sink doesn't have clear(). We can just make a new Sink or use stop().
                // Calling stop() clears the queue.
            }
            // For looping BGM
            let source = source.repeat_infinite();
            self.bgm_sink.append(source);
            self.bgm_sink.play();
        } else {
            // SFX - fire and forget, fail-soft on sink creation errors.
            let sink = match Sink::try_new(&self.stream_handle) {
                Ok(sink) => sink,
                Err(e) => {
                    eprintln!("Failed to create SFX sink: {}", e);
                    return;
                }
            };
            sink.append(source);
            sink.detach(); // Let it play to completion
        }
    }
}

impl Audio for RodioBackend {
    fn play_music(&mut self, id: &str) {
        let data = match self.assets.load_bytes(id) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Audio Error: {}", e);
                return;
            }
        };

        let cursor = Cursor::new(data);
        match Decoder::new(cursor) {
            Ok(decoder) => {
                let source = decoder.convert_samples::<f32>();
                self.play_source(Box::new(source), true);
            }
            Err(e) => eprintln!("Failed to decode music '{}': {}", id, e),
        }
    }

    fn stop_music(&mut self) {
        self.bgm_sink.stop();
    }

    fn play_sfx(&mut self, id: &str) {
        let data = match self.assets.load_bytes(id) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Audio Error: {}", e);
                return;
            }
        };

        let cursor = Cursor::new(data);
        match Decoder::new(cursor) {
            Ok(decoder) => {
                let source = decoder.convert_samples::<f32>();
                self.play_source(Box::new(source), false);
            }
            Err(e) => eprintln!("Failed to decode sfx '{}': {}", id, e),
        }
    }
}

/// No-op audio backend for environments where sound output is disabled/unavailable.
#[derive(Default)]
pub struct SilentAudio;

impl Audio for SilentAudio {
    fn play_music(&mut self, _id: &str) {}

    fn stop_music(&mut self) {}

    fn play_sfx(&mut self, _id: &str) {}
}

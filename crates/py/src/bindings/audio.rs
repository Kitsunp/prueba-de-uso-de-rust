use pyo3::prelude::*;
use std::time::Duration;
use visual_novel_engine::{AssetId, AudioCommand};

use super::engine::PyEngine;

#[pyclass(name = "AudioController")]
pub struct PyAudio {
    engine: Py<PyEngine>,
}

impl PyAudio {
    pub fn new(_py: Python<'_>, engine: Py<PyEngine>) -> PyResult<Self> {
        Ok(Self { engine })
    }
}

#[pymethods]
impl PyAudio {
    #[pyo3(signature = (resource, r#loop=true, fade_in=0.0))]
    fn play_bgm(&self, py: Python<'_>, resource: &str, r#loop: bool, fade_in: f64) -> PyResult<()> {
        let mut engine = self.engine.borrow_mut(py);
        engine.inner.queue_audio_command(AudioCommand::PlayBgm {
            resource: AssetId::from_path(resource),
            r#loop,
            fade_in: Duration::from_secs_f64(fade_in.max(0.0)),
        });
        Ok(())
    }

    #[pyo3(signature = (fade_out=0.0))]
    fn stop_all(&self, py: Python<'_>, fade_out: f64) -> PyResult<()> {
        let mut engine = self.engine.borrow_mut(py);
        engine.inner.queue_audio_command(AudioCommand::StopBgm {
            fade_out: Duration::from_secs_f64(fade_out.max(0.0)),
        });
        Ok(())
    }

    fn play_sfx(&self, py: Python<'_>, resource: &str) -> PyResult<()> {
        let mut engine = self.engine.borrow_mut(py);
        engine.inner.queue_audio_command(AudioCommand::PlaySfx {
            resource: AssetId::from_path(resource),
        });
        Ok(())
    }
}

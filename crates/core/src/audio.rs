use std::time::Duration;

use crate::assets::AssetId;
use crate::event::SharedStr;

/// Audio commands emitted by the engine.
/// Each command includes both AssetId (for caching) and path (for playback).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioCommand {
    PlayBgm {
        resource: AssetId,
        path: SharedStr,
        r#loop: bool,
        fade_in: Duration,
    },
    StopBgm {
        fade_out: Duration,
    },
    PlaySfx {
        resource: AssetId,
        path: SharedStr,
    },
}

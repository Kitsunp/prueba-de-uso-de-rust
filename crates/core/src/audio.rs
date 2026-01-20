use std::time::Duration;

use crate::assets::AssetId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioCommand {
    PlayBgm {
        resource: AssetId,
        r#loop: bool,
        fade_in: Duration,
    },
    StopBgm {
        fade_out: Duration,
    },
    PlaySfx {
        resource: AssetId,
    },
}

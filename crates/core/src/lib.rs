mod assets;
mod audio;
mod engine;
mod error;
mod event;
mod render;
mod resource;
mod script;
mod security;
mod state;
mod storage;
mod trace;
mod ui;
mod version;
mod visual;

pub use assets::{AssetId, AssetManifest};
pub use audio::AudioCommand;
pub use engine::Engine;
pub use engine::StateChange;
pub use error::{VnError, VnResult};
pub use event::{
    CharacterPatchCompiled, CharacterPatchRaw, CharacterPlacementCompiled, CharacterPlacementRaw,
    ChoiceCompiled, ChoiceOptionCompiled, ChoiceOptionRaw, ChoiceRaw, CmpOp, CondCompiled, CondRaw,
    DialogueCompiled, DialogueRaw, EventCompiled, EventRaw, ScenePatchCompiled, ScenePatchRaw,
    SceneUpdateCompiled, SceneUpdateRaw, SharedStr,
};
pub use render::{RenderBackend, RenderOutput, TextRenderer};
pub use resource::{LruCache, ResourceLimiter};
pub use script::{ScriptCompiled, ScriptRaw};
pub use security::SecurityPolicy;
pub use state::EngineState;
pub use storage::{compute_script_id, SaveData, SaveError, ScriptId};
pub use trace::{StateDigest, UiTrace, UiTraceStep, UiView as TraceUiView, VisualDigest};
pub use ui::{UiState, UiView};
pub use version::{COMPILED_FORMAT_VERSION, SAVE_FORMAT_VERSION, SCRIPT_SCHEMA_VERSION};
pub use visual::VisualState;

pub type Event = EventCompiled;
pub type Script = ScriptRaw;

// Python bindings are now handled in the `vnengine_py` crate.
// Core remains agnostic to the language binding layer.

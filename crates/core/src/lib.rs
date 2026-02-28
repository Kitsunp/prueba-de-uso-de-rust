mod assets;
mod audio;
mod engine;
mod entity;
mod error;
mod event;
mod graph;
pub mod manifest;
mod render;
mod resource;
mod script;
mod security;
mod state;
mod storage;
mod timeline;
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
    AudioActionCompiled, AudioActionRaw, CharacterPatchCompiled, CharacterPatchRaw,
    CharacterPlacementCompiled, CharacterPlacementRaw, ChoiceCompiled, ChoiceOptionCompiled,
    ChoiceOptionRaw, ChoiceRaw, CmpOp, CondCompiled, CondRaw, DialogueCompiled, DialogueRaw,
    EventCompiled, EventRaw, ScenePatchCompiled, ScenePatchRaw, SceneTransitionCompiled,
    SceneTransitionRaw, SceneUpdateCompiled, SceneUpdateRaw, SetCharacterPositionCompiled,
    SetCharacterPositionRaw, SharedStr,
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

// Phase 1: Entity System exports
pub use entity::{
    AudioData, CharacterData, Entity, EntityId, EntityKind, ImageData, SceneState, TextData,
    Transform, VideoData, MAX_ENTITIES,
};

// Phase 2: Timeline System exports
pub use timeline::{
    Easing, Fixed, Keyframe, PropertyType, PropertyValue, Timeline, TimelineError, Track,
    MAX_KEYFRAMES_PER_TRACK, MAX_TRACKS,
};

// Phase 3: Story Graph exports
pub use graph::{EdgeType, GraphEdge, GraphNode, GraphStats, NodeType, StoryGraph};

pub type Event = EventCompiled;
pub type Script = ScriptRaw;

// Python bindings are now handled in the `vnengine_py` crate.
// Core remains agnostic to the language binding layer.

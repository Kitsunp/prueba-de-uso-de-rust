//! Rendering helpers for compiled events.

use crate::event::{EventCompiled, SceneUpdateCompiled};
use crate::visual::VisualState;

/// Renderer interface used by the engine.
pub trait RenderBackend {
    fn render(&self, event: &EventCompiled, visual: &VisualState) -> RenderOutput;
}

/// Rendered text output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderOutput {
    pub text: String,
}

/// Simple renderer that formats events as text.
#[derive(Clone, Debug, Default)]
pub struct TextRenderer;

impl TextRenderer {
    fn render_scene(&self, scene: &SceneUpdateCompiled, visual: &VisualState) -> String {
        let mut lines = Vec::new();
        if let Some(background) = scene.background.as_deref().or(visual.background.as_deref()) {
            lines.push(format!("Background: {background}"));
        }
        if let Some(music) = scene.music.as_deref().or(visual.music.as_deref()) {
            lines.push(format!("Music: {music}"));
        }
        if !visual.characters.is_empty() {
            let roster = visual
                .characters
                .iter()
                .map(|character| {
                    let mut descriptor = character.name.to_string();
                    if let Some(expression) = &character.expression {
                        descriptor.push_str(&format!(" ({expression})"));
                    }
                    if let Some(position) = &character.position {
                        descriptor.push_str(&format!(" @ {position}"));
                    }
                    descriptor
                })
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("Characters: {roster}"));
        }
        if lines.is_empty() {
            "Scene updated".to_string()
        } else {
            lines.join("\n")
        }
    }
}

impl RenderBackend for TextRenderer {
    fn render(&self, event: &EventCompiled, visual: &VisualState) -> RenderOutput {
        let text = match event {
            EventCompiled::Dialogue(dialogue) => {
                format!("{}: {}", dialogue.speaker, dialogue.text)
            }
            EventCompiled::Choice(choice) => {
                let options = choice
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| format!("{}. {}", idx + 1, option.text))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}", choice.prompt, options)
            }
            EventCompiled::Scene(scene) => self.render_scene(scene, visual),
            EventCompiled::Jump { target_ip } => format!("Jump to {target_ip}"),
            EventCompiled::SetFlag { flag_id, value } => {
                format!("Flag {flag_id} = {value}")
            }
        };
        RenderOutput { text }
    }
}

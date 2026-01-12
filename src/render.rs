use crate::event::{Event, SceneUpdate};
use crate::visual::VisualState;

pub trait RenderBackend {
    fn render(&self, event: &Event, visual: &VisualState) -> RenderOutput;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderOutput {
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct TextRenderer;

impl TextRenderer {
    fn render_scene(&self, scene: &SceneUpdate, visual: &VisualState) -> String {
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
                    let mut descriptor = character.name.clone();
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
    fn render(&self, event: &Event, visual: &VisualState) -> RenderOutput {
        let text = match event {
            Event::Dialogue(dialogue) => format!("{}: {}", dialogue.speaker, dialogue.text),
            Event::Choice(choice) => {
                let options = choice
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| format!("{}. {}", idx + 1, option.text))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}", choice.prompt, options)
            }
            Event::Scene(scene) => self.render_scene(scene, visual),
            Event::Jump { target } => format!("Jump to {target}"),
            Event::SetFlag { key, value } => format!("Flag {key} = {value}"),
        };
        RenderOutput { text }
    }
}

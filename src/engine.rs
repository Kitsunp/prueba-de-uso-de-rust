use crate::error::{VnError, VnResult};
use crate::event::Event;
use crate::render::{RenderBackend, RenderOutput};
use crate::resource::ResourceLimiter;
use crate::script::Script;
use crate::security::SecurityPolicy;
use crate::state::EngineState;

#[derive(Clone, Debug)]
pub struct Engine {
    script: Script,
    state: EngineState,
    policy: SecurityPolicy,
    limits: ResourceLimiter,
}

impl Engine {
    pub fn new(script: Script, policy: SecurityPolicy, limits: ResourceLimiter) -> VnResult<Self> {
        policy.validate(&script, limits)?;
        let position = script.start_index()?;
        let mut state = EngineState::new(position);
        if let Some(Event::Scene(scene)) = script.events.get(position) {
            state.visual.apply_scene(scene);
        }
        Ok(Self {
            script,
            state,
            policy,
            limits,
        })
    }

    pub fn current_event(&self) -> VnResult<Event> {
        if self.state.position >= self.script.events.len() {
            return Err(VnError::EndOfScript);
        }
        self.script
            .events
            .get(self.state.position)
            .cloned()
            .ok_or(VnError::EndOfScript)
    }

    pub fn step(&mut self) -> VnResult<Event> {
        let event = self.current_event()?;
        self.advance_from(&event)?;
        Ok(event)
    }

    pub fn choose(&mut self, option_index: usize) -> VnResult<Event> {
        let event = self.current_event()?;
        match &event {
            Event::Choice(choice) => {
                let option = choice
                    .options
                    .get(option_index)
                    .ok_or(VnError::InvalidChoice)?;
                self.jump_to_label(&option.target)?;
            }
            _ => return Err(VnError::InvalidChoice),
        }
        Ok(event)
    }

    fn advance_from(&mut self, event: &Event) -> VnResult<()> {
        match event {
            Event::Jump { target } => self.jump_to_label(target),
            Event::SetFlag { key, value } => {
                self.state.flags.insert(key.clone(), *value);
                self.advance_position()
            }
            Event::Scene(scene) => {
                self.state.visual.apply_scene(scene);
                self.advance_position()
            }
            Event::Choice(_) => Ok(()),
            Event::Dialogue(_) => self.advance_position(),
        }
    }

    fn advance_position(&mut self) -> VnResult<()> {
        if self.state.position + 1 >= self.script.events.len() {
            self.state.position = self.script.events.len();
            return Ok(());
        }
        self.state.position = self.state.position.saturating_add(1);
        Ok(())
    }

    fn jump_to_label(&mut self, label: &str) -> VnResult<()> {
        if label.len() > self.limits.max_label_length {
            return Err(VnError::ResourceLimit("jump label length".to_string()));
        }
        let position = self
            .script
            .labels
            .get(label)
            .copied()
            .ok_or_else(|| VnError::InvalidScript(format!("unknown label '{label}'")))?;
        self.state.position = position;
        Ok(())
    }

    pub fn state(&self) -> &EngineState {
        &self.state
    }

    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    pub fn visual_state(&self) -> &crate::visual::VisualState {
        &self.state.visual
    }

    pub fn render_current<R: RenderBackend>(&self, renderer: &R) -> VnResult<RenderOutput> {
        let event = self.current_event()?;
        Ok(renderer.render(&event, &self.state.visual))
    }

    pub fn current_event_json(&self) -> VnResult<String> {
        let event = self.current_event()?;
        Ok(event.to_json_string())
    }
}

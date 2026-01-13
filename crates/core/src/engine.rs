//! Runtime engine that executes compiled scripts.

use crate::error::{VnError, VnResult};
use crate::event::{CmpOp, CondCompiled, EventCompiled};
use crate::render::{RenderBackend, RenderOutput};
use crate::resource::ResourceLimiter;
use crate::script::{ScriptCompiled, ScriptRaw};
use crate::security::SecurityPolicy;
use crate::state::EngineState;

/// Execution engine for compiled scripts.
#[derive(Clone, Debug)]
pub struct Engine {
    script: ScriptCompiled,
    state: EngineState,
    policy: SecurityPolicy,
}

impl Engine {
    /// Builds an engine by validating and compiling a raw script.
    pub fn new(
        script: ScriptRaw,
        policy: SecurityPolicy,
        limits: ResourceLimiter,
    ) -> VnResult<Self> {
        policy.validate_raw(&script, limits)?;
        let script = script.compile()?;
        policy.validate_compiled(&script, limits)?;
        let position = script.start_ip;
        let mut state = EngineState::new(position, script.flag_count);
        if let Some(EventCompiled::Scene(scene)) = script.events.get(position as usize) {
            state.visual.apply_scene(scene);
        }
        Ok(Self {
            script,
            state,
            policy,
        })
    }

    /// Builds an engine directly from a compiled script.
    pub fn from_compiled(
        script: ScriptCompiled,
        policy: SecurityPolicy,
        limits: ResourceLimiter,
    ) -> VnResult<Self> {
        policy.validate_compiled(&script, limits)?;
        let position = script.start_ip;
        let mut state = EngineState::new(position, script.flag_count);
        if let Some(EventCompiled::Scene(scene)) = script.events.get(position as usize) {
            state.visual.apply_scene(scene);
        }
        Ok(Self {
            script,
            state,
            policy,
        })
    }

    /// Returns a reference to the compiled script.
    pub fn script(&self) -> &ScriptCompiled {
        &self.script
    }

    /// Returns a reference to the current compiled event.
    pub fn current_event_ref(&self) -> VnResult<&EventCompiled> {
        if self.state.position as usize >= self.script.events.len() {
            return Err(VnError::EndOfScript);
        }
        self.script
            .events
            .get(self.state.position as usize)
            .ok_or(VnError::EndOfScript)
    }

    /// Returns a clone of the current compiled event.
    pub fn current_event(&self) -> VnResult<EventCompiled> {
        self.current_event_ref().cloned()
    }

    /// Advances the engine by applying the current event.
    pub fn step(&mut self) -> VnResult<()> {
        let event = self.current_event()?;
        self.advance_from(&event)
    }

    /// Returns the current event and advances the engine.
    pub fn step_event(&mut self) -> VnResult<EventCompiled> {
        let event = self.current_event()?;
        self.advance_from(&event)?;
        Ok(event)
    }

    /// Applies a choice selection on the current choice event.
    pub fn choose(&mut self, option_index: usize) -> VnResult<EventCompiled> {
        let event = self.current_event()?;
        match &event {
            EventCompiled::Choice(choice) => {
                let option = choice
                    .options
                    .get(option_index)
                    .ok_or(VnError::InvalidChoice)?;
                self.jump_to_ip(option.target_ip)?;
            }
            _ => return Err(VnError::InvalidChoice),
        }
        Ok(event)
    }

    fn advance_from(&mut self, event: &EventCompiled) -> VnResult<()> {
        match event {
            EventCompiled::Jump { target_ip } => self.jump_to_ip(*target_ip),
            EventCompiled::SetFlag { flag_id, value } => {
                self.state.set_flag(*flag_id, *value);
                self.advance_position()
            }
            EventCompiled::Scene(scene) => {
                self.state.visual.apply_scene(scene);
                self.advance_position()
            }
            EventCompiled::Choice(_) => Ok(()),
            EventCompiled::Dialogue(dialogue) => {
                self.state.record_dialogue(dialogue);
                self.advance_position()
            }
            EventCompiled::SetVar { var_id, value } => {
                self.state.set_var(*var_id, *value);
                self.advance_position()
            }
            EventCompiled::JumpIf { cond, target_ip } => {
                if self.evaluate_cond(cond) {
                    self.jump_to_ip(*target_ip)
                } else {
                    self.advance_position()
                }
            }
            EventCompiled::Patch(patch) => {
                self.state.visual.apply_patch(patch);
                self.advance_position()
            }
        }
    }

    fn evaluate_cond(&self, cond: &CondCompiled) -> bool {
        match cond {
            CondCompiled::Flag { flag_id, is_set } => self.state.get_flag(*flag_id) == *is_set,
            CondCompiled::VarCmp { var_id, op, value } => {
                let var_val = self.state.get_var(*var_id);
                match op {
                    CmpOp::Eq => var_val == *value,
                    CmpOp::Ne => var_val != *value,
                    CmpOp::Lt => var_val < *value,
                    CmpOp::Le => var_val <= *value,
                    CmpOp::Gt => var_val > *value,
                    CmpOp::Ge => var_val >= *value,
                }
            }
        }
    }

    fn advance_position(&mut self) -> VnResult<()> {
        let next = self.state.position.saturating_add(1);
        if next as usize >= self.script.events.len() {
            self.state.position = self.script.events.len() as u32;
            return Ok(());
        }
        self.state.position = next;
        Ok(())
    }

    fn jump_to_ip(&mut self, target_ip: u32) -> VnResult<()> {
        if target_ip as usize >= self.script.events.len() {
            return Err(VnError::InvalidScript(format!(
                "jump target '{target_ip}' outside script"
            )));
        }
        self.state.position = target_ip;
        Ok(())
    }

    /// Returns the full engine state.
    pub fn state(&self) -> &EngineState {
        &self.state
    }

    /// Returns the security policy in use.
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Returns the current visual state.
    pub fn visual_state(&self) -> &crate::visual::VisualState {
        &self.state.visual
    }

    /// Returns the configured flag count.
    pub fn flag_count(&self) -> u32 {
        self.script.flag_count
    }

    /// Returns compiled script labels.
    pub fn labels(&self) -> &std::collections::BTreeMap<String, u32> {
        &self.script.labels
    }

    /// Sets a flag value by id.
    pub fn set_flag(&mut self, id: u32, value: bool) {
        self.state.set_flag(id, value);
    }

    /// Jumps to a label by name.
    pub fn jump_to_label(&mut self, label: &str) -> VnResult<()> {
        let target_ip = self
            .script
            .labels
            .get(label)
            .copied()
            .ok_or_else(|| VnError::InvalidScript(format!("label '{label}' not found")))?;
        self.jump_to_ip(target_ip)
    }

    /// Restores the engine state from a saved snapshot.
    pub fn set_state(&mut self, state: EngineState) -> VnResult<()> {
        if state.position as usize > self.script.events.len() {
            return Err(VnError::InvalidScript(format!(
                "state position '{}' outside script",
                state.position
            )));
        }
        self.state = state;
        Ok(())
    }

    /// Renders the current event using the provided renderer.
    pub fn render_current<R: RenderBackend>(&self, renderer: &R) -> VnResult<RenderOutput> {
        let event = self.current_event_ref()?;
        Ok(renderer.render(event, &self.state.visual))
    }

    /// Returns the current compiled event serialized as JSON.
    pub fn current_event_json(&self) -> VnResult<String> {
        let event = self.current_event()?;
        Ok(event.to_json_string())
    }
}

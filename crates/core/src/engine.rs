//! Runtime engine that executes compiled scripts.

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use crate::assets::AssetId;
use crate::audio::AudioCommand;
use crate::error::{VnError, VnResult};
use crate::event::{CmpOp, CondCompiled, EventCompiled};
use crate::render::{RenderBackend, RenderOutput};
use crate::resource::ResourceLimiter;
use crate::script::{ScriptCompiled, ScriptRaw};
use crate::security::SecurityPolicy;
use crate::state::EngineState;

const DEFAULT_FADE_MS: u64 = 500;
const CHOICE_HISTORY_LIMIT: usize = 512;

/// Recorded decision made by the player at a Choice event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceHistoryEntry {
    pub event_ip: u32,
    pub option_index: usize,
    pub option_text: String,
    pub target_ip: u32,
}

/// Execution engine for compiled scripts.
#[derive(Clone, Debug)]
pub struct Engine {
    script: ScriptCompiled,
    state: EngineState,
    policy: SecurityPolicy,
    queued_audio: Vec<AudioCommand>,
    read_dialogue_ips: BTreeSet<u32>,
    choice_history: VecDeque<ChoiceHistoryEntry>,
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
        let queued_audio = initial_audio_commands(&state);
        Ok(Self {
            script,
            state,
            policy,
            queued_audio,
            read_dialogue_ips: BTreeSet::new(),
            choice_history: VecDeque::with_capacity(64),
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
        let queued_audio = initial_audio_commands(&state);
        Ok(Self {
            script,
            state,
            policy,
            queued_audio,
            read_dialogue_ips: BTreeSet::new(),
            choice_history: VecDeque::with_capacity(64),
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
    pub fn step(&mut self) -> VnResult<(Vec<AudioCommand>, StateChange)> {
        let event = self.current_event()?;
        let mut audio_commands = self.take_audio_commands();
        self.advance_from(&event, &mut audio_commands)?;
        let change = StateChange {
            event,
            visual: self.state.visual.clone(),
        };
        Ok((audio_commands, change))
    }

    /// Returns the current event and advances the engine.
    pub fn step_event(&mut self) -> VnResult<EventCompiled> {
        let (_audio, change) = self.step()?;
        Ok(change.event)
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
                self.record_choice_decision(
                    self.state.position,
                    option_index,
                    option.text.as_ref(),
                    option.target_ip,
                );
                self.jump_to_ip(option.target_ip)?;
            }
            _ => return Err(VnError::InvalidChoice),
        }
        Ok(event)
    }

    fn advance_from(
        &mut self,
        event: &EventCompiled,
        audio_commands: &mut Vec<AudioCommand>,
    ) -> VnResult<()> {
        let current_ip = self.state.position;
        match event {
            EventCompiled::Jump { target_ip } => self.jump_to_ip(*target_ip),
            EventCompiled::SetFlag { flag_id, value } => {
                self.state.set_flag(*flag_id, *value);
                self.advance_position()
            }
            EventCompiled::Scene(scene) => {
                let before_music = self.state.visual.music.clone();
                self.state.visual.apply_scene(scene);
                append_music_delta(before_music, &self.state.visual.music, audio_commands);
                self.advance_position()
            }
            EventCompiled::Choice(_) => Ok(()),
            EventCompiled::Dialogue(dialogue) => {
                self.state.record_dialogue(dialogue);
                self.read_dialogue_ips.insert(current_ip);
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
                let before_music = self.state.visual.music.clone();
                self.state.visual.apply_patch(patch);
                append_music_delta(before_music, &self.state.visual.music, audio_commands);
                self.advance_position()
            }
            EventCompiled::ExtCall { .. } => Ok(()),
            EventCompiled::AudioAction(action) => {
                use crate::audio::AudioCommand;
                // Mapping: channel 0=BGM, 1=SFX, 2=Voice (currently routed to SFX backend).
                // Action: 0=Play, 1=Stop, 2=FadeOut.
                let cmd = match action.action {
                    0 => {
                        // Play
                        if let Some(path) = &action.asset {
                            if action.channel == 0 {
                                Some(AudioCommand::PlayBgm {
                                    resource: AssetId::from_path(path.as_ref()),
                                    path: path.clone(),
                                    r#loop: action.loop_playback.unwrap_or(true),
                                    fade_in: Duration::from_millis(
                                        action.fade_duration_ms.unwrap_or(DEFAULT_FADE_MS),
                                    ),
                                })
                            } else {
                                Some(AudioCommand::PlaySfx {
                                    resource: AssetId::from_path(path.as_ref()),
                                    path: path.clone(),
                                })
                            }
                        } else {
                            None
                        }
                    }
                    1 | 2 => {
                        // Stop/FadeOut (for BGM, both map to stop with fade_out duration)
                        if action.channel == 0 {
                            Some(AudioCommand::StopBgm {
                                fade_out: Duration::from_millis(
                                    action.fade_duration_ms.unwrap_or(DEFAULT_FADE_MS),
                                ),
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(c) = cmd {
                    audio_commands.push(c);
                }
                self.advance_position()
            }
            EventCompiled::SetCharacterPosition(pos) => {
                self.state.visual.set_character_position(pos);
                self.advance_position()
            }
            EventCompiled::Transition(_) => self.advance_position(),
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

    pub fn take_audio_commands(&mut self) -> Vec<AudioCommand> {
        std::mem::take(&mut self.queued_audio)
    }

    pub fn queue_audio_command(&mut self, command: AudioCommand) {
        self.queued_audio.push(command);
    }

    pub fn resume(&mut self) -> VnResult<()> {
        let event = self.current_event()?;
        match event {
            EventCompiled::ExtCall { .. } => self.advance_position(),
            _ => Ok(()),
        }
    }

    pub fn peek_next_assets(&self, depth: usize) -> Vec<AssetId> {
        let mut seen = std::collections::HashSet::new();
        let mut assets = Vec::new();
        let start = self.state.position as usize;
        let end = (start + depth).min(self.script.events.len());
        for event in &self.script.events[start..end] {
            match event {
                EventCompiled::Scene(scene) => {
                    if let Some(background) = &scene.background {
                        let id = AssetId::from_path(background.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                    }
                    if let Some(music) = &scene.music {
                        let id = AssetId::from_path(music.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                    }
                    for character in &scene.characters {
                        let id = AssetId::from_path(character.name.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                        if let Some(expression) = &character.expression {
                            let id = AssetId::from_path(expression.as_ref());
                            if seen.insert(id) {
                                assets.push(id);
                            }
                        }
                    }
                }
                EventCompiled::Patch(patch) => {
                    if let Some(background) = &patch.background {
                        let id = AssetId::from_path(background.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                    }
                    if let Some(music) = &patch.music {
                        let id = AssetId::from_path(music.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                    }
                    // ... (simplified loop for patch additions/updates similar to scene)
                    for character in &patch.add {
                        let id = AssetId::from_path(character.name.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                        if let Some(expression) = &character.expression {
                            let id = AssetId::from_path(expression.as_ref());
                            if seen.insert(id) {
                                assets.push(id);
                            }
                        }
                    }
                    for character in &patch.update {
                        let id = AssetId::from_path(character.name.as_ref());
                        if seen.insert(id) {
                            assets.push(id);
                        }
                        if let Some(expression) = &character.expression {
                            let id = AssetId::from_path(expression.as_ref());
                            if seen.insert(id) {
                                assets.push(id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        assets
    }

    /// Returns unique upcoming asset paths that can be prefetched safely.
    ///
    /// This intentionally excludes non-path semantic fields (for example, character display names)
    /// to avoid prefetching invalid resources.
    pub fn peek_next_asset_paths(&self, depth: usize) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut paths = Vec::new();
        let start = self.state.position as usize;
        let end = (start + depth).min(self.script.events.len());
        for event in &self.script.events[start..end] {
            collect_prefetch_paths_from_event(event, &mut seen, &mut paths);
        }
        paths
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
        self.read_dialogue_ips.clear();
        self.choice_history.clear();
        Ok(())
    }

    /// Returns `true` if a dialogue at the given instruction pointer was already displayed.
    pub fn is_dialogue_read(&self, ip: u32) -> bool {
        self.read_dialogue_ips.contains(&ip)
    }

    /// Returns `true` when the current event is a dialogue previously displayed.
    pub fn is_current_dialogue_read(&self) -> bool {
        matches!(self.current_event_ref(), Ok(EventCompiled::Dialogue(_)))
            && self.read_dialogue_ips.contains(&self.state.position)
    }

    /// Returns the current in-memory choice history.
    pub fn choice_history(&self) -> &VecDeque<ChoiceHistoryEntry> {
        &self.choice_history
    }

    /// Clears runtime-only session history (read dialogue marks and choice history).
    pub fn clear_session_history(&mut self) {
        self.read_dialogue_ips.clear();
        self.choice_history.clear();
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

#[derive(Clone, Debug)]
pub struct StateChange {
    pub event: EventCompiled,
    pub visual: crate::visual::VisualState,
}

fn initial_audio_commands(state: &EngineState) -> Vec<AudioCommand> {
    let mut commands = Vec::new();
    if let Some(music) = &state.visual.music {
        commands.push(AudioCommand::PlayBgm {
            resource: AssetId::from_path(music.as_ref()),
            path: music.clone(),
            r#loop: true,
            fade_in: Duration::from_millis(DEFAULT_FADE_MS),
        });
    }
    commands
}

fn append_music_delta(
    before: Option<crate::event::SharedStr>,
    after: &Option<crate::event::SharedStr>,
    audio_commands: &mut Vec<AudioCommand>,
) {
    if before.as_deref() == after.as_deref() {
        return;
    }
    match after {
        Some(music) => audio_commands.push(AudioCommand::PlayBgm {
            resource: AssetId::from_path(music.as_ref()),
            path: music.clone(),
            r#loop: true,
            fade_in: Duration::from_millis(DEFAULT_FADE_MS),
        }),
        None => audio_commands.push(AudioCommand::StopBgm {
            fade_out: Duration::from_millis(DEFAULT_FADE_MS),
        }),
    }
}

fn collect_prefetch_paths_from_event(
    event: &EventCompiled,
    seen: &mut std::collections::HashSet<String>,
    output: &mut Vec<String>,
) {
    match event {
        EventCompiled::Scene(scene) => {
            if let Some(background) = &scene.background {
                push_unique_prefetch_path(background.as_ref(), seen, output);
            }
            if let Some(music) = &scene.music {
                push_unique_prefetch_path(music.as_ref(), seen, output);
            }
            for character in &scene.characters {
                if let Some(expression) = &character.expression {
                    push_unique_prefetch_path(expression.as_ref(), seen, output);
                }
            }
        }
        EventCompiled::Patch(patch) => {
            if let Some(background) = &patch.background {
                push_unique_prefetch_path(background.as_ref(), seen, output);
            }
            if let Some(music) = &patch.music {
                push_unique_prefetch_path(music.as_ref(), seen, output);
            }
            for character in &patch.add {
                if let Some(expression) = &character.expression {
                    push_unique_prefetch_path(expression.as_ref(), seen, output);
                }
            }
            for character in &patch.update {
                if let Some(expression) = &character.expression {
                    push_unique_prefetch_path(expression.as_ref(), seen, output);
                }
            }
        }
        EventCompiled::AudioAction(action) => {
            if action.action == 0 {
                if let Some(asset) = &action.asset {
                    push_unique_prefetch_path(asset.as_ref(), seen, output);
                }
            }
        }
        _ => {}
    }
}

fn push_unique_prefetch_path(
    value: &str,
    seen: &mut std::collections::HashSet<String>,
    output: &mut Vec<String>,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if seen.insert(trimmed.to_string()) {
        output.push(trimmed.to_string());
    }
}

impl Engine {
    fn record_choice_decision(
        &mut self,
        event_ip: u32,
        option_index: usize,
        option_text: &str,
        target_ip: u32,
    ) {
        if self.choice_history.len() >= CHOICE_HISTORY_LIMIT {
            self.choice_history.pop_front();
        }
        self.choice_history.push_back(ChoiceHistoryEntry {
            event_ip,
            option_index,
            option_text: option_text.to_string(),
            target_ip,
        });
    }
}

#[cfg(test)]
#[path = "tests/engine_tests.rs"]
mod tests;

use std::collections::HashMap;
use std::time::Instant;
use evdev::{KeyCode, RelativeAxisCode};
use crate::mappings::{OutputSink, ProcessedMappings};
use crate::profile::{ComplexMapping, InputSimple, MappingDef, ModifierDef};

// What keys each physical input is currently holding. Recorded at press time so
// release is always symmetric — modifier state changes between press and release
// cannot cause a key to get stuck.
struct HeldState {
    keys: Vec<KeyCode>,
    activated_modifier: Option<String>,
    // Which modifier's scope this press was resolved under (for passive key management).
    using_modifier_scope: Option<String>,
    on_up_keys: Vec<KeyCode>,
}

impl Default for HeldState {
    fn default() -> Self {
        HeldState {
            keys: Vec::new(),
            activated_modifier: None,
            using_modifier_scope: None,
            on_up_keys: Vec::new(),
        }
    }
}

#[derive(Default)]
struct ActionPlan {
    held_keys: Vec<KeyCode>,
    one_shot_down: Vec<KeyCode>,
    one_shot_up: Vec<KeyCode>,
    activate_modifier: Option<String>,
    using_modifier_scope: Option<String>,
}

pub struct Engine {
    output: HashMap<String, KeyCode>,
    // Physical key code for each named input — used for passthrough when no mapping fires.
    physical_inputs: HashMap<String, KeyCode>,
    input_mappings: HashMap<String, MappingDef>,
    modifiers: HashMap<String, ModifierDef>,
    // Stack: last entry = highest-priority modifier for override lookup.
    active_modifiers: Vec<String>,
    held_states: HashMap<String, HeldState>,
    // Whether the passive key for each modifier is currently pressed.
    passive_key_active: HashMap<String, bool>,
    // Count of inputs currently resolved under each modifier's scope overrides.
    // Passive key is suppressed while this count is non-zero.
    modifier_scope_active: HashMap<String, usize>,
    // Timestamp of the last axis event per input, used to detect when movement has stopped.
    axis_last_move: HashMap<String, Instant>,
    sink: Box<dyn OutputSink>,
}

impl Engine {
    pub fn new(
        output: HashMap<String, KeyCode>,
        physical_inputs: HashMap<String, KeyCode>,
        mappings: ProcessedMappings,
        sink: Box<dyn OutputSink>,
    ) -> Self {
        Engine {
            output,
            physical_inputs,
            input_mappings: mappings.input_mappings,
            modifiers: mappings.modifiers,
            active_modifiers: Vec::new(),
            held_states: HashMap::new(),
            passive_key_active: HashMap::new(),
            modifier_scope_active: HashMap::new(),
            axis_last_move: HashMap::new(),
            sink,
        }
    }

    pub fn on_button_down(&mut self, input_name: &str) {
        let plan = self.plan_down(input_name);
        self.execute_plan(input_name, plan);
    }

    pub fn on_button_up(&mut self, input_name: &str) {
        let state = self.held_states.remove(input_name).unwrap_or_default();

        // Symmetric release: ignore current modifier state entirely.
        for &key in &state.keys {
            self.sink.release(key);
        }

        // Leave modifier scope: re-press passive if this was the last in-scope input.
        if let Some(ref scope_mod) = state.using_modifier_scope {
            let current = self.modifier_scope_active.get(scope_mod).copied().unwrap_or(0);
            let new_count = current.saturating_sub(1);
            self.modifier_scope_active.insert(scope_mod.clone(), new_count);

            let should_reactivate = new_count == 0
                && self.active_modifiers.contains(scope_mod)
                && !self.passive_key_active.get(scope_mod).copied().unwrap_or(true);

            if should_reactivate {
                if let Some(key) = self.get_passive_key(scope_mod) {
                    self.sink.press(key);
                }
                self.passive_key_active.insert(scope_mod.clone(), true);
            }
        }

        // Deactivate modifier: release passive key and remove from stack.
        if let Some(ref mod_name) = state.activated_modifier {
            self.active_modifiers.retain(|m| m != mod_name);
            if self.passive_key_active.remove(mod_name).unwrap_or(false) {
                if let Some(key) = self.get_passive_key(mod_name) {
                    self.sink.release(key);
                }
            }
            self.modifier_scope_active.remove(mod_name);
        }

        for &key in &state.on_up_keys {
            self.sink.press(key);
            self.sink.release(key);
        }
    }

    // Returns true if a mapping fired (caller should schedule the release timer).
    // Returns false if no mapping is active (caller should relay the raw REL event).
    pub fn on_axis_move(&mut self, input_name: &str) -> bool {
        if self.effective_mapping(input_name).is_none() {
            return false;
        }
        self.axis_last_move.insert(input_name.to_string(), Instant::now());
        if !self.held_states.contains_key(input_name) {
            let plan = self.plan_down(input_name);
            self.execute_plan(input_name, plan);
        }
        true
    }

    // Called after a timeout delay. Releases the key only if no axis event has
    // arrived within `timeout_ms` — i.e., movement has actually stopped.
    pub fn on_axis_release_maybe(&mut self, input_name: &str, timeout_ms: u64) {
        let elapsed_ms = self.axis_last_move
            .get(input_name)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(u64::MAX);
        if elapsed_ms >= timeout_ms {
            self.on_button_up(input_name);
            self.axis_last_move.remove(input_name);
        }
    }

    pub fn relay_axis(&mut self, axis: RelativeAxisCode, delta: i32) {
        self.sink.relay_axis(axis, delta);
    }

    fn plan_down(&self, input_name: &str) -> ActionPlan {
        let mut plan = ActionPlan::default();
        match self.effective_mapping(input_name) {
            None => {
                // No mapping — relay the physical key for this input.
                if let Some(&key) = self.physical_inputs.get(input_name) {
                    plan.held_keys.push(key);
                }
            }
            Some((mapping, source_mod)) => {
                self.plan_from_mapping(input_name, mapping, &mut plan);
                plan.using_modifier_scope = source_mod.map(|s| s.to_string());
            }
        }
        plan
    }

    // Walk active_modifiers newest-first so the last-activated modifier wins.
    // Returns the mapping and the name of the modifier that provided it (if any).
    fn effective_mapping<'a>(&'a self, input_name: &str) -> Option<(&'a MappingDef, Option<&'a str>)> {
        for mod_name in self.active_modifiers.iter().rev() {
            if let Some(mod_def) = self.modifiers.get(mod_name) {
                if let Some(m) = mod_def.entries.get(input_name) {
                    return Some((m, Some(mod_name.as_str())));
                }
            }
        }
        self.input_mappings.get(input_name).map(|m| (m, None))
    }

    fn plan_from_mapping(&self, input_name: &str, mapping: &MappingDef, plan: &mut ActionPlan) {
        match mapping {
            MappingDef::Simple(target) => self.plan_simple(target, plan),
            MappingDef::Multi(targets) => {
                for target in targets {
                    if let Some(&key) = self.output.get(target.as_str()) {
                        plan.held_keys.push(key);
                    }
                }
            }
            MappingDef::Complex(complex) => self.plan_complex(input_name, complex, plan),
            MappingDef::Modifier(_) => {}
        }
    }

    // A simple string is either a modifier name or an output key name.
    fn plan_simple(&self, target: &str, plan: &mut ActionPlan) {
        if self.modifiers.contains_key(target) {
            plan.activate_modifier = Some(target.to_string());
        } else if let Some(&key) = self.output.get(target) {
            plan.held_keys.push(key);
        }
    }

    fn plan_complex(&self, input_name: &str, complex: &ComplexMapping, plan: &mut ActionPlan) {
        if complex.input_passthrough {
            if let Some(&key) = self.physical_inputs.get(input_name) {
                plan.held_keys.push(key);
            }
        }
        if let Some(simple) = &complex.input_simple {
            match simple {
                InputSimple::One(target) => self.plan_simple(target, plan),
                InputSimple::Many(targets) => {
                    for target in targets {
                        if self.modifiers.contains_key(target.as_str()) {
                            plan.activate_modifier = Some(target.clone());
                        } else if let Some(&key) = self.output.get(target.as_str()) {
                            plan.held_keys.push(key);
                        }
                    }
                }
            }
        }
        if let Some(target) = &complex.input_down {
            if let Some(&key) = self.output.get(target.as_str()) {
                plan.one_shot_down.push(key);
            }
        }
        if let Some(target) = &complex.input_up {
            if let Some(&key) = self.output.get(target.as_str()) {
                plan.one_shot_up.push(key);
            }
        }
        if complex.input_short.is_some()
            || complex.input_long.is_some()
            || complex.input_delayed.is_some()
            || complex.input_variable.is_some()
        {
            eprintln!("[engine] timed/variable mappings for '{}' not yet implemented", input_name);
        }
    }

    fn execute_plan(&mut self, input_name: &str, plan: ActionPlan) {
        for &key in &plan.held_keys {
            self.sink.press(key);
        }
        for &key in &plan.one_shot_down {
            self.sink.press(key);
            self.sink.release(key);
        }

        // Enter modifier scope: suppress passive key when first in-scope input arrives.
        if let Some(ref scope_mod) = plan.using_modifier_scope {
            let current = self.modifier_scope_active.get(scope_mod).copied().unwrap_or(0);
            self.modifier_scope_active.insert(scope_mod.clone(), current + 1);
            if current == 0 && self.passive_key_active.get(scope_mod).copied().unwrap_or(false) {
                if let Some(key) = self.get_passive_key(scope_mod) {
                    self.sink.release(key);
                }
                self.passive_key_active.insert(scope_mod.clone(), false);
            }
        }

        // Activate modifier: press passive key if it has one.
        if let Some(ref mod_name) = plan.activate_modifier {
            self.active_modifiers.push(mod_name.clone());
            if let Some(key) = self.get_passive_key(mod_name) {
                self.sink.press(key);
                self.passive_key_active.insert(mod_name.clone(), true);
            }
        }

        self.held_states.insert(input_name.to_string(), HeldState {
            keys: plan.held_keys,
            activated_modifier: plan.activate_modifier,
            using_modifier_scope: plan.using_modifier_scope,
            on_up_keys: plan.one_shot_up,
        });
    }

    // Release everything — called on focus loss so no keys stay stuck.
    pub fn release_all(&mut self) {
        let held: Vec<HeldState> = self.held_states.drain().map(|(_, s)| s).collect();
        for state in held {
            for &key in &state.keys {
                self.sink.release(key);
            }
        }
        let passive: Vec<(String, bool)> = self.passive_key_active.drain().collect();
        for (mod_name, was_active) in passive {
            if was_active {
                if let Some(key) = self.get_passive_key(&mod_name) {
                    self.sink.release(key);
                }
            }
        }
        self.active_modifiers.clear();
        self.modifier_scope_active.clear();
        self.axis_last_move.clear();
    }

    fn get_passive_key(&self, mod_name: &str) -> Option<KeyCode> {
        self.modifiers.get(mod_name)?
            .input_passive.as_deref()
            .and_then(|p| self.output.get(p))
            .copied()
    }
}

#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;

/// Top-level mapping entry. Serde tries variants in order:
/// Simple and Multi cover the shorthand forms; Complex covers objects
/// with only `input_*` fields; Modifier catches everything else
/// (objects containing input-name keys or `input_passive`).
#[derive(Deserialize)]
#[serde(untagged)]
pub enum MappingDef {
    Simple(String),
    Multi(Vec<String>),
    Complex(Box<ComplexMapping>),
    Modifier(Box<ModifierDef>),
}

/// Object mapping. `deny_unknown_fields` makes deserialization fail when
/// any key is not in this list, which lets the untagged enum fall through
/// to `ModifierDef` for modifier-like objects.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ComplexMapping {
    #[serde(default)]
    pub input_passthrough: bool,
    pub input_simple: Option<InputSimple>,
    pub input_down: Option<String>,
    pub input_up: Option<String>,
    pub input_short: Option<TimedMapping>,
    pub input_long: Option<TimedMapping>,
    pub input_delayed: Option<DelayedMapping>,
    pub input_variable: Option<VariableMapping>,
    pub input_combo: Option<String>,
    /// Combo entry args: (output_key, careful). `careful=false` means no
    /// masking delay when transitioning between solo and combo states.
    pub input_arguments: Option<(String, bool)>,
}

/// Single or multiple simultaneous outputs for `input_simple`.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum InputSimple {
    One(String),
    Many(Vec<String>),
}

/// Used by both `input_short` (fire if released within N ms) and
/// `input_long` (fire after held for N ms). Contains exactly one action.
#[derive(Deserialize)]
pub struct TimedMapping {
    pub input_arguments: (u64,),
    pub input_simple: Option<InputSimple>,
    pub input_cycle: Option<Vec<String>>,
}

/// `input_delayed`: wrap an output with independent activation/deactivation delays.
/// `input_arguments`: (delay_on_ms, delay_off_ms).
#[derive(Deserialize)]
pub struct DelayedMapping {
    pub input_arguments: (u64, u64),
    pub input_simple: Option<InputSimple>,
}

/// `input_variable`:
/// - 1 arg `[var_name]` → read variable and hold its current value
/// - 2 args `[var_name, value]` → write value to variable
#[derive(Deserialize)]
pub struct VariableMapping {
    pub input_arguments: VariableArgs,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum VariableArgs {
    Read((String,)),
    Write((String, String)),
}

/// Modifier definition. Keys in `entries` are either input-name overrides
/// (looked up at runtime against the `input` section) or named combo
/// definitions (referenced via `input_combo`).
#[derive(Deserialize)]
pub struct ModifierDef {
    pub input_passive: Option<String>,
    #[serde(flatten)]
    pub entries: HashMap<String, MappingDef>,
}

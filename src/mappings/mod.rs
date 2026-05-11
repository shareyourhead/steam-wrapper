use anyhow::Result;
use evdev::{KeyCode, RelativeAxisCode};
use std::collections::HashMap;

use crate::profile::{MappingDef, ModifierDef};

pub mod simple;
pub mod multi;

#[allow(dead_code)]
pub trait OutputSink: Send {
    fn press(&mut self, key: KeyCode);
    fn release(&mut self, key: KeyCode);
    fn relay_axis(&mut self, _axis: RelativeAxisCode, _delta: i32) {}
}

#[allow(dead_code)]
pub trait Mapping: Send {
    fn on_down(&mut self, sink: &mut dyn OutputSink);
    fn on_up(&mut self, sink: &mut dyn OutputSink);
    fn on_move(&mut self, _delta: i32, _sink: &mut dyn OutputSink) {}
}

/// Raw parsed mappings split by kind. The engine consumes these.
#[allow(dead_code)]
pub struct ProcessedMappings {
    /// Entries whose key appeared in the `input` section.
    pub input_mappings: HashMap<String, MappingDef>,
    /// Entries whose key did NOT appear in `input` (modifier definitions).
    pub modifiers: HashMap<String, ModifierDef>,
}

/// Split `defs` into input mappings and modifier definitions.
/// `MappingDef::Modifier` variants go to `modifiers`; everything else is
/// stored as-is for the engine to resolve against output bindings at runtime.
pub fn build_mappings(
    defs: HashMap<String, MappingDef>,
    _output: &HashMap<String, KeyCode>,
) -> Result<ProcessedMappings> {
    let mut input_mappings = HashMap::new();
    let mut modifiers = HashMap::new();

    for (name, def) in defs {
        match def {
            MappingDef::Modifier(mod_def) => {
                modifiers.insert(name, *mod_def);
            }
            other => {
                input_mappings.insert(name, other);
            }
        }
    }

    Ok(ProcessedMappings { input_mappings, modifiers })
}

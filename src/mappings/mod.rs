use anyhow::Result;
use evdev::{KeyCode, RelativeAxisCode};
use std::collections::HashMap;

use crate::profile::{MappingDef, ModifierDef};

pub trait OutputSink: Send {
    fn press(&mut self, key: KeyCode);
    fn release(&mut self, key: KeyCode);
    fn relay_axis(&mut self, _axis: RelativeAxisCode, _delta: i32) {}
}

pub struct ProcessedMappings {
    pub input_mappings: HashMap<String, MappingDef>,
    pub modifiers: HashMap<String, ModifierDef>,
}

pub fn build_mappings(defs: HashMap<String, MappingDef>) -> Result<ProcessedMappings> {
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

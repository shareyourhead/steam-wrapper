use anyhow::Result;
use evdev::KeyCode;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use super::codes::parse_key_code;
use super::input::InputDef;

#[derive(Deserialize)]
pub struct OutputSection {
    #[serde(default)]
    default: IndexMap<String, DefaultEntry>,
    #[serde(default)]
    rebinds: IndexMap<String, ReboundEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DefaultEntry {
    Context(IndexMap<String, String>),
    Direct(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReboundEntry {
    Context(IndexMap<String, String>),
    Direct(String),
}

pub struct ResolvedOutput {
    pub bindings: HashMap<String, KeyCode>,
    // (game_action, display_label) — display_label is the original pre-resolution string
    // so input-name refs like "right_click" show as "right_click" not "BTN_RIGHT".
    pub rebinds: Vec<(String, String)>,
    pub only_rebound: bool,
}

fn flatten_default(default: IndexMap<String, DefaultEntry>) -> IndexMap<String, String> {
    let mut flat = IndexMap::new();
    for (name, entry) in default {
        match entry {
            DefaultEntry::Context(actions) => {
                for (action, key) in actions {
                    flat.insert(format!("{}/{}", name, action), key);
                }
            }
            DefaultEntry::Direct(key) => {
                flat.insert(name, key);
            }
        }
    }
    flat
}

fn flatten_rebound(rebound: IndexMap<String, ReboundEntry>) -> IndexMap<String, String> {
    let mut flat = IndexMap::new();
    for (name, entry) in rebound {
        match entry {
            ReboundEntry::Context(actions) => {
                for (action, key) in actions {
                    flat.insert(format!("{}/{}", name, action), key);
                }
            }
            ReboundEntry::Direct(key) => {
                flat.insert(name, key);
            }
        }
    }
    flat
}

// `input_refs` is a secondary lookup (input-name → key-code string) so that
// rebind values like "right_click" resolve to "BTN_RIGHT" without being in the
// output flat map itself.
fn resolve_refs(flat: &mut IndexMap<String, String>, input_refs: &HashMap<String, String>) {
    loop {
        let snapshot = flat.clone();
        let mut changed = false;
        for value in flat.values_mut() {
            let resolved = snapshot.get(value.as_str())
                .or_else(|| input_refs.get(value.as_str()));
            if let Some(resolved) = resolved {
                if resolved != value {
                    *value = resolved.clone();
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }
}

fn parse_flat(flat: IndexMap<String, String>) -> Result<HashMap<String, KeyCode>> {
    flat.into_iter()
        .map(|(k, v)| {
            parse_key_code(&v)
                .map_err(|e| anyhow::anyhow!("output '{}': {}", k, e))
                .map(|code| (k, code))
        })
        .collect()
}

impl OutputSection {
    pub fn has_rebinds(&self) -> bool {
        !self.rebinds.is_empty()
    }

    pub fn resolve(self, inputs: &HashMap<String, InputDef>) -> Result<ResolvedOutput> {
        let input_refs: HashMap<String, String> = inputs.iter()
            .filter_map(|(name, def)| match def {
                InputDef::Button(code_str) => Some((name.clone(), code_str.clone())),
                _ => None,
            })
            .collect();
        let has_default = !self.default.is_empty();
        let has_rebound = !self.rebinds.is_empty();

        // Only default: parse directly, no merging or dereferencing needed
        if has_default && !has_rebound {
            let flat = flatten_default(self.default);
            return Ok(ResolvedOutput {
                bindings: parse_flat(flat)?,
                rebinds: Vec::new(),
                only_rebound: false,
            });
        }

        // Only rebound: dereference and mark all controls as needing rebind,
        // preserving JSON5 insertion order
        if !has_default && has_rebound {
            let mut flat = flatten_rebound(self.rebinds);
            // Capture display labels before resolution so input-name refs stay readable.
            let rebinds: Vec<(String, String)> = flat.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            resolve_refs(&mut flat, &input_refs);
            return Ok(ResolvedOutput {
                bindings: parse_flat(flat)?,
                rebinds,
                only_rebound: true,
            });
        }

        // Both: full resolve with merge and rebind detection
        let default_flat = flatten_default(self.default);
        let default_order: Vec<String> = default_flat.keys().cloned().collect();

        let mut rebound_override_keys: HashSet<String> = HashSet::new();
        let mut merged_flat = default_flat.clone();
        let mut rebound_display: HashMap<String, String> = HashMap::new();
        for (k, v) in flatten_rebound(self.rebinds) {
            if default_flat.contains_key(&k) {
                rebound_override_keys.insert(k.clone());
            }
            rebound_display.insert(k.clone(), v.clone());
            merged_flat.insert(k, v);
        }

        let mut default_resolved = default_flat;
        resolve_refs(&mut default_resolved, &input_refs);
        resolve_refs(&mut merged_flat, &input_refs);

        let mut rebinds: Vec<(String, String)> = rebound_override_keys
            .into_iter()
            .filter(|k| merged_flat.get(k) != default_resolved.get(k))
            .map(|k| {
                let display = rebound_display.get(&k).cloned().unwrap_or_else(|| k.clone());
                (k, display)
            })
            .collect();
        rebinds.sort_by_key(|(k, _)| default_order.iter().position(|o| o == k).unwrap_or(usize::MAX));

        Ok(ResolvedOutput {
            bindings: parse_flat(merged_flat)?,
            rebinds,
            only_rebound: false,
        })
    }
}

pub fn print_rebinds(resolved: &ResolvedOutput) {
    if resolved.rebinds.is_empty() {
        return;
    }
    println!("\nHEY YOU!\nRemember to rebind the following controls in-game:");
    for (binding, display) in &resolved.rebinds {
        println!("  {} -> {}", binding, display);
    }
    if !resolved.only_rebound {
        println!("All other controls are default.");
    }
    println!();
}

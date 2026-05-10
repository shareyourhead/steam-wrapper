use anyhow::Result;
use evdev::KeyCode;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use super::codes::parse_key_code;

#[derive(Deserialize)]
pub struct OutputSection {
    #[serde(default)]
    default: IndexMap<String, IndexMap<String, String>>,
    #[serde(default)]
    rebinds: IndexMap<String, ReboundEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReboundEntry {
    Context(IndexMap<String, String>),
    Direct(String),
}

pub struct ResolvedOutput {
    pub bindings: HashMap<String, KeyCode>,
    pub rebinds: Vec<(String, KeyCode)>,
    pub only_rebound: bool,
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

fn resolve_refs(flat: &mut IndexMap<String, String>) {
    loop {
        let snapshot = flat.clone();
        let mut changed = false;
        for value in flat.values_mut() {
            if let Some(resolved) = snapshot.get(value.as_str()) {
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

    pub fn resolve(self) -> Result<ResolvedOutput> {
        let has_default = !self.default.is_empty();
        let has_rebound = !self.rebinds.is_empty();

        // Only default: parse directly, no merging or dereferencing needed
        if has_default && !has_rebound {
            let flat: IndexMap<String, String> = self.default
                .into_iter()
                .flat_map(|(ctx, actions)| {
                    actions.into_iter().map(move |(action, key)| {
                        (format!("{}/{}", ctx, action), key)
                    })
                })
                .collect();
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
            resolve_refs(&mut flat);
            let rebinds: Vec<(String, KeyCode)> = flat.iter()
                .map(|(k, v)| {
                    parse_key_code(v)
                        .map_err(|e| anyhow::anyhow!("output '{}': {}", k, e))
                        .map(|code| (k.clone(), code))
                })
                .collect::<Result<_>>()?;
            return Ok(ResolvedOutput {
                bindings: parse_flat(flat)?,
                rebinds,
                only_rebound: true,
            });
        }

        // Both: full resolve with merge and rebind detection
        let mut default_order: Vec<String> = Vec::new();
        let default_flat: IndexMap<String, String> = self.default
            .into_iter()
            .flat_map(|(ctx, actions)| {
                actions.into_iter().map(move |(action, key)| {
                    (format!("{}/{}", ctx, action), key)
                })
            })
            .inspect(|(k, _)| default_order.push(k.clone()))
            .collect();

        let mut rebound_override_keys: HashSet<String> = HashSet::new();
        let mut merged_flat = default_flat.clone();
        for (k, v) in flatten_rebound(self.rebinds) {
            if default_flat.contains_key(&k) {
                rebound_override_keys.insert(k.clone());
            }
            merged_flat.insert(k, v);
        }

        let mut default_resolved = default_flat;
        resolve_refs(&mut default_resolved);
        resolve_refs(&mut merged_flat);

        let mut rebinds: Vec<(String, KeyCode)> = rebound_override_keys
            .into_iter()
            .filter(|k| merged_flat.get(k) != default_resolved.get(k))
            .map(|k| {
                parse_key_code(&merged_flat[&k])
                    .map_err(|e| anyhow::anyhow!("output '{}': {}", k, e))
                    .map(|code| (k, code))
            })
            .collect::<Result<_>>()?;
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
    for (binding, code) in &resolved.rebinds {
        println!("  {} -> {:?}", binding, code);
    }
    if !resolved.only_rebound {
        println!("All other controls are default.");
    }
    println!();
}

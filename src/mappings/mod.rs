use anyhow::Result;
use evdev::KeyCode;
use std::collections::HashMap;

use crate::profile::MappingDef;

mod one_to_one;
use one_to_one::OneToOne;

// Abstraction over the uinput device — real implementation comes later
#[allow(dead_code)]
pub trait OutputSink: Send {
    fn press(&mut self, key: KeyCode);
    fn release(&mut self, key: KeyCode);
}

#[allow(dead_code)]
pub trait Mapping: Send {
    fn on_down(&mut self, sink: &mut dyn OutputSink);
    fn on_up(&mut self, sink: &mut dyn OutputSink);
    fn on_move(&mut self, _delta: i32, _sink: &mut dyn OutputSink) {}
}

pub fn build_mappings(
    defs: HashMap<String, MappingDef>,
    output: &HashMap<String, KeyCode>,
) -> Result<HashMap<String, Box<dyn Mapping>>> {
    defs.into_iter()
        .map(|(input_name, def)| {
            let mapping: Box<dyn Mapping> = match def {
                MappingDef::OneToOne(output_key) => {
                    let key = output.get(&output_key)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!(
                            "mapping '{}': output key '{}' not found", input_name, output_key
                        ))?;
                    Box::new(OneToOne::new(key))
                }
            };
            Ok((input_name, mapping))
        })
        .collect()
}

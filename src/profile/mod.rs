use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

mod codes;
mod device;
mod input;
mod mappings;
mod output;

pub use device::DeviceConfig;
pub use input::InputDef;
pub use mappings::MappingDef;
pub use output::{OutputSection, print_rebinds};

#[derive(Deserialize)]
pub struct Profile {
    pub device: DeviceConfig,
    pub input: HashMap<String, InputDef>,
    pub output: OutputSection,
    pub mappings: HashMap<String, MappingDef>,
}

pub fn load_config(path: &str) -> Result<Profile> {
    let config_str = fs::read_to_string(path)?;
    let config: Profile = json5::from_str(&config_str)?;
    Ok(config)
}

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum MappingDef {
    OneToOne(String),
    // Future mapping types will be objects with a "type" field
}

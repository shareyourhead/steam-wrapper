use serde::Deserialize;

#[derive(Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub id: String,
}

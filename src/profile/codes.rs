use anyhow::Result;
use evdev::{KeyCode, RelativeAxisCode};
use std::collections::HashMap;
use std::sync::OnceLock;

static KEY_CODES: OnceLock<HashMap<String, KeyCode>> = OnceLock::new();
static AXIS_CODES: OnceLock<HashMap<String, RelativeAxisCode>> = OnceLock::new();

fn key_codes() -> &'static HashMap<String, KeyCode> {
    KEY_CODES.get_or_init(|| {
        (0u16..=767)
            .map(KeyCode::new)
            .map(|k| (format!("{k:?}"), k))
            .collect()
    })
}

fn axis_codes() -> &'static HashMap<String, RelativeAxisCode> {
    AXIS_CODES.get_or_init(|| {
        (0u16..=15)
            .map(RelativeAxisCode)
            .map(|a| (format!("{a:?}"), a))
            .collect()
    })
}

pub fn parse_key_code(s: &str) -> Result<KeyCode> {
    key_codes()
        .get(s)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("Unknown key code: {}", s))
}

pub fn parse_axis_code(s: &str) -> Result<RelativeAxisCode> {
    axis_codes()
        .get(s)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("Unknown axis code: {}", s))
}

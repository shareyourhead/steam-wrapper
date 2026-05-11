use anyhow::Result;
use evdev::{KeyCode, RelativeAxisCode};
use serde::Deserialize;

use crate::input::{AxisWatcher, ButtonWatcher, InputWatcher};
use super::codes::{parse_axis_code, parse_key_code};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum InputDef {
    Button(String), // handle "BTN_LEFT"
    KeyCode(u16),   // handle 280
    Axis(AxisDef),  // handle { axis: "REL_Y", delta: -1 }
}

#[derive(Deserialize)]
pub struct AxisDef {
    pub axis: String,
    pub delta: Delta,
    #[serde(default)]
    pub noisy: bool,
}

#[derive(Deserialize, PartialEq)]
#[serde(try_from = "i8")]
pub enum Delta {
    Positive,
    Negative,
}

impl TryFrom<i8> for Delta {
    type Error = String;
    fn try_from(v: i8) -> std::result::Result<Self, Self::Error> {
        match v {
            1  => Ok(Delta::Positive),
            -1 => Ok(Delta::Negative),
            _  => Err(format!("delta must be 1 or -1, got {}", v)),
        }
    }
}

impl InputDef {
    pub fn physical_key(&self) -> Option<KeyCode> {
        match self {
            InputDef::Button(code_str) => parse_key_code(code_str).ok(),
            InputDef::KeyCode(code) => Some(KeyCode::new(*code)),
            InputDef::Axis(_) => None,
        }
    }

    pub fn into_watcher(
        self,
        name: String,
        on_button_down: impl FnMut() + Send + 'static,
        on_button_up: impl FnMut() + Send + 'static,
        mut on_axis: impl FnMut(RelativeAxisCode, i32) + Send + 'static,
        print_noisy: bool,
    ) -> Result<Box<dyn InputWatcher>> {
        match self {
            InputDef::Button(code_str) => {
                let key_code = parse_key_code(&code_str)?;
                Ok(Box::new(ButtonWatcher::new(key_code, on_button_down, on_button_up)))
            }
            InputDef::KeyCode(code) => {
                let key_code = KeyCode::new(code);
                Ok(Box::new(ButtonWatcher::new(key_code, on_button_down, on_button_up)))
            }
            InputDef::Axis(AxisDef { axis, delta, noisy }) => {
                let axis_code = parse_axis_code(&axis)?;
                let should_print = !noisy || print_noisy;
                Ok(Box::new(AxisWatcher::new(
                    axis_code,
                    delta == Delta::Positive,
                    1,
                    move |d| {
                        if should_print { println!("{}", name) }
                        on_axis(axis_code, d);
                    },
                )))
            }
        }
    }
}

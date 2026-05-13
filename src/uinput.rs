use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode};
use crate::mappings::OutputSink;

pub struct UInputSink {
    device: VirtualDevice,
}

impl UInputSink {
    pub fn new(keys: impl IntoIterator<Item = KeyCode>) -> std::io::Result<Self> {
        let mut key_set = AttributeSet::<KeyCode>::new();
        for k in keys {
            key_set.insert(k);
        }
        let mut axis_set = AttributeSet::<RelativeAxisCode>::new();
        for axis in [
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_HWHEEL,
        ] {
            axis_set.insert(axis);
        }
        let device = VirtualDevice::builder()?
            .name("steam-wrapper")
            .with_keys(&key_set)?
            .with_relative_axes(&axis_set)?
            .build()?;
        Ok(Self { device })
    }
}

impl OutputSink for UInputSink {
    fn press(&mut self, key: KeyCode) {
        let _ = self.device.emit(&[
            InputEvent::new(EventType::KEY.0, key.0, 1),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ]);
    }
    fn release(&mut self, key: KeyCode) {
        let _ = self.device.emit(&[
            InputEvent::new(EventType::KEY.0, key.0, 0),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ]);
    }
    fn relay_axis(&mut self, axis: RelativeAxisCode, delta: i32) {
        let _ = self.device.emit(&[
            InputEvent::new(EventType::RELATIVE.0, axis.0, delta),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ]);
    }
}

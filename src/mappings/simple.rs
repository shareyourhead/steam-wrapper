use evdev::KeyCode;
use super::{Mapping, OutputSink};

#[allow(dead_code)]
pub struct SimpleKey {
    key: KeyCode,
}

impl SimpleKey {
    #[allow(dead_code)]
    pub fn new(key: KeyCode) -> Self {
        Self { key }
    }
}

impl Mapping for SimpleKey {
    fn on_down(&mut self, sink: &mut dyn OutputSink) {
        sink.press(self.key);
    }
    fn on_up(&mut self, sink: &mut dyn OutputSink) {
        sink.release(self.key);
    }
}

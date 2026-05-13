use evdev::KeyCode;
use super::{Mapping, OutputSink};

#[allow(dead_code)]
pub struct MultiKey {
    keys: Vec<KeyCode>,
}

impl MultiKey {
    #[allow(dead_code)]
    pub fn new(keys: Vec<KeyCode>) -> Self {
        Self { keys }
    }
}

impl Mapping for MultiKey {
    fn on_down(&mut self, sink: &mut dyn OutputSink) {
        for &key in &self.keys {
            sink.press(key);
        }
    }
    fn on_up(&mut self, sink: &mut dyn OutputSink) {
        for &key in &self.keys {
            sink.release(key);
        }
    }
}

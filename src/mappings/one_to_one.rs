use evdev::KeyCode;
use super::{Mapping, OutputSink};

pub struct OneToOne {
    #[allow(dead_code)]
    key: KeyCode,
}

impl OneToOne {
    pub fn new(key: KeyCode) -> Self {
        Self { key }
    }
}

impl Mapping for OneToOne {
    fn on_down(&mut self, sink: &mut dyn OutputSink) {
        sink.press(self.key);
    }
    fn on_up(&mut self, sink: &mut dyn OutputSink) {
        sink.release(self.key);
    }
}

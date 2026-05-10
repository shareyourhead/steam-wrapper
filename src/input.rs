use evdev::{EventSummary, InputEvent, KeyCode, RelativeAxisCode};

pub trait InputWatcher: Send {
    fn handle_event(&mut self, event: &InputEvent);
}

// ---------------------------------------------------------------------------
// ButtonWatcher — discrete keys/buttons (BTN_*, KEY_*)
//
// evdev key values:
//   1 = pressed  (on_down)
//   0 = released (on_up)
//   2 = autorepeat (ignored)
// ---------------------------------------------------------------------------

pub struct ButtonWatcher {
    key_code: KeyCode,
    on_down: Box<dyn FnMut() + Send>,
    on_up: Box<dyn FnMut() + Send>,
}

impl ButtonWatcher {
    pub fn new(
        key_code: KeyCode,
        on_down: impl FnMut() + Send + 'static,
        on_up: impl FnMut() + Send + 'static,
    ) -> Self {
        Self {
            key_code,
            on_down: Box::new(on_down),
            on_up: Box::new(on_up),
        }
    }
}

impl InputWatcher for ButtonWatcher {
    fn handle_event(&mut self, event: &InputEvent) {
        if let EventSummary::Key(_ev, code, value) = event.destructure() {
            if code == self.key_code {
                match value {
                    1 => (self.on_down)(),
                    0 => (self.on_up)(),
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AxisWatcher — relative axes (REL_X, REL_Y, REL_WHEEL, REL_HWHEEL)
//
// REL events carry a delta each poll. direction=true fires on positive deltas,
// direction=false on negative. threshold filters jitter.
// ---------------------------------------------------------------------------

pub struct AxisWatcher {
    axis: RelativeAxisCode,
    /// true = watch positive deltas, false = watch negative deltas
    direction: bool,
    threshold: i32,
    on_move: Box<dyn FnMut(i32) + Send>,
}

impl AxisWatcher {
    pub fn new(
        axis: RelativeAxisCode,
        direction: bool,
        threshold: i32,
        on_move: impl FnMut(i32) + Send + 'static,
    ) -> Self {
        Self {
            axis,
            direction,
            threshold,
            on_move: Box::new(on_move),
        }
    }
}

impl InputWatcher for AxisWatcher {
    fn handle_event(&mut self, event: &InputEvent) {
        if let EventSummary::RelativeAxis(_ev, code, value) = event.destructure() {
            if code == self.axis && value.abs() >= self.threshold {
                if (value > 0) == self.direction {
                    (self.on_move)(value);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

pub async fn run_event_loop(
    mut stream: evdev::EventStream,
    watchers: &mut Vec<Box<dyn InputWatcher>>,
) -> std::io::Result<()> {
    loop {
        let event = stream.next_event().await?;
        for watcher in watchers.iter_mut() {
            watcher.handle_event(&event);
        }
    }
}

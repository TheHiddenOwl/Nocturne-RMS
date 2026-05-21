use std::time::Duration;

pub struct ReconnectStrategy {
    base_delay: Duration,
    max_delay: Duration,
    current_delay: Duration,
}

impl ReconnectStrategy {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            current_delay: base_delay,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current_delay;
        self.current_delay = (self.current_delay * 2).min(self.max_delay);
        delay
    }

    pub fn reset(&mut self) {
        self.current_delay = self.base_delay;
    }
}

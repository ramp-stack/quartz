#[derive(Debug, Clone)]
pub struct Timer {
    duration:  f32,
    elapsed:   f32,
    looped:    bool,
    finished:  bool,
}

impl Timer {
    /// Create a one-shot timer that fires once after `duration` seconds.
    pub fn new(duration: f32) -> Self {
        Self { duration: duration.max(0.001), elapsed: 0.0, looped: false, finished: false }
    }

    /// Create a looping timer that resets every `duration` seconds.
    pub fn new_looped(duration: f32) -> Self {
        Self { duration: duration.max(0.001), elapsed: 0.0, looped: true, finished: false }
    }

    /// Advance the timer by `dt` seconds. Returns `true` on the frame
    /// the timer fires (once for one-shot, every cycle for looped).
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.finished { return false; }

        self.elapsed += dt;

        if self.elapsed >= self.duration {
            if self.looped {
                self.elapsed -= self.duration;
                true
            } else {
                self.elapsed = self.duration;
                self.finished = true;
                true
            }
        } else {
            false
        }
    }

    /// Progress from 0.0 (start) to 1.0 (done/cycle complete).
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).min(1.0)
    }

    /// True once a one-shot timer has completed. Always false for looped timers.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Reset the timer to the beginning.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
    }

    /// Remaining time in seconds.
    pub fn remaining(&self) -> f32 {
        (self.duration - self.elapsed).max(0.0)
    }

    /// The configured duration.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Current elapsed time.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }
}

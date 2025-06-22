use std::time::Instant;

/// Frame timing statistics for the renderer.
///
/// `TimeStats` tracks how much time has elapsed since
/// the renderer started as well as the delta time between
/// consecutive frames. All times are measured in seconds.
pub struct TimeStats {
    start_time: Instant,
    prev_frame: Instant,
    /// Seconds since [`TimeStats`] was created.
    pub total_time: f32,
    /// Seconds since the previous `update` call.
    pub delta_time: f32,
}

impl Default for TimeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeStats {
    /// Create a new timer starting at the current instant.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            prev_frame: now,
            total_time: 0.0,
            delta_time: 0.0,
        }
    }

    /// Update timing statistics for the current frame.
    ///
    /// `total_time` becomes the elapsed time since creation and
    /// `delta_time` is the time since the last `update`.
    pub fn update(&mut self) {
        let now = Instant::now();
        self.total_time = (now - self.start_time).as_secs_f32();
        self.delta_time = (now - self.prev_frame).as_secs_f32();
        self.prev_frame = now;
    }
}

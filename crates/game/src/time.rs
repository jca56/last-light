use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since Unix epoch.
pub type Timestamp = u64;

/// Duration in milliseconds.
pub type DurationMs = u64;

pub const SECOND: DurationMs = 1_000;
pub const MINUTE: DurationMs = 60 * SECOND;

pub fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as u64
}

pub fn is_complete(started_at: Timestamp, duration: DurationMs) -> bool {
    now() >= started_at + duration
}

pub fn remaining(started_at: Timestamp, duration: DurationMs) -> DurationMs {
    let end = started_at + duration;
    let current = now();
    if current >= end { 0 } else { end - current }
}

pub fn progress_fraction(started_at: Timestamp, duration: DurationMs) -> f64 {
    let elapsed = now().saturating_sub(started_at);
    (elapsed as f64 / duration as f64).min(1.0)
}

//! Progress maths for the in-process encode (ADR-0013).
//!
//! The transcode loop reports position from the muxed video packet PTS; this
//! module turns that position (in microseconds) plus the probed duration into
//! the clamped percentage the frontend shows. v1's `-progress pipe:1` text
//! parser is gone (ADR-0027).

/// Real percentage from the probed duration (ADR-0013): `out_time ÷ duration`,
/// clamped to 0–100. A zero/unknown duration yields 0 (no divide-by-zero).
pub fn percent_of(out_time_us: i64, duration_secs: f64) -> f64 {
    if duration_secs <= 0.0 {
        return 0.0;
    }
    let elapsed_secs = out_time_us as f64 / 1_000_000.0;
    (elapsed_secs / duration_secs * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_uses_probed_duration() {
        assert_eq!(percent_of(5_000_000, 10.0), 50.0);
        assert_eq!(percent_of(10_000_000, 10.0), 100.0);
    }

    #[test]
    fn percent_clamps_and_guards_zero_duration() {
        assert_eq!(percent_of(20_000_000, 10.0), 100.0); // clamp overshoot
        assert_eq!(percent_of(5_000_000, 0.0), 0.0); // no divide-by-zero
    }
}

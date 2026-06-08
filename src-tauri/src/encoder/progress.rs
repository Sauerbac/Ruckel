//! Parses FFmpeg `-progress pipe:1` key=value lines into progress updates
//! (ADR-0013). FFmpeg writes one `key=value` per line and closes each block
//! with a `progress=continue|end` line; we accumulate the keys we care about
//! and emit a snapshot when the block closes.

/// A closed `-progress` block: the latest known position and rate.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSnapshot {
    /// Microseconds of output produced so far (`out_time_us`).
    pub out_time_us: i64,
    pub fps: f64,
    /// Encode speed as a multiple of realtime (the `x` is stripped).
    pub speed: f64,
    /// True once FFmpeg reports `progress=end`.
    pub finished: bool,
}

/// Accumulates `-progress` lines across one block.
#[derive(Debug, Default)]
pub struct ProgressParser {
    out_time_us: i64,
    fps: f64,
    speed: f64,
}

impl ProgressParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one line of FFmpeg's `-progress` stream. Returns a snapshot only
    /// when a `progress=` line closes the current block; otherwise updates the
    /// running state and returns `None`. Unparseable values (e.g. early
    /// `N/A`) are ignored, keeping the last good value.
    pub fn feed_line(&mut self, line: &str) -> Option<ProgressSnapshot> {
        let (key, value) = line.trim().split_once('=')?;
        let value = value.trim();
        match key {
            "out_time_us" => {
                if let Ok(v) = value.parse::<i64>() {
                    self.out_time_us = v;
                }
            }
            "fps" => {
                if let Ok(v) = value.parse::<f64>() {
                    self.fps = v;
                }
            }
            "speed" => {
                if let Ok(v) = value.trim_end_matches('x').trim().parse::<f64>() {
                    self.speed = v;
                }
            }
            "progress" => {
                return Some(ProgressSnapshot {
                    out_time_us: self.out_time_us,
                    fps: self.fps,
                    speed: self.speed,
                    finished: value == "end",
                });
            }
            _ => {}
        }
        None
    }
}

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

    fn feed_block(parser: &mut ProgressParser, lines: &[&str]) -> Option<ProgressSnapshot> {
        let mut last = None;
        for line in lines {
            if let Some(snap) = parser.feed_line(line) {
                last = Some(snap);
            }
        }
        last
    }

    #[test]
    fn emits_snapshot_only_when_block_closes() {
        let mut p = ProgressParser::new();
        assert_eq!(p.feed_line("fps=24.0"), None);
        assert_eq!(p.feed_line("out_time_us=2000000"), None);
        let snap = p.feed_line("progress=continue").unwrap();
        assert_eq!(snap.out_time_us, 2_000_000);
        assert_eq!(snap.fps, 24.0);
        assert!(!snap.finished);
    }

    #[test]
    fn strips_x_suffix_from_speed_and_flags_end() {
        let mut p = ProgressParser::new();
        let snap = feed_block(
            &mut p,
            &[
                "frame=120",
                "fps=30.0",
                "out_time_us=4000000",
                "speed=1.85x",
                "progress=end",
            ],
        )
        .unwrap();
        assert_eq!(snap.speed, 1.85);
        assert!(snap.finished);
    }

    #[test]
    fn ignores_na_values_and_keeps_last_good() {
        let mut p = ProgressParser::new();
        p.feed_line("out_time_us=1000000");
        // A later N/A must not clobber the known position.
        p.feed_line("out_time_us=N/A");
        let snap = p.feed_line("progress=continue").unwrap();
        assert_eq!(snap.out_time_us, 1_000_000);
    }

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

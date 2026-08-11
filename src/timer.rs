#![allow(dead_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::TimerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone)]
pub struct Timer {
    phase: Phase,
    status: Status,
    remaining: Duration,
    completed_focus_sessions: u32,
}

impl Timer {
    pub fn new(config: &TimerConfig) -> Self {
        Self {
            phase: Phase::Focus,
            status: Status::Idle,
            remaining: Self::phase_duration_of(config, Phase::Focus),
            completed_focus_sessions: 0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn remaining(&self) -> Duration {
        self.remaining
    }

    pub fn completed_focus_sessions(&self) -> u32 {
        self.completed_focus_sessions
    }

    pub fn start(&mut self) {
        if self.status != Status::Running {
            self.status = Status::Running;
        }
    }

    pub fn pause(&mut self) {
        if self.status == Status::Running {
            self.status = Status::Paused;
        }
    }

    pub fn toggle(&mut self) {
        self.status = if self.status == Status::Running {
            Status::Paused
        } else {
            Status::Running
        };
    }

    pub fn reset(&mut self, config: &TimerConfig) {
        self.status = Status::Idle;
        self.remaining = self.phase_duration(config);
    }

    pub fn skip(&mut self, config: &TimerConfig) -> Phase {
        self.advance(config)
    }

    pub fn tick(&mut self, dt: Duration, config: &TimerConfig) -> Option<Phase> {
        if self.status != Status::Running {
            return None;
        }
        self.remaining = self.remaining.saturating_sub(dt);
        if self.remaining.is_zero() {
            Some(self.advance(config))
        } else {
            None
        }
    }

    pub fn progress(&self, config: &TimerConfig) -> f64 {
        let total = self.phase_duration(config).as_secs_f64();
        if total <= 0.0 {
            return 1.0;
        }
        let p = 1.0 - self.remaining.as_secs_f64() / total;
        p.clamp(0.0, 1.0)
    }

    pub fn remaining_mmss(&self) -> String {
        let secs = self.remaining.as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    pub fn phase_duration(&self, config: &TimerConfig) -> Duration {
        Self::phase_duration_of(config, self.phase)
    }

    pub fn advance(&mut self, config: &TimerConfig) -> Phase {
        let next = match self.phase {
            Phase::Focus => {
                self.completed_focus_sessions += 1;
                let cycle = config.cycles_before_long_break.max(1);
                if self.completed_focus_sessions.is_multiple_of(cycle) {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak | Phase::LongBreak => Phase::Focus,
        };
        self.phase = next;
        self.remaining = Self::phase_duration_of(config, next);
        self.status = match next {
            Phase::Focus if config.auto_start_focus => Status::Running,
            Phase::Focus => Status::Idle,
            _ if config.auto_start_breaks => Status::Running,
            _ => Status::Idle,
        };
        next
    }

    fn phase_duration_of(config: &TimerConfig, phase: Phase) -> Duration {
        let minutes = match phase {
            Phase::Focus => config.focus_minutes,
            Phase::ShortBreak => config.short_break_minutes,
            Phase::LongBreak => config.long_break_minutes,
        };
        Duration::from_secs(u64::from(minutes) * 60)
    }

    // --- Persistence helpers (pure logic; no GTK) ---

    pub fn snapshot(&self) -> TimerSnapshot {
        TimerSnapshot {
            version: 1,
            phase: self.phase,
            status: self.status,
            remaining_secs: self.remaining.as_secs(),
            completed_focus_sessions: self.completed_focus_sessions,
            saved_at: Utc::now(),
        }
    }

    pub fn restore(snapshot: TimerSnapshot, config: &TimerConfig) -> Self {
        Self::restore_at(snapshot, config, Utc::now())
    }

    pub fn restore_at(snapshot: TimerSnapshot, config: &TimerConfig, now: DateTime<Utc>) -> Self {
        let mut remaining = Duration::from_secs(snapshot.remaining_secs);
        let mut phase = snapshot.phase;
        let mut completed = snapshot.completed_focus_sessions;
        let mut status = snapshot.status;

        if status == Status::Running {
            let elapsed = now
                .signed_duration_since(snapshot.saved_at)
                .num_seconds()
                .max(0) as u64;
            let mut elapsed_dur = Duration::from_secs(elapsed);

            // Fast-forward through elapsed wall time. Cap iterations to avoid pathological loops.
            for _ in 0..32 {
                if elapsed_dur < remaining {
                    remaining -= elapsed_dur;
                    break;
                }
                // Current phase finished while we were away.
                elapsed_dur -= remaining;
                // Advance once (reuse the same logic as Timer::advance but without needing self).
                let next = match phase {
                    Phase::Focus => {
                        completed += 1;
                        let cycle = config.cycles_before_long_break.max(1);
                        if completed.is_multiple_of(cycle) {
                            Phase::LongBreak
                        } else {
                            Phase::ShortBreak
                        }
                    }
                    Phase::ShortBreak | Phase::LongBreak => Phase::Focus,
                };
                phase = next;
                remaining = Self::phase_duration_of(config, next);
                status = match next {
                    Phase::Focus if config.auto_start_focus => Status::Running,
                    Phase::Focus => Status::Idle,
                    _ if config.auto_start_breaks => Status::Running,
                    _ => Status::Idle,
                };
                if status != Status::Running {
                    // Stopped; remaining stays at full duration, discard leftover elapsed.
                    break;
                }
            }
        }

        Self {
            phase,
            status,
            remaining,
            completed_focus_sessions: completed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimerSnapshot {
    pub version: u32,
    pub phase: Phase,
    pub status: Status,
    pub remaining_secs: u64,
    pub completed_focus_sessions: u32,
    pub saved_at: DateTime<Utc>,
}

pub fn timer_state_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tomato")
        .join("timer.json")
}

pub fn load_timer_snapshot() -> Option<TimerSnapshot> {
    load_timer_snapshot_from(&timer_state_path())
}

pub fn save_timer_snapshot(snapshot: &TimerSnapshot) -> anyhow::Result<()> {
    save_timer_snapshot_to(snapshot, &timer_state_path())
}

fn load_timer_snapshot_from(path: &Path) -> Option<TimerSnapshot> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => return None,
        Err(e) => {
            eprintln!("tomato: failed to read timer state {}: {e}", path.display());
            return None;
        }
    };
    match serde_json::from_str(&content) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("tomato: corrupt timer state {}, backing up: {e}", path.display());
            let _ = fs::rename(path, path.with_extension("json.bak"));
            None
        }
    }
}

fn save_timer_snapshot_to(snapshot: &TimerSnapshot, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(snapshot)?;
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_advances_to_long_break_at_cycle_boundary() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.completed_focus_sessions = 3;
        t.start();
        let next = t.tick(Duration::from_secs(25 * 60), &cfg);
        assert_eq!(next, Some(Phase::LongBreak));
        assert_eq!(t.phase(), Phase::LongBreak);
        assert_eq!(t.completed_focus_sessions(), 4);
        assert_eq!(t.remaining(), Duration::from_secs(15 * 60));
    }

    #[test]
    fn focus_advances_to_short_break_off_boundary() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        let next = t.tick(Duration::from_secs(25 * 60), &cfg);
        assert_eq!(next, Some(Phase::ShortBreak));
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(t.completed_focus_sessions(), 1);
        assert_eq!(t.remaining(), Duration::from_secs(5 * 60));
    }

    #[test]
    fn paused_tick_does_not_decrement() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        t.pause();
        let before = t.remaining();
        assert_eq!(t.tick(Duration::from_secs(60), &cfg), None);
        assert_eq!(t.remaining(), before);
    }

    #[test]
    fn reset_restores_full_phase_duration() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        t.tick(Duration::from_secs(60), &cfg);
        t.reset(&cfg);
        assert_eq!(t.status(), Status::Idle);
        assert_eq!(t.remaining(), Duration::from_secs(25 * 60));
    }

    #[test]
    fn skip_advances_phase() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        let next = t.skip(&cfg);
        assert_eq!(next, Phase::ShortBreak);
        assert_eq!(t.completed_focus_sessions(), 1);
        assert_eq!(t.remaining(), Duration::from_secs(5 * 60));
    }

    #[test]
    fn break_returns_to_focus_and_auto_starts() {
        let mut cfg = TimerConfig::default();
        cfg.auto_start_focus = true;
        let mut t = Timer::new(&cfg);
        t.skip(&cfg);
        t.skip(&cfg);
        assert_eq!(t.phase(), Phase::Focus);
        assert_eq!(t.status(), Status::Running);
        assert_eq!(t.remaining(), Duration::from_secs(25 * 60));
    }

    #[test]
    fn progress_and_mmss_are_correct() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        t.tick(Duration::from_secs(25 * 60 / 2), &cfg);
        let p = t.progress(&cfg);
        assert!((p - 0.5).abs() < 0.001);
        assert_eq!(t.remaining_mmss(), "12:30");
    }

    #[test]
    fn snapshot_round_trips() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        t.tick(Duration::from_secs(60), &cfg);
        let snap = t.snapshot();
        let restored = Timer::restore_at(snap, &cfg, Utc::now());
        assert_eq!(restored.phase(), t.phase());
        assert_eq!(restored.status(), t.status());
        assert_eq!(restored.remaining(), t.remaining());
        assert_eq!(restored.completed_focus_sessions(), t.completed_focus_sessions());
    }

    #[test]
    fn restore_running_subtracts_elapsed() {
        let cfg = TimerConfig::default();
        let mut t = Timer::new(&cfg);
        t.start();
        // Pretend we saved 90s ago with 25:00 remaining
        let saved_at = Utc::now() - chrono::Duration::seconds(90);
        let snap = TimerSnapshot {
            version: 1,
            phase: Phase::Focus,
            status: Status::Running,
            remaining_secs: 25 * 60,
            completed_focus_sessions: 0,
            saved_at,
        };
        let restored = Timer::restore_at(snap, &cfg, Utc::now());
        // 90s elapsed, so ~23:30 left (allow 2s drift for test execution)
        let rem = restored.remaining().as_secs();
        assert!(rem >= 23 * 60 && rem <= 24 * 60, "remaining {rem}s not in 23..24m");
        assert_eq!(restored.phase(), Phase::Focus);
        assert_eq!(restored.status(), Status::Running);
    }

    #[test]
    fn restore_paused_ignores_elapsed() {
        let cfg = TimerConfig::default();
        let saved_at = Utc::now() - chrono::Duration::seconds(3600);
        let snap = TimerSnapshot {
            version: 1,
            phase: Phase::Focus,
            status: Status::Paused,
            remaining_secs: 10 * 60,
            completed_focus_sessions: 2,
            saved_at,
        };
        let restored = Timer::restore_at(snap, &cfg, Utc::now());
        assert_eq!(restored.remaining(), Duration::from_secs(10 * 60));
        assert_eq!(restored.status(), Status::Paused);
        assert_eq!(restored.completed_focus_sessions(), 2);
    }

    #[test]
    fn restore_running_advances_when_elapsed_exceeds_remaining() {
        let mut cfg = TimerConfig::default();
        cfg.auto_start_breaks = true;
        // Saved with 30s remaining, 3600s ago -> should have advanced at least one focus
        let snap = TimerSnapshot {
            version: 1,
            phase: Phase::Focus,
            status: Status::Running,
            remaining_secs: 30,
            completed_focus_sessions: 0,
            saved_at: Utc::now() - chrono::Duration::seconds(3600),
        };
        let restored = Timer::restore_at(snap, &cfg, Utc::now());
        // With default auto_start_focus=false, after Focus->ShortBreak->Focus we stop at Idle Focus.
        assert_eq!(restored.completed_focus_sessions(), 1);
        assert_eq!(restored.status(), Status::Idle);
        assert_eq!(restored.phase(), Phase::Focus);
    }

    #[test]
    fn save_and_load_snapshot_round_trip_via_file() {
        let dir = std::env::temp_dir().join(format!("tomato-timer-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("timer.json");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        let _ = fs::remove_file(path.with_extension("json.bak"));
        let snap = TimerSnapshot {
            version: 1,
            phase: Phase::ShortBreak,
            status: Status::Idle,
            remaining_secs: 5 * 60,
            completed_focus_sessions: 1,
            saved_at: Utc::now(),
        };
        save_timer_snapshot_to(&snap, &path).unwrap();
        let loaded = load_timer_snapshot_from(&path).unwrap();
        assert_eq!(loaded.phase, Phase::ShortBreak);
        assert_eq!(loaded.remaining_secs, 5 * 60);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_snapshot_returns_none_and_baks() {
        let dir = std::env::temp_dir().join(format!("tomato-timer-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("timer_corrupt.json");
        fs::write(&path, "{ not json").unwrap();
        assert!(load_timer_snapshot_from(&path).is_none());
        assert!(path.with_extension("json.bak").exists());
        let _ = fs::remove_file(path.with_extension("json.bak"));
    }
}

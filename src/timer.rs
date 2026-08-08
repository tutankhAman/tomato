#![allow(dead_code)]

use std::time::Duration;

use crate::config::TimerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

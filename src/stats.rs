#![allow(dead_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DayEntry {
    pub date: String,
    pub sessions: u32,
    pub minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsStore {
    pub version: u32,
    pub entries: Vec<DayEntry>,
}

impl Default for StatsStore {
    fn default() -> Self {
        Self { version: 1, entries: Vec::new() }
    }
}

impl StatsStore {
    pub fn load() -> Self {
        Self::load_from(&stats_path())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&stats_path())
    }

    pub fn record_focus(&mut self, minutes: u32) {
        let today = today_string();
        if let Some(entry) = self.entries.iter_mut().find(|e| e.date == today) {
            entry.sessions += 1;
            entry.minutes += minutes;
        } else {
            self.entries.push(DayEntry { date: today, sessions: 1, minutes });
        }
        // Keep entries sorted by date string (YYYY-MM-DD lexicographically == chronological)
        self.entries.sort_by(|a, b| a.date.cmp(&b.date));
        // Cap to last 60 days to bound file size
        if self.entries.len() > 60 {
            let drain = self.entries.len() - 60;
            self.entries.drain(0..drain);
        }
    }

    pub fn today(&self) -> Option<&DayEntry> {
        let today = today_string();
        self.entries.iter().find(|e| e.date == today)
    }

    pub fn week_totals(&self) -> (u32, u32) {
        // Last 7 days inclusive of today (by date string window)
        let today = Local::now().date_naive();
        let week_ago = today - chrono::Duration::days(6);
        let mut sessions = 0;
        let mut minutes = 0;
        for e in &self.entries {
            if let Ok(d) = NaiveDate::parse_from_str(&e.date, "%Y-%m-%d")
                && d >= week_ago
                && d <= today
            {
                sessions += e.sessions;
                minutes += e.minutes;
            }
        }
        (sessions, minutes)
    }

    pub fn today_totals(&self) -> (u32, u32) {
        self.today().map(|e| (e.sessions, e.minutes)).unwrap_or((0, 0))
    }

    fn load_from(path: &Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!("tomato: failed to read stats store {}: {e}", path.display());
                return Self::default();
            }
        };
        match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("tomato: corrupt stats store {}, backing up: {e}", path.display());
                let _ = fs::rename(path, path.with_extension("json.bak"));
                Self::default()
            }
        }
    }

    fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&tmp, content)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

pub fn stats_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("tomato").join("stats.json")
}

fn today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tomato-stats-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{name}.json"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.tmp"));
        let _ = fs::remove_file(path.with_extension("json.bak"));
        path
    }

    #[test]
    fn record_and_week_totals() {
        let mut s = StatsStore::default();
        s.record_focus(25);
        s.record_focus(25);
        assert_eq!(s.today_totals().0, 2);
        assert_eq!(s.today_totals().1, 50);
        let (w_s, w_m) = s.week_totals();
        assert_eq!(w_s, 2);
        assert_eq!(w_m, 50);
    }

    #[test]
    fn save_load_round_trip() {
        let path = test_path("roundtrip");
        let mut s = StatsStore::default();
        s.record_focus(25);
        s.save_to(&path).unwrap();
        let loaded = StatsStore::load_from(&path);
        assert_eq!(s, loaded);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_baks_and_returns_default() {
        let path = test_path("corrupt");
        fs::write(&path, "{ not json").unwrap();
        let s = StatsStore::load_from(&path);
        assert!(s.entries.is_empty());
        assert!(path.with_extension("json.bak").exists());
        let _ = fs::remove_file(path.with_extension("json.bak"));
    }

    #[test]
    fn caps_at_sixty_entries() {
        let mut s = StatsStore::default();
        for i in 0..70 {
            s.entries.push(DayEntry { date: format!("2020-01-{i:02}"), sessions: 1, minutes: 25 });
        }
        s.record_focus(25);
        assert!(s.entries.len() <= 60);
    }
}

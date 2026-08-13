#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_focus_minutes() -> u32 {
    25
}
fn default_short_break_minutes() -> u32 {
    5
}
fn default_long_break_minutes() -> u32 {
    15
}
fn default_cycles_before_long_break() -> u32 {
    4
}
fn default_auto_start_breaks() -> bool {
    true
}
fn default_auto_start_focus() -> bool {
    false
}
fn default_cycle_target() -> u32 {
    0
}

fn default_notifications_enabled() -> bool {
    true
}

fn default_anchor() -> String {
    "top-right".to_string()
}
fn default_margin() -> i32 {
    16
}
fn default_opacity() -> f64 {
    0.97
}
fn default_always_on_top() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimerConfig {
    #[serde(default = "default_focus_minutes")]
    pub focus_minutes: u32,
    #[serde(default = "default_short_break_minutes")]
    pub short_break_minutes: u32,
    #[serde(default = "default_long_break_minutes")]
    pub long_break_minutes: u32,
    #[serde(default = "default_cycles_before_long_break")]
    pub cycles_before_long_break: u32,
    #[serde(default = "default_auto_start_breaks")]
    pub auto_start_breaks: bool,
    #[serde(default = "default_auto_start_focus")]
    pub auto_start_focus: bool,
    /// Daily goal of completed cycles; 0 disables the target (counts only).
    #[serde(default = "default_cycle_target")]
    pub cycle_target: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationConfig {
    #[serde(default = "default_notifications_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowConfig {
    #[serde(default = "default_anchor")]
    pub anchor: String,
    #[serde(default = "default_margin")]
    pub margin_x: i32,
    #[serde(default = "default_margin")]
    pub margin_y: i32,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub timer: TimerConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub window: WindowConfig,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            focus_minutes: default_focus_minutes(),
            short_break_minutes: default_short_break_minutes(),
            long_break_minutes: default_long_break_minutes(),
            cycles_before_long_break: default_cycles_before_long_break(),
            auto_start_breaks: default_auto_start_breaks(),
            auto_start_focus: default_auto_start_focus(),
            cycle_target: default_cycle_target(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_notifications_enabled(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            anchor: default_anchor(),
            margin_x: default_margin(),
            margin_y: default_margin(),
            opacity: default_opacity(),
            always_on_top: default_always_on_top(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tomato")
        .join("config.toml")
}

impl Config {
    pub fn load() -> Config {
        let path = config_path();
        let mut cfg: Config = match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("tomato: failed to parse config {}: {e}", path.display());
                    Config::default()
                }
            },
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("tomato: failed to read config {}: {e}", path.display());
                }
                Config::default()
            }
        };
        // Clamp window geometry that can push the layer-shell surface off-screen
        // (e.g. a previous drag bug could persist 12k+ margins). Keep the window
        // at least partially visible on any monitor up to 4K.
        cfg.window.margin_x = cfg.window.margin_x.clamp(0, 4000);
        cfg.window.margin_y = cfg.window.margin_y.clamp(0, 4000);
        cfg.window.opacity = cfg.window.opacity.clamp(0.30, 1.0);
        cfg
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        let content = toml::to_string_pretty(self)?;
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_toml() {
        let cfg = Config::default();
        let s = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn partial_config_defaults_missing_fields() {
        let s = "[timer]\nfocus_minutes = 50\n";
        let cfg: Config = toml::from_str(s).unwrap();
        assert_eq!(cfg.timer.focus_minutes, 50);
        assert_eq!(cfg.timer.short_break_minutes, 5);
        assert_eq!(cfg.timer.long_break_minutes, 15);
        assert_eq!(cfg.timer.cycles_before_long_break, 4);
        assert!(cfg.timer.auto_start_breaks);
        assert!(!cfg.timer.auto_start_focus);
        assert_eq!(cfg.timer.cycle_target, 0);
        assert!(cfg.notifications.enabled);
        assert_eq!(cfg.window.anchor, "top-right");
        assert_eq!(cfg.window.margin_x, 16);
        assert_eq!(cfg.window.margin_y, 16);
        assert_eq!(cfg.window.opacity, 0.97);
        assert!(cfg.window.always_on_top);
    }

    #[test]
    fn legacy_sound_and_compact_keys_are_ignored() {
        let s = "[notifications]\nenabled = false\nsound = true\n[window]\nanchor = \"center\"\ncompact = true\n";
        let cfg: Config = toml::from_str(s).unwrap();
        assert!(!cfg.notifications.enabled);
        assert_eq!(cfg.window.anchor, "center");
    }
}

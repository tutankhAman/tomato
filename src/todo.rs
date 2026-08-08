#![allow(dead_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_id() -> String {
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{millis}-{seq}")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub pomodoros_done: u32,
    pub pomodoros_estimated: u32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoStore {
    pub version: u32,
    #[serde(default)]
    pub active_id: Option<String>,
    pub items: Vec<Todo>,
}

impl Default for TodoStore {
    fn default() -> Self {
        Self {
            version: 1,
            active_id: None,
            items: Vec::new(),
        }
    }
}

impl TodoStore {
    pub fn load() -> Self {
        Self::load_from(&data_path())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&data_path())
    }

    pub fn add(&mut self, title: String) -> String {
        let id = generate_id();
        let todo = Todo {
            id: id.clone(),
            title,
            done: false,
            pomodoros_done: 0,
            pomodoros_estimated: 0,
            created_at: Utc::now(),
            completed_at: None,
        };
        self.items.push(todo);
        id
    }

    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(todo) = self.items.iter_mut().find(|t| t.id == id) {
            todo.done = !todo.done;
            todo.completed_at = if todo.done { Some(Utc::now()) } else { None };
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
        let len = self.items.len();
        self.items.retain(|t| t.id != id);
        self.items.len() != len
    }

    pub fn clear_completed(&mut self) {
        if let Some(ref active) = self.active_id
            && self.items.iter().any(|t| t.id == *active && t.done)
        {
            self.active_id = None;
        }
        self.items.retain(|t| !t.done);
    }

    pub fn increment_pomodoro(&mut self, id: &str) -> bool {
        if let Some(todo) = self.items.iter_mut().find(|t| t.id == id) {
            todo.pomodoros_done += 1;
            true
        } else {
            false
        }
    }

    pub fn active_task(&self) -> Option<&Todo> {
        let active_id = self.active_id.as_deref()?;
        self.items.iter().find(|t| t.id == active_id)
    }

    pub fn set_active(&mut self, id: Option<String>) {
        self.active_id = id;
    }

    pub fn increment_active_pomodoro(&mut self) -> bool {
        let active_id = match &self.active_id {
            Some(id) => id.clone(),
            None => return false,
        };
        self.increment_pomodoro(&active_id)
    }

    pub fn remaining_count(&self) -> usize {
        self.items.iter().filter(|t| !t.done).count()
    }

    fn load_from(path: &Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!("tomato: failed to read todo store {}: {e}", path.display());
                return Self::default();
            }
        };
        match serde_json::from_str(&content) {
            Ok(store) => store,
            Err(e) => {
                eprintln!(
                    "tomato: corrupt todo store {}, backing it up: {e}",
                    path.display()
                );
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

pub fn data_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tomato")
        .join("todos.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tomato-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{name}.json"));
        let tmp = path.with_extension("json.tmp");
        let bak = path.with_extension("json.bak");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&bak);
        path
    }

    #[test]
    fn add_save_load_round_trip() {
        let path = test_path("roundtrip");
        let mut store = TodoStore::default();
        store.add("Ship the ring widget".to_string());
        store.add("Write tests".to_string());
        store.save_to(&path).unwrap();

        let loaded = TodoStore::load_from(&path);
        assert_eq!(store, loaded);
    }

    #[test]
    fn toggle_sets_completed_at() {
        let mut store = TodoStore::default();
        let id = store.add("Do the thing".to_string());
        assert!(store.toggle(&id));
        let todo = store.items.iter().find(|t| t.id == id).unwrap();
        assert!(todo.done);
        assert!(todo.completed_at.is_some());
        assert!(store.toggle(&id));
        let todo = store.items.iter().find(|t| t.id == id).unwrap();
        assert!(!todo.done);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn corrupt_file_yields_empty_store_and_backup() {
        let path = test_path("corrupt");
        fs::write(&path, "{ not json !!!").unwrap();
        let store = TodoStore::load_from(&path);
        assert!(store.items.is_empty());
        assert!(path.with_extension("json.bak").exists());
        assert!(!path.exists());
    }

    #[test]
    fn active_task_management() {
        let mut store = TodoStore::default();
        let id = store.add("Active task".to_string());
        store.set_active(Some(id.clone()));
        assert_eq!(store.active_task().unwrap().title, "Active task");

        assert!(store.increment_active_pomodoro());
        assert_eq!(store.active_task().unwrap().pomodoros_done, 1);

        assert!(store.remove(&id));
        assert!(store.active_task().is_none());
    }
}

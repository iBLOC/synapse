//! `ConfigManager` — lobe configuration with hot-reload support.
//!
//! Each lobe has a config file (YAML or TOML) that specifies its model,
//! providers, and runtime parameters. The manager loads the config at
//! startup, watches the file for changes, and triggers reload callbacks
//! when the file is modified.
//!
//! Hot-reload strategy: when config changes, the callback should mark
//! the agent for restart. The agent checks `should_reload()` between
//! requests and gracefully restarts when no active sessions exist.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::RwLock;
use serde::de::DeserializeOwned;

#[cfg(not(target_arch = "wasm32"))]
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Lobe configuration trait. Each lobe implements this with its own
/// config struct.
pub trait LobeConfig: DeserializeOwned + Clone + Send + Sync + 'static {
    fn config_path() -> &'static str;

    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    fn load_from_file(path: &Path) -> Result<Self, String>
    where
        Self: Sized,
    {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read config: {e}"))?;
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext {
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| format!("yaml parse: {e}")),
            "toml" => toml::from_str(&content)
                .map_err(|e| format!("toml parse: {e}")),
            _ => serde_json::from_str(&content)
                .map_err(|e| format!("json parse: {e}")),
        }
    }
}

/// Configuration manager. Loads config at startup, optionally watches
/// the file for changes, and dispatches reload callbacks.
pub struct ConfigManager<C: LobeConfig> {
    current: Arc<RwLock<C>>,
    path: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    _watcher: Option<RecommendedWatcher>,
    reload_state: Arc<GracefulReload>,
}

impl<C: LobeConfig> ConfigManager<C> {
    pub fn load() -> Result<Self, String> {
        let path = PathBuf::from(C::config_path());
        let config = C::load_from_file(&path)?;
        config.validate()?;
        Ok(Self {
            current: Arc::new(RwLock::new(config)),
            path,
            _watcher: None,
            reload_state: Arc::new(GracefulReload::new()),
        })
    }

    pub fn load_with_watcher<F>(on_change: F) -> Result<Self, String>
    where
        F: Fn(&C) + Send + 'static,
    {
        let mut manager = Self::load()?;
        manager.start_watcher(on_change)?;
        Ok(manager)
    }

    pub fn get(&self) -> C {
        self.current.read().clone()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reload_state(&self) -> Arc<GracefulReload> {
        self.reload_state.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_watcher<F>(&mut self, on_change: F) -> Result<(), String>
    where
        F: Fn(&C) + Send + 'static,
    {
        let path = self.path.clone();
        let current = self.current.clone();
        let reload_state = self.reload_state.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    if let Ok(new_config) = C::load_from_file(&path) {
                        if new_config.validate().is_ok() {
                            *current.write() = new_config.clone();
                            reload_state.mark_reload_pending();
                            on_change(&new_config);
                        }
                    }
                }
            }
        })
        .map_err(|e| format!("watcher init: {e}"))?;

        let watch_path = self.path.parent().unwrap_or(&self.path).to_path_buf();
        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("watch: {e}"))?;

        self._watcher = Some(watcher);
        Ok(())
    }
}

/// Tracks active sessions and pending reloads for graceful restart.
pub struct GracefulReload {
    active_sessions: AtomicUsize,
    reload_pending: std::sync::atomic::AtomicBool,
}

impl GracefulReload {
    pub fn new() -> Self {
        Self {
            active_sessions: AtomicUsize::new(0),
            reload_pending: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn session_started(&self) {
        self.active_sessions.fetch_add(1, Ordering::SeqCst);
    }

    pub fn session_ended(&self) {
        self.active_sessions.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn mark_reload_pending(&self) {
        self.reload_pending.store(true, Ordering::SeqCst);
    }

    pub fn clear_pending(&self) {
        self.reload_pending.store(false, Ordering::SeqCst);
    }

    pub fn should_reload(&self) -> bool {
        self.reload_pending.load(Ordering::SeqCst)
            && self.active_sessions.load(Ordering::SeqCst) == 0
    }

    pub fn active_count(&self) -> usize {
        self.active_sessions.load(Ordering::SeqCst)
    }
}

impl Default for GracefulReload {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graceful_reload_tracks_sessions() {
        let r = GracefulReload::new();
        assert_eq!(r.active_count(), 0);
        assert!(!r.should_reload());

        r.session_started();
        r.session_started();
        assert_eq!(r.active_count(), 2);

        r.mark_reload_pending();
        assert!(!r.should_reload());

        r.session_ended();
        r.session_ended();
        assert_eq!(r.active_count(), 0);
        assert!(r.should_reload());

        r.clear_pending();
        assert!(!r.should_reload());
    }

    #[test]
    fn load_yaml_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(&path, "name: hello\ncount: 42\n").unwrap();

        #[derive(serde::Deserialize, Clone)]
        struct TestConfig {
            name: String,
            count: u32,
        }

        impl LobeConfig for TestConfig {
            fn config_path() -> &'static str {
                "/nonexistent"
            }
        }

        let config = TestConfig::load_from_file(&path).unwrap();
        assert_eq!(config.name, "hello");
        assert_eq!(config.count, 42);
    }

    #[test]
    fn load_json_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        std::fs::write(&path, r#"{"name":"world","count":7}"#).unwrap();

        #[derive(serde::Deserialize, Clone)]
        struct TestConfig {
            name: String,
            count: u32,
        }

        impl LobeConfig for TestConfig {
            fn config_path() -> &'static str {
                "/nonexistent"
            }
        }

        let config = TestConfig::load_from_file(&path).unwrap();
        assert_eq!(config.name, "world");
        assert_eq!(config.count, 7);
    }
}

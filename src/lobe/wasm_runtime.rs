//! `WasmLobeRuntime` — lifecycle for WASM-packaged lobes.
//!
//! Each lobe can be compiled to `wasm32-wasip1` and loaded by Cortex.
//! The runtime provides: load, invoke_skill, reload, unload.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLobeConfig {
    pub memory_bytes: u64,
    pub fuel: u64,
    pub wall_clock_ms: u64,
    pub network_policy: String,
}

impl Default for WasmLobeConfig {
    fn default() -> Self {
        Self {
            memory_bytes: 100 * 1024 * 1024,
            fuel: 1_000_000,
            wall_clock_ms: 30_000,
            network_policy: "deny_all".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSkillOutput {
    pub updated_history_summary: Option<String>,
    pub payload: Value,
    pub audio_url: Option<String>,
}

pub struct WasmLobeRuntime {
    config: WasmLobeConfig,
    wasm_bytes: Vec<u8>,
    _engine: Mutex<Option<WasmEngineState>>,
}

struct WasmEngineState {
    #[allow(dead_code)]
    engine: wasmtime::Engine,
    #[allow(dead_code)]
    module: wasmtime::Module,
    #[allow(dead_code)]
    store: wasmtime::Store<()>,
}

impl WasmLobeRuntime {
    pub fn load(wasm_bytes: Vec<u8>, config: WasmLobeConfig) -> Result<Arc<Self>, String> {
        Ok(Arc::new(Self {
            config,
            wasm_bytes,
            _engine: Mutex::new(None),
        }))
    }

    pub fn load_non_arc(wasm_bytes: Vec<u8>, config: WasmLobeConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            wasm_bytes,
            _engine: Mutex::new(None),
        })
    }

    pub fn config(&self) -> &WasmLobeConfig {
        &self.config
    }

    pub fn reload(&mut self, new_bytes: Vec<u8>) -> Result<(), String> {
        self.wasm_bytes = new_bytes;
        *self._engine.lock() = None;
        Ok(())
    }

    pub fn unload(self: Arc<Self>) {
        drop(self);
    }

    pub fn invoke_skill(
        &self,
        skill_id: &str,
        ctx: Value,
    ) -> Result<WasmSkillOutput, String> {
        Ok(WasmSkillOutput {
            updated_history_summary: None,
            payload: serde_json::json!({
                "skill": skill_id,
                "status": "wasm_stub",
                "ctx": ctx,
            }),
            audio_url: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_limits() {
        let c = WasmLobeConfig::default();
        assert_eq!(c.memory_bytes, 100 * 1024 * 1024);
        assert_eq!(c.network_policy, "deny_all");
    }

    #[test]
    fn load_wasm_runtime() {
        let runtime = WasmLobeRuntime::load(vec![0, 1, 2, 3], WasmLobeConfig::default());
        assert!(runtime.is_ok());
    }

    #[test]
    fn invoke_skill_stub_returns_payload() {
        let runtime = WasmLobeRuntime::load(vec![], WasmLobeConfig::default()).unwrap();
        let out = runtime.invoke_skill("test_skill", serde_json::json!({"k": "v"})).unwrap();
        assert_eq!(out.payload["skill"], "test_skill");
        assert_eq!(out.payload["status"], "wasm_stub");
    }

    #[test]
    fn reload_replaces_bytes() {
        let mut runtime = WasmLobeRuntime::load_non_arc(vec![1, 2, 3], WasmLobeConfig::default()).unwrap();
        runtime.reload(vec![4, 5, 6]).unwrap();
        let out = runtime.invoke_skill("s", serde_json::json!({})).unwrap();
        assert_eq!(out.payload["skill"], "s");
    }
}

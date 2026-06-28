use super::package::{PackageId, PackageVersion, WasmPackage};
use std::path::Path;
use wasmtime::{Config, Engine, Memory, MemoryType, Module, Store};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub fuel_limit: Option<u64>,
    pub memory_limit_mb: Option<u32>,
    pub max_stack_kb: Option<u32>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            fuel_limit: Some(10_000_000),
            memory_limit_mb: Some(128),
            max_stack_kb: Some(1024),
        }
    }
}

impl RuntimeConfig {
    pub fn strict() -> Self {
        Self {
            fuel_limit: Some(1_000_000),
            memory_limit_mb: Some(64),
            max_stack_kb: Some(512),
        }
    }

    pub fn permissive() -> Self {
        Self {
            fuel_limit: None,
            memory_limit_mb: Some(512),
            max_stack_kb: Some(4096),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: String,
    pub fuel_consumed: u64,
    pub memory_used_mb: u32,
    pub duration_ms: u64,
}

pub struct WasmRuntime {
    engine: Engine,
    config: RuntimeConfig,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        let mut wasm_config = Config::default();
        wasm_config.consume_fuel(true);
        wasm_config.max_wasm_stack(config.max_stack_kb.unwrap_or(1024) as usize * 1024);

        Self {
            engine: Engine::new(&wasm_config).expect("Failed to create WASM engine"),
            config,
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub fn load_module(&self, package: &WasmPackage) -> Result<Module, String> {
        self.load_module_from_bytes(&package.module)
    }

    pub fn load_module_from_bytes(&self, bytes: &[u8]) -> Result<Module, String> {
        Module::from_binary(&self.engine, bytes)
            .map_err(|e| format!("Failed to load WASM module: {}", e))
    }

    pub fn load_module_from_file(&self, path: impl AsRef<Path>) -> Result<Module, String> {
        Module::from_file(&self.engine, path.as_ref())
            .map_err(|e| format!("Failed to load WASM module: {}", e))
    }

    pub fn execute(&self, module: &Module, entry: &str, input: &str) -> Result<ExecutionResult, String> {
        let start = std::time::Instant::now();
        let mut store = Store::new(&self.engine, ());

        if let Some(fuel) = self.config.fuel_limit {
            store.set_fuel(fuel).map_err(|e| format!("Failed to set fuel: {}", e))?;
        }

        let memory = Memory::new(&mut store, MemoryType::new(1, Some(16)))
            .map_err(|e| format!("Failed to create memory: {}", e))?;

        let instance = wasmtime::Linker::<()>::new(&self.engine)
            .instantiate(&mut store, module)
            .map_err(|e| format!("Failed to instantiate: {}", e))?;

        let memory_used = memory.data_size(&store) / (1024 * 1024);

        let output = self.call_entry(&mut store, &instance, entry, input)?;
        let fuel_consumed = self.config.fuel_limit.unwrap_or(0)
            .saturating_sub(store.get_fuel().unwrap_or(0));
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            output,
            fuel_consumed,
            memory_used_mb: std::cmp::max(1, memory_used as u32),
            duration_ms,
        })
    }

    pub fn execute_package(
        &self,
        package: &WasmPackage,
        input: &str,
    ) -> Result<ExecutionResult, String> {
        let module = self.load_module(package)?;
        self.execute(&module, &package.metadata.entry, input)
    }

    fn call_entry(
        &self,
        store: &mut Store<()>,
        instance: &wasmtime::Instance,
        entry: &str,
        input: &str,
    ) -> Result<String, String> {
        let func = instance
            .get_typed_func::<(i32, i32), i32>(&mut *store, entry)
            .map_err(|e| format!("entry '{}' signature mismatch: {}", entry, e))?;

        let mem = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| "memory export not found".to_string())?;

        let input_bytes = input.as_bytes();
        let ptr = 1024;
        mem.write(&mut *store, ptr, input_bytes)
            .map_err(|e| format!("Failed to write to memory: {}", e))?;

        let result = func
            .call(&mut *store, (ptr as i32, input_bytes.len() as i32))
            .map_err(|e| format!("WASM execution failed: {}", e))?;

        Ok(format!("[wasm:{}] result={} input_len={}", entry, result, input_bytes.len()))
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::PackageMetadata;

    #[test]
    fn test_runtime_creation() {
        let runtime = WasmRuntime::new();
        assert_eq!(runtime.config.fuel_limit, Some(10_000_000));
    }

    #[test]
    fn test_strict_config() {
        let config = RuntimeConfig::strict();
        assert_eq!(config.fuel_limit, Some(1_000_000));
        assert_eq!(config.memory_limit_mb, Some(64));
    }

    #[test]
    fn test_permissive_config() {
        let config = RuntimeConfig::permissive();
        assert!(config.fuel_limit.is_none());
        assert_eq!(config.memory_limit_mb, Some(512));
    }

    #[test]
    fn test_load_module_from_bytes() {
        let runtime = WasmRuntime::new();
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "process") (param i32 i32) (result i32)
                    local.get 0
                )
            )
        "#;

        let bytes = wat::parse_str(wat).unwrap();
        let module = runtime.load_module_from_bytes(&bytes).unwrap();
        let result = runtime.execute(&module, "process", "test input").unwrap();
        assert!(result.memory_used_mb >= 1);
    }

    #[test]
    fn test_load_module_from_file() {
        let dir = std::env::temp_dir().join(format!("wasm-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wasm_path = dir.join("test.wasm");

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "run") (param i32 i32) (result i32)
                    i32.const 42
                )
            )
        "#;
        std::fs::write(&wasm_path, wat::parse_str(wat).unwrap()).unwrap();

        let runtime = WasmRuntime::new();
        let module = runtime.load_module_from_file(&wasm_path).unwrap();
        let result = runtime.execute(&module, "run", "input").unwrap();
        assert!(result.output.contains("wasm:run"));
        assert!(result.output.contains("42"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_execute_package() {
        let runtime = WasmRuntime::new();
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "handle") (param i32 i32) (result i32)
                    local.get 0
                )
            )
        "#;

        let bytes = wat::parse_str(wat).unwrap();
        let meta = PackageMetadata::new(
            PackageId::new("test-pkg"),
            PackageVersion::new(1, 0, 0),
            "Test",
        )
        .with_entry("handle");

        let pkg = WasmPackage::new(meta, bytes);
        let result = runtime.execute_package(&pkg, "hello").unwrap();
        assert!(result.output.contains("wasm:handle"));
    }

    #[test]
    fn test_invalid_wasm_rejected() {
        let runtime = WasmRuntime::new();
        let result = runtime.load_module_from_bytes(&[0, 1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_entry_rejected() {
        let runtime = WasmRuntime::new();
        let wat = r#"
            (module
                (memory (export "memory") 1)
            )
        "#;

        let bytes = wat::parse_str(wat).unwrap();
        let module = runtime.load_module_from_bytes(&bytes).unwrap();
        let result = runtime.execute(&module, "nonexistent", "x");
        assert!(result.is_err());
    }
}
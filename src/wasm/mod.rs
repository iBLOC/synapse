//! WASM agent runtime environment
//!
//! Provides:
//! - Package identifiers (`PackageId`, `PackageVersion`) and metadata
//! - `WasmPackage` with binary content + signature + SHA-256 hash
//! - `WasmPackageRegistry` — local package store
//! - `WasmPackageManager` — CRUD, version control, rollback
//! - `WasmRuntime` — actual wasmtime execution environment
//! - `AgentInstance` / `InstanceManager` — running instances
//!
//! This module is **independent of any Layer 3 runtime**. Layer 3
//! projects only add platform glue (HTTP API, discovery, persistence).

pub mod package;
pub mod registry;
pub mod manager;
pub mod instance;
pub mod runtime;
pub mod host_http;

pub use package::{
    PackageId, PackageVersion, PackageMetadata, WasmPackage, hex_encode,
};
pub use registry::{PackageRegistry, RegistryError, RegistryResult, PackageInfo};
pub use manager::{PackageManager, UpdateOptions, UpdateResult};
pub use instance::{AgentInstance, InstanceStatus, InstanceManager};
pub use runtime::{WasmRuntime, RuntimeConfig, ExecutionResult};
pub use host_http::{
    guard_url, ip_is_disallowed, HttpHostConfig, HttpPolicy, HttpTransport, WasmHttpRequest,
    WasmHttpResponse,
};
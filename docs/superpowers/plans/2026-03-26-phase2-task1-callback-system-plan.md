# Phase 2 Task 1: Unified Callback System — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Implement a complete callback system matching Eino's design — 5 timing hooks, TimingChecker, HandlerBuilder, global + per-invocation handlers, Context propagation.

**Architecture:** Callback system built around a `Handler` trait with 5 async methods (OnStart, OnEnd, OnError, OnStartWithStreamInput, OnEndWithStreamOutput). `CallbackManager` stores both global and per-invocation handlers. `TimingChecker` allows handlers to opt out of unused timings. `HandlerBuilder` enables ergonomic handler construction.

**Tech Stack:** Rust (no new dependencies — use existing `std`, `sync`, `Arc`, `Pin`, `Box<dyn Future>` pattern from existing codebase).

---

## File Structure

- **Modify:** `src/callbacks/mod.rs` — completely rewrite (current 169 lines → ~350 lines)
- **Modify:** `src/compose/runnable/context.rs` — update to use new callback types
- **Modify:** `src/lib.rs` — ensure callbacks module is exported
- **Create:** `src/callbacks/tests/` directory with callback tests

---

## Task 1: Rewrite callbacks/mod.rs

**Files:**
- Modify: `src/callbacks/mod.rs` (complete rewrite)

### Step 1: Add tests for the callback system

- [ ] **Step 1: Write callback system tests**

Create `src/callbacks/tests/callback_test.rs`:

```rust
//! Tests for the unified callback system.

use crate::callbacks::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[test]
fn test_callback_timing_enum() {
    assert_eq!(CallbackTiming::OnStart as u8, 0);
    assert_eq!(CallbackTiming::OnEnd as u8, 1);
    assert_eq!(CallbackTiming::OnError as u8, 2);
    assert_eq!(CallbackTiming::OnStartWithStreamInput as u8, 3);
    assert_eq!(CallbackTiming::OnEndWithStreamOutput as u8, 4);
}

#[test]
fn test_handler_builder_on_start() {
    use std::sync::atomic::AtomicUsize;
    static CALLED: AtomicUsize = AtomicUsize::new(0);

    let handler = HandlerBuilder::new()
        .on_start(|_info, _input| {
            CALLED.fetch_add(1, Ordering::SeqCst);
        })
        .build();

    let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "my_graph");
    let input = serde_json::json!({"key": "value"});
    handler.on_start(&info, &input);
    assert_eq!(CALLED.load(Ordering::SeqCst), 1);
}

#[test]
fn test_handler_builder_all_timings() {
    static CALLED: AtomicUsize = AtomicUsize::new(0);

    let handler = HandlerBuilder::new()
        .on_start(|_, _| { CALLED.fetch_add(1, Ordering::SeqCst); })
        .on_end(|_, _| { CALLED.fetch_add(10, Ordering::SeqCst); })
        .on_error(|_, _| { CALLED.fetch_add(100, Ordering::SeqCst); })
        .build();

    let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "g");
    let input = serde_json::json!({});
    let output = serde_json::json!({"result": "ok"});
    let err = "test error";

    handler.on_start(&info, &input);
    assert_eq!(CALLED.load(Ordering::SeqCst), 1);
    handler.on_end(&info, &output);
    assert_eq!(CALLED.load(Ordering::SeqCst), 11);
    handler.on_error(&info, err);
    assert_eq!(CALLED.load(Ordering::SeqCst), 111);
}

#[test]
fn test_timing_checker_none_needed() {
    struct SelectiveHandler;
    impl TimingChecker for SelectiveHandler {
        fn needed(&self, _info: &RunInfo, timing: CallbackTiming) -> bool {
            timing == CallbackTiming::OnStart
        }
    }
    impl Handler for SelectiveHandler {
        fn on_start(&self, _info: &RunInfo, _input: &serde_json::Value) {}
    }

    let h = Arc::new(SelectiveHandler);
    assert!(h.needed(&RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g"), CallbackTiming::OnStart));
    assert!(!h.needed(&RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g"), CallbackTiming::OnEnd));
}

#[test]
fn test_callback_manager_global_and_local() {
    static CALLED: AtomicUsize = AtomicUsize::new(0);
    static LOCAL_CALLED: AtomicUsize = AtomicUsize::new(0);

    let global = HandlerBuilder::new()
        .on_start(|_, _| { CALLED.fetch_add(1, Ordering::SeqCst); })
        .build();

    let local = HandlerBuilder::new()
        .on_start(|_, _| { LOCAL_CALLED.fetch_add(1, Ordering::SeqCst); })
        .build();

    let manager = CallbackManager::new()
        .with_global(Arc::new(global))
        .with_handler(Arc::new(local));

    let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "g");
    manager.run_start(&info, &serde_json::json!({}));

    assert_eq!(CALLED.load(Ordering::SeqCst), 1);
    assert_eq!(LOCAL_CALLED.load(Ordering::SeqCst), 1);
}

#[test]
fn test_run_info_new() {
    let info = RunInfo::new("node1", RunType::Invoke, ComponentType::ChatModel, "my_model");
    assert_eq!(info.name, "node1");
    assert_eq!(info.run_type, RunType::Invoke);
    assert_eq!(info.component, ComponentType::ChatModel);
    assert_eq!(info.component_name, "my_model");
}
```

Run: `cargo test -p synapse callbacks::tests -- --nocapture`
Expected: FAIL (modules don't exist yet)

### Step 2: Implement the callback types

- [ ] **Step 2: Rewrite callbacks/mod.rs**

Replace entire content of `src/callbacks/mod.rs` with:

```rust
use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::HashMap;

// ============================================================================
// Error types
// ============================================================================

pub type CallbackResult = Result<(), CallbackError>;

#[derive(Debug, thiserror::Error)]
pub enum CallbackError {
    #[error("callback error: {0}")]
    Callback(String),
}

// ============================================================================
// CallbackTiming — the 5 lifecycle moments
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CallbackTiming {
    /// Fires just before the component begins processing.
    OnStart = 0,
    /// Fires after the component returns a result successfully.
    OnEnd = 1,
    /// Fires when the component returns a non-nil error.
    OnError = 2,
    /// Fires when the component receives a streaming input.
    OnStartWithStreamInput = 3,
    /// Fires after the component returns a streaming output.
    OnEndWithStreamOutput = 4,
}

// ============================================================================
// ComponentType — what kind of component triggered the callback
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    ChatModel,
    Tool,
    Retriever,
    Embedding,
    Indexer,
    DocumentLoader,
    Agent,
    Graph,
    Chain,
    Workflow,
    Lambda,
    Custom,
}

// ============================================================================
// RunType — how the component was invoked
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Invoke,
    Stream,
    Collect,
    Transform,
}

// ============================================================================
// RunInfo — metadata about what triggered the callback
// ============================================================================

#[derive(Debug, Clone)]
pub struct RunInfo {
    /// Business-meaningful name (node name for graph nodes, set explicitly otherwise).
    pub name: String,
    /// Implementation identity, e.g. "OpenAI", "gpt-4". Falls back to struct name.
    pub run_type: RunType,
    /// Category constant for the component.
    pub component: ComponentType,
    /// The specific component name.
    pub component_name: String,
}

impl RunInfo {
    pub fn new(
        name: &str,
        run_type: RunType,
        component: ComponentType,
        component_name: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            run_type,
            component,
            component_name: component_name.to_string(),
        }
    }
}

// ============================================================================
// CallbackInput / CallbackOutput — opaque containers for callback data
// ============================================================================

/// Opaque input value passed to OnStart callbacks. Component-defined type.
pub struct CallbackInput(pub serde_json::Value);

/// Opaque output value passed to OnEnd callbacks. Component-defined type.
pub struct CallbackOutput(pub serde_json::Value);

// ============================================================================
// TimingChecker — allow handlers to opt out of unused timings
// ============================================================================

/// Optional interface for Handler implementations.
/// Return false for timings you don't handle to skip setup overhead.
pub trait TimingChecker: Send + Sync {
    /// Returns true if this handler cares about the given timing.
    fn needed(&self, info: &RunInfo, timing: CallbackTiming) -> bool;
}

// Default: handler needs all timings
impl<T: Send + Sync> TimingChecker for T
where
    Self: Handler,
{
    fn needed(&self, _info: &RunInfo, _timing: CallbackTiming) -> bool {
        true
    }
}

// ============================================================================
// Handler — the unified callback interface
// ============================================================================

/// The unified callback handler interface.
/// Implement any subset of the 5 timing methods (use default implementations for unused ones).
/// Also implement TimingChecker to opt out of timings you don't handle.
pub trait Handler: Send + Sync {
    /// Called just before the component begins processing.
    fn on_start(&self, info: &RunInfo, input: &serde_json::Value) {}

    /// Called after the component returns a result successfully.
    fn on_end(&self, info: &RunInfo, output: &serde_json::Value) {}

    /// Called when the component returns an error.
    fn on_error(&self, info: &RunInfo, error: &str) {}

    /// Called when the component receives a streaming input (Collect / Transform paradigm).
    /// Default: delegate to on_start.
    fn on_start_with_stream_input(&self, info: &RunInfo, input: &serde_json::Value) {
        self.on_start(info, input);
    }

    /// Called after the component returns a streaming output (Stream / Transform paradigm).
    /// Default: delegate to on_end.
    fn on_end_with_stream_output(&self, info: &RunInfo, output: &serde_json::Value) {
        self.on_end(info, output);
    }
}

// Blanket impl: every dyn Handler also implements TimingChecker
impl dyn Handler {
    pub fn timing_checker(&self) -> &dyn TimingChecker {
        self as &dyn TimingChecker
    }
}

// ============================================================================
// HandlerBuilder — ergonomic handler construction
// ============================================================================

pub struct HandlerBuilder {
    on_start_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_error_fn: Option<Box<dyn Fn(&RunInfo, &str) + Send + Sync>>,
    on_start_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
}

impl HandlerBuilder {
    pub fn new() -> Self {
        Self {
            on_start_fn: None,
            on_end_fn: None,
            on_error_fn: None,
            on_start_stream_fn: None,
            on_end_stream_fn: None,
        }
    }

    pub fn on_start<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_start_fn = Some(Box::new(f));
        self
    }

    pub fn on_end<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_end_fn = Some(Box::new(f));
        self
    }

    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &str) + Send + Sync + 'static,
    {
        self.on_error_fn = Some(Box::new(f));
        self
    }

    pub fn on_start_with_stream_input<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_start_stream_fn = Some(Box::new(f));
        self
    }

    pub fn on_end_with_stream_output<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_end_stream_fn = Some(Box::new(f));
        self
    }

    pub fn build(self) -> BuiltHandler {
        BuiltHandler {
            on_start_fn: self.on_start_fn,
            on_end_fn: self.on_end_fn,
            on_error_fn: self.on_error_fn,
            on_start_stream_fn: self.on_start_stream_fn,
            on_end_stream_fn: self.on_end_stream_fn,
        }
    }
}

impl Default for HandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The concrete handler produced by HandlerBuilder.
pub struct BuiltHandler {
    on_start_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_error_fn: Option<Box<dyn Fn(&RunInfo, &str) + Send + Sync>>,
    on_start_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
}

impl Handler for BuiltHandler {
    fn on_start(&self, info: &RunInfo, input: &serde_json::Value) {
        if let Some(ref f) = self.on_start_fn {
            f(info, input);
        }
    }

    fn on_end(&self, info: &RunInfo, output: &serde_json::Value) {
        if let Some(ref f) = self.on_end_fn {
            f(info, output);
        }
    }

    fn on_error(&self, info: &RunInfo, error: &str) {
        if let Some(ref f) = self.on_error_fn {
            f(info, error);
        }
    }

    fn on_start_with_stream_input(&self, info: &RunInfo, input: &serde_json::Value) {
        if let Some(ref f) = self.on_start_stream_fn {
            f(info, input);
        } else {
            self.on_start(info, input);
        }
    }

    fn on_end_with_stream_output(&self, info: &RunInfo, output: &serde_json::Value) {
        if let Some(ref f) = self.on_end_stream_fn {
            f(info, output);
        } else {
            self.on_end(info, output);
        }
    }
}

// ============================================================================
// CallbackManager — manages global + per-invocation handlers
// ============================================================================

/// Manages global (process-wide) and per-invocation callback handlers.
/// Global handlers run first; per-invocation handlers run after.
#[derive(Clone)]
pub struct CallbackManager {
    global_handlers: Arc<Vec<Arc<dyn Handler>>>,
    handlers: Arc<Vec<Arc<dyn Handler>>>,
}

impl Default for CallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackManager {
    pub fn new() -> Self {
        Self {
            global_handlers: Arc::new(Vec::new()),
            handlers: Arc::new(Vec::new()),
        }
    }

    /// Add a per-invocation handler.
    pub fn with_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        Arc::make_mut(&mut self.handlers).push(handler);
        self
    }

    /// Add a global (process-wide) handler.
    pub fn with_global(mut self, handler: Arc<dyn Handler>) -> Self {
        Arc::make_mut(&mut self.global_handlers).push(handler);
        self
    }

    fn all_handlers(&self) -> Vec<Arc<dyn Handler>> {
        let mut all = self.global_handlers.as_ref().clone();
        all.extend(self.handlers.as_ref().clone());
        all
    }

    pub fn run_start(&self, info: &RunInfo, input: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_start(info, input);
        }
    }

    pub fn run_end(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_end(info, output);
        }
    }

    pub fn run_error(&self, info: &RunInfo, error: &str) {
        for h in self.all_handlers() {
            h.on_error(info, error);
        }
    }

    pub fn run_start_with_stream_input(&self, info: &RunInfo, input: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_start_with_stream_input(info, input);
        }
    }

    pub fn run_end_with_stream_output(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_end_with_stream_output(info, output);
        }
    }
}
```

Run: `cargo test -p synapse callbacks::tests -- --nocapture`
Expected: PASS (all 6 tests)

### Step 3: Update context.rs to use new types

- [ ] **Step 3: Update runnable/context.rs**

The `Context` struct should expose `CallbackManager` instead of raw handler storage. Update `src/compose/runnable/context.rs`:

1. Change the `callbacks` field type from `Arc<Vec<Arc<dyn CallbackHandler>>>` to `Option<CallbackManager>`
2. Update `with_callbacks()` → `with_callback_manager()` (already done)
3. Update `callbacks()` return type to `Option<CallbackManager>`
4. Update test imports

```rust
pub use crate::callbacks::{CallbackHandler, CallbackManager, ComponentType, RunInfo, RunType};
```
(Note: `CallbackHandler` → `Handler` after rename)

Replace all `CallbackHandler` references with `Handler`:
- In `Context` struct: `Arc<Vec<Arc<dyn Handler>>>`
- In `with_callbacks`: `Vec<Arc<dyn Handler>>`
- In `callbacks()` return: `Option<CallbackManager>`
- In tests: `impl Handler for NoopHandler {}`

Run: `cargo build -p synapse` → should compile clean
Run: `cargo test -p synapse` → should pass all tests

### Step 4: Verify integration

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p synapse 2>&1`
Expected: 29+ tests passing, build clean

---

## Task 2: Commit

- [ ] **Step 5: Commit**

```bash
git add src/callbacks/mod.rs src/callbacks/tests/ src/compose/runnable/context.rs src/lib.rs
git commit -m "feat(callbacks): unified callback system — 5 timing hooks, TimingChecker, HandlerBuilder, CallbackManager

- Add CallbackTiming enum with 5 lifecycle moments (OnStart, OnEnd, OnError, OnStartWithStreamInput, OnEndWithStreamOutput)
- Add Handler trait with 5 async methods (sync in Rust, callbacks run synchronously)
- Add TimingChecker trait for opt-in timing selection (avoids overhead for unused timings)
- Add HandlerBuilder for ergonomic handler construction with closure-based callbacks
- Add CallbackManager with global + per-invocation handlers (global runs first)
- Add RunInfo, ComponentType, RunType, CallbackInput, CallbackOutput types
- Update Context to use CallbackManager instead of raw handler storage
- Add comprehensive tests for all callback types"
```

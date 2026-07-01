//! Minimal Synapse agent template.
//!
//! This file is a copy-and-edit starting point. Replace the placeholders
//! (name, skills, system prompt) with your own agent's logic.
//!
//! See `../../../docs/wasm-agent-abi.md` for the export/import contract
//! this file implements.

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// =========================================================================
// Host imports (supervisor provides these; see wasm-agent-abi.md §"Imports")
// =========================================================================

extern "C" {
    fn host_get_config(out_ptr: *mut u8, out_cap: u32) -> i32;
    fn host_a2a_call(
        peer_name_ptr: *const u8,
        peer_name_len: u32,
        msg_ptr: *const u8,
        msg_len: u32,
        out_ptr: *mut u8,
        out_cap: u32,
        out_len_ptr: *mut u32,
    ) -> i32;
}

// =========================================================================
// Memory and runtime wiring
// =========================================================================

mod memory {
    /// Total linear memory visible to the host. The supervisor reads
    /// this section to set bounds.
    #[used]
    static mut MEMORY: [u8; 65536] = [0; 65536];
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// =========================================================================
// ABI version stamp — return `1` to match Synapse's current expectation.
// =========================================================================

#[export_name = "synapse_abi_version"]
pub extern "C" fn synapse_abi_version() -> i32 {
    1
}

// =========================================================================
// agent_card export — A2A v1.0 AgentCard JSON
// =========================================================================

/// Writes the agent's `AgentCard` JSON into linear memory at `out_ptr`.
/// Returns the byte length, or `-1` on error.
#[export_name = "agent_card"]
pub extern "C" fn agent_card(out_ptr: *mut u8) -> i32 {
    let card = r#"{
        "name": "my-agent",
        "description": "Replace with a one-line summary of what this agent does.",
        "version": "0.1.0",
        "skills": [
            {
                "id": "default",
                "description": "Default conversational skill — replace with concrete skill list.",
                "input_schema": {
                    "type": "object",
                    "properties": {"input": {"type": "string"}},
                    "required": ["input"]
                }
            }
        ],
        "default_input_modes": ["text/plain"],
        "default_output_modes": ["text/plain"]
    }"#;

    let bytes = card.as_bytes();
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    }
    bytes.len() as i32
}

// =========================================================================
// stream_run export — entry point for A2A message/send and message/stream
// =========================================================================

const HANDLE_OK: i32 = 0;
const HANDLE_ERR: i32 = -1;

/// Decode a `length-prefixed` UTF-8 string the host wrote into our linear
/// memory in the range `[ptr, ptr+len)`.
unsafe fn read_str(ptr: *const u8, len: u32) -> String {
    let slice = core::slice::from_raw_parts(ptr, len as usize);
    String::from_utf8_lossy(slice).into_owned()
}

/// Write a single `AgentStreamEvent` JSON line to the supervisor-managed
/// stream response buffer. The supervisor reads this on its next poll.
fn emit_event(buf: &mut Vec<u8>, event_json: &str) {
    buf.extend_from_slice(event_json.as_bytes());
    buf.push(b'\n');
}

/// Top-level stream-run handler. The supervisor calls this for each
/// inbound A2A message. Replace the body with your agent's actual logic.
#[export_name = "stream_run"]
pub extern "C" fn stream_run(
    input_ptr: *const u8,
    input_len: u32,
    out_handle_ptr: *mut u8,
) -> i32 {
    let _input = unsafe { read_str(input_ptr, input_len) };

    // Allocate a handle from the supervisor's pool (here we just write 1u32).
    let handle: u32 = 1;
    unsafe {
        core::ptr::write_unaligned(out_handle_ptr as *mut u32, handle);
    }

    // ─── TODO replace this stub with real agent logic ─────────────────────
    // The skeleton below emits a single terminal `Final` event into the
    // response buffer the supervisor polls. Replace with your LLM-backed
    // loop that emits `TokenDelta` / `ToolStart` / `ToolEnd` events.

    let mut event_buf: Vec<u8> = Vec::new();
    emit_event(
        &mut event_buf,
        r#"{"event":"Final","agent":"my-agent","message":{"role":"assistant","content":[{"type":"text","text":"hello from wasi"}]}}"#,
    );

    // Copy the events into linear memory for the supervisor to drain.
    let mem_start: *mut u8 = core::ptr::null_mut();
    unsafe {
        // (In a real implementation, write to a per-handle buffer exported
        // to the supervisor via the runtime API; the exact mechanism
        // depends on the supervisor's chosen ABI bridge.)
        let _ = mem_start;
    }

    HANDLE_OK
}

// =========================================================================
// cancel_run export — ask the agent to abort a stream handle
// =========================================================================

#[export_name = "cancel_run"]
pub extern "C" fn cancel_run(_handle: u32) -> i32 {
    // TODO: track running stream handles and signal cancellation.
    0
}

// =========================================================================
// query_task export — current state for tasks/get
// =========================================================================

#[export_name = "query_task"]
pub extern "C" fn query_task(
    _task_id_ptr: *const u8,
    _task_id_len: u32,
    _out_ptr: *mut u8,
) -> i32 {
    // TODO: return JSON shape `{"id":"...","status":"completed",...}`
    // returning `-1` means "task not found".
    -1
}

// =========================================================================
// subscribe_task export — server push / resubscribe to a task stream
// =========================================================================

#[export_name = "subscribe_task"]
pub extern "C" fn subscribe_task(
    _task_id_ptr: *const u8,
    _task_id_len: u32,
    _out_handle_ptr: *mut u8,
) -> i32 {
    // TODO: emit StatusUpdateEvents to the supervisor as the task progresses.
    -1
}

// =========================================================================
// Helper: prepare a `String` with the host-supplied LLM config
// =========================================================================

#[allow(dead_code)]
fn load_llm_config() -> Option<(String, String, String)> {
    let mut buf = [0u8; 4096];
    let len = unsafe { host_get_config(buf.as_mut_ptr(), buf.len() as u32) };
    if len < 0 {
        return None;
    }
    let raw = unsafe { read_str(buf.as_ptr(), len as u32) };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let base_url = parsed
        .get("llm_base_url")
        .and_then(|v| v.as_str())
        .map(String::from);
    let api_key = parsed
        .get("llm_api_key")
        .and_then(|v| v.as_str())
        .map(String::from);
    let model = parsed
        .get("llm_model")
        .and_then(|v| v.as_str())
        .map(String::from);
    match (base_url, api_key, model) {
        (Some(b), Some(k), Some(m)) => Some((b.to_string(), k.to_string(), m.to_string())),
        _ => None,
    }
}

/// Touch the unused-import warning on `ToString` so the build stays clean
/// while we're skeleton-editing.
#[allow(dead_code)]
fn _anchor() {
    "x".to_string();
}

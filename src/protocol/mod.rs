pub mod a2a_adapter;
pub mod adapter;
#[cfg(not(target_arch = "wasm32"))]
pub mod opencode_adapter;
#[cfg(not(target_arch = "wasm32"))]
pub mod openglow_adapter;

pub use a2a_adapter::A2AAdapter;
pub use adapter::*;
#[cfg(not(target_arch = "wasm32"))]
pub use opencode_adapter::OpenCodeAdapter;
#[cfg(not(target_arch = "wasm32"))]
pub use openglow_adapter::OpenGlowAdapter;

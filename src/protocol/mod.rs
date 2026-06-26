pub mod adapter;
pub mod a2a_adapter;
#[cfg(not(target_arch = "wasm32"))]
pub mod opencode_adapter;
#[cfg(not(target_arch = "wasm32"))]
pub mod openglow_adapter;

pub use adapter::*;
pub use a2a_adapter::A2AAdapter;
#[cfg(not(target_arch = "wasm32"))]
pub use opencode_adapter::OpenCodeAdapter;
#[cfg(not(target_arch = "wasm32"))]
pub use openglow_adapter::OpenGlowAdapter;

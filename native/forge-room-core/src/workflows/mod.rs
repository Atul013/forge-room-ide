//! Declarative, review-first local workflows. This module never starts a process or grants authority.
mod engine;
mod schema;
mod templates;
pub use engine::*;
pub use schema::*;
use sha2::{Digest, Sha256};
pub use templates::*;

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn seal(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| digest(&bytes))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;

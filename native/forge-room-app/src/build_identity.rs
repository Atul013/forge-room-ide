//! Recorded build provenance, not a signature or publisher attestation.
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub version: &'static str,
    pub source_revision: Option<&'static str>,
    pub native_tree: Option<&'static str>,
    pub target: &'static str,
    pub profile: &'static str,
    pub recorded: bool,
}
pub fn current() -> Identity {
    let source_revision = Some(env!("FORGE_ROOM_BUILD_SOURCE")).filter(|v| !v.is_empty());
    let native_tree = Some(env!("FORGE_ROOM_BUILD_NATIVE_TREE")).filter(|v| !v.is_empty());
    Identity {
        version: env!("CARGO_PKG_VERSION"),
        source_revision,
        native_tree,
        target: env!("FORGE_ROOM_BUILD_TARGET"),
        profile: env!("FORGE_ROOM_BUILD_PROFILE"),
        recorded: source_revision.is_some() && native_tree.is_some(),
    }
}
pub fn label() -> String {
    let identity = current();
    match identity.source_revision.filter(|_| identity.recorded) {
        Some(sha) => format!("{} / source {}", identity.version, &sha[..12]),
        None => format!("{} / unrecorded local build", identity.version),
    }
}
pub fn json() -> String {
    serde_json::to_string_pretty(&current()).expect("static build identity")
}

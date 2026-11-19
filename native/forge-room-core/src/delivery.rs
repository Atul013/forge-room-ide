//! Release evidence records: independent human reports and observed file integrity, never execution.
use crate::{now_epoch_seconds, workflows};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
pub const MAX_DOSSIER: usize = 256 * 1024;
pub const MAX_ARTIFACT: u64 = 512 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    Source,
    Package,
    Desktop,
    Provider,
    PhysicalClients,
}
impl Gate {
    pub const ALL: [Self; 5] = [
        Self::Source,
        Self::Package,
        Self::Desktop,
        Self::Provider,
        Self::PhysicalClients,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "Source / CI",
            Self::Package => "Package / install preservation",
            Self::Desktop => "Visible desktop / interaction",
            Self::Provider => "Real native provider",
            Self::PhysicalClients => "Physical two-client",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    #[default]
    Unknown,
    ReportedPass,
    ReportedFail,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateReport {
    pub gate: Gate,
    pub outcome: Outcome,
    pub observer: String,
    pub note: String,
    pub reference: String,
    pub recorded_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAnchor {
    pub commit: String,
    pub tree: String,
    pub status_digest: String,
    pub dirty_paths: u64,
    pub observed_at: u64,
}
impl SourceAnchor {
    pub fn same_source(&self, other: &Self) -> bool {
        self.commit == other.commit
            && self.tree == other.tree
            && self.status_digest == other.status_digest
            && self.dirty_paths == other.dirty_paths
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
    pub observed_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dossier {
    pub version: u32,
    pub id: String,
    pub title: String,
    pub source: Option<SourceAnchor>,
    pub gates: Vec<GateReport>,
    pub artifacts: Vec<Artifact>,
    pub notes: String,
}
fn hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len()) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn text(value: &str, max: usize, required: bool) -> Result<(), String> {
    if value.len() > max
        || required && value.trim().is_empty()
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("Invalid evidence text or byte limit".into());
    }
    Ok(())
}
impl Dossier {
    pub fn new(title: String) -> Result<Self, String> {
        let value = Self {
            version: 1,
            id: workflows::new_id()?,
            title,
            source: None,
            gates: Gate::ALL
                .into_iter()
                .map(|gate| GateReport {
                    gate,
                    outcome: Outcome::Unknown,
                    observer: String::new(),
                    note: String::new(),
                    reference: String::new(),
                    recorded_at: 0,
                })
                .collect(),
            artifacts: vec![],
            notes: String::new(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !hex(&self.id, &[32])
            || self.gates.len() != 5
            || self.artifacts.len() > 64
        {
            return Err("Unsupported dossier identity/version/limits".into());
        }
        text(&self.title, 160, true)?;
        text(&self.notes, 4000, false)?;
        if let Some(s) = &self.source {
            if !hex(&s.commit, &[40, 64])
                || !hex(&s.tree, &[40, 64])
                || !hex(&s.status_digest, &[64])
                || s.dirty_paths > 100000
            {
                return Err("Invalid source anchor".into());
            }
        }
        for gate in Gate::ALL {
            if self.gates.iter().filter(|r| r.gate == gate).count() != 1 {
                return Err("All five acceptance gates must appear exactly once".into());
            }
        }
        for r in &self.gates {
            let required = r.outcome != Outcome::Unknown;
            text(&r.observer, 80, required)?;
            text(&r.note, 4000, required)?;
            text(&r.reference, 1000, false)?;
            if required && r.recorded_at == 0 {
                return Err("Reported outcomes require an observation time".into());
            }
        }
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        let mut total = 0u64;
        for a in &self.artifacts {
            text(&a.name, 200, true)?;
            if !hex(&a.id, &[32])
                || !ids.insert(&a.id)
                || !names.insert(&a.name)
                || !hex(&a.sha256, &[64])
                || a.bytes > MAX_ARTIFACT
                || a.name.contains(['/', '\\', ':'])
            {
                return Err("Invalid artifact receipt".into());
            }
            total = total.checked_add(a.bytes).ok_or("Artifact size overflow")?;
        }
        if total > 2 * 1024 * 1024 * 1024 {
            return Err("Artifact set exceeds 2 GiB".into());
        }
        Ok(())
    }
    pub fn seal(&self) -> Result<String, String> {
        self.validate()?;
        workflows::seal(self)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_DOSSIER {
            return Err("Dossier exceeds 256 KiB".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Invalid dossier schema at line {}, column {}",
                e.line(),
                e.column()
            )
        })?;
        value.validate()?;
        Ok(value)
    }
    pub fn anchor(&mut self, source: SourceAnchor) {
        if self
            .source
            .as_ref()
            .is_none_or(|old| !old.same_source(&source))
        {
            self.reset_reports();
        }
        self.source = Some(source);
    }
    fn reset_reports(&mut self) {
        for report in &mut self.gates {
            report.outcome = Outcome::Unknown;
            report.observer.clear();
            report.note.clear();
            report.reference.clear();
            report.recorded_at = 0;
        }
    }
    /// Replacing an artifact with different bytes invalidates every previously reported acceptance.
    pub fn attach_artifact(&mut self, artifact: Artifact) -> Result<Option<String>, String> {
        let old = self
            .artifacts
            .iter()
            .find(|a| a.name == artifact.name)
            .cloned();
        let mut candidate = self.clone();
        candidate.artifacts.retain(|a| a.name != artifact.name);
        let changed = old
            .as_ref()
            .is_none_or(|a| a.sha256 != artifact.sha256 || a.bytes != artifact.bytes);
        candidate.artifacts.push(artifact);
        candidate.validate()?;
        if changed {
            candidate.reset_reports();
        }
        *self = candidate;
        Ok(old.map(|a| a.id))
    }
    pub fn remove_artifact(&mut self, index: usize) -> Result<Artifact, String> {
        if index >= self.artifacts.len() {
            return Err("Artifact receipt no longer exists".into());
        }
        let removed = self.artifacts.remove(index);
        self.reset_reports();
        Ok(removed)
    }
    pub fn blockers(&self) -> Vec<String> {
        let mut result = vec![];
        match &self.source {None=>result.push("Source identity has not been captured".into()),Some(s) if s.dirty_paths>0=>result.push("Source has dirty/untracked paths; a clean committed source is required for a checked handoff".into()),_=>()}
        for r in &self.gates {
            if r.outcome != Outcome::ReportedPass {
                result.push(format!("{}: {:?}", r.gate.label(), r.outcome));
            }
        }
        if self.artifacts.is_empty() {
            result.push("No artifact fingerprints attached".into());
        }
        result
    }
    /// Public status projection only. Reports remain attributed claims, not independent acceptance.
    pub fn brief(&self) -> String {
        let mut out = format!(
            "Release evidence: {}\nSource: {}\n",
            self.title,
            self.source
                .as_ref()
                .map(|s| s.commit.as_str())
                .unwrap_or("not captured")
        );
        for r in &self.gates {
            out.push_str(&format!(
                "{}: {:?} (manual report)\n",
                r.gate.label(),
                r.outcome
            ));
        }
        out.push_str(&format!("Artifact fingerprints: {}. Integrity checks do not establish desktop/provider/physical acceptance.",self.artifacts.len()));
        out
    }
    pub fn compare(&self, other: &Self) -> Vec<String> {
        let mut changes = vec![];
        if self.id != other.id {
            changes.push("Different dossier identities".into());
        }
        if self
            .source
            .as_ref()
            .zip(other.source.as_ref())
            .is_none_or(|(a, b)| !a.same_source(b))
        {
            changes.push("Source identity/status differs or is absent".into());
        }
        for gate in Gate::ALL {
            let a = self.gates.iter().find(|r| r.gate == gate);
            let b = other.gates.iter().find(|r| r.gate == gate);
            if a != b {
                changes.push(format!("{} report changed", gate.label()));
            }
        }
        for a in &self.artifacts {
            match other.artifacts.iter().find(|b| b.name == a.name) {
                None => changes.push(format!("Artifact absent from comparison: {}", a.name)),
                Some(b) if a.sha256 != b.sha256 || a.bytes != b.bytes => {
                    changes.push(format!("Artifact bytes changed: {}", a.name))
                }
                _ => (),
            }
        }
        for b in &other.artifacts {
            if !self.artifacts.iter().any(|a| a.name == b.name) {
                changes.push(format!("Artifact added in comparison: {}", b.name));
            }
        }
        changes
    }
}
pub struct ArtifactInspection {
    pub path: PathBuf,
    pub receipt: Artifact,
}
/// Bounded streaming inspection; neither reads a ZIP's contents nor runs an artifact.
pub fn inspect_artifact(path: &Path) -> Result<ArtifactInspection, String> {
    let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        before.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = before.file_type().is_symlink();
    if linked || !before.is_file() || before.len() > MAX_ARTIFACT {
        return Err("Choose a regular non-linked artifact up to 512 MiB".into());
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let mut file = fs::File::open(&canonical).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut bytes = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > MAX_ARTIFACT {
            return Err("Artifact grew beyond 512 MiB".into());
        }
        hash.update(&buffer[..count]);
    }
    let after = fs::metadata(&canonical).map_err(|e| e.to_string())?;
    if bytes != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
        || path.canonicalize().map_err(|e| e.to_string())? != canonical
    {
        return Err("Artifact changed during inspection; retry".into());
    }
    let receipt = Artifact {
        id: workflows::new_id()?,
        name: canonical
            .file_name()
            .ok_or("Artifact name unavailable")?
            .to_string_lossy()
            .into_owned(),
        sha256: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
        bytes,
        observed_at: now_epoch_seconds(),
    };
    Ok(ArtifactInspection {
        path: canonical,
        receipt,
    })
}
pub fn recheck_artifact(path: &Path, expected: &Artifact) -> Result<(), String> {
    let actual = inspect_artifact(path)?.receipt;
    if actual.sha256 != expected.sha256 || actual.bytes != expected.bytes {
        return Err(format!("Artifact changed: {}", expected.name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> SourceAnchor {
        SourceAnchor {
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            status_digest: workflows::digest(b""),
            dirty_paths: 0,
            observed_at: 1,
        }
    }
    #[test]
    fn acceptance_gates_are_independent_and_source_changes_reset_manual_reports() {
        let mut d = Dossier::new("Release".into()).unwrap();
        d.anchor(source());
        let r = &mut d.gates[0];
        r.outcome = Outcome::ReportedPass;
        r.observer = "Reviewer".into();
        r.note = "CI receipt checked".into();
        r.recorded_at = 1;
        d.validate().unwrap();
        assert_eq!(d.blockers().len(), 5);
        assert!(d.brief().contains("Physical two-client: Unknown"));
        let mut changed = source();
        changed.commit = "c".repeat(40);
        d.anchor(changed);
        assert!(d.gates.iter().all(|r| r.outcome == Outcome::Unknown));
        assert!(Dossier::parse(&serde_json::to_vec(&d).unwrap()).is_ok());
        d.gates[1] = d.gates[0].clone();
        assert!(d.validate().is_err());
    }
    #[test]
    fn file_integrity_rechecks_preserve_sources_and_fail_after_mutation() {
        let root =
            std::env::temp_dir().join(format!("forge-delivery-{}", workflows::new_id().unwrap()));
        fs::create_dir(&root).unwrap();
        let file = root.join("package.zip");
        fs::write(&file, b"original").unwrap();
        let inspected = inspect_artifact(&file).unwrap();
        assert_eq!(inspected.receipt.sha256, workflows::digest(b"original"));
        recheck_artifact(&file, &inspected.receipt).unwrap();
        fs::write(&file, b"changed").unwrap();
        assert!(recheck_artifact(&file, &inspected.receipt).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"changed");
        fs::remove_file(file).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn portable_receipts_reject_paths_oversize_unknown_fields_and_do_not_hide_comparison_changes() {
        let mut a = Dossier::new("Release".into()).unwrap();
        a.anchor(source());
        let mut b = a.clone();
        b.source.as_mut().unwrap().observed_at = 9;
        assert!(a.compare(&b).is_empty());
        b.source.as_mut().unwrap().tree = "c".repeat(40);
        assert!(a.compare(&b).iter().any(|c| c.contains("Source")));
        a.artifacts.push(Artifact {
            id: "d".repeat(32),
            name: "C:\\private\\file.zip".into(),
            sha256: "e".repeat(64),
            bytes: 1,
            observed_at: 1,
        });
        assert!(a.validate().is_err());
        assert!(Dossier::parse(&vec![b' '; MAX_DOSSIER + 1]).is_err());
        assert!(Dossier::parse(br#"{"token":"secret"}"#)
            .unwrap_err()
            .contains("schema"));
    }
    #[test]
    fn changed_artifact_sets_reset_reports_but_same_bytes_preserve_them_and_invalid_additions_are_atomic(
    ) {
        let mut d = Dossier::new("Release".into()).unwrap();
        let artifact = Artifact {
            id: "a".repeat(32),
            name: "package.zip".into(),
            sha256: "b".repeat(64),
            bytes: 3,
            observed_at: 1,
        };
        d.attach_artifact(artifact.clone()).unwrap();
        d.gates[0].outcome = Outcome::ReportedPass;
        d.gates[0].observer = "Reviewer".into();
        d.gates[0].note = "Observed".into();
        d.gates[0].recorded_at = 1;
        let mut same = artifact.clone();
        same.id = "c".repeat(32);
        same.observed_at = 2;
        d.attach_artifact(same.clone()).unwrap();
        assert_eq!(d.gates[0].outcome, Outcome::ReportedPass);
        let before = d.clone();
        let mut invalid = artifact.clone();
        invalid.name = "../private".into();
        assert!(d.attach_artifact(invalid).is_err());
        assert_eq!(d, before);
        same.sha256 = "d".repeat(64);
        d.attach_artifact(same).unwrap();
        assert!(d.gates.iter().all(|r| r.outcome == Outcome::Unknown));
        assert!(d.remove_artifact(10).is_err());
        d.remove_artifact(0).unwrap();
        assert!(d.artifacts.is_empty());
    }
    #[test]
    fn room_status_projection_omits_observers_notes_references_and_artifact_hashes() {
        let mut d = Dossier::new("Release".into()).unwrap();
        d.anchor(source());
        d.gates[0].observer = "private-observer".into();
        d.gates[0].note = "private-note".into();
        d.gates[0].reference = "private-reference".into();
        d.notes = "private-release-notes".into();
        d.attach_artifact(Artifact {
            id: "d".repeat(32),
            name: "package.zip".into(),
            sha256: "e".repeat(64),
            bytes: 1,
            observed_at: 1,
        })
        .unwrap();
        d.gates[0].observer = "private-observer".into();
        d.gates[0].note = "private-note".into();
        d.gates[0].reference = "private-reference".into();
        let brief = d.brief();
        assert!(!brief.contains("private-"));
        assert!(!brief.contains(&"e".repeat(64)));
        assert!(!brief.contains("package.zip"));
        assert!(brief.contains("manual report"));
    }
}

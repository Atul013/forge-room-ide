//! Declarative local resources. Parsing or activation plans never execute code, fetch or write files.
mod profile;
mod skills;
pub use skills::*;
mod widgets;
use crate::workflows::{self, text};
pub use profile::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};
pub use widgets::*;
pub const API: u32 = 1;
pub const MAX_PACK: usize = 4 * 1024 * 1024;
pub const MAX_RESOURCE: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version(pub [u16; 3]);
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0[0], self.0[1], self.0[2])
    }
}
impl FromStr for Version {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err("Use major.minor.patch without prerelease metadata".into());
        }
        let mut numbers = [0; 3];
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty()
                || part.len() > 5
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err("Invalid pack version".into());
            }
            numbers[index] = part
                .parse()
                .map_err(|_| "Pack version component exceeds 65535")?;
        }
        Ok(Self(numbers))
    }
}
impl TryFrom<String> for Version {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        value.parse()
    }
}
impl From<Version> for String {
    fn from(value: Version) -> Self {
        value.to_string()
    }
}
pub fn pack_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 40
                && part.as_bytes()[0].is_ascii_lowercase()
                && part.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Theme,
    Grammar,
    Snippets,
    Workflow,
    Skill,
    Profile,
    Widget,
    Readme,
    RoomBlueprint,
}
impl Kind {
    pub const ALL: [Self; 9] = [
        Self::Theme,
        Self::Grammar,
        Self::Snippets,
        Self::Workflow,
        Self::Skill,
        Self::Profile,
        Self::Widget,
        Self::Readme,
        Self::RoomBlueprint,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Color theme",
            Self::Grammar => "Syntax grammar",
            Self::Snippets => "Plain snippets",
            Self::Workflow => "Workflow library",
            Self::Skill => "Skill text",
            Self::Profile => "Appearance/layout profile",
            Self::Widget => "Read-only widget",
            Self::Readme => "Documentation",
            Self::RoomBlueprint => "Room blueprint",
        }
    }
    pub fn max_bytes(self) -> usize {
        match self {
            Self::Theme => 512 * 1024,
            Self::Grammar | Self::Snippets => 128 * 1024,
            Self::Workflow => MAX_RESOURCE,
            Self::RoomBlueprint => crate::transport::MAX_BLUEPRINT,
            Self::Skill | Self::Readme => 32 * 1024,
            Self::Profile | Self::Widget => 64 * 1024,
        }
    }
    pub fn permission(self) -> &'static str {
        match self {
            Self::Theme | Self::Grammar | Self::Snippets => "Local declarative rendering/insertion",
            Self::Workflow => {
                "Stage a workflow definition; execution still requires a new frozen review"
            }
            Self::Skill => "Inspect text; AI inclusion and send are separate choices",
            Self::Profile => "Preview supported appearance/layout settings; explicit Keep/Restore",
            Self::Widget => "Only explicitly granted current-workspace summary bindings",
            Self::Readme => "Local text inspection",
            Self::RoomBlueprint => {
                "Stage public setup; fresh host creation requires separate review"
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub content: String,
    pub sha256: String,
}
impl Resource {
    pub fn new(id: String, label: String, kind: Kind, content: String) -> Result<Self, String> {
        let sha256 = workflows::digest(content.as_bytes());
        let resource = Self {
            id,
            label,
            kind,
            content,
            sha256,
        };
        resource.validate()?;
        Ok(resource)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !workflows::identifier(&self.id) {
            return Err("Resource identities use 1–48 letters/digits/hyphens/underscores".into());
        }
        text(&self.label, 160, true)?;
        text(&self.content, self.kind.max_bytes(), true)?;
        if self
            .content
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
            || workflows::digest(self.content.as_bytes()) != self.sha256
        {
            return Err("Resource content is unsupported or its digest changed".into());
        }
        match self.kind {
            Kind::RoomBlueprint => {
                crate::transport::RoomBlueprint::parse(self.content.as_bytes())?;
            }
            Kind::Profile => {
                Profile::parse(&self.content)?;
            }
            Kind::Widget => {
                Widget::parse(&self.content)?;
            }
            Kind::Workflow => {
                let library: WorkflowSet =
                    serde_json::from_str(&self.content).map_err(|e| e.to_string())?;
                library.validate()?;
            }
            _ => (),
        };
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub id: String,
    pub minimum: Version,
    #[serde(default)]
    pub before: Option<Version>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub format: u32,
    pub api: u32,
    pub id: String,
    pub version: Version,
    pub name: String,
    pub publisher: String,
    pub summary: String,
    pub license: String,
    #[serde(default)]
    pub license_text: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub readme: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    pub resources: Vec<Resource>,
}
impl Pack {
    pub fn key(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }
    pub fn digest(&self) -> Result<String, String> {
        workflows::seal(self)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PACK {
            return Err("Pack exceeds 4 MiB".into());
        }
        let pack: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        pack.validate()?;
        Ok(pack)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != 1
            || self.api != API
            || !pack_id(&self.id)
            || self.resources.is_empty()
            || self.resources.len() > 32
            || self.dependencies.len() > 16
        {
            return Err("Unsupported pack format/API/identity or resource count".into());
        }
        for (value, max, required) in [
            (&self.name, 160, true),
            (&self.publisher, 160, true),
            (&self.summary, 4000, false),
            (&self.license, 128, true),
            (&self.license_text, 32 * 1024, false),
            (&self.source, 1024, false),
            (&self.readme, 32 * 1024, false),
        ] {
            text(value, max, required)?;
        }
        if self.source.chars().any(char::is_control) {
            return Err("Pack source claim contains control characters".into());
        }
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        for resource in &self.resources {
            resource.validate()?;
            if !ids.insert(&resource.id) {
                return Err("Resource identity is repeated".into());
            }
            bytes = bytes.saturating_add(resource.content.len());
        }
        if bytes > 2 * 1024 * 1024
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_PACK
        {
            return Err("Pack resources exceed 2 MiB or serialized size exceeds 4 MiB".into());
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &self.dependencies {
            if !pack_id(&dependency.id)
                || dependency.id == self.id
                || !dependencies.insert(&dependency.id)
                || dependency
                    .before
                    .is_some_and(|upper| upper <= dependency.minimum)
            {
                return Err("Invalid, repeated, self or empty-range pack dependency".into());
            }
        }
        Ok(())
    }
}
/// Same wire shape as the project workflow library; installation only stages these definitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSet {
    pub version: u32,
    pub workflows: Vec<workflows::Workflow>,
}
impl WorkflowSet {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.workflows.len() > 32 {
            return Err("Unsupported workflow library".into());
        }
        let mut ids = BTreeSet::new();
        for workflow in &self.workflows {
            workflow.validate()?;
            if !ids.insert(&workflow.id) {
                return Err("Repeated workflow identity".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub version: Version,
    pub resources: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub format: u32,
    pub revision: u64,
    pub enabled: BTreeMap<String, Selection>,
}
impl Default for Activation {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            enabled: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub resource: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub version: u32,
    pub expected: String,
    pub next: Activation,
    pub packs: BTreeMap<String, String>,
    pub changes: Vec<Change>,
    pub seal: String,
}
pub type Catalog = BTreeMap<String, Pack>;
impl Activation {
    pub fn validate(&self, catalog: &Catalog) -> Result<Vec<String>, String> {
        if self.format != 1 || self.enabled.len() > 32 || catalog.len() > 64 {
            return Err("Activation/catalog limits exceeded".into());
        }
        let mut total = 0usize;
        let mut bytes = 0usize;
        for (id, selection) in &self.enabled {
            if !pack_id(id) || selection.resources.is_empty() {
                return Err("Enabled packs need a valid identity and selected resources".into());
            }
            let key = format!("{id}@{}", selection.version);
            let pack = catalog
                .get(&key)
                .ok_or_else(|| format!("Install required pack {key} first"))?;
            pack.validate()?;
            if pack.key() != key {
                return Err("Installed pack identity changed".into());
            }
            for resource in &selection.resources {
                let resource = pack
                    .resources
                    .iter()
                    .find(|entry| &entry.id == resource)
                    .ok_or("Selected resource no longer exists")?;
                total += 1;
                bytes = bytes.saturating_add(resource.content.len());
            }
            for dependency in &pack.dependencies {
                let selected = self
                    .enabled
                    .get(&dependency.id)
                    .ok_or_else(|| format!("Enable dependency {} first", dependency.id))?;
                if selected.version < dependency.minimum
                    || dependency
                        .before
                        .is_some_and(|maximum| selected.version >= maximum)
                {
                    return Err(format!(
                        "Dependency version is outside the supported range: {}",
                        dependency.id
                    ));
                }
            }
        }
        if total > 128 || bytes > 8 * 1024 * 1024 {
            return Err("Activation exceeds 128 resources / 8 MiB".into());
        }
        let mut ordered = Vec::new();
        let mut done = BTreeSet::new();
        while ordered.len() < self.enabled.len() {
            let before = done.len();
            for (id, selection) in &self.enabled {
                let pack = &catalog[&format!("{id}@{}", selection.version)];
                if !done.contains(id)
                    && pack
                        .dependencies
                        .iter()
                        .all(|dependency| done.contains(&dependency.id))
                {
                    done.insert(id.clone());
                    ordered.push(id.clone());
                }
            }
            if before == done.len() {
                return Err("Pack dependency cycle".into());
            }
        }
        Ok(ordered)
    }
    pub fn resource_seals(&self, catalog: &Catalog) -> Result<BTreeMap<String, String>, String> {
        self.validate(catalog)?;
        let mut resources = BTreeMap::new();
        for (id, selection) in &self.enabled {
            for resource in &catalog[&format!("{id}@{}", selection.version)].resources {
                if selection.resources.contains(&resource.id) {
                    resources.insert(format!("{id}/{}", resource.id), workflows::seal(resource)?);
                }
            }
        }
        Ok(resources)
    }
    pub fn review(
        &self,
        enabled: BTreeMap<String, Selection>,
        catalog: &Catalog,
    ) -> Result<Review, String> {
        let before = self.resource_seals(catalog)?;
        let next = Activation {
            format: 1,
            revision: self
                .revision
                .checked_add(1)
                .ok_or("Activation revision exhausted")?,
            enabled,
        };
        let after = next.resource_seals(catalog)?;
        let mut changes = Vec::new();
        for resource in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
            if before.get(resource) != after.get(resource) {
                changes.push(Change {
                    resource: resource.clone(),
                    before: before.get(resource).cloned(),
                    after: after.get(resource).cloned(),
                });
            }
        }
        let packs = next
            .enabled
            .iter()
            .map(|(id, selected)| {
                let key = format!("{id}@{}", selected.version);
                catalog[&key].digest().map(|digest| (key, digest))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let expected = workflows::seal(self)?;
        let seal = workflows::seal(&(&expected, &next, &packs, &changes))?;
        Ok(Review {
            version: 1,
            expected,
            next,
            packs,
            changes,
            seal,
        })
    }
    pub fn accept(&self, review: &Review, catalog: &Catalog) -> Result<Self, String> {
        let expected = self.review(review.next.enabled.clone(), catalog)?;
        if *review != expected {
            return Err("Pack activation review is stale or changed".into());
        }
        Ok(review.next.clone())
    }
}
#[cfg(test)]
mod tests;

use crate::workflows::{self, text};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Workspace,
    Git,
    Problems,
    Workflows,
    Room,
}
impl DataSource {
    pub const ALL: [Self; 5] = [
        Self::Workspace,
        Self::Git,
        Self::Problems,
        Self::Workflows,
        Self::Room,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Workspace => "Workspace summary",
            Self::Git => "Cached Git summary",
            Self::Problems => "Cached Problems summary",
            Self::Workflows => "Workflow status counts",
            Self::Room => "Visible Room counts",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Metric,
    List,
    Table,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub label: String,
    pub source: DataSource,
    pub view: View,
    pub limit: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub columns: u8,
    pub refresh_seconds: u32,
    pub bindings: Vec<Binding>,
}
impl Widget {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 64 * 1024 {
            return Err("Widget exceeds 64 KiB".into());
        }
        let widget: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        widget.validate()?;
        Ok(widget)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !workflows::identifier(&self.id)
            || !(1..=2).contains(&self.columns)
            || !(1..=60).contains(&self.refresh_seconds)
            || self.bindings.is_empty()
            || self.bindings.len() > 12
        {
            return Err("Unsupported widget identity, layout, refresh or binding count".into());
        }
        text(&self.title, 160, true)?;
        text(&self.description, 2000, false)?;
        let mut ids = BTreeSet::new();
        for binding in &self.bindings {
            if !workflows::identifier(&binding.id)
                || !ids.insert(&binding.id)
                || !(1..=32).contains(&binding.limit)
            {
                return Err("Invalid widget binding identity or row limit".into());
            }
            text(&binding.label, 160, true)?;
        }
        Ok(())
    }
    pub fn requested(&self) -> BTreeSet<DataSource> {
        self.bindings.iter().map(|binding| binding.source).collect()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub detail: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub workspace: String,
    pub captured: u64,
    pub simulated: bool,
    pub values: BTreeMap<DataSource, Vec<Row>>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        text(&self.workspace, 128, true)?;
        let mut bytes = 0;
        for rows in self.values.values() {
            if rows.len() > 32 {
                return Err("Widget source exceeds 32 rows".into());
            }
            for row in rows {
                for value in [&row.label, &row.value, &row.detail] {
                    text(value, 512, false)?;
                    bytes += value.len();
                }
            }
        }
        if bytes > 32 * 1024 {
            return Err("Widget snapshot exceeds 32 KiB".into());
        }
        Ok(())
    }
}
/// Deliberately not serialized: data grants do not survive an app/project session change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grants {
    pub workspace: String,
    pub sources: BTreeSet<DataSource>,
}
impl Grants {
    pub fn rows<'a>(
        &self,
        widget: &Widget,
        binding: &Binding,
        snapshot: &'a Snapshot,
    ) -> Result<Vec<&'a Row>, String> {
        widget.validate()?;
        snapshot.validate()?;
        if !widget.bindings.contains(binding) {
            return Err("Binding is not part of the reviewed widget".into());
        }
        if self.workspace != snapshot.workspace || !self.sources.contains(&binding.source) {
            return Err("Grant this summary binding for the current workspace first".into());
        }
        Ok(snapshot
            .values
            .get(&binding.source)
            .into_iter()
            .flatten()
            .take(usize::from(binding.limit))
            .collect())
    }
}

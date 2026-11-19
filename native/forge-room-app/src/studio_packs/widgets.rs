use super::runtime;
use eframe::egui;
use forge_room_core::packs::{DataSource, Grants, Row, Snapshot, View, Widget};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Panel {
    selected: String,
    pinned: BTreeSet<String>,
    grants: BTreeMap<String, Grants>,
    snapshot: Option<Snapshot>,
    workspace: String,
    trusted: bool,
    last: Option<Instant>,
    pub notice: String,
}
impl Panel {
    pub fn reconcile(&mut self, workspace: &str, trusted: bool) {
        if self.workspace != workspace || self.trusted != trusted {
            self.workspace = workspace.into();
            self.trusted = trusted;
            self.clear();
        }
    }
    pub fn clear(&mut self) {
        self.grants.clear();
        self.snapshot = None;
        self.last = None;
    }
    pub fn resources_changed(&mut self) {
        self.clear();
        let resources = runtime::current();
        self.pinned.retain(|key| {
            resources
                .assets
                .get(key)
                .is_some_and(|asset| matches!(asset.value, runtime::Value::Widget(_)))
        });
    }
    pub fn sources(
        &self,
        ctx: &egui::Context,
        center_visible: bool,
    ) -> Option<BTreeSet<DataSource>> {
        if !self.trusted {
            return None;
        }
        let mut shown = self.pinned.clone();
        if center_visible {
            shown.insert(self.selected.clone());
        }
        let resources = runtime::current();
        let mut sources = BTreeSet::new();
        let mut interval = 60u32;
        for key in shown {
            if let Some(runtime::Asset {
                value: runtime::Value::Widget(widget),
                ..
            }) = resources.assets.get(&key)
                && let Some(grants) = self.grants.get(&key)
                && grants.workspace == self.workspace
            {
                sources.extend(grants.sources.iter().copied());
                interval = interval.min(widget.refresh_seconds);
            }
        }
        if sources.is_empty() {
            return None;
        }
        let interval = Duration::from_secs(u64::from(interval));
        if let Some(last) = self.last
            && last.elapsed() < interval
        {
            ctx.request_repaint_after(interval.saturating_sub(last.elapsed()));
            return None;
        }
        ctx.request_repaint_after(interval);
        Some(sources)
    }
    pub fn capture(&mut self, snapshot: Snapshot) -> Result<(), String> {
        snapshot.validate()?;
        if snapshot.workspace != self.workspace || snapshot.simulated {
            return Err("Live widget snapshot scope changed or contained simulated data".into());
        }
        let allowed = self
            .grants
            .values()
            .filter(|grant| grant.workspace == self.workspace)
            .flat_map(|grant| grant.sources.iter().copied())
            .collect::<BTreeSet<_>>();
        if snapshot
            .values
            .keys()
            .any(|source| !allowed.contains(source))
        {
            return Err("Snapshot includes a source not granted in this workspace".into());
        }
        self.snapshot = Some(snapshot);
        self.last = Some(Instant::now());
        Ok(())
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let resources = runtime::current();
        ui.heading("Read-only workspace widgets");
        ui.weak("Only supported cached summaries are available. No code, arbitrary files, terminal output, network API or automatic AI transmission.");
        egui::ComboBox::from_id_salt("pack-widget-select")
            .selected_text(if self.selected.is_empty() {
                "Choose an enabled widget"
            } else {
                &self.selected
            })
            .show_ui(ui, |ui| {
                for (key, asset) in &resources.assets {
                    if matches!(asset.value, runtime::Value::Widget(_)) {
                        ui.selectable_value(
                            &mut self.selected,
                            key.clone(),
                            format!("{} ({key})", asset.label),
                        );
                    }
                }
            });
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        let Some(runtime::Asset {
            value: runtime::Value::Widget(widget),
            pack,
            ..
        }) = resources.assets.get(&self.selected)
        else {
            ui.label(
                "Enable a widget resource from the pack library, or author one in a local pack.",
            );
            return;
        };
        ui.label(format!("Source {pack}"));
        ui.collapsing("Simulated preview and exact widget definition", |ui| {
            ui.colored_label(
                crate::theme::warn(),
                "SIMULATED: demonstration values, not project evidence",
            );
            let values = widget
                .requested()
                .into_iter()
                .map(|source| {
                    (
                        source,
                        vec![Row {
                            label: source.label().into(),
                            value: "Example value".into(),
                            detail: "Simulated preview only".into(),
                        }],
                    )
                })
                .collect();
            let snapshot = Snapshot {
                workspace: "simulated-preview".into(),
                captured: 0,
                simulated: true,
                values,
            };
            let grants = Grants {
                workspace: snapshot.workspace.clone(),
                sources: widget.requested(),
            };
            render(ui, widget, &grants, Some(&snapshot));
            ui.monospace(serde_json::to_string_pretty(widget).unwrap_or_default());
        });
        let key = self.selected.clone();
        self.permissions(ui, &key, widget);
        ui.horizontal(|ui| {
            let mut pinned = self.pinned.contains(&key);
            if ui
                .checkbox(&mut pinned, "Pin as a separate native panel")
                .changed()
            {
                if pinned {
                    if self.pinned.len() < 4 {
                        self.pinned.insert(key.clone());
                    } else {
                        self.notice = "At most four widget panels can be pinned".into();
                    }
                } else {
                    self.pinned.remove(&key);
                }
            }
            if ui.button("Refresh cached snapshot").clicked() {
                self.last = None;
            }
            if ui.button("Export widget definition…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("widget.json")
                    .save_file()
            {
                self.notice = serde_json::to_vec_pretty(widget)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| crate::local_records::create_file(&path, &bytes, 64 * 1024))
                    .err()
                    .unwrap_or_else(|| "Widget exported to a new file".into());
            }
        });
        let grants = self.grants.get(&key).cloned().unwrap_or_else(|| Grants {
            workspace: self.workspace.clone(),
            sources: BTreeSet::new(),
        });
        render(ui, widget, &grants, self.snapshot.as_ref());
    }
    fn permissions(&mut self, ui: &mut egui::Ui, key: &str, widget: &Widget) {
        ui.label("Data permissions for this workspace/session");
        if !self.trusted {
            ui.label("Open and trust a project before granting summary data.");
        }
        let grants = self.grants.entry(key.into()).or_insert_with(|| Grants {
            workspace: self.workspace.clone(),
            sources: BTreeSet::new(),
        });
        for source in widget.requested() {
            let mut allowed = grants.sources.contains(&source);
            if ui
                .add_enabled(
                    self.trusted,
                    egui::Checkbox::new(&mut allowed, source.label()),
                )
                .changed()
            {
                if allowed {
                    grants.sources.insert(source);
                } else {
                    grants.sources.remove(&source);
                }
                self.snapshot = None;
                self.last = None;
                ui.ctx().request_repaint();
            }
        }
    }
    pub fn windows(&mut self, ctx: &egui::Context) {
        let resources = runtime::current();
        for key in self.pinned.clone() {
            let Some(runtime::Asset {
                value: runtime::Value::Widget(widget),
                ..
            }) = resources.assets.get(&key)
            else {
                continue;
            };
            let mut open = true;
            egui::Window::new(&widget.title)
                .id(egui::Id::unique(("pack-widget", &key)))
                .open(&mut open)
                .default_size([360.0, 300.0])
                .show(ctx, |ui| {
                    ui.label(&key);
                    self.permissions(ui, &key, widget);
                    let grants = self.grants.get(&key).cloned().unwrap_or_else(|| Grants {
                        workspace: self.workspace.clone(),
                        sources: BTreeSet::new(),
                    });
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| render(ui, widget, &grants, self.snapshot.as_ref()));
                });
            if !open {
                self.pinned.remove(&key);
            }
        }
    }
}
fn render(ui: &mut egui::Ui, widget: &Widget, grants: &Grants, snapshot: Option<&Snapshot>) {
    ui.strong(&widget.title);
    ui.label(&widget.description);
    let Some(snapshot) = snapshot else {
        ui.label("No captured summary data. Choose permissions or refresh.");
        return;
    };
    ui.label(if snapshot.simulated {
        "Simulated preview".into()
    } else {
        format!(
            "Cached summary snapshot taken at {} (not a fresh tool run)",
            snapshot.captured
        )
    });
    egui::Grid::new(("pack-widget-bindings", &widget.id, snapshot.simulated))
        .num_columns(usize::from(widget.columns))
        .spacing([16.0, 12.0])
        .show(ui, |ui| {
            for (index, binding) in widget.bindings.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.strong(&binding.label);
                    match grants.rows(widget, binding, snapshot) {
                        Err(error) => {
                            ui.label(error);
                        }
                        Ok(rows) if rows.is_empty() => {
                            ui.label("No summary is available from this source");
                        }
                        Ok(rows) => match binding.view {
                            View::Metric => {
                                for row in rows {
                                    ui.heading(&row.value);
                                    ui.label(&row.label);
                                    if !row.detail.is_empty() {
                                        ui.weak(&row.detail);
                                    }
                                }
                            }
                            View::List => {
                                for row in rows {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(&row.label);
                                        ui.strong(&row.value);
                                    });
                                    if !row.detail.is_empty() {
                                        ui.weak(&row.detail);
                                    }
                                }
                            }
                            View::Table => {
                                egui::Grid::new((
                                    "pack-widget-rows",
                                    &widget.id,
                                    &binding.id,
                                    snapshot.simulated,
                                ))
                                .num_columns(3)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Summary");
                                    ui.strong("Value");
                                    ui.strong("Provenance");
                                    ui.end_row();
                                    for row in rows {
                                        ui.label(&row.label);
                                        ui.strong(&row.value);
                                        ui.weak(&row.detail);
                                        ui.end_row();
                                    }
                                });
                            }
                        },
                    }
                });
                if (index + 1) % usize::from(widget.columns) == 0 {
                    ui.end_row();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(workspace: &str, simulated: bool) -> Snapshot {
        Snapshot {
            workspace: workspace.into(),
            simulated,
            captured: 1,
            values: [(DataSource::Git, vec![])].into_iter().collect(),
        }
    }
    #[test]
    fn snapshots_require_session_grants_and_are_cleared_on_scope_trust_or_resource_change() {
        let mut panel = Panel::default();
        panel.reconcile("first", true);
        assert!(panel.capture(snapshot("first", false)).is_err());
        panel.grants.insert(
            "local.widget/widget".into(),
            Grants {
                workspace: "first".into(),
                sources: [DataSource::Git].into_iter().collect(),
            },
        );
        assert!(panel.capture(snapshot("other", false)).is_err());
        assert!(panel.capture(snapshot("first", true)).is_err());
        panel.capture(snapshot("first", false)).unwrap();
        panel.reconcile("second", true);
        assert!(panel.snapshot.is_none());
        assert!(panel.grants.is_empty());
        panel.grants.insert(
            "local.widget/widget".into(),
            Grants {
                workspace: "second".into(),
                sources: [DataSource::Git].into_iter().collect(),
            },
        );
        panel.reconcile("second", false);
        assert!(panel.grants.is_empty());
        assert!(panel.sources(&egui::Context::default(), true).is_none());
        panel.resources_changed();
        assert!(panel.snapshot.is_none());
    }
}

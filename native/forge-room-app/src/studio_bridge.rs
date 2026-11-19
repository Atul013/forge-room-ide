//! The only bridge from live IDE state into declarative Studio resources.
use crate::{ForgeRoomApp, egui, settings, studio_packs, theme};
use forge_room_core::{
    packs::{DataSource, Density, Layout, Profile, Row, Snapshot},
    workflows,
};
use std::collections::{BTreeMap, BTreeSet};
impl ForgeRoomApp {
    pub(super) fn pack_scope(&self) -> String {
        self.repo_root()
            .map(|root| workflows::digest(root.to_string_lossy().to_lowercase().as_bytes()))
            .unwrap_or_default()
    }
    pub(super) fn appearance(&self) -> Profile {
        Profile {
            name: "Current appearance".into(),
            theme: Some(studio_packs::runtime::theme_reference(
                &theme::current().key,
            )),
            editor_scale: self.editor_scale,
            ui_scale: self.ui_scale,
            density: self.density,
            reduced_motion: self.reduced_motion,
            layout: Layout {
                sidebar: self.show_sidebar,
                dock: self.show_right,
                terminal: self.show_terminal,
                preview: self.show_preview,
                sidebar_width: self.panel_sizes.0.clamp(180, 560),
                dock_width: self.panel_sizes.1.clamp(280, 640),
                terminal_height: self.panel_sizes.2.clamp(120, 720),
            },
            ..Profile::default()
        }
    }
    fn appearance_style(&self) {
        self.egui_ctx.all_styles_mut(|style| {
            let (gap, padding) = match self.density {
                Density::Compact => (4.0, 4.0),
                Density::Comfortable => (6.0, 6.0),
                Density::Spacious => (9.0, 9.0),
            };
            style.spacing.item_spacing = egui::vec2(8.0, gap);
            style.spacing.button_padding = egui::vec2(12.0, padding);
            style.animation_time = if self.reduced_motion { 0.0 } else { 0.15 };
        });
    }
    pub(super) fn apply_appearance(&mut self, profile: &Profile) -> Result<(), String> {
        profile.validate()?;
        let resources = studio_packs::runtime::current();
        let actual = profile
            .theme
            .as_ref()
            .map(|reference| {
                resources
                    .theme_refs
                    .get(reference)
                    .cloned()
                    .or_else(|| theme::find(reference).map(|_| reference.clone()))
                    .ok_or_else(|| {
                        format!("Enable the profile's theme before preview: {reference}")
                    })
            })
            .transpose()?;
        if let Some(actual) = actual {
            theme::set(&self.egui_ctx, &actual);
        }
        self.editor_scale = profile.editor_scale;
        self.ui_scale = profile.ui_scale;
        self.density = profile.density;
        self.reduced_motion = profile.reduced_motion;
        self.show_sidebar = profile.layout.sidebar;
        self.show_right = profile.layout.dock;
        self.show_terminal = profile.layout.terminal;
        self.show_preview = profile.layout.preview;
        self.panel_sizes = (
            profile.layout.sidebar_width,
            profile.layout.dock_width,
            profile.layout.terminal_height,
        );
        self.layout_generation = self.layout_generation.wrapping_add(1);
        self.terminal_maximized = false;
        let native = self
            .egui_ctx
            .native_pixels_per_point()
            .unwrap_or(self.native_ppp);
        self.native_ppp = native;
        let points = self
            .egui_ctx
            .input(|input| input.viewport().monitor_size.map(|size| (size.x, size.y)));
        let cap = settings::ui_scale_cap(points, self.egui_ctx.pixels_per_point());
        self.egui_ctx.set_zoom_factor(settings::ui_zoom_factor(
            self.ui_scale.map(|scale| scale.min(cap)),
            native,
        ));
        self.appearance_style();
        self.egui_ctx.request_repaint();
        Ok(())
    }
    pub(super) fn studio_packs_frame(
        &mut self,
        ctx: &egui::Context,
        config: Option<&crate::native_models::Config>,
    ) {
        let theme_reference = studio_packs::runtime::theme_reference(&theme::current().key);
        self.studio_packs.tick(ctx);
        if std::mem::take(&mut self.studio_packs.changed) {
            let resources = studio_packs::runtime::current();
            let saved = settings::load().theme;
            let actual = resources
                .theme_refs
                .get(&theme_reference)
                .cloned()
                .or_else(|| saved.filter(|key| theme::find(key).is_some()))
                .unwrap_or_else(|| theme::DEFAULT_THEME.into());
            theme::set(ctx, &actual);
            self.studio_packs.widgets.resources_changed();
        }
        self.appearance_style();
        let scope = self.pack_scope();
        let current = self.appearance();
        self.studio_packs.profiles.settle(&current);
        self.studio_packs
            .widgets
            .reconcile(&scope, self.trusted && !scope.is_empty());
        if let Some(sources) = self
            .studio_packs
            .widgets
            .sources(ctx, self.studio_packs.open)
        {
            let snapshot = self.widget_snapshot(scope.clone(), &sources);
            if let Err(error) = self.studio_packs.widgets.capture(snapshot) {
                self.status = error;
            }
        }
        self.studio_packs.show(ctx, &current, &scope, config);
        if let Some(action) = self.studio_packs.profiles.requested.take() {
            let result = match action {
                studio_packs::ProfileAction::Preview(profile) => {
                    self.apply_appearance(&profile).map(|_| {
                        let after = self.appearance();
                        self.studio_packs
                            .profiles
                            .applied(current.clone(), after, scope.clone());
                    })
                }
                studio_packs::ProfileAction::Keep => {
                    let mut profile = self.appearance();
                    if let Some(candidate) = &self.studio_packs.profiles.candidate {
                        profile.name = candidate.name.clone();
                    }
                    let value = serde_json::to_value(&profile).map_err(|e| e.to_string());
                    value
                        .and_then(|value| {
                            settings::update(|settings| {
                                settings.appearance_profile = Some(value);
                                settings.theme = Some(theme::current().key.clone());
                                settings.editor_scale = Some(profile.editor_scale);
                                settings.ui_scale = profile.ui_scale;
                            })
                            .map_err(|e| e.to_string())
                        })
                        .map(|_| {
                            let notice = self
                                .studio_packs
                                .record_appearance(&profile)
                                .err()
                                .map(|error| {
                                    format!("Appearance kept; history was not saved: {error}")
                                })
                                .unwrap_or_else(|| {
                                    "Appearance kept and private history recorded".into()
                                });
                            self.studio_packs.profiles.ended(notice);
                            self.save_session(true);
                        })
                }
                studio_packs::ProfileAction::Restore {
                    expected,
                    mut previous,
                    preserve_layout,
                } => {
                    if self.appearance() != expected {
                        Err("Appearance changed since restore review; inspect it again".into())
                    } else {
                        if preserve_layout {
                            previous.layout = current.layout.clone();
                        }
                        self.apply_appearance(&previous).map(|_| {
                            self.studio_packs
                                .profiles
                                .ended("Previous appearance restored".into())
                        })
                    }
                }
            };
            if let Err(error) = result {
                self.studio_packs.profiles.notice = error;
            }
        }
        if let Some(library) = self.studio_packs.stage_workflows.take() {
            self.status = if self.trusted {
                self.repo_root()
                    .ok_or("Open a project before staging definitions".into())
                    .and_then(|root| self.workflow_center.import_library(&root, library))
                    .err()
                    .unwrap_or_else(|| {
                        "Definitions staged in Workflow editor; inspect and save explicitly".into()
                    })
            } else {
                "Trust the active project before staging definitions".into()
            };
        }
        if let Some((title, text)) = self.studio_packs.stage_skill.take() {
            self.status = if self.trusted && !self.ai.busy() && !self.ai.context_locked() {
                self.repo_root()
                    .ok_or("Open an active project".into())
                    .and_then(|root| {
                        self.ai.context.capture(
                            &root,
                            crate::ai_context::Kind::Skills,
                            title,
                            text,
                            false,
                        )
                    })
                    .err()
                    .unwrap_or_else(|| {
                        "Pack Skill captured locally; explicitly Include before sending".into()
                    })
            } else {
                "Skill capture needs an idle trusted active project outside a controlled session"
                    .into()
            };
        }
    }
    fn widget_snapshot(&self, scope: String, sources: &BTreeSet<DataSource>) -> Snapshot {
        let mut values = BTreeMap::new();
        for source in sources {
            let rows = match source {
                DataSource::Workspace => vec![summary(
                    "Open buffers",
                    self.tabs.tabs.len(),
                    "Application buffer count; contents and paths excluded",
                )],
                DataSource::Git => self
                    .git_status
                    .as_ref()
                    .and_then(|status| status.as_ref().ok())
                    .map(|status| {
                        vec![
                            summary(
                                "Repository changes",
                                status.changes.len(),
                                "Cached repository-wide status; paths excluded",
                            ),
                            summary(
                                "Staged changes",
                                status.changes.iter().filter(|c| c.is_staged()).count(),
                                "Cached status; not a fresh Git command",
                            ),
                            summary(
                                "Conflicts",
                                status.changes.iter().filter(|c| c.is_conflict()).count(),
                                "Cached repository-wide status",
                            ),
                        ]
                    })
                    .unwrap_or_default(),
                DataSource::Problems => self
                    .repo_root()
                    .and_then(|root| {
                        self.problems
                            .as_ref()
                            .filter(|report| {
                                report
                                    .manifest
                                    .as_ref()
                                    .is_some_and(|path| path.starts_with(&root))
                            })
                            .map(|report| {
                                vec![
                                    summary(
                                        "Reported errors",
                                        report
                                            .problems
                                            .iter()
                                            .filter(|p| p.error && p.file.starts_with(&root))
                                            .count(),
                                        "Cached diagnostics; no messages or file names",
                                    ),
                                    summary(
                                        "Reported warnings",
                                        report
                                            .problems
                                            .iter()
                                            .filter(|p| !p.error && p.file.starts_with(&root))
                                            .count(),
                                        "Cached diagnostics; absence is not a check result",
                                    ),
                                ]
                            })
                    })
                    .unwrap_or_default(),
                DataSource::Workflows => self
                    .workflow_center
                    .widget_summary()
                    .map(|(total, passed, failed, active)| {
                        vec![
                            summary("Workflow records", total, "Local active-project history"),
                            summary(
                                "Succeeded records",
                                passed,
                                "Mixed manual/check/model steps; not CI acceptance",
                            ),
                            summary("Failed records", failed, "Local active-project history"),
                            summary("Owned active steps", active, "Bounded workflow runner"),
                        ]
                    })
                    .unwrap_or_default(),
                DataSource::Room => {
                    let tasks = self.room.workflow_tasks();
                    vec![summary(
                        "Visible public network tasks",
                        tasks.len(),
                        "Cached Room task count; messages, identities and private data excluded",
                    )]
                }
            };
            values.insert(*source, rows);
        }
        Snapshot {
            workspace: scope,
            captured: forge_room_core::now_epoch_seconds(),
            simulated: false,
            values,
        }
    }
}
fn summary(label: &str, value: usize, detail: &str) -> Row {
    Row {
        label: label.into(),
        value: value.to_string(),
        detail: detail.into(),
    }
}

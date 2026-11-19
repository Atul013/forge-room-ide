//! Frozen public deadline scenarios. Imports are comparison data, never executable plans.
use eframe::egui;
use forge_room_core::{
    TaskStatus,
    transport::{
        Action, Countdown, MAX_RESCUE_REPORT, RescueReport, RoomLifecycle, Snapshot, rescue_anchor,
        rescue_report,
    },
};
use std::{io::Read, time::Instant};

#[derive(Default)]
pub struct Panel {
    room: String,
    selected: Vec<String>,
    reviewed: Option<(RescueReport, Instant)>,
    baseline: Option<RescueReport>,
    required_only: bool,
    query: String,
    confirmed_reset: bool,
    notice: String,
}

impl Panel {
    fn capture(&mut self, snapshot: &Snapshot, remaining: u64) -> Result<(), String> {
        let report = rescue_report(
            snapshot,
            &self.selected,
            &Countdown {
                remaining_seconds: remaining,
                running: snapshot.shared.countdown.running,
            },
            snapshot.server_time_epoch_seconds,
        )?;
        self.reviewed = Some((report, Instant::now()));
        self.confirmed_reset = false;
        Ok(())
    }
    fn current(&self, snapshot: &Snapshot) -> bool {
        self.reviewed.as_ref().is_some_and(|(report, at)| {
            report.room == snapshot.shared.room_id
                && at.elapsed().as_secs() < 120
                && rescue_anchor(
                    &snapshot.shared.room_id,
                    &snapshot.shared.tasks,
                    &snapshot.shared.readiness,
                    &snapshot.shared.shipping,
                )
                .is_ok_and(|a| a == report.anchor)
                && {
                    let mut selected = self.selected.clone();
                    selected.sort();
                    selected == report.selected
                }
        })
    }
    fn publish(&self, snapshot: &Snapshot, host: bool) -> Result<Action, String> {
        if !host
            || snapshot.lifecycle != RoomLifecycle::Active
            || !self.current(snapshot)
            || !self.confirmed_reset
        {
            return Err(
                "Review current active host scope and acknowledge the report reset first".into(),
            );
        }
        let report = &self.reviewed.as_ref().ok_or("Review a scenario first")?.0;
        let mut published = snapshot.shared.shipping.plan.required_tasks.clone();
        published.sort();
        if snapshot.shared.shipping.revision == 0 || report.required == published {
            return Err("Publish an initial Ship scope or choose a different scenario".into());
        }
        Ok(Action::SetRescueScope {
            expected_anchor: report.anchor.clone(),
            selected: report.selected.clone(),
        })
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        host: bool,
        remaining: u64,
        actions: &mut Vec<Action>,
    ) -> Option<String> {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        if self.room != snapshot.shared.room_id {
            self.room = snapshot.shared.room_id.clone();
            self.selected = snapshot.shared.shipping.plan.required_tasks.clone();
            self.reviewed = None;
            self.confirmed_reset = false;
            self.notice.clear();
        }
        ui.heading("Deadline rescue");
        ui.label("Compare a smaller release scope, review dependency risk and agree help with owners. Estimates and check results are manual declarations.");
        ui.small("Scenarios do not run checks, start agents, change task owners or deploy. Critical-path estimates assume independent branches can run in parallel; shared-owner capacity is not scheduled.");
        if !snapshot.shared.shipping.supported || !snapshot.shared.readiness.supported {
            ui.label("Update the host to support Ship and readiness coordination.");
            return None;
        }
        egui::CollapsingHeader::new("Choose release targets").default_open(self.reviewed.is_none()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Use published scope").clicked() {
                    self.selected = snapshot.shared.shipping.plan.required_tasks.clone(); self.confirmed_reset = false;
                }
                if ui.button("Use unfinished critical tasks").clicked() {
                    self.selected = snapshot.shared.tasks.iter().filter(|t| t.priority == forge_room_core::TaskPriority::Critical && t.status != TaskStatus::Done).map(|t| t.id.clone()).collect(); self.confirmed_reset = false;
                }
            });
            ui.small("Prerequisites are included automatically. Deferred tasks remain in the Room; unfinished critical tasks continue to block Ship even when deferred.");
            for task in &snapshot.shared.tasks {
                let mut selected = self.selected.contains(&task.id);
                if ui.checkbox(&mut selected, format!("{} · {} · {:?}", task.id, task.title, task.priority)).changed() {
                    if selected { self.selected.push(task.id.clone()); } else { self.selected.retain(|id| id != &task.id); }
                    self.confirmed_reset = false;
                }
            }
        });
        if ui.button("Review frozen scenario").clicked() {
            self.notice = match self.capture(snapshot, remaining) {
                Ok(()) => "Scenario captured from the current public Room snapshot".into(),
                Err(e) => e,
            };
        }
        let current = self.current(snapshot);
        let mut inspect = None;
        if let Some((report, at)) = &self.reviewed {
            ui.separator();
            ui.strong(format!(
                "{} required ({} targets), {} deferred · {} unresolved items",
                report.required.len(),
                report.selected.len(),
                report.deferred.len(),
                report.blockers.len()
            ));
            ui.small(format!(
                "Frozen host time {}, {}s since review · {} · countdown {}s ({})",
                report.observed_at,
                at.elapsed().as_secs(),
                if current {
                    "task/readiness/Ship source current"
                } else {
                    "review again before publishing"
                },
                report.countdown_seconds,
                if report.countdown_running {
                    "running at capture"
                } else {
                    "paused at capture"
                }
            ));
            ui.label("Time continues after capture. This report is a recommendation snapshot, not verified readiness or a promise of completion.");
            egui::CollapsingHeader::new("Required workload by owner").default_open(true).show(ui, |ui| {
                for owner in &report.owners {
                    let name = report.rows.iter().find(|r| r.owner == owner.owner).map(|r| r.owner_name.as_str()).unwrap_or("Unknown owner");
                    ui.label(format!("{} · {} min known + {} unknown estimates · {} unfinished tasks", name, owner.known_minutes, owner.unknown_estimates, owner.tasks.len()));
                }
                ui.small("Sums current-owner remaining estimates once per required task. These are workload bounds, not a resource schedule or delivery guarantee.");
            });
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.required_only, "Required work only");
                ui.label("Find task or owner");
                ui.add(egui::TextEdit::singleline(&mut self.query).char_limit(200));
            });
            let query = self.query.to_lowercase();
            for row in &report.rows {
                if self.required_only && !report.required.contains(&row.task) {
                    continue;
                }
                if !format!("{} {} {}", row.task, row.title, row.owner_name)
                    .to_lowercase()
                    .contains(&query)
                {
                    continue;
                }
                ui.push_id(&row.task, |ui| {
                    egui::CollapsingHeader::new(format!(
                        "{} · {} · {} · {:?} · {} warnings / {} unknowns",
                        row.task,
                        row.title,
                        if row.selected {
                            "target"
                        } else if row.prerequisite {
                            "prerequisite"
                        } else {
                            "deferred"
                        },
                        row.priority,
                        row.warnings.len(),
                        row.unknowns.len()
                    ))
                    .show(ui, |ui| {
                        ui.label(format!("Owner: {} · {:?}", row.owner_name, row.status));
                        ui.label(format!(
                            "Dependency critical path: {} · deadline budget: {}",
                            row.critical_minutes
                                .map_or("unknown / complete".into(), |m| format!("{m} min")),
                            row.budget_seconds
                                .map_or("unknown / paused".into(), |s| format!("{s}s"))
                        ));
                        ui.label(&row.assistance);
                        for warning in &row.warnings {
                            ui.label(format!("Warning: {warning}"));
                        }
                        for gap in &row.unknowns {
                            ui.label(format!("Unknown: {gap}"));
                        }
                        if ui
                            .button("Open current owner readiness and help controls")
                            .clicked()
                        {
                            inspect = Some(row.task.clone());
                        }
                    });
                });
            }
            egui::CollapsingHeader::new("Scenario release blockers").show(ui, |ui| {
                for blocker in &report.blockers { ui.label(blocker); }
                if report.blockers.is_empty() { ui.label("No declared blockers. Actual desktop, provider, two-client, preview and submission checks still require their own evidence."); }
            });
            egui::CollapsingHeader::new("Inspect public report JSON").show(ui, |ui| {
                ui.small("Includes public task/owner identifiers, titles, dependency edges, estimates, consent summaries and blockers. Chat, private patches, local branches, source files and credentials are excluded by projection. Public text may still contain sensitive information members entered. Review before sharing.");
                if let Ok(json) = report.json() {
                    let mut text = json.as_str();
                    ui.add(egui::TextEdit::multiline(&mut text).desired_rows(8).desired_width(f32::INFINITY));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Keep as comparison baseline").clicked() {
                    self.baseline = Some(report.clone());
                }
                if ui.button("Copy frozen public JSON").clicked() {
                    match report.json() {
                        Ok(json) => ui.ctx().copy_text(json),
                        Err(e) => self.notice = e,
                    }
                }
                if ui.button("Save frozen public JSON as…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_file_name("room-rescue-report.json")
                        .save_file()
                {
                    self.notice = match report.json().and_then(|json| {
                        crate::local_records::create_file(&path, json.as_bytes(), MAX_RESCUE_REPORT)
                    }) {
                        Ok(()) => "Frozen report saved to a new file".into(),
                        Err(e) => format!("Report not saved: {e}"),
                    };
                }
            });
        }
        if ui.button("Import comparison report…").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Rescue report", &["json"])
                .pick_file()
        {
            self.notice = match read_report(&path) {
                Ok(report) => {
                    self.baseline = Some(report);
                    "Imported comparison data; current scenario and authority unchanged".into()
                }
                Err(e) => format!("Comparison not imported: {e}"),
            };
        }
        if let Some(baseline) = &self.baseline {
            ui.small(format!("Comparison baseline: host time {} · revision {} · imported reports are unverified claims", baseline.observed_at, baseline.ship_revision));
            if let Some((report, _)) = &self.reviewed {
                match baseline.compare(report) {
                    Ok(changes) if changes.is_empty() => {
                        ui.label("No compared coordination or risk changes");
                    }
                    Ok(changes) => {
                        for change in changes {
                            ui.label(change);
                        }
                    }
                    Err(e) => {
                        ui.label(e);
                    }
                }
            }
        }
        if self.baseline.is_some() && ui.small_button("Clear comparison baseline").clicked() {
            self.baseline = None;
        }
        if host && self.reviewed.is_some() {
            ui.separator();
            ui.strong("Publish reviewed release scope");
            ui.label("Changes only the required-task list, including prerequisites. Preserves release title, assigned checks and preview/submission references. All existing manual check reports reset and need owner reports for the new scope.");
            ui.add_enabled(
                current,
                egui::Checkbox::new(
                    &mut self.confirmed_reset,
                    "I reviewed the new scope and the reset of all manual check reports",
                ),
            );
            let eligibility = self.publish(snapshot, host);
            let enabled = eligibility.is_ok();
            if let Err(reason) = eligibility {
                ui.small(reason);
            }
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new("Publish reviewed scope and reset reports"),
                )
                .clicked()
            {
                match self.publish(snapshot, host) {
                    Ok(action) => {
                        actions.push(action);
                        self.confirmed_reset = false;
                        self.notice =
                            "Scope change queued; wait for host confirmation in Ship".into();
                    }
                    Err(e) => self.notice = e,
                }
            }
        }
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        inspect
    }
}

fn read_report(path: &std::path::Path) -> Result<RescueReport, String> {
    crate::local_records::file(path)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_RESCUE_REPORT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    RescueReport::parse(&bytes)
}

#[cfg(test)]
mod tests;

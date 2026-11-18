//! Human-controlled release scope, check reports and public handoff review.
use eframe::egui;
use forge_room_core::transport::{
    Action, ShipCheck, ShipPlan, ShipResult, Snapshot, ship_blockers, ship_handoff,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Panel {
    draft: Option<(u64, ShipPlan)>,
    report_plan: Option<ShipPlan>,
    notes: HashMap<usize, String>,
    check_title: String,
    check_owner: String,
    focus: bool,
    export: Option<String>,
    notice: String,
}
impl Panel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        host: bool,
        actions: &mut Vec<Action>,
    ) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let state = &snapshot.shared.shipping;
        ui.heading("Ship review");
        ui.label("Release scope and check reports are shared with everyone in this Room. Reports and references are manual declarations.");
        if !state.supported {
            ui.label("This host does not support Ship review. Update the host to use it.");
            return;
        }
        if self.report_plan.as_ref() != Some(&state.plan) {
            self.notes.clear();
            self.report_plan = Some(state.plan.clone());
        }
        let active = snapshot.lifecycle == forge_room_core::transport::RoomLifecycle::Active;
        let blockers = ship_blockers(snapshot);
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Revision {} · {} unresolved items",
                state.revision,
                blockers.len()
            ));
            ui.checkbox(&mut self.focus, "Focus on blockers");
            if ui.button("Review handoff export").clicked() {
                match ship_handoff(snapshot) {
                    Ok(text) => self.export = Some(text),
                    Err(error) => self.notice = error,
                }
            }
        });
        if blockers.is_empty() {
            ui.label("All declared release gates are satisfied. Verify the actual build, preview and submission before shipping.");
        }
        for issue in blockers {
            ui.label(format!("• {issue}"));
        }
        ui.separator();
        ui.strong(if state.plan.title.is_empty() {
            "No published release scope"
        } else {
            &state.plan.title
        });
        ui.label(format!(
            "Preview reference: {}",
            if state.plan.preview_reference.is_empty() {
                "Not provided"
            } else {
                &state.plan.preview_reference
            }
        ));
        ui.label(format!(
            "Submission reference: {}",
            if state.plan.submission_reference.is_empty() {
                "Not provided"
            } else {
                &state.plan.submission_reference
            }
        ));
        for (index, check) in state.plan.checks.iter().enumerate() {
            let report = state.reports.iter().find(|report| report.check == index);
            if self.focus && report.is_some_and(|report| report.result == ShipResult::ClaimedPass) {
                continue;
            }
            ui.push_id(index, |ui| {
                ui.group(|ui| {
                    let owner = snapshot
                        .members
                        .iter()
                        .find(|member| member.id == check.owner)
                        .map(|member| member.name.as_str())
                        .unwrap_or("Departed member");
                    ui.strong(format!("{} · {}", check.title, owner));
                    if let Some(report) = report {
                        ui.label(format!(
                            "{:?} (manual) · host timestamp {}",
                            report.result, report.recorded_at
                        ));
                        ui.label(&report.note);
                    } else {
                        ui.label("Unknown — no owner report");
                    }
                    if snapshot.you == check.owner {
                        let note = self.notes.entry(index).or_default();
                        ui.add(
                            egui::TextEdit::singleline(note)
                                .hint_text("Public check result or evidence reference")
                                .char_limit(1000)
                                .desired_rows(2),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for (label, result) in [
                                ("Report pass", ShipResult::ClaimedPass),
                                ("Report fail", ShipResult::ClaimedFail),
                                ("Reset to unknown", ShipResult::Unknown),
                            ] {
                                if ui
                                    .add_enabled(
                                        active
                                            && (result == ShipResult::Unknown
                                                || !note.trim().is_empty()),
                                        egui::Button::new(label),
                                    )
                                    .clicked()
                                {
                                    actions.push(Action::ReportShipCheck {
                                        expected_revision: state.revision,
                                        check: index,
                                        result,
                                        note: note.clone(),
                                    });
                                }
                            }
                        });
                    }
                });
            });
        }
        if host {
            egui::CollapsingHeader::new("Edit release scope (host)").show(ui, |ui| {
                if self.draft.is_none() || ui.button("Reload published scope").clicked() { self.draft = Some((state.revision, state.plan.clone())); }
                let (revision, plan) = self.draft.as_mut().unwrap();
                if *revision != state.revision { ui.label("The shared revision changed. Reload before publishing; your draft has been kept."); }
                ui.label("Release title");
                ui.add(egui::TextEdit::singleline(&mut plan.title).char_limit(160));
                ui.label("Required tasks (other tasks remain optional for this release)");
                for task in &snapshot.shared.tasks {
                    let mut selected = plan.required_tasks.contains(&task.id);
                    if ui.checkbox(&mut selected, format!("{} · {} · {:?}", task.id, task.title, task.status)).changed() {
                        if selected { plan.required_tasks.push(task.id.clone()); } else { plan.required_tasks.retain(|id| id != &task.id); }
                    }
                }
                ui.label("Preview reference (public text; never fetched automatically)");
                ui.add(egui::TextEdit::singleline(&mut plan.preview_reference).char_limit(500));
                ui.label("Actual submission reference (leave empty until submitted)");
                ui.add(egui::TextEdit::singleline(&mut plan.submission_reference).char_limit(500));
                let mut remove = None;
                for (index, check) in plan.checks.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&check.title);
                        if ui.small_button(format!("Remove check {}", index + 1)).clicked() { remove = Some(index); }
                    });
                }
                if let Some(index) = remove { plan.checks.remove(index); }
                ui.label("New release check");
                ui.add(egui::TextEdit::singleline(&mut self.check_title).hint_text("Build, smoke test, demo or submission check").char_limit(160));
                egui::ComboBox::from_id_salt("ship-check-owner").selected_text(snapshot.members.iter().find(|member| member.id == self.check_owner).map(|member| member.name.as_str()).unwrap_or("Choose owner")).show_ui(ui, |ui| {
                    for member in &snapshot.members { ui.selectable_value(&mut self.check_owner, member.id.clone(), &member.name); }
                });
                if ui.add_enabled(plan.checks.len() < 24 && !self.check_title.trim().is_empty() && !self.check_owner.is_empty(), egui::Button::new("Add check to draft")).clicked() {
                    plan.checks.push(ShipCheck { title: self.check_title.trim().into(), owner: self.check_owner.clone() });
                    self.check_title.clear();
                }
                ui.label("Publishing replaces release scope and resets all check reports. Owners must report again against the new scope.");
                if ui.add_enabled(active && *revision == state.revision && !plan.title.trim().is_empty(), egui::Button::new("Publish scope and reset reports")).clicked() {
                    actions.push(Action::SetShipPlan { expected_revision: *revision, plan: plan.clone() });
                }
            });
        }
        if let Some(text) = &self.export {
            ui.separator();
            ui.strong("Reviewed public handoff snapshot");
            ui.small("This frozen snapshot can become stale. Review again to refresh. No chat, private patches, local source or credentials are included by the exporter; public text may contain information members entered.");
            let mut view = text.as_str();
            ui.add(
                egui::TextEdit::multiline(&mut view)
                    .desired_rows(8)
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy reviewed JSON").clicked() {
                    ui.ctx().copy_text(text.clone());
                }
                if ui.button("Save reviewed JSON as...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_file_name("room-ship-handoff.json")
                        .save_file()
                {
                    use std::io::Write;
                    self.notice = match std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)
                        .and_then(|mut file| {
                            file.write_all(text.as_bytes())
                                .and_then(|()| file.sync_all())
                        }) {
                        Ok(()) => "Handoff saved".into(),
                        Err(error) => {
                            format!("Handoff not saved; choose a new filename: {error}")
                        }
                    };
                }
            });
        }
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_scope_clears_report_drafts_but_retains_reviewed_export_without_sending() {
        let mut snapshot = Snapshot {
            you: "a".repeat(64),
            ..Default::default()
        };
        snapshot.shared.shipping.supported = true;
        snapshot.shared.shipping.revision = 1;
        snapshot.shared.shipping.plan.title = "Release".into();
        snapshot.shared.shipping.plan.checks.push(ShipCheck {
            title: "Smoke".into(),
            owner: snapshot.you.clone(),
        });
        let mut panel = Panel {
            report_plan: Some(snapshot.shared.shipping.plan.clone()),
            ..Default::default()
        };
        panel.notes.insert(0, "Old scope note".into());
        panel.export = Some(ship_handoff(&snapshot).unwrap());
        let original = panel.export.clone();
        snapshot.shared.shipping.plan.title = "Changed release".into();
        let context = egui::Context::default();
        let mut actions = Vec::new();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            panel.show(ui, &snapshot, false, &mut actions)
        });
        output.textures_delta.clear();
        assert!(panel.notes.get(&0).is_none_or(String::is_empty));
        assert_eq!(panel.export, original);
        assert!(actions.is_empty());
    }
}

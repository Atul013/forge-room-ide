//! Explicit network Room coordination. Model execution and ownership changes stay separate.
use eframe::egui;
use forge_room_core::transport::{
    Action, Countdown, Declaration, ReadinessProfile, Snapshot, assess,
};
use std::{collections::HashMap, time::Instant};
struct Draft {
    revision: u64,
    profile: ReadinessProfile,
    files: String,
}
impl Draft {
    fn from_published(
        published: Option<&forge_room_core::transport::TaskReadiness>,
        owner: &str,
    ) -> Self {
        let mut profile = published.map(|p| p.profile.clone()).unwrap_or_default();
        if published.is_some_and(|p| p.owner != owner) {
            profile.help_allowed = false;
        }
        Self {
            revision: published.map_or(0, |p| p.revision),
            files: profile.files.join("\n"),
            profile,
        }
    }
}
#[derive(Default)]
struct ImpactDraft {
    target: String,
    file: String,
    revision: String,
    summary: String,
}
#[derive(Default)]
pub struct Panel {
    pub preflight: crate::room_preflight::State,
    scan: crate::room_impact_scan::State,
    impact_drafts: HashMap<String, ImpactDraft>,
    help_drafts: HashMap<String, String>,
    drafts: HashMap<String, Draft>,
    selected: Option<String>,
    sample: Option<(u64, Instant)>,
    pub brief: Option<String>,
    notice: String,
}
fn declaration(ui: &mut egui::Ui, name: &str, value: &mut Declaration) {
    egui::ComboBox::from_id_salt(name)
        .selected_text(format!("{name}: {value:?} (manual)"))
        .show_ui(ui, |ui| {
            for status in [
                Declaration::Unknown,
                Declaration::ClaimedPass,
                Declaration::ClaimedFail,
                Declaration::NotRequired,
            ] {
                ui.selectable_value(value, status, format!("{status:?}"));
            }
        });
}
impl Panel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        root: Option<&std::path::Path>,
        remaining: u64,
        actions: &mut Vec<Action>,
    ) {
        let state = &snapshot.shared.readiness;
        ui.heading("Readiness, assistance and impact");
        ui.label("Owner estimates and declarations are manual. No automatic test verification, task completion, agent start or takeover occurs here.");
        if !state.supported {
            ui.label("This host does not advertise readiness coordination. Use an updated host; existing Room features remain available.");
            return;
        }
        if let Some(suggestion) = self.scan.show(ui, root, snapshot) {
            self.selected = Some(suggestion.source.clone());
            self.impact_drafts.insert(
                suggestion.source,
                ImpactDraft {
                    target: suggestion.target,
                    file: suggestion.file,
                    revision: suggestion.base,
                    summary: suggestion.summary,
                },
            );
        }
        if self
            .sample
            .as_ref()
            .is_none_or(|(epoch, _)| *epoch != snapshot.server_time_epoch_seconds)
        {
            self.sample = Some((snapshot.server_time_epoch_seconds, Instant::now()));
        }
        let age = self
            .sample
            .as_ref()
            .map_or(0, |(_, at)| at.elapsed().as_secs());
        let now = snapshot.server_time_epoch_seconds.saturating_add(age);
        ui.small(format!("Host-time anchor {}s old. Countdown {}. Paused countdowns do not consume task deadline budgets.",age,if snapshot.shared.countdown.running{"running"}else{"paused/ended"}));
        let assessments = assess(
            &snapshot.shared.tasks,
            state,
            &Countdown {
                remaining_seconds: remaining,
                running: snapshot.shared.countdown.running,
            },
            now,
        );
        egui::CollapsingHeader::new("Room risk overview")
            .default_open(true)
            .show(ui, |ui| {
                for assessment in &assessments {
                    let Some(task) = snapshot
                        .shared
                        .tasks
                        .iter()
                        .find(|t| t.id == assessment.task)
                    else {
                        continue;
                    };
                    ui.label(format!(
                        "{}: {} | {} warning(s), {} unknown(s)",
                        task.id,
                        task.title,
                        assessment.reasons.len(),
                        assessment.gaps.len()
                    ));
                    if ui.small_button(format!("Inspect {}", task.id)).clicked() {
                        self.selected = Some(task.id.clone());
                    }
                }
            });
        egui::ComboBox::from_id_salt("readiness-task")
            .selected_text(self.selected.as_deref().unwrap_or("Choose shared task"))
            .show_ui(ui, |ui| {
                for task in &snapshot.shared.tasks {
                    ui.selectable_value(
                        &mut self.selected,
                        Some(task.id.clone()),
                        format!("{}: {}", task.id, task.title),
                    );
                }
            });
        let Some(task) = self
            .selected
            .as_ref()
            .and_then(|id| snapshot.shared.tasks.iter().find(|t| &t.id == id))
        else {
            return;
        };
        let owner = task.owner_id == snapshot.you;
        let published = state.profile(&task.id);
        let revision = published.map_or(0, |p| p.revision);
        if let Some(a) = assessments.iter().find(|a| a.task == task.id) {
            ui.label(format!(
                "Dependency critical path: {} | Deadline budget: {}",
                a.critical_minutes
                    .map_or("unknown".into(), |m| format!("{m} min")),
                a.budget_seconds
                    .map_or("unknown / paused".into(), |s| format!("{} min", s / 60))
            ));
            ui.small("Critical path assumes independent branches can proceed in parallel; it is not a resource schedule or completion guarantee.");
            if let Some(recommendation) = &a.recommendation {
                ui.label(format!("Recommendation: {recommendation}"));
            }
            for reason in &a.reasons {
                ui.label(format!("Warning: {reason}"));
            }
            for gap in &a.gaps {
                ui.label(format!("Unknown: {gap}"));
            }
            ui.label(if a.help_allowed{"Owner allows assistance recommendations; starting an agent still needs separate approval."}else{"Assistance recommendations are opted out until the current owner publishes opt-in."});
        }
        self.preflight.show(ui, root, snapshot, &task.id, actions);
        if ui
            .button("Stage task brief for AI context review")
            .clicked()
        {
            self.brief = Some(brief(snapshot, &task.id));
        }
        if let Some(p) = published {
            ui.small(format!(
                "Readiness revision {}, author {}, updated Unix {}{}",
                p.revision,
                p.owner.chars().take(8).collect::<String>(),
                p.updated,
                if p.owner != task.owner_id {
                    "; prior owner data requires current-owner review"
                } else {
                    ""
                }
            ));
        }
        let draft = self
            .drafts
            .entry(task.id.clone())
            .or_insert_with(|| Draft::from_published(published, &task.owner_id));
        ui.add_enabled_ui(owner&&task.status!=forge_room_core::TaskStatus::Done,|ui|{
            if draft.revision!=revision{ui.label("Published readiness changed. Your draft is preserved; reload deliberately before publishing.");}
            if ui.button("Reload published readiness (replaces this draft)").clicked(){*draft=Draft::from_published(published,&task.owner_id);}
            let mut estimated=draft.profile.estimate_minutes.is_some();if ui.checkbox(&mut estimated,"Remaining estimate known").changed(){draft.profile.estimate_minutes=estimated.then_some(30);}
            if let Some(minutes)=&mut draft.profile.estimate_minutes{ui.add(egui::DragValue::new(minutes).range(1..=10080).suffix(" min"));}
            ui.label("Blocker (shared with the Room)");ui.add(egui::TextEdit::singleline(&mut draft.profile.blocker).char_limit(500));
            ui.label("Explicit linked relative paths, one per line. Names are shared; contents are not uploaded.");ui.add(egui::TextEdit::multiline(&mut draft.files).desired_rows(3).char_limit(4104));
            declaration(ui,"Checks",&mut draft.profile.checks);declaration(ui,"Preview",&mut draft.profile.preview);declaration(ui,"Submission",&mut draft.profile.submission);
            ui.checkbox(&mut draft.profile.help_allowed,"Allow assistance recommendations (owner opt-in)");
            if ui.add_enabled(draft.revision==revision,egui::Button::new("Publish owner readiness")).clicked(){let mut profile=draft.profile.clone();profile.files=draft.files.lines().map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();match profile.validate(){Ok(())=>actions.push(Action::UpdateReadiness{task:task.id.clone(),expected_revision:draft.revision,profile}),Err(e)=>self.notice=e}}
        });
        let reason = self.help_drafts.entry(task.id.clone()).or_default();
        egui::CollapsingHeader::new("Assistance requests")
            .default_open(true)
            .show(ui, |ui| {
                for h in state.help.iter().filter(|h| h.task == task.id) {
                    ui.label(format!(
                        "{}: {}{}",
                        if h.closed { "Closed" } else { "Open" },
                        h.reason,
                        if h.actor != task.owner_id {
                            " (prior owner; inactive)"
                        } else {
                            ""
                        }
                    ));
                    if owner
                        && !h.closed
                        && ui.button("Close / reclaim assistance request").clicked()
                    {
                        actions.push(Action::CloseHelp {
                            task: task.id.clone(),
                        });
                    }
                }
                ui.add_enabled_ui(
                    owner
                        && published
                            .is_some_and(|p| p.owner == task.owner_id && p.profile.help_allowed),
                    |ui| {
                        ui.add(
                            egui::TextEdit::singleline(reason)
                                .hint_text("Describe the help needed")
                                .char_limit(500),
                        );
                        if ui.button("Request assistance; keep my ownership").clicked() {
                            actions.push(Action::RequestHelp {
                                task: task.id.clone(),
                                expected_revision: revision,
                                reason: reason.clone(),
                            });
                        }
                    },
                );
            });
        let impact_draft = self.impact_drafts.entry(task.id.clone()).or_default();
        egui::CollapsingHeader::new("Dependency and file impacts").default_open(true).show(ui,|ui|{
            for impact in state.impacts.iter().filter(|i|i.source==task.id||i.target==task.id){
                let current_target=snapshot.shared.tasks.iter().find(|t|t.id==impact.target);let reviewed=current_target.is_some_and(|t|impact.acknowledged_by.as_deref()==Some(t.owner_id.as_str()));
                ui.label(format!("#{} {} -> {} | {} | {}",impact.id,impact.source,impact.target,impact.file,if reviewed{if impact.accepted==Some(true){"acknowledged (manual)"}else{"disputed"}}else{"needs current-owner review"}));ui.label(&impact.summary);ui.monospace(format!("Owner-declared revision: {}",impact.revision));
                if impact.target==task.id&&owner&&!reviewed{ui.horizontal(|ui|{if ui.button(format!("Acknowledge #{}",impact.id)).clicked(){actions.push(Action::AcknowledgeImpact{id:impact.id,accepted:true});}
                    if ui.button(format!("Dispute #{}",impact.id)).clicked(){actions.push(Action::AcknowledgeImpact{id:impact.id,accepted:false});}});}
            }
            ui.add_enabled_ui(owner,|ui|{
                ui.small("Declare an observed change, not inferred authorship. Target must depend on this task or publish the same linked file. These notices never run checks or propagate code.");
                egui::ComboBox::from_id_salt("impact-target").selected_text(if impact_draft.target.is_empty(){"Choose affected task"}else{&impact_draft.target}).show_ui(ui,|ui|{for target in snapshot.shared.tasks.iter().filter(|t|t.id!=task.id){ui.selectable_value(&mut impact_draft.target,target.id.clone(),format!("{}: {}",target.id,target.title));}});
                egui::ComboBox::from_id_salt("impact-file").selected_text(if impact_draft.file.is_empty(){"Choose published source path"}else{&impact_draft.file}).show_ui(ui,|ui|{if let Some(p)=published.filter(|p|p.owner==task.owner_id){for file in &p.profile.files{ui.selectable_value(&mut impact_draft.file,file.clone(),file);}}});
                ui.add(egui::TextEdit::singleline(&mut impact_draft.summary).hint_text("Exact change and expected impact; human observation").char_limit(500));
                ui.add(egui::TextEdit::singleline(&mut impact_draft.revision).hint_text("Full 40/64-char commit or diff digest (owner-declared)").char_limit(64));
                if ui.button("Publish impact for target-owner review").clicked(){actions.push(Action::DeclareImpact{revision:impact_draft.revision.clone(),source:task.id.clone(),target:impact_draft.target.clone(),file:impact_draft.file.clone(),summary:impact_draft.summary.clone()});}
            });
        });
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
    }
}
pub fn brief(snapshot: &Snapshot, task: &str) -> String {
    let task_state = snapshot.shared.tasks.iter().find(|t| t.id == task);
    serde_json::json!({"source":"explicit network Room coordination snapshot; manual declarations, not verified checks","room":snapshot.room,"task":task_state,"readiness":snapshot.shared.readiness.profile(task),"task_evidence":snapshot.shared.evidence.iter().rev().filter(|e|e.task_id==task).take(8).collect::<Vec<_>>(),"help":snapshot.shared.readiness.help.iter().filter(|h|h.task==task).collect::<Vec<_>>(),"impacts":snapshot.shared.readiness.impacts.iter().filter(|i|i.source==task||i.target==task).take(16).collect::<Vec<_>>(),"host_time":snapshot.server_time_epoch_seconds,"note":"No DMs, patches, local files or agent permission are included. Review and Include separately before sending to a provider."}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_brief_excludes_private_messages_and_patch_contents() {
        let mut snapshot = Snapshot {
            room: "room".into(),
            you: "a".repeat(64),
            ..Default::default()
        };
        snapshot.events.push(forge_room_core::transport::Event {
            sequence: 1,
            actor: "b".repeat(64),
            text: "PRIVATE_DM_MARKER".into(),
            recipient: Some(snapshot.you.clone()),
            task: None,
            decision: false,
            parent: None,
        });
        snapshot.patches.push(forge_room_core::transport::Patch {
            id: 1,
            actor: "b".repeat(64),
            recipient: snapshot.you.clone(),
            path: "private.rs".into(),
            before: "PRIVATE_PATCH_BASE".into(),
            after: "PRIVATE_PATCH_AFTER".into(),
        });
        for (task, text) in [
            ("T1", "VISIBLE_TASK_RECEIPT"),
            ("T2", "OTHER_TASK_EVIDENCE_PRIVATE"),
        ] {
            snapshot.shared.evidence.push(forge_room_core::Evidence {
                id: task.into(),
                task_id: task.into(),
                actor_id: "b".repeat(64),
                kind: forge_room_core::EvidenceKind::Check,
                summary: text.into(),
                recorded_at_epoch_seconds: 1,
            });
        }
        let brief = brief(&snapshot, "T1");
        assert!(brief.contains("VISIBLE_TASK_RECEIPT"));
        assert!(!brief.contains("OTHER_TASK_EVIDENCE_PRIVATE"));
        assert!(!brief.contains("PRIVATE_DM_MARKER"));
        assert!(!brief.contains("PRIVATE_PATCH"));
        assert!(!brief.contains("private.rs"));
        assert!(brief.contains("manual declarations"));
    }
    #[test]
    fn prior_owner_assistance_optin_is_not_inherited_by_a_new_owner_draft() {
        let p = forge_room_core::transport::TaskReadiness {
            task: "T1".into(),
            revision: 4,
            owner: "old".into(),
            updated: 1,
            profile: ReadinessProfile {
                estimate_minutes: Some(20),
                help_allowed: true,
                ..Default::default()
            },
        };
        let draft = Draft::from_published(Some(&p), "new");
        assert!(!draft.profile.help_allowed);
        assert_eq!(draft.revision, 4);
        assert_eq!(draft.profile.estimate_minutes, Some(20));
        assert!(Draft::from_published(Some(&p), "old").profile.help_allowed);
    }
}

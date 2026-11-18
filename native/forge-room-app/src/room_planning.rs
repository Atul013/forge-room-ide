//! Reviewed public task planning. No files or private Room content enter a plan.
use eframe::egui;
use forge_room_core::{
    Task, TaskPriority, TaskStatus,
    transport::{Action, PlannedTask, Snapshot, TaskPlan, preview_plan},
};
use std::{
    io::{Read, Write},
    path::Path,
};

const MAX_PLAN_BYTES: usize = 128 * 1024;
struct Row {
    task: PlannedTask,
    criteria: String,
    dependencies: String,
}
impl Row {
    fn new(task: PlannedTask) -> Self {
        Self {
            criteria: task.criteria.join("\n"),
            dependencies: task.dependencies.join(", "),
            task,
        }
    }
    fn value(&self) -> PlannedTask {
        let mut task = self.task.clone();
        task.criteria = self
            .criteria
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        task.dependencies = self
            .dependencies
            .split(',')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        task
    }
}
#[derive(Default)]
pub struct Panel {
    rows: Vec<Row>,
    reviewed: Option<(Vec<Task>, TaskPlan, Vec<Task>)>,
    notice: String,
    query: String,
    owner_filter: Option<String>,
    blocked_only: bool,
    counter: u64,
    bulk_owner: String,
}
fn owner_label(snapshot: &Snapshot, id: &str) -> String {
    if id.is_empty() {
        return "Choose owner".into();
    }
    let name = snapshot
        .members
        .iter()
        .find(|member| member.id == id)
        .map(|member| member.name.as_str())
        .or_else(|| {
            snapshot
                .shared
                .tasks
                .iter()
                .find(|task| task.owner_id == id)
                .map(|task| task.owner_name.as_str())
        })
        .unwrap_or("Unavailable owner");
    format!("{} ({})", name, id.chars().take(8).collect::<String>())
}
fn blocked(snapshot: &Snapshot, task: &Task) -> Vec<String> {
    task.dependencies
        .iter()
        .filter(|id| {
            !snapshot
                .shared
                .tasks
                .iter()
                .any(|dependency| &dependency.id == *id && dependency.status == TaskStatus::Done)
        })
        .cloned()
        .collect()
}
fn from_task(task: &Task) -> PlannedTask {
    PlannedTask {
        key: task.id.clone(),
        existing: Some(task.id.clone()),
        title: task.title.clone(),
        owner: task.owner_id.clone(),
        criteria: task.acceptance_criteria.clone(),
        dependencies: task.dependencies.clone(),
        priority: task.priority,
        due_hours: None,
        clear_due: false,
    }
}
fn load(path: &Path) -> Result<TaskPlan, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_PLAN_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PLAN_BYTES {
        return Err("Plan exceeds 128 KiB".into());
    }
    let plan: TaskPlan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if plan.version != 1 || plan.tasks.is_empty() || plan.tasks.len() > 100 {
        return Err("Unsupported plan version or row count".into());
    }
    Ok(plan)
}
fn save(path: &Path, plan: &TaskPlan) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(plan).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PLAN_BYTES {
        return Err("Plan exceeds 128 KiB".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
fn reusable(plan: &TaskPlan) -> TaskPlan {
    let mut aliases = std::collections::HashMap::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let key = format!("copy-{}", index + 1);
        aliases.insert(task.key.clone(), key.clone());
        if let Some(id) = &task.existing {
            aliases.insert(id.clone(), key);
        }
    }
    TaskPlan {
        version: 1,
        tasks: plan
            .tasks
            .iter()
            .map(|task| {
                let mut task = task.clone();
                task.key = aliases[&task.key].clone();
                task.existing = None;
                task.owner.clear();
                task.dependencies = task
                    .dependencies
                    .iter()
                    .map(|id| {
                        aliases
                            .get(id)
                            .cloned()
                            .unwrap_or_else(|| format!("external-{id}"))
                    })
                    .collect();
                task
            })
            .collect(),
    }
}
impl Panel {
    fn plan(&self) -> TaskPlan {
        TaskPlan {
            version: 1,
            tasks: self.rows.iter().map(Row::value).collect(),
        }
    }
    fn add(&mut self, snapshot: &Snapshot, title: &str, dependency: Option<String>) -> String {
        self.counter += 1;
        while self
            .rows
            .iter()
            .any(|row| row.task.key == format!("new-{}", self.counter))
        {
            self.counter += 1;
        }
        let key = format!("new-{}", self.counter);
        self.rows.push(Row::new(PlannedTask {
            key: key.clone(),
            existing: None,
            title: title.into(),
            owner: snapshot.you.clone(),
            criteria: vec![],
            dependencies: dependency.into_iter().collect(),
            priority: TaskPriority::Normal,
            due_hours: None,
            clear_due: false,
        }));
        key
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        host: bool,
        actions: &mut Vec<Action>,
    ) -> Option<String> {
        ui.heading("Room planning workspace");
        ui.label("Public tasks, dependencies and ownership. Plans change no files, run no checks and never mark work complete.");
        let mut open = None;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Find task, ID or owner")
                    .char_limit(200),
            );
            ui.checkbox(&mut self.blocked_only, "Blocked only");
            egui::ComboBox::from_id_salt("planning-owner-filter")
                .selected_text(
                    self.owner_filter
                        .as_deref()
                        .map_or_else(|| "All owners".to_owned(), |id| owner_label(snapshot, id)),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.owner_filter, None, "All owners");
                    for member in &snapshot.members {
                        ui.selectable_value(
                            &mut self.owner_filter,
                            Some(member.id.clone()),
                            owner_label(snapshot, &member.id),
                        );
                    }
                });
        });
        let query = self.query.trim().to_lowercase();
        egui::ScrollArea::horizontal()
            .id_salt("planning-board")
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for status in [TaskStatus::Open, TaskStatus::InProgress, TaskStatus::Done] {
                        ui.vertical(|ui| {
                            ui.set_width(240.0);
                            ui.label(egui::RichText::new(status.to_string()).strong());
                            for task in &snapshot.shared.tasks {
                                let unfinished = blocked(snapshot, task);
                                if task.status != status
                                    || self
                                        .owner_filter
                                        .as_ref()
                                        .is_some_and(|id| id != &task.owner_id)
                                    || (self.blocked_only && unfinished.is_empty())
                                    || (!query.is_empty()
                                        && !format!(
                                            "{} {} {}",
                                            task.id, task.title, task.owner_name
                                        )
                                        .to_lowercase()
                                        .contains(&query))
                                {
                                    continue;
                                }
                                ui.group(|ui| {
                                    ui.label(format!("{} · {}", task.id, task.title));
                                    ui.label(format!("{} · {:?}", task.owner_name, task.priority));
                                    if !unfinished.is_empty() {
                                        ui.label(format!("Waiting for {}", unfinished.join(", ")));
                                    }
                                    if let Some(due) = task.due_at_epoch_seconds {
                                        ui.label(if due < snapshot.server_time_epoch_seconds {
                                            "Overdue"
                                        } else {
                                            "Deadline set"
                                        });
                                    }
                                    if ui.button("Open task / checklist").clicked() {
                                        open = Some(task.id.clone());
                                    }
                                    if host
                                        && self.rows.len() < 100
                                        && task.status != TaskStatus::Done
                                        && ui.button("Add to reviewed plan").clicked()
                                        && !self
                                            .rows
                                            .iter()
                                            .any(|row| row.task.existing.as_ref() == Some(&task.id))
                                    {
                                        self.rows.push(Row::new(from_task(task)));
                                        self.reviewed = None;
                                    }
                                });
                            }
                        });
                    }
                });
            });
        ui.separator();
        if !snapshot.shared.planning_supported {
            ui.label("This host does not support atomic task planning.");
            return open;
        }
        if !host {
            ui.label("The host prepares and applies shared plans. Use task handoffs to change existing ownership.");
            return open;
        }
        ui.heading("Reviewed plan");
        ui.label("Existing owners and status stay unchanged. Editing a task clears its readiness/help declarations and affected impact acknowledgements; release check reports require review again. Handoffs and completed tasks are protected.");
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Add all Room tasks to draft").clicked(){let missing:Vec<_>=snapshot.shared.tasks.iter().filter(|task|!self.rows.iter().any(|row|row.task.existing.as_ref()==Some(&task.id))).collect();if self.rows.len()+missing.len()>100{self.notice="Adding all tasks would exceed the 100-row draft limit; remove draft rows first.".into();}else{self.rows.extend(missing.into_iter().map(|task|Row::new(from_task(task))));self.reviewed=None;}}
            if ui.add_enabled(self.rows.len()<100,egui::Button::new("Add task")).clicked(){self.add(snapshot,"",None);self.reviewed=None;}
            for (label,titles) in [("Insert feature workflow",["Design interface","Implement feature","Verify integration"]),("Insert bugfix workflow",["Reproduce defect","Implement fix","Verify regression"])] {
                if ui.add_enabled(self.rows.len()<=97,egui::Button::new(label)).clicked(){let a=self.add(snapshot,titles[0],None);let b=self.add(snapshot,titles[1],Some(a));self.add(snapshot,titles[2],Some(b));self.reviewed=None;}
            }
            if ui.button("Import plan...").clicked() && let Some(path)=rfd::FileDialog::new().add_filter("Task plan",&["json"]).pick_file() {match load(&path){Ok(plan)=>{self.rows=plan.tasks.into_iter().map(Row::new).collect();self.reviewed=None;self.counter=self.counter.saturating_add(100);self.notice="Imported draft only. Map owners/dependencies, then review.".into();},Err(error)=>self.notice=error}}
            for (label,template) in [("Save plan...",false),("Save reusable template...",true)] {
                if ui.add_enabled(!self.rows.is_empty(),egui::Button::new(label)).clicked() && let Some(path)=rfd::FileDialog::new().set_file_name("room-plan.json").save_file(){let plan=if template{reusable(&self.plan())}else{self.plan()};self.notice=match save(&path,&plan){Ok(())=>"Saved a new public draft file. Templates require owner remapping and review before use.".into(),Err(e)=>e};}
            }
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("plan-bulk-owner")
                .selected_text(owner_label(snapshot, &self.bulk_owner))
                .show_ui(ui, |ui| {
                    for member in &snapshot.members {
                        ui.selectable_value(
                            &mut self.bulk_owner,
                            member.id.clone(),
                            owner_label(snapshot, &member.id),
                        );
                    }
                });
            if ui
                .add_enabled(
                    snapshot
                        .members
                        .iter()
                        .any(|member| member.id == self.bulk_owner),
                    egui::Button::new("Assign all new draft tasks to this owner"),
                )
                .clicked()
            {
                for row in &mut self.rows {
                    if row.task.existing.is_none() {
                        row.task.owner = self.bulk_owner.clone();
                    }
                }
                self.reviewed = None;
            }
        });
        let mut remove = None;
        let expand_rows = self.rows.len() < 4;
        for (index, row) in self.rows.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                egui::CollapsingHeader::new(format!(
                    "{} · {}",
                    row.task.key,
                    if row.task.title.is_empty() {
                        "Untitled draft"
                    } else {
                        &row.task.title
                    }
                ))
                .default_open(expand_rows)
                .show(ui, |ui| {
                    ui.label(row.task.existing.as_ref().map_or("New task".into(), |id| {
                        format!("Edit {id}; owner/status preserved")
                    }));
                    ui.label("Plan key");
                    ui.add(egui::TextEdit::singleline(&mut row.task.key).char_limit(32));
                    ui.label("Title");
                    ui.add(egui::TextEdit::singleline(&mut row.task.title).char_limit(200));
                    if row.task.existing.is_none() {
                        egui::ComboBox::from_id_salt("draft-owner")
                            .selected_text(owner_label(snapshot, &row.task.owner))
                            .show_ui(ui, |ui| {
                                for member in &snapshot.members {
                                    ui.selectable_value(
                                        &mut row.task.owner,
                                        member.id.clone(),
                                        owner_label(snapshot, &member.id),
                                    );
                                }
                            });
                    }
                    ui.label("Criteria: one per line, at most eight");
                    ui.add(
                        egui::TextEdit::multiline(&mut row.criteria)
                            .desired_rows(3)
                            .char_limit(1608),
                    );
                    ui.label("Dependency keys or existing task IDs, comma-separated");
                    ui.add(egui::TextEdit::singleline(&mut row.dependencies).char_limit(1024));
                    egui::ComboBox::from_id_salt("draft-priority")
                        .selected_text(format!("{:?}", row.task.priority))
                        .show_ui(ui, |ui| {
                            for priority in [
                                TaskPriority::Normal,
                                TaskPriority::High,
                                TaskPriority::Critical,
                            ] {
                                ui.selectable_value(
                                    &mut row.task.priority,
                                    priority,
                                    format!("{priority:?}"),
                                );
                            }
                        });
                    let mut override_due = row.task.due_hours.is_some();
                    if ui
                        .checkbox(&mut override_due, "Set deadline in hours from apply")
                        .changed()
                    {
                        row.task.due_hours = override_due.then_some(24);
                        if override_due {
                            row.task.clear_due = false;
                        }
                    }
                    if let Some(hours) = &mut row.task.due_hours {
                        ui.add(egui::DragValue::new(hours).range(0..=720).suffix(" hours"));
                    } else {
                        ui.checkbox(&mut row.task.clear_due, "Clear existing deadline");
                    }
                    if ui.button("Remove from draft").clicked() {
                        remove = Some(index);
                    }
                });
            });
        }
        if let Some(index) = remove {
            self.rows.remove(index);
            self.reviewed = None;
        }
        let plan = self.plan();
        if self
            .reviewed
            .as_ref()
            .is_some_and(|(expected, reviewed, _)| {
                expected != &snapshot.shared.tasks || reviewed != &plan
            })
        {
            self.reviewed = None;
            self.notice = "Draft or Room tasks changed. Review again before applying.".into();
        }
        if ui
            .add_enabled(
                !self.rows.is_empty(),
                egui::Button::new("Review complete plan"),
            )
            .clicked()
        {
            match preview_plan(snapshot, &plan) {
                Ok(candidate) => {
                    self.reviewed = Some((snapshot.shared.tasks.clone(), plan.clone(), candidate));
                    self.notice.clear();
                }
                Err(error) => {
                    self.reviewed = None;
                    self.notice = error;
                }
            }
        }
        if let Some((expected, reviewed_plan, candidate)) = &self.reviewed {
            ui.group(|ui|{ui.label("Frozen public change preview");for task in candidate{if !expected.contains(task){ui.label(format!("{} {} · {} · {:?}",task.id,task.title,task.owner_name,task.priority));ui.label(format!("Dependencies: {}; criteria: {}",task.dependencies.join(", "),task.acceptance_criteria.join("; ")));if let Some(before)=expected.iter().find(|old|old.id==task.id){ui.label(format!("Before: {}; dependencies {}; criteria {}",before.title,before.dependencies.join(", "),before.acceptance_criteria.join("; ")));}}}ui.label("Apply changes every listed task together, or none. No existing owner/status changes; no files or checks execute.");});
            for entry in &reviewed_plan.tasks {
                ui.label(format!(
                    "{} deadline: {}",
                    entry.key,
                    if entry.clear_due {
                        "clear".into()
                    } else if let Some(hours) = entry.due_hours {
                        format!("{hours} hours from apply")
                    } else if entry.existing.is_some() {
                        "keep existing".into()
                    } else {
                        "none".into()
                    }
                ));
            }
            if ui
                .add_enabled(
                    snapshot.lifecycle == forge_room_core::transport::RoomLifecycle::Active,
                    egui::Button::new("Apply reviewed shared plan"),
                )
                .clicked()
                && let Some((expected_tasks, plan, _)) = self.reviewed.take()
            {
                actions.push(Action::ApplyTaskPlan {
                    expected_tasks,
                    plan,
                });
                self.notice =
                    "Plan submitted. Draft retained; server errors appear in Room notices.".into();
            }
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reusable_plan_remaps_internal_dependencies_and_requires_fresh_owner_choices() {
        let original = TaskPlan {
            version: 1,
            tasks: vec![
                PlannedTask {
                    key: "T1".into(),
                    existing: Some("T1".into()),
                    title: "First".into(),
                    owner: "a".repeat(64),
                    criteria: vec!["manual check".into()],
                    dependencies: vec![],
                    priority: TaskPriority::High,
                    due_hours: None,
                    clear_due: false,
                },
                PlannedTask {
                    key: "T2".into(),
                    existing: Some("T2".into()),
                    title: "Second".into(),
                    owner: "a".repeat(64),
                    criteria: vec![],
                    dependencies: vec!["T1".into()],
                    priority: TaskPriority::Normal,
                    due_hours: Some(24),
                    clear_due: false,
                },
            ],
        };
        let mut external = original.clone();
        external.tasks[1].dependencies.push("T99".into());
        assert_eq!(reusable(&external).tasks[1].dependencies[1], "external-T99");
        let template = reusable(&original);
        assert_eq!(template.tasks[1].dependencies, vec!["copy-1"]);
        assert!(
            template
                .tasks
                .iter()
                .all(|task| task.existing.is_none() && task.owner.is_empty())
        );
        assert_eq!(original.tasks[1].dependencies, vec!["T1"]);
        let path = std::env::temp_dir().join(format!(
            "forge-plan-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        save(&path, &template).unwrap();
        assert_eq!(load(&path).unwrap(), template);
        assert!(save(&path, &original).is_err());
        assert_eq!(load(&path).unwrap(), template);
        std::fs::write(&path, b"{invalid").unwrap();
        assert!(load(&path).is_err());
        std::fs::write(&path, vec![b' '; MAX_PLAN_BYTES + 1]).unwrap();
        assert!(load(&path).unwrap_err().contains("128 KiB"));
        let mut unknown = serde_json::to_value(&template).unwrap();
        unknown["execute"] = serde_json::json!("not supported");
        std::fs::write(&path, serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn starter_workflow_is_a_reviewable_dependency_chain_and_blocking_is_explicit() {
        let mut panel = Panel::default();
        let snapshot = Snapshot::default();
        let a = panel.add(&snapshot, "Design", None);
        let b = panel.add(&snapshot, "Build", Some(a.clone()));
        panel.add(&snapshot, "Verify", Some(b.clone()));
        let plan = panel.plan();
        assert_eq!(plan.tasks[1].dependencies, vec![a]);
        assert_eq!(plan.tasks[2].dependencies, vec![b]);
        assert!(panel.reviewed.is_none());
        assert!(
            preview_plan(&snapshot, &plan).is_err(),
            "an owner must be chosen before apply"
        );
    }
}

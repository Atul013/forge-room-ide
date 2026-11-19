//! Portable setup intent. A blueprint contains no identity, credentials or completed work.
use super::*;
use crate::TaskPriority;
pub const MAX_BLUEPRINT: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintTask {
    pub key: String,
    pub title: String,
    pub criteria: Vec<String>,
    pub dependencies: Vec<String>,
    pub priority: TaskPriority,
    pub due_hours: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlueprintMilestone {
    pub title: String,
    pub due_hours: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomBlueprint {
    pub version: u32,
    pub title: String,
    pub description: String,
    pub countdown_seconds: u64,
    pub tasks: Vec<BlueprintTask>,
    pub milestones: Vec<BlueprintMilestone>,
    pub required_tasks: Vec<String>,
    pub release_checks: Vec<String>,
}
impl RoomBlueprint {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BLUEPRINT {
            return Err("Blueprint exceeds 256 KiB".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Invalid blueprint schema at line {}, column {}",
                e.line(),
                e.column()
            )
        })?;
        value.validate()?;
        Ok(value)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BLUEPRINT {
            return Err("Blueprint exceeds 256 KiB".into());
        }
        Ok(bytes)
    }
    fn plan(&self, owner: &str) -> TaskPlan {
        TaskPlan {
            version: 1,
            tasks: self
                .tasks
                .iter()
                .map(|t| PlannedTask {
                    key: t.key.clone(),
                    existing: None,
                    title: t.title.clone(),
                    owner: owner.into(),
                    criteria: t.criteria.clone(),
                    dependencies: t.dependencies.clone(),
                    priority: t.priority,
                    due_hours: t.due_hours,
                    clear_due: false,
                })
                .collect(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.tasks.len() > 100
            || self.milestones.len() > 20
            || self.countdown_seconds > 7 * 86400
            || self.release_checks.len() > 24
        {
            return Err("Unsupported blueprint version or limits".into());
        }
        bounded(&self.title, 80)?;
        if self.description.len() > 2000
            || self
                .description
                .chars()
                .any(|c| c.is_control() && c != '\n')
        {
            return Err("Invalid blueprint description".into());
        }
        let owner = "a".repeat(64);
        let mut snapshot = Snapshot::default();
        snapshot.members.push(Member {
            id: owner.clone(),
            name: "Launching member".into(),
            online: true,
        });
        if !self.tasks.is_empty() {
            preview_plan(&snapshot, &self.plan(&owner))?;
        }
        let keys = self
            .tasks
            .iter()
            .map(|t| t.key.as_str())
            .collect::<HashSet<_>>();
        if self.required_tasks.len() > 100
            || self.required_tasks.iter().collect::<HashSet<_>>().len() != self.required_tasks.len()
            || self
                .required_tasks
                .iter()
                .any(|k| !keys.contains(k.as_str()))
        {
            return Err("Required blueprint task is missing or repeated".into());
        }
        if self.release_checks.iter().collect::<HashSet<_>>().len() != self.release_checks.len() {
            return Err("Duplicate release checks".into());
        }
        for check in &self.release_checks {
            bounded(check, 160)?;
        }
        for m in &self.milestones {
            bounded(&m.title, 200)?;
            if m.due_hours.is_some_and(|h| h > 720) {
                return Err("Milestone deadline exceeds 720 hours".into());
            }
        }
        Ok(())
    }
    /// Capture only explicit public setup. Owners, completion, messages and artifacts are excluded.
    pub fn capture(snapshot: &Snapshot) -> Result<Self> {
        let hours = |due: Option<u64>| {
            due.map(|at| {
                at.saturating_sub(crate::now_epoch_seconds())
                    .div_ceil(3600)
                    .min(720)
            })
        };
        let keys = snapshot
            .shared
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), format!("task_{}", i + 1)))
            .collect::<HashMap<_, _>>();
        let value=Self {version:1,title:snapshot.room.clone(),description:"Captured public setup; all tasks/checks are reassigned to the launching member and completion is reset.".into(),countdown_seconds:snapshot.shared.countdown.remaining_seconds,tasks:snapshot.shared.tasks.iter().map(|t|BlueprintTask{key:keys[&t.id].clone(),title:t.title.clone(),criteria:t.acceptance_criteria.clone(),dependencies:t.dependencies.iter().filter_map(|id|keys.get(id).cloned()).collect(),priority:t.priority,due_hours:hours(t.due_at_epoch_seconds)}).collect(),milestones:snapshot.shared.milestones.iter().map(|m|BlueprintMilestone{title:m.title.clone(),due_hours:hours(m.due_at_epoch_seconds)}).collect(),required_tasks:snapshot.shared.shipping.plan.required_tasks.iter().filter_map(|id|keys.get(id).cloned()).collect(),release_checks:snapshot.shared.shipping.plan.checks.iter().map(|c|c.title.clone()).collect()};
        value.validate()?;
        Ok(value)
    }
    pub fn starters() -> Vec<Self> {
        [("Blank Room",vec![]),("Feature delivery",vec!["Define acceptance","Implement feature","Review and verify","Prepare handoff"]),("Bug investigation",vec!["Reproduce failure","Identify cause","Fix and regress","Verify release"]),("Hackathon",vec!["Choose demo scope","Build core journey","Test integration","Prepare demo"])].into_iter().map(|(title,titles)|{
            let count=titles.len();
            Self{version:1,title:title.into(),description:"All initial work belongs to the launching member. Use explicit handoffs after teammates join.".into(),countdown_seconds:if title=="Hackathon"{24*3600}else{0},tasks:titles.into_iter().enumerate().map(|(i,title)|BlueprintTask{key:format!("step_{}",i+1),title:title.into(),criteria:vec!["Reviewed acceptance evidence attached".into()],dependencies:if i==0{vec![]}else{vec![format!("step_{i}")]},priority:TaskPriority::Normal,due_hours:None}).collect(),milestones:vec![],required_tasks:(1..=count).map(|i|format!("step_{i}")).collect(),release_checks:if count==0{vec![]}else{vec!["Source checks verified".into(),"Visible product acceptance recorded".into()]}}
        }).collect()
    }
}
impl Host {
    /// Returns a fully configured fresh host and joined launcher. No external invite is published.
    /// Call on a worker; failures drop the temporary host. A blueprint never modifies an existing Room.
    pub fn from_blueprint(
        bind: SocketAddr,
        blueprint: &RoomBlueprint,
        name: &str,
        run_clock: bool,
    ) -> Result<(Self, Client, Snapshot)> {
        blueprint.validate()?;
        bounded(name, 64)?;
        let host = Self::start(bind, &blueprint.title)?;
        let (mut client, _) = Client::join(host.invite(Duration::from_secs(60))?, name)?;
        if !blueprint.tasks.is_empty() {
            host.control(Action::ApplyTaskPlan {
                expected_tasks: vec![],
                plan: blueprint.plan(&client.identity),
            })?;
        }
        for m in &blueprint.milestones {
            host.control(Action::CreateMilestone {
                title: m.title.clone(),
                due_hours: m.due_hours,
            })?;
        }
        host.control(Action::Countdown {
            seconds: Some(blueprint.countdown_seconds),
            running: run_clock && blueprint.countdown_seconds > 0,
        })?;
        if !blueprint.required_tasks.is_empty() || !blueprint.release_checks.is_empty() {
            let tasks = host.snapshot()?.shared.tasks;
            let ids = blueprint
                .tasks
                .iter()
                .zip(tasks)
                .map(|(a, b)| (a.key.clone(), b.id))
                .collect::<HashMap<_, _>>();
            host.control(Action::SetShipPlan {
                expected_revision: 0,
                plan: ShipPlan {
                    title: blueprint.title.clone(),
                    required_tasks: blueprint
                        .required_tasks
                        .iter()
                        .map(|k| ids[k].clone())
                        .collect(),
                    checks: blueprint
                        .release_checks
                        .iter()
                        .map(|title| ShipCheck {
                            title: title.clone(),
                            owner: client.identity.clone(),
                        })
                        .collect(),
                    ..Default::default()
                },
            })?;
        }
        let snapshot = client.poll()?;
        Ok((host, client, snapshot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blueprint_validation_is_strict_and_reuses_plan_graph_rules() {
        for value in RoomBlueprint::starters() {
            assert_eq!(
                RoomBlueprint::parse(&value.encode().unwrap()).unwrap(),
                value
            );
        }
        let mut b = RoomBlueprint::starters().remove(1);
        b.tasks[0].dependencies = vec!["step_4".into()];
        assert!(b.validate().is_err());
        b.tasks[0].dependencies.clear();
        b.required_tasks.push("missing".into());
        assert!(b.validate().is_err());
        assert!(RoomBlueprint::parse(br#"{"version":1,"token":"private"}"#)
            .unwrap_err()
            .contains("schema"));
        assert!(RoomBlueprint::parse(&vec![b' '; MAX_BLUEPRINT + 1]).is_err());
    }
    #[test]
    fn launches_full_setup_and_capture_discards_identity_completion_and_private_content() {
        let mut b = RoomBlueprint::starters().remove(1);
        b.milestones.push(BlueprintMilestone {
            title: "Demo".into(),
            due_hours: Some(2),
        });
        b.countdown_seconds = 3600;
        let (host, mut client, snapshot) =
            Host::from_blueprint("127.0.0.1:0".parse().unwrap(), &b, "Launcher", false).unwrap();
        assert_eq!(snapshot.shared.tasks.len(), 4);
        assert_eq!(snapshot.shared.milestones.len(), 1);
        assert!(snapshot
            .shared
            .tasks
            .iter()
            .all(|t| t.owner_id == client.identity));
        assert!(!snapshot.shared.countdown.running);
        assert_eq!(snapshot.shared.shipping.plan.checks.len(), 2);
        client
            .execute(Action::message("private sentinel content"))
            .unwrap();
        let captured = RoomBlueprint::capture(&host.snapshot().unwrap()).unwrap();
        let bytes = captured.encode().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains(&client.identity));
        assert!(!text.contains("sentinel"));
        assert!(!text.contains(&snapshot.shared.room_id));
        let (_, second, next) =
            Host::from_blueprint("127.0.0.1:0".parse().unwrap(), &captured, "Second", false)
                .unwrap();
        assert_ne!(second.identity, client.identity);
        assert_ne!(next.shared.room_id, snapshot.shared.room_id);
        assert_eq!(
            next.shared.tasks[1].dependencies,
            vec![next.shared.tasks[0].id.clone()]
        );
    }
}

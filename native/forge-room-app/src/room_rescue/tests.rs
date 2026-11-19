use super::*;
use forge_room_core::{Task, TaskPriority};

fn fixture() -> Snapshot {
    let mut s = Snapshot {
        room: "room".into(),
        server_time_epoch_seconds: 1000,
        ..Default::default()
    };
    s.shared.room_id = "room-id".into();
    s.shared.shipping.supported = true;
    s.shared.readiness.supported = true;
    s.shared.shipping.revision = 1;
    s.shared.shipping.plan.title = "Release".into();
    for id in ["T1", "T2"] {
        s.shared.tasks.push(Task {
            id: id.into(),
            title: id.into(),
            owner_id: "a".repeat(64),
            owner_name: "Ada".into(),
            status: TaskStatus::Open,
            priority: TaskPriority::Normal,
            dependencies: vec![],
            acceptance_criteria: vec![],
            completed_criteria: vec![],
            worktree_branch: None,
            due_at_epoch_seconds: None,
            updated_at_epoch_seconds: 1,
        });
    }
    s.shared.shipping.plan.required_tasks = vec!["T1".into()];
    s
}
#[test]
fn publish_requires_fresh_review_host_active_room_reset_ack_and_matching_selection() {
    let mut s = fixture();
    let mut panel = Panel {
        room: s.shared.room_id.clone(),
        selected: vec!["T2".into()],
        ..Default::default()
    };
    assert!(panel.publish(&s, true).is_err());
    panel.capture(&s, 1800).unwrap();
    assert!(panel.publish(&s, true).is_err());
    panel.confirmed_reset = true;
    assert!(panel.publish(&s, false).is_err());
    assert!(panel.publish(&s, true).is_ok());
    s.lifecycle = RoomLifecycle::Paused;
    assert!(panel.publish(&s, true).is_err());
    s.lifecycle = RoomLifecycle::Active;
    panel.selected = vec!["T1".into()];
    assert!(panel.publish(&s, true).is_err());
    panel.selected = vec!["T2".into()];
    s.shared.tasks[0].title = "changed".into();
    assert!(panel.publish(&s, true).is_err());
    panel.capture(&s, 1800).unwrap();
    panel.confirmed_reset = true;
    panel.reviewed.as_mut().unwrap().1 = Instant::now() - std::time::Duration::from_secs(121);
    assert!(panel.publish(&s, true).is_err());
}
#[test]
fn rendering_preserves_frozen_export_and_never_queues_mutation_at_normal_and_narrow_sizes() {
    let s = fixture();
    let mut panel = Panel {
        room: s.shared.room_id.clone(),
        selected: vec!["T2".into()],
        ..Default::default()
    };
    panel.capture(&s, 1800).unwrap();
    let original = panel.reviewed.as_ref().unwrap().0.clone();
    panel.baseline = Some(original.clone());
    for width in [320.0, 1280.0] {
        let context = egui::Context::default();
        let mut actions = Vec::new();
        let mut out = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 800.0),
                )),
                ..Default::default()
            },
            |ui| {
                assert!(panel.show(ui, &s, true, 1700, &mut actions).is_none());
            },
        );
        out.textures_delta.clear();
        assert!(actions.is_empty());
        assert_eq!(panel.reviewed.as_ref().unwrap().0, original);
    }
}
#[test]
fn room_switch_clears_authority_review_and_import_cannot_become_publish_source() {
    let mut s = fixture();
    let mut panel = Panel {
        room: s.shared.room_id.clone(),
        selected: vec!["T2".into()],
        ..Default::default()
    };
    panel.capture(&s, 1800).unwrap();
    panel.baseline = Some(panel.reviewed.as_ref().unwrap().0.clone());
    panel.confirmed_reset = true;
    s.shared.room_id = "other".into();
    let context = egui::Context::default();
    let mut actions = Vec::new();
    let mut out = context.run_ui(egui::RawInput::default(), |ui| {
        panel.show(ui, &s, true, 1800, &mut actions);
    });
    out.textures_delta.clear();
    assert!(panel.reviewed.is_none());
    assert!(!panel.confirmed_reset);
    assert!(panel.baseline.is_some());
    assert!(panel.publish(&s, true).is_err());
    assert!(actions.is_empty());
}

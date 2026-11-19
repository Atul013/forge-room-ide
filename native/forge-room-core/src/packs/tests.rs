use super::*;
fn pack(id: &str, version: &str) -> Pack {
    Pack {
        format: 1,
        api: 1,
        id: id.into(),
        version: version.parse().unwrap(),
        name: "Local resource pack".into(),
        publisher: "Local author claim".into(),
        summary: "Declarative resources".into(),
        license: "MIT".into(),
        license_text: String::new(),
        source: String::new(),
        readme: String::new(),
        dependencies: vec![],
        resources: vec![Resource::new(
            "guide".into(),
            "Local guide".into(),
            Kind::Readme,
            "Inspect this content".into(),
        )
        .unwrap()],
    }
}
fn select(pack: &Pack) -> Selection {
    Selection {
        version: pack.version,
        resources: pack
            .resources
            .iter()
            .map(|resource| resource.id.clone())
            .collect(),
    }
}
#[test]
fn strict_pack_versions_identities_content_and_unknown_kinds_fail_before_activation() {
    for version in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "01.2.3",
        "1.2.-3",
        "1.2.65536",
        "1.2.3-alpha",
    ] {
        assert!(version.parse::<Version>().is_err(), "{version}");
    }
    assert_eq!("1.2.3".parse::<Version>().unwrap().to_string(), "1.2.3");
    for id in [
        "../outside",
        "Upper.id",
        "a..b",
        "a/b",
        "a@1",
        ".pack",
        "a:",
    ] {
        assert!(!pack_id(id), "{id}");
    }
    let original = pack("author.tools", "1.0.0");
    let bytes = serde_json::to_vec(&original).unwrap();
    assert_eq!(Pack::parse(&bytes).unwrap(), original);
    let mut bad = original.clone();
    bad.resources[0].content.push('!');
    assert!(bad.validate().is_err());
    let mut bad = serde_json::to_value(&original).unwrap();
    bad["resources"][0]["kind"] = serde_json::json!("script");
    assert!(Pack::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    bad = serde_json::to_value(&original).unwrap();
    bad["post_install"] = serde_json::json!("run arbitrary command");
    assert!(Pack::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
}
#[test]
fn activation_requires_explicit_resources_compatible_dependencies_and_fresh_review() {
    let base = pack("author.base", "1.0.0");
    let mut tools = pack("author.tools", "2.0.0");
    tools.dependencies.push(Dependency {
        id: base.id.clone(),
        minimum: base.version,
        before: Some("2.0.0".parse().unwrap()),
    });
    let mut catalog = [(base.key(), base.clone()), (tools.key(), tools.clone())]
        .into_iter()
        .collect::<Catalog>();
    let initial = Activation::default();
    let enabled = [(tools.id.clone(), select(&tools))].into_iter().collect();
    assert!(initial.review(enabled, &catalog).is_err());
    let enabled = [
        (base.id.clone(), select(&base)),
        (tools.id.clone(), select(&tools)),
    ]
    .into_iter()
    .collect();
    let review = initial.review(enabled, &catalog).unwrap();
    assert_eq!(review.changes.len(), 2);
    let next = initial.accept(&review, &catalog).unwrap();
    assert_eq!(next.revision, 1);
    assert!(next.accept(&review, &catalog).is_err());
    catalog.get_mut(&tools.key()).unwrap().publisher = "Changed identity claim".into();
    assert!(initial.accept(&review, &catalog).is_err());
}
#[test]
fn selective_disable_upgrade_and_rollback_keep_revision_and_dependency_guards() {
    let old = pack("author.tools", "1.0.0");
    let mut new = pack("author.tools", "2.0.0");
    new.resources[0] = Resource::new(
        "guide".into(),
        "Changed guide".into(),
        Kind::Readme,
        "A changed resource".into(),
    )
    .unwrap();
    let catalog = [(old.key(), old.clone()), (new.key(), new.clone())]
        .into_iter()
        .collect::<Catalog>();
    let initial = Activation::default();
    let first = initial
        .review(
            [(old.id.clone(), select(&old))].into_iter().collect(),
            &catalog,
        )
        .unwrap();
    let one = initial.accept(&first, &catalog).unwrap();
    let upgrade = one
        .review(
            [(new.id.clone(), select(&new))].into_iter().collect(),
            &catalog,
        )
        .unwrap();
    assert_eq!(upgrade.changes.len(), 1);
    let two = one.accept(&upgrade, &catalog).unwrap();
    let rollback = two.review(one.enabled.clone(), &catalog).unwrap();
    let three = two.accept(&rollback, &catalog).unwrap();
    assert_eq!(three.revision, 3);
    assert_eq!(three.enabled, one.enabled);
    let off = three.review(BTreeMap::new(), &catalog).unwrap();
    assert_eq!(off.changes.len(), 1);
    assert!(three.accept(&off, &catalog).unwrap().enabled.is_empty());
}
#[test]
fn cyclic_dependencies_missing_resources_and_api_limits_are_denied() {
    let mut a = pack("author.a", "1.0.0");
    let mut b = pack("author.b", "1.0.0");
    a.dependencies.push(Dependency {
        id: b.id.clone(),
        minimum: b.version,
        before: None,
    });
    b.dependencies.push(Dependency {
        id: a.id.clone(),
        minimum: a.version,
        before: None,
    });
    let catalog = [(a.key(), a.clone()), (b.key(), b.clone())]
        .into_iter()
        .collect::<Catalog>();
    let enabled = [(a.id.clone(), select(&a)), (b.id.clone(), select(&b))]
        .into_iter()
        .collect();
    assert!(Activation::default().review(enabled, &catalog).is_err());
    let mut bad = a;
    bad.api = 2;
    assert!(bad.validate().is_err());
    let mut selected = select(&b);
    selected.resources.insert("missing".into());
    assert!(Activation::default()
        .review([(b.id.clone(), selected)].into_iter().collect(), &catalog)
        .is_err());
}
#[test]
fn profiles_are_bounded_typed_appearance_data_without_permission_or_file_settings() {
    let profile = Profile::default();
    let text = serde_json::to_string(&profile).unwrap();
    assert_eq!(Profile::parse(&text).unwrap(), profile);
    let mut changed = profile.clone();
    changed.editor_scale = 1.2;
    changed.reduced_motion = true;
    assert_eq!(profile.changes(&changed).unwrap().len(), 2);
    changed.editor_scale = f32::NAN;
    assert!(changed.validate().is_err());
    let mut bad = serde_json::to_value(&profile).unwrap();
    bad["trusted_folders"] = serde_json::json!(["C:/"]);
    assert!(Profile::parse(&bad.to_string()).is_err());
    bad = serde_json::to_value(&profile).unwrap();
    bad["layout"]["dock_width"] = serde_json::json!(9000);
    assert!(Profile::parse(&bad.to_string()).is_err());
}
#[test]
fn widgets_reject_arbitrary_sources_and_revoke_data_on_workspace_or_grant_changes() {
    let widget = Widget {
        version: 1,
        id: "checks".into(),
        title: "Checks".into(),
        description: String::new(),
        columns: 1,
        refresh_seconds: 2,
        bindings: vec![Binding {
            id: "runs".into(),
            label: "Current runs".into(),
            source: DataSource::Workflows,
            view: View::List,
            limit: 1,
        }],
    };
    widget.validate().unwrap();
    let snapshot = Snapshot {
        workspace: "workspace-a".into(),
        captured: 5,
        simulated: false,
        values: [(
            DataSource::Workflows,
            vec![
                Row {
                    label: "Run".into(),
                    value: "Passed".into(),
                    detail: String::new(),
                },
                Row {
                    label: "Other".into(),
                    value: "Failed".into(),
                    detail: String::new(),
                },
            ],
        )]
        .into_iter()
        .collect(),
    };
    let mut grants = Grants {
        workspace: "workspace-a".into(),
        sources: widget.requested(),
    };
    assert_eq!(
        grants
            .rows(&widget, &widget.bindings[0], &snapshot)
            .unwrap()
            .len(),
        1
    );
    grants.workspace = "workspace-b".into();
    assert!(grants
        .rows(&widget, &widget.bindings[0], &snapshot)
        .is_err());
    grants.workspace = "workspace-a".into();
    grants.sources.clear();
    assert!(grants
        .rows(&widget, &widget.bindings[0], &snapshot)
        .is_err());
    let mut bad = serde_json::to_value(&widget).unwrap();
    bad["bindings"][0]["source"] = serde_json::json!("terminal_raw_output");
    assert!(Widget::parse(&bad.to_string()).is_err());
    bad = serde_json::to_value(&widget).unwrap();
    bad["script"] = serde_json::json!("fetch('https://example.test')");
    assert!(Widget::parse(&bad.to_string()).is_err());
}

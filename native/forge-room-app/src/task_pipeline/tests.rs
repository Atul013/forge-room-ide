use super::*;
use serde_json::json;
fn root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "forge-task ' $(literal)-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("sub")).unwrap();
    root.canonicalize().unwrap()
}
#[test]
fn graph_validates_cycles_duplicates_missing_parallel_and_background_before_running() {
    let root = root();
    let valid = json!([{"label":"base","command":"echo base"},{"label":"left","command":"echo left","dependsOn":"base"},{"label":"right","command":"echo right","dependsOn":"base"},{"label":"all","dependsOn":["left","right"],"dependsOrder":"sequence"}]);
    let result = plan("all", valid.as_array().unwrap(), &root, None).unwrap();
    assert_eq!(
        result
            .steps
            .iter()
            .map(|s| s.label.as_str())
            .collect::<Vec<_>>(),
        vec!["base", "left", "right"]
    );
    for invalid in [
        json!([{"label":"a","command":"echo a","dependsOn":"b"},{"label":"b","command":"echo b","dependsOn":"a"}]),
        json!([{"label":"a","command":"echo a"},{"label":"a","command":"echo b"}]),
        json!([{"label":"a","command":"echo a","dependsOn":"missing"}]),
        json!([{"label":"a","command":"echo a","dependsOn":["b","c"]},{"label":"b","command":"echo b"},{"label":"c","command":"echo c"}]),
        json!([{"label":"a","command":"echo a","isBackground":true}]),
        json!([{"label":"a","type":"npm","command":"test"}]),
        json!([{"label":"a","command":"echo ${input:secret}"}]),
        json!([{"label":"a","command":"echo a","options":{"shell":{"executable":"cmd.exe"}}}]),
    ] {
        assert!(plan("a", invalid.as_array().unwrap(), &root, None).is_err());
    }
}
#[test]
fn variables_cwd_environment_and_jsonc_are_bounded_and_visible() {
    let root = root();
    let file = root.join("unit's $name.py");
    fs::write(&file, "print(1)").unwrap();
    let tasks = json!([{"label":"test","type":"process","command":"python","args":["${file}","${relativeFile}","${workspaceFolderBasename}"],"options":{"cwd":"${fileDirname}","env":{"FORGE_TEST_FILE":"${fileBasenameNoExtension}","FORGE_TEST_UNSET":null}}}]);
    let p = plan("test", tasks.as_array().unwrap(), &root, Some(&file)).unwrap();
    assert!(p.steps[0].command.contains("unit''s $name.py"));
    assert_eq!(
        p.steps[0].env["FORGE_TEST_FILE"].as_deref(),
        Some("unit's $name")
    );
    assert!(p.steps[0].env["FORGE_TEST_UNSET"].is_none());
    assert!(plan("test", tasks.as_array().unwrap(), &root, None).is_err());
    let invalid = json!([{"label":"a","command":"echo a","options":{"cwd":".."}}]);
    assert!(plan("a", invalid.as_array().unwrap(), &root, None).is_err());
    assert!(expand("${env:HOME}", &root, None).is_err());
    assert!(expand("${workspaceFolder:other}", &root, None).is_err());
    fs::create_dir_all(root.join(".vscode")).unwrap();
    fs::write(root.join(".vscode/tasks.json"),"{\n// comment\n\"version\":\"2.0.0\",\"tasks\":[{\"label\":\"build\",\"command\":\"echo ok\",},],}").unwrap();
    assert_eq!(read(&root).unwrap().unwrap().len(), 1);
    fs::write(
        root.join(".vscode/tasks.json"),
        "{\"version\":\"2.0.0\",\"tasks\":[],\"options\":{}}",
    )
    .unwrap();
    assert!(read(&root).is_err());
    let depth:Vec<_>=(0..18).map(|i|json!({"label":format!("n{i}"),"command":"echo test","dependsOn":if i==17 {vec![]} else {vec![format!("n{}",i+1)]}})).collect();
    assert!(
        plan("n0", &depth, &root, None)
            .err()
            .unwrap()
            .contains("depth")
    );
}
#[test]
fn cached_plans_follow_the_active_file_and_workspace_and_can_be_refreshed() {
    let root = root();
    fs::create_dir_all(root.join(".vscode")).unwrap();
    fs::write(root.join(".vscode/tasks.json"),r#"{"version":"2.0.0","tasks":[{"label":"file","type":"process","command":"python","args":["${file}"],"options":{"cwd":"generated"}}]}"#).unwrap();
    let a = root.join("a.py");
    let b = root.join("b.py");
    fs::write(&a, "a").unwrap();
    fs::write(&b, "b").unwrap();
    let mut state = State::default();
    state.refresh(&root, Some(&a));
    assert!(state.cards[0].plan.is_err());
    fs::create_dir(root.join("generated")).unwrap();
    state.context = None;
    state.refresh(&root, Some(&a));
    assert!(
        state.cards[0].plan.as_ref().unwrap().steps[0]
            .command
            .contains("a.py")
    );
    state.refresh(&root, Some(&b));
    assert!(
        state.cards[0].plan.as_ref().unwrap().steps[0]
            .command
            .contains("b.py")
    );
    state.refresh(&root.join("sub"), None);
    assert!(state.cards.is_empty());
}
#[cfg(windows)]
fn powershell(script: &str, root: &Path) -> std::process::Output {
    use std::os::windows::process::CommandExt;
    let program = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    std::process::Command::new(program)
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .current_dir(crate::tabs::normalize(root))
        .env("FORGE_PIPE_ENV", "original")
        .creation_flags(0x0800_0000)
        .output()
        .unwrap()
}
#[cfg(windows)]
#[test]
fn real_powershell_pipeline_orders_dependencies_and_restores_environment() {
    let root = root();
    let order = crate::tabs::normalize(&root.join("order.txt"));
    let append = |value: &str| {
        format!(
            "Add-Content -LiteralPath {} {}",
            quote(&order.to_string_lossy()),
            quote(value)
        )
    };
    let tasks = json!([
        {"label":"base","command":format!("if ($env:FORGE_PIPE_ENV -ne 'task') {{ throw 'missing task env' }}; {}",append("base")),"options":{"cwd":"sub","env":{"FORGE_PIPE_ENV":"task"}}},
        {"label":"left","command":format!("if ($env:FORGE_PIPE_ENV -ne 'original') {{ throw 'leaked env' }}; {}",append("left")),"dependsOn":"base"},
        {"label":"right","command":append("right"),"dependsOn":"base"},
        {"label":"all","dependsOn":["left","right"],"dependsOrder":"sequence"}
    ]);
    let p = plan("all", tasks.as_array().unwrap(), &root, None).unwrap();
    let out = powershell(&p.script, &root);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(order)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        vec!["base", "left", "right"]
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("All 3 steps succeeded"));
}
#[cfg(windows)]
#[test]
fn real_native_exit_failure_stops_dependents_and_finally_restores_context() {
    let root = root();
    let marker = crate::tabs::normalize(&root.join("must-not-exist.txt"));
    let tasks = json!([
        {"label":"fail","type":"process","command":"powershell.exe","args":["-NoProfile","-NonInteractive","-Command","exit 7"],"options":{"cwd":"sub","env":{"FORGE_PIPE_ENV":"changed"}}},
        {"label":"after","command":format!("Set-Content -LiteralPath {} 'bad'",quote(&marker.to_string_lossy())),"dependsOn":"fail"}
    ]);
    let p = plan("after", tasks.as_array().unwrap(), &root, None).unwrap();
    let script = format!(
        "$forgeBefore = (Get-Location).Path; try {{ {} }} catch {{ if ($env:FORGE_PIPE_ENV -eq 'original' -and (Get-Location).Path -eq $forgeBefore) {{ Write-Output 'context restored'; exit 3 }}; exit 9 }}",
        p.script
    );
    let out = powershell(&script, &root);
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("context restored"));
    assert!(!marker.exists());
}
#[cfg(windows)]
#[test]
fn process_arguments_preserve_literal_variables_in_quoted_workspace_paths() {
    let root = root();
    let file = root.join("source's $name.py");
    fs::write(&file, "pass").unwrap();
    let exe = root.join("argument-peer.exe");
    let compiled = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/task_arguments.rs"))
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let tasks = json!([{"label":"args","type":"process","command":"${workspaceFolder}/argument-peer.exe","args":["${workspaceFolder}/arguments.txt","${workspaceFolder}","${file}"]}]);
    let p = plan("args", tasks.as_array().unwrap(), &root, Some(&file)).unwrap();
    let out = powershell(&p.script, &root);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let args = fs::read_to_string(root.join("arguments.txt")).unwrap();
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        vec![
            crate::tabs::normalize(&root).to_str().unwrap(),
            crate::tabs::normalize(&file).to_str().unwrap()
        ]
    );
}

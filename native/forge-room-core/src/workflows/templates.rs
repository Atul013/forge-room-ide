use super::*;
fn check(id: &str, title: &str, recipe: CheckRecipe, needs: &[&str]) -> Step {
    Step {
        id: id.into(),
        title: title.into(),
        action: StepKind::Check { recipe },
        needs: needs.iter().map(|id| (*id).into()).collect(),
        when: When::Success,
        context: "primary".into(),
        cwd: ".".into(),
        inputs: vec![],
        artifacts: vec![],
        resources: vec![],
        timeout_seconds: 120,
        retries: 1,
        optional: false,
        review_each_attempt: false,
        room_task: None,
        room_id: None,
    }
}
fn gate(id: &str, title: &str, needs: &[&str]) -> Step {
    let mut step = check(id, title, CheckRecipe::CargoCheck, needs);
    step.action=StepKind::Manual{instructions:"Inspect the evidence and record an explicit manual result. This gate does not verify itself.".into()};
    step
}
fn workflow(id: &str, title: &str, steps: Vec<Step>) -> Workflow {
    Workflow {
        version: 1,
        id: id.into(),
        title: title.into(),
        description:
            "Starter workflow. Review roots, commands, inputs and manual gates before execution."
                .into(),
        parameters: vec![],
        steps,
        policy: Policy {
            max_attempts: 64,
            allow_retry: true,
            ..Default::default()
        },
    }
}
pub fn starter_workflows() -> Vec<Workflow> {
    let mut workflows = vec![
        workflow(
            "rust-quality",
            "Rust quality and release review",
            vec![
                check(
                    "format",
                    "Formatting check",
                    CheckRecipe::CargoFmtCheck,
                    &[],
                ),
                check(
                    "check",
                    "Check all targets",
                    CheckRecipe::CargoCheck,
                    &["format"],
                ),
                check(
                    "clippy",
                    "Warning-free Clippy",
                    CheckRecipe::CargoClippy,
                    &["check"],
                ),
                check(
                    "tests",
                    "Unit/integration tests",
                    CheckRecipe::CargoTest,
                    &["clippy"],
                ),
                check("build", "Build", CheckRecipe::CargoBuild, &["tests"]),
                gate("review", "Review changes and release evidence", &["build"]),
            ],
        ),
        workflow(
            "python-quality",
            "Python compile, tests and review",
            vec![
                check(
                    "compile",
                    "Compile selected Python inputs",
                    CheckRecipe::PythonCompile,
                    &[],
                ),
                check("tests", "Pytest", CheckRecipe::Pytest, &["compile"]),
                gate("review", "Review Python test evidence", &["tests"]),
            ],
        ),
        workflow(
            "python-unittest",
            "Python standard-library tests",
            vec![
                check(
                    "tests",
                    "Unittest discovery",
                    CheckRecipe::PythonUnittest,
                    &[],
                ),
                gate("review", "Review unittest evidence", &["tests"]),
            ],
        ),
        workflow(
            "node-tests",
            "Node test and review",
            vec![
                check(
                    "tests",
                    "Node native test runner",
                    CheckRecipe::NodeTest,
                    &[],
                ),
                gate("review", "Review Node evidence", &["tests"]),
            ],
        ),
        workflow(
            "go-quality",
            "Go vet, tests and review",
            vec![
                check("vet", "Go vet", CheckRecipe::GoVet, &[]),
                check("tests", "Go tests", CheckRecipe::GoTest, &["vet"]),
                gate("review", "Review Go evidence", &["tests"]),
            ],
        ),
        workflow(
            "dotnet-quality",
            ".NET build, tests and review",
            vec![
                check(
                    "build",
                    "Build selected project",
                    CheckRecipe::DotnetBuild,
                    &[],
                ),
                check(
                    "tests",
                    "Test selected project",
                    CheckRecipe::DotnetTest,
                    &["build"],
                ),
                gate("review", "Review .NET evidence", &["tests"]),
            ],
        ),
        workflow(
            "manual-release",
            "Manual release checklist",
            vec![
                gate("scope", "Confirm release scope and owners", &[]),
                gate("smoke", "Perform actual smoke trial", &["scope"]),
                gate(
                    "submit",
                    "Record submitted artifact/preview reference",
                    &["smoke"],
                ),
            ],
        ),
        workflow(
            "regression",
            "Reproduction and regression loop",
            vec![
                gate("reproduce", "Reproduce and describe the defect", &[]),
                gate("fix", "Review the implemented fix", &["reproduce"]),
                check(
                    "tests",
                    "Run regression tests",
                    CheckRecipe::CargoTest,
                    &["fix"],
                ),
                gate("accept", "Accept the observed behavior", &["tests"]),
            ],
        ),
    ];
    let mut ai = workflow(
        "review-with-ai",
        "Tests, explicitly reviewed AI synthesis and human acceptance",
        vec![
            check("tests", "Run tests", CheckRecipe::CargoTest, &[]),
            gate("ai", "Review evidence with a native model", &["tests"]),
            gate(
                "accept",
                "Accept or reject the model's suggestions",
                &["ai"],
            ),
        ],
    );
    ai.steps[1].action=StepKind::AiReview{prompt:"Summarize the explicitly attached workflow evidence. Distinguish observed command results from suggestions. Do not claim that files were edited or additional tests ran.".into(),include_statuses:true,include_inputs:false,max_tokens:2048};
    ai.policy.model_calls = 1;
    ai.policy.model_output_tokens = 2048;
    workflows.push(ai);
    workflows
}
pub fn from_room_tasks(id: &str, title: &str, tasks: &[crate::Task]) -> Result<Workflow, String> {
    if tasks.is_empty() || tasks.len() > MAX_STEPS {
        return Err("Choose 1–128 public Room tasks".into());
    }
    let steps=tasks.iter().map(|task|Step{id:format!("room-{}",task.id),title:task.title.clone(),action:StepKind::Manual{instructions:format!("Room task {} owned by {}. Acceptance criteria:\n{}\nRecord your own observed result. Room completion is not automatic.",task.id,task.owner_name,task.acceptance_criteria.join("\n"))},needs:task.dependencies.iter().map(|id|format!("room-{id}")).collect(),when:When::Success,context:"primary".into(),cwd:".".into(),inputs:vec![],artifacts:vec![],resources:vec![],timeout_seconds:120,retries:1,optional:false,review_each_attempt:true,room_task:Some(task.id.clone()),room_id:None}).collect();
    let result = workflow(id, title, steps);
    result.validate()?;
    Ok(result)
}

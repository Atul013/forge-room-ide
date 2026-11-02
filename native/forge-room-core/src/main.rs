use forge_room_core::{
    now_epoch_seconds, EvidenceKind, Participant, Role, RoomState, StateStore, Task, TaskStatus,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("Forge Room Early Preview: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "init" => {
            require_len(
                &args,
                5,
                "init <state.json> <room-id> <host-id> <host-name>",
            )?;
            let store = StateStore::new(PathBuf::from(&args[1]));
            let state =
                RoomState::new(&args[2], participant(&args[3], &args[4], Role::Human, None));
            store.save(&state)?;
            println!(
                "created room {} at {}",
                state.room_id,
                store.path().display()
            );
        }
        "join" => {
            require_len(
                &args,
                6,
                "join <state.json> <host-id> <participant-id> <name> <human|agent> [owner-id]",
            )?;
            let (store, mut state) = load(&args[1])?;
            let role = Role::from_str(&args[5])?;
            let owner_id = args.get(6).cloned();
            state.add_participant(&args[2], participant(&args[3], &args[4], role, owner_id))?;
            store.save(&state)?;
            println!("participant {} joined room {}", args[3], state.room_id);
        }
        "add" => {
            require_len(&args, 7, "add <state.json> <actor-id> <task-id> <title> <owner-id> <owner-name> [criterion ...]")?;
            let (store, mut state) = load(&args[1])?;
            let task = Task {
                id: args[3].clone(),
                title: args[4].clone(),
                owner_id: args[5].clone(),
                owner_name: args[6].clone(),
                status: TaskStatus::Open,
                dependencies: Vec::new(),
                acceptance_criteria: args[7..].to_vec(),
                completed_criteria: Vec::new(),
                worktree_branch: None,
                updated_at_epoch_seconds: now_epoch_seconds(),
            };
            state.create_task(&args[2], task)?;
            store.save(&state)?;
            println!("created task {}", args[3]);
        }
        "status" => {
            require_len(&args, 5, "status <state.json> <actor-id> <task-id> <open|in-progress|done> [confirmed criterion ...]")?;
            let (store, mut state) = load(&args[1])?;
            state.transition_task(
                &args[2],
                &args[3],
                TaskStatus::from_str(&args[4])?,
                args[5..].to_vec(),
            )?;
            store.save(&state)?;
            println!("updated task {}", args[3]);
        }
        "handoff-request" => {
            require_len(
                &args,
                5,
                "handoff-request <state.json> <owner-id> <task-id> <target-id>",
            )?;
            let (store, mut state) = load(&args[1])?;
            state.request_handoff(&args[2], &args[3], &args[4])?;
            store.save(&state)?;
            println!("requested handoff for task {}", args[3]);
        }
        "handoff-approve" => {
            require_len(&args, 4, "handoff-approve <state.json> <host-id> <task-id>")?;
            let (store, mut state) = load(&args[1])?;
            state.approve_handoff(&args[2], &args[3])?;
            store.save(&state)?;
            println!("approved handoff for task {}", args[3]);
        }
        "evidence" => {
            require_len(
                &args,
                6,
                "evidence <state.json> <actor-id> <task-id> <check|review|handoff> <summary>",
            )?;
            let (store, mut state) = load(&args[1])?;
            let kind = match args[4].as_str() {
                "check" => EvidenceKind::Check,
                "review" => EvidenceKind::Review,
                "handoff" => EvidenceKind::Handoff,
                _ => return Err("evidence kind must be check, review, or handoff".into()),
            };
            state.record_evidence(&args[2], &args[3], kind, &args[5])?;
            store.save(&state)?;
            println!("recorded evidence for task {}", args[3]);
        }
        "list" => {
            require_len(&args, 2, "list <state.json>")?;
            let (_, state) = load(&args[1])?;
            println!(
                "Forge Room Early Preview {}",
                forge_room_core::NATIVE_PREVIEW_VERSION
            );
            println!("room: {}", state.room_id);
            for task in state.tasks {
                println!(
                    "{}\t{}\t{}\t{}",
                    task.id, task.status, task.owner_name, task.title
                );
            }
        }
        _ => print_help(),
    }
    Ok(())
}

fn load(path: &str) -> Result<(StateStore, RoomState), Box<dyn Error>> {
    let store = StateStore::new(PathBuf::from(path));
    let outcome = store.load()?;
    if outcome.recovered_from_backup {
        eprintln!(
            "warning: primary state was invalid; recovered the previous snapshot from {}",
            store.backup_path().display()
        );
    }
    Ok((store, outcome.state))
}

fn participant(id: &str, name: &str, role: Role, owner_id: Option<String>) -> Participant {
    Participant {
        id: id.into(),
        name: name.into(),
        role,
        owner_id,
    }
}

fn require_len(args: &[String], minimum: usize, usage: &str) -> Result<(), Box<dyn Error>> {
    if args.len() < minimum {
        Err(format!("usage: forge-room-preview {usage}").into())
    } else {
        Ok(())
    }
}

fn print_help() {
    println!(
        "Forge Room Early Preview {}",
        forge_room_core::NATIVE_PREVIEW_VERSION
    );
    println!(
        "Native offline room/task slice. State stays local unless the user explicitly exports it."
    );
    println!("commands: init, join, add, status, handoff-request, handoff-approve, evidence, list");
}

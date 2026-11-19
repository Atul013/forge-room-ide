//! Closed validation recipes, explicit argv, owned-process cancellation and bounded capture.
pub use forge_room_core::workflows::CheckRecipe as Recipe;
#[cfg(windows)]
mod job;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct Plan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    pub recipe: Recipe,
}
pub(crate) fn find(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
        let file = dir.join(if cfg!(windows) {
            format!("{program}.exe")
        } else {
            program.into()
        });
        if file.is_file() {
            return Some(file);
        }
    }
    None
}
impl Plan {
    pub fn resolve(root: &Path, recipe: Recipe, files: &[PathBuf]) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let (program, args) = match recipe {
            Recipe::CargoCheck
            | Recipe::CargoTest
            | Recipe::CargoBuild
            | Recipe::CargoFmtCheck
            | Recipe::CargoClippy
            | Recipe::CargoDoc => {
                let manifest = crate::diagnostics::find_manifest(&root)
                    .ok_or("No Cargo.toml was found")?
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !manifest.starts_with(&root) {
                    return Err("Cargo manifest resolves outside this workspace".into());
                }
                let mut args = vec![
                    match recipe {
                        Recipe::CargoCheck => "check",
                        Recipe::CargoTest => "test",
                        Recipe::CargoBuild => "build",
                        Recipe::CargoFmtCheck => "fmt",
                        Recipe::CargoClippy => "clippy",
                        Recipe::CargoDoc => "doc",
                        _ => unreachable!(),
                    }
                    .into(),
                    "--manifest-path".into(),
                    crate::tabs::normalize(&manifest)
                        .to_string_lossy()
                        .into_owned(),
                ];
                match recipe {
                    Recipe::CargoCheck => args.push("--all-targets".into()),
                    Recipe::CargoFmtCheck => {
                        args.extend(["--all".into(), "--".into(), "--check".into()])
                    }
                    Recipe::CargoClippy => args.extend([
                        "--all-targets".into(),
                        "--".into(),
                        "-D".into(),
                        "warnings".into(),
                    ]),
                    Recipe::CargoDoc => args.push("--no-deps".into()),
                    _ => (),
                }
                (
                    find("cargo").ok_or("Cargo is unavailable on an absolute PATH")?,
                    args,
                )
            }
            Recipe::PythonCompile | Recipe::Pytest | Recipe::PythonUnittest => {
                let venv = if cfg!(windows) {
                    "Scripts/python.exe"
                } else {
                    "bin/python"
                };
                let program = [root.join(".venv").join(venv), root.join("venv").join(venv)]
                    .into_iter()
                    .find(|p| p.is_file())
                    .or_else(|| find("python"))
                    .or_else(|| find("python3"))
                    .ok_or("Python is unavailable")?;
                let mut args = vec![
                    "-m".into(),
                    match recipe {
                        Recipe::PythonCompile => "py_compile",
                        Recipe::Pytest => "pytest",
                        Recipe::PythonUnittest => "unittest",
                        _ => unreachable!(),
                    }
                    .into(),
                ];
                if recipe == Recipe::Pytest {
                    args.push("-q".into());
                } else if recipe == Recipe::PythonUnittest {
                    args.extend(["discover".into(), "-v".into()]);
                } else {
                    for file in files
                        .iter()
                        .filter(|p| p.extension().is_some_and(|e| e == "py"))
                    {
                        let path = file.canonicalize().map_err(|e| e.to_string())?;
                        if !path.starts_with(&root) {
                            return Err("Python file is outside the workspace".into());
                        }
                        args.push(crate::tabs::normalize(&path).to_string_lossy().into_owned());
                    }
                    if args.len() == 2 {
                        return Err("Select at least one saved .py file".into());
                    }
                }
                (program, args)
            }
            Recipe::NodeTest => {
                let mut args = vec!["--test".into()];
                for file in files.iter().filter(|path| {
                    path.extension().is_some_and(|extension| {
                        ["js", "cjs", "mjs"]
                            .iter()
                            .any(|allowed| extension == *allowed)
                    })
                }) {
                    let path = file.canonicalize().map_err(|e| e.to_string())?;
                    if !path.starts_with(&root) {
                        return Err("Node test input is outside the workspace".into());
                    }
                    args.push(crate::tabs::normalize(&path).to_string_lossy().into_owned());
                }
                (find("node").ok_or("Node is unavailable")?, args)
            }
            Recipe::GoTest | Recipe::GoVet => {
                let manifest = root
                    .join("go.mod")
                    .canonicalize()
                    .map_err(|_| "Choose the Go module folder containing go.mod")?;
                if !manifest.starts_with(&root) {
                    return Err("Go manifest is outside this root".into());
                }
                (
                    find("go").ok_or("Go is unavailable")?,
                    vec![
                        if recipe == Recipe::GoTest {
                            "test".into()
                        } else {
                            "vet".into()
                        },
                        "./...".into(),
                    ],
                )
            }
            Recipe::DotnetBuild | Recipe::DotnetTest => {
                let mut candidates = files
                    .iter()
                    .filter(|path| {
                        path.extension().is_some_and(|extension| {
                            ["sln", "slnx", "csproj", "fsproj", "vbproj"]
                                .iter()
                                .any(|allowed| extension == *allowed)
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    candidates = std::fs::read_dir(&root)
                        .map_err(|e| e.to_string())?
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension().is_some_and(|extension| {
                                ["sln", "slnx"].iter().any(|allowed| extension == *allowed)
                            })
                        })
                        .collect();
                }
                if candidates.is_empty() {
                    candidates = std::fs::read_dir(&root)
                        .map_err(|e| e.to_string())?
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.extension().is_some_and(|extension| {
                                ["csproj", "fsproj", "vbproj"]
                                    .iter()
                                    .any(|allowed| extension == *allowed)
                            })
                        })
                        .collect();
                }
                if candidates.len() != 1 {
                    return Err("Select exactly one .NET solution/project, or use a folder with one unambiguous project".into());
                }
                let target = candidates[0].canonicalize().map_err(|e| e.to_string())?;
                if !target.starts_with(&root) {
                    return Err(".NET target is outside this root".into());
                }
                (
                    find("dotnet").ok_or(".NET SDK is unavailable")?,
                    vec![
                        if recipe == Recipe::DotnetBuild {
                            "build".into()
                        } else {
                            "test".into()
                        },
                        crate::tabs::normalize(&target)
                            .to_string_lossy()
                            .into_owned(),
                    ],
                )
            }
        };
        Ok(Self {
            program,
            args,
            root,
            recipe,
        })
    }
    pub fn display(&self) -> String {
        format!(
            "{}\nargs: {:?}\nfolder: {}",
            self.program.display(),
            self.args,
            self.root.display()
        )
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub recipe: Recipe,
    pub exit_code: Option<i32>,
    pub stopped: bool,
    pub timed_out: bool,
    pub truncated: bool,
    pub capture_complete: bool,
    pub elapsed_ms: u128,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Default)]
struct Capture {
    text: Vec<u8>,
    truncated: bool,
    done: bool,
    failed: bool,
}
fn reader(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<Capture>> {
    let out = Arc::new(Mutex::new(Capture::default()));
    let work = out.clone();
    std::thread::spawn(move || {
        let mut bytes = [0; 4096];
        loop {
            let n = match pipe.read(&mut bytes) {
                Ok(n) => n,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    work.lock().unwrap().failed = true;
                    break;
                }
            };
            if n == 0 {
                break;
            }
            let mut out = work.lock().unwrap();
            let room = (64 * 1024usize).saturating_sub(out.text.len());
            out.text.extend_from_slice(&bytes[..n.min(room)]);
            out.truncated |= n > room;
        }
        work.lock().unwrap().done = true;
    });
    out
}
fn kill(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let tool =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32/taskkill.exe");
        let _ = Command::new(tool)
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}
pub struct Run {
    child: Arc<Mutex<Option<Child>>>,
    stop: Arc<AtomicBool>,
    receiver: mpsc::Receiver<Result<Evidence, String>>,
}
impl Run {
    pub fn start(plan: Plan, seconds: u64, wake: impl Fn() + Send + 'static) -> Self {
        let child = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, receiver) = mpsc::channel();
        let slot = child.clone();
        let canceled = stop.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<Evidence, String> {
                if canceled.load(Ordering::Acquire) {
                    return Err("Validation canceled before spawning".into());
                }
                #[cfg(windows)]
                let job = job::Guard::new()?;
                let mut command = Command::new(&plan.program);
                command
                    .args(&plan.args)
                    .current_dir(&plan.root)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    command.creation_flags(0x0800_0000);
                }
                let mut process = command
                    .spawn()
                    .map_err(|e| format!("Validation could not start: {e}"))?;
                #[cfg(windows)]
                if let Err(error) = job.assign(&process) {
                    kill(&mut process);
                    let _ = process.wait();
                    return Err(error);
                }
                let stdout = reader(process.stdout.take().unwrap());
                let stderr = reader(process.stderr.take().unwrap());
                let start = Instant::now();
                {
                    let mut slot = slot.lock().unwrap();
                    if canceled.load(Ordering::Acquire) {
                        kill(&mut process);
                        let _ = process.wait();
                        return Err("Validation stopped before startup".into());
                    }
                    *slot = Some(process);
                }
                let mut timed_out = false;
                let mut stopped = false;
                let status = loop {
                    let mut slot = slot.lock().unwrap();
                    let process = slot.as_mut().ok_or("Validation was canceled")?;
                    if let Some(status) = process.try_wait().map_err(|e| e.to_string())? {
                        break status;
                    }
                    timed_out |= start.elapsed() >= Duration::from_secs(seconds);
                    stopped |= canceled.load(Ordering::Acquire);
                    if timed_out || stopped {
                        kill(process);
                        break process.wait().map_err(|e| e.to_string())?;
                    }
                    drop(slot);
                    std::thread::sleep(Duration::from_millis(20));
                };
                slot.lock().unwrap().take();
                let deadline = Instant::now() + Duration::from_millis(500);
                while Instant::now() < deadline
                    && (!stdout.lock().unwrap().done || !stderr.lock().unwrap().done)
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                let stdout = stdout.lock().unwrap();
                let stderr = stderr.lock().unwrap();
                Ok(Evidence {
                    recipe: plan.recipe,
                    exit_code: status.code(),
                    stopped,
                    timed_out,
                    truncated: stdout.truncated || stderr.truncated,
                    capture_complete: stdout.done
                        && stderr.done
                        && !stdout.failed
                        && !stderr.failed,
                    elapsed_ms: start.elapsed().as_millis(),
                    stdout: crate::agent::sanitize(&String::from_utf8_lossy(&stdout.text)),
                    stderr: crate::agent::sanitize(&String::from_utf8_lossy(&stderr.text)),
                })
            })();
            if let Some(mut process) = slot.lock().unwrap().take() {
                kill(&mut process);
                let _ = process.wait();
            }
            let _ = tx.send(result);
            wake();
        });
        Self {
            child,
            stop,
            receiver,
        }
    }
    pub fn poll(&mut self) -> Option<Result<Evidence, String>> {
        match self.receiver.try_recv() {
            Ok(e) => Some(e),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Validation worker stopped unexpectedly".into()))
            }
            Err(_) => None,
        }
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let slot = self.child.clone();
        std::thread::spawn(move || {
            if let Some(mut child) = slot.lock().unwrap().take() {
                kill(&mut child);
                let _ = child.wait();
            }
        });
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    const CRASH_ROLE: &str = "FORGE_VALIDATION_CRASH_FIXTURE";
    fn fixture_selected(name: &str) -> bool {
        let args = std::env::args().collect::<Vec<_>>();
        args.windows(2).any(|pair| {
            pair[0] == "--exact"
                && pair[1] == format!("controlled_build::validation::tests::{name}")
        })
    }
    fn fixture_root(prefix: &str) -> PathBuf {
        let root = std::env::current_dir().unwrap().canonicalize().unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(prefix)
        );
        root
    }
    fn fixture_plan(root: PathBuf, name: &str) -> Plan {
        Plan {
            program: std::env::current_exe().unwrap(),
            args: vec![
                "--exact".into(),
                format!("controlled_build::validation::tests::{name}"),
                "--nocapture".into(),
                "--test-threads=1".into(),
            ],
            root,
            recipe: Recipe::CargoCheck,
        }
    }
    #[test]
    fn fixture_output() {
        if !fixture_selected("fixture_output") {
            return;
        }
        use std::io::Write;
        std::io::stdout()
            .lock()
            .write_all(&vec![b'x'; 100000])
            .unwrap();
        std::io::stderr().lock().write_all(b"failure").unwrap();
        std::process::exit(7);
    }
    #[test]
    fn fixture_wait() {
        if !fixture_selected("fixture_wait") {
            return;
        }
        let root = fixture_root("forge-validation-probe-");
        std::fs::write(root.join("fixture-ready"), "ready").unwrap();
        std::thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn fixture_crash_child() {
        if !fixture_selected("fixture_crash_child") {
            return;
        }
        let root = fixture_root("forge-owned-crash-");
        assert!(std::env::var_os(CRASH_ROLE).is_some());
        std::fs::write(root.join("child-pid"), std::process::id().to_string()).unwrap();
        std::fs::write(root.join("child-started"), "owned").unwrap();
        std::thread::sleep(Duration::from_secs(30));
        std::fs::write(root.join("child-finished"), "orphan").unwrap();
    }
    #[test]
    fn owned_child_cannot_outlive_runner_process() {
        const TEST: &str =
            "controlled_build::validation::tests::owned_child_cannot_outlive_runner_process";
        if let Some(root) = std::env::var_os(CRASH_ROLE) {
            let root = PathBuf::from(root).canonicalize().unwrap();
            assert_eq!(root, fixture_root("forge-owned-crash-"));
            let mut run = Run::start(fixture_plan(root.clone(), "fixture_crash_child"), 30, || {});
            let deadline = Instant::now() + Duration::from_secs(15);
            while !root.join("child-started").exists() {
                assert!(run.poll().is_none(), "Owned child ended before startup");
                assert!(
                    Instant::now() < deadline,
                    "Owned Rust child did not become ready"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            std::fs::write(root.join("helper-ready"), "ready").unwrap();
            loop {
                assert!(
                    run.poll().is_none(),
                    "Helper was not terminated before child completion"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let temp = std::env::temp_dir().canonicalize().unwrap();
        let root = temp.join(format!(
            "forge-owned-crash-{}",
            forge_room_core::workflows::new_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
            .env(CRASH_ROLE, crate::tabs::normalize(&root))
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(std::fs::File::create(root.join("helper.log")).unwrap())
            .stderr(std::fs::File::create(root.join("helper-error.log")).unwrap());
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut helper = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !root.join("helper-ready").exists() {
            let exited = helper.try_wait().unwrap();
            if exited.is_some() || Instant::now() >= deadline {
                let _ = helper.kill();
                let _ = helper.wait();
                panic!(
                    "Runner helper readiness failed ({exited:?}): {} / {}",
                    std::fs::read_to_string(root.join("helper.log")).unwrap_or_default(),
                    std::fs::read_to_string(root.join("helper-error.log")).unwrap_or_default()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::{
            Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let pid = std::fs::read_to_string(root.join("child-pid"))
            .unwrap()
            .parse::<u32>()
            .unwrap();
        // SAFETY: The fixture recorded its own PID before readiness. This handle only waits; it cannot terminate another process.
        let raw = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        assert!(
            !raw.is_null(),
            "Could not observe owned fixture child: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: OpenProcess returned a live owned handle, released exactly once here.
        let child = unsafe { OwnedHandle::from_raw_handle(raw) };
        // SAFETY: The process handle stays live for both wait calls.
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle(), 0) },
            WAIT_TIMEOUT
        );
        helper.kill().unwrap();
        helper.wait().unwrap();
        // SAFETY: Waiting on our observed fixture process is read-only and bounded.
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle(), 10000) },
            WAIT_OBJECT_0,
            "Owned child outlived its force-terminated runner"
        );
        assert!(!root.join("child-finished").exists());
        drop(child);
        assert!(
            root.starts_with(&temp)
                && root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-owned-crash-")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    fn run(fixture: &str, seconds: u64, cancel: bool) -> Evidence {
        let temp = std::env::temp_dir().canonicalize().unwrap();
        let root = temp.join(format!(
            "forge-validation-probe-{}",
            forge_room_core::workflows::new_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut run = Run::start(fixture_plan(root.clone(), fixture), seconds, || {});
        let deadline = Instant::now() + Duration::from_secs(30);
        if cancel {
            while !root.join("fixture-ready").exists() {
                if let Some(result) = run.poll() {
                    panic!(
                        "Fixture ended before cancellation checkpoint: {}",
                        result
                            .map(|e| format!(
                                "exit={:?}, stopped={}, timeout={}, stderr={}",
                                e.exit_code, e.stopped, e.timed_out, e.stderr
                            ))
                            .unwrap_or_else(|e| e)
                    );
                }
                assert!(
                    Instant::now() < deadline,
                    "Rust fixture did not become ready"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            run.cancel();
        }
        loop {
            if let Some(result) = run.poll() {
                let evidence = result.unwrap();
                drop(run);
                assert!(
                    root.starts_with(&temp)
                        && root
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("forge-validation-probe-")
                );
                std::fs::remove_dir_all(root).unwrap();
                return evidence;
            }
            assert!(Instant::now() < deadline, "validation did not stop");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    #[test]
    fn owned_process_captures_failure_and_bounds_output() {
        let e = run("fixture_output", 10, false);
        assert_eq!(
            e.exit_code,
            Some(7),
            "stdout={} stderr={}",
            e.stdout,
            e.stderr
        );
        assert!(e.truncated);
        assert!(e.capture_complete);
        assert_eq!(e.stdout.len(), 65536);
        assert_eq!(e.stderr, "failure");
    }
    #[test]
    fn owned_process_timeout_and_cancellation() {
        let timed = run("fixture_wait", 1, false);
        assert!(timed.timed_out);
        let cancelled = run("fixture_wait", 30, true);
        assert!(cancelled.stopped);
    }
    #[test]
    fn read_failure_is_not_complete_capture() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("broken"))
            }
        }
        let capture = reader(Broken);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !capture.lock().unwrap().done {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(capture.lock().unwrap().failed);
    }
}

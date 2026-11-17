//! Closed validation recipes, explicit argv, owned-process cancellation and bounded capture.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipe {
    CargoCheck,
    CargoTest,
    PythonCompile,
    Pytest,
}
impl Recipe {
    pub const ALL: [Self; 4] = [
        Self::CargoCheck,
        Self::CargoTest,
        Self::PythonCompile,
        Self::Pytest,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo check --all-targets",
            Self::CargoTest => "cargo test",
            Self::PythonCompile => "Python compile selected .py files",
            Self::Pytest => "python -m pytest -q",
        }
    }
}
#[derive(Clone)]
pub struct Plan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    pub recipe: Recipe,
}
fn find(program: &str) -> Option<PathBuf> {
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
            Recipe::CargoCheck | Recipe::CargoTest => {
                let manifest = crate::diagnostics::find_manifest(&root)
                    .ok_or("No Cargo.toml was found")?
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !manifest.starts_with(&root) {
                    return Err("Cargo manifest resolves outside this workspace".into());
                }
                let mut args = vec![
                    if recipe == Recipe::CargoCheck {
                        "check"
                    } else {
                        "test"
                    }
                    .into(),
                    "--manifest-path".into(),
                    crate::tabs::normalize(&manifest)
                        .to_string_lossy()
                        .into_owned(),
                ];
                if recipe == Recipe::CargoCheck {
                    args.push("--all-targets".into());
                }
                (
                    find("cargo").ok_or("Cargo is unavailable on an absolute PATH")?,
                    args,
                )
            }
            Recipe::PythonCompile | Recipe::Pytest => {
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
                    if recipe == Recipe::PythonCompile {
                        "py_compile"
                    } else {
                        "pytest"
                    }
                    .into(),
                ];
                if recipe == Recipe::Pytest {
                    args.push("-q".into());
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
#[derive(Clone, Serialize)]
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
    fn run(script: &str, seconds: u64, cancel: bool) -> Evidence {
        let plan = Plan {
            program: PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                .join("System32/WindowsPowerShell/v1.0/powershell.exe"),
            args: vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                script.into(),
            ],
            root: std::env::temp_dir(),
            recipe: Recipe::CargoCheck,
        };
        let mut run = Run::start(plan, seconds, || {});
        if cancel {
            std::thread::sleep(Duration::from_millis(300));
            run.cancel();
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(result) = run.poll() {
                return result.unwrap();
            }
            assert!(Instant::now() < deadline, "validation did not stop");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    #[test]
    fn owned_process_captures_failure_and_bounds_output() {
        let e = run(
            "[Console]::Out.Write(('x' * 100000)); [Console]::Error.Write('failure'); exit 7",
            10,
            false,
        );
        assert_eq!(e.exit_code, Some(7));
        assert!(e.truncated);
        assert!(e.capture_complete);
        assert_eq!(e.stdout.len(), 65536);
        assert_eq!(e.stderr, "failure");
    }
    #[test]
    fn owned_process_timeout_and_cancellation() {
        let timed = run("Start-Sleep -Seconds 10", 1, false);
        assert!(timed.timed_out);
        let canceled = run("Start-Sleep -Seconds 10", 10, true);
        assert!(canceled.stopped);
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

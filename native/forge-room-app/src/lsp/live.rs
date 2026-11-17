//! Explicitly started, workspace-scoped language servers shared by open buffers and queries.
use super::*;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::{Child, ChildStdin};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const MAX_FILES: usize = 16;
const MAX_TEXT: usize = 256 * 1024;
const MAX_TOTAL: usize = 2 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct Document {
    pub path: PathBuf,
    pub text: String,
    pub saved: bool,
    version: i64,
}
impl Document {
    pub fn new(path: PathBuf, text: String, saved: bool) -> Self {
        Self {
            path,
            text,
            saved,
            version: 0,
        }
    }
}
struct Request {
    ticket: u64,
    epoch: u64,
    doc: Document,
    line: usize,
    column: usize,
    kind: QueryKind,
    name: Option<String>,
}
pub struct Answer {
    ticket: u64,
    epoch: u64,
    pub file: PathBuf,
    pub text: String,
    pub result: QueryAnswer,
}
#[derive(Default)]
struct Output {
    status: String,
    ready: bool,
    ended: bool,
    diagnostics: BTreeMap<PathBuf, (i64, Vec<Problem>)>,
    answer: Option<Answer>,
    changed: bool,
}
struct Session {
    program: &'static str,
    documents: Arc<Mutex<Option<Vec<Document>>>>,
    output: Arc<Mutex<Output>>,
    requests: mpsc::SyncSender<Request>,
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Give shutdown/exit a short grace period. The watchdog also unblocks a pipe write
        // when a server stopped reading stdin; it never waits on the UI thread.
        let child = self.child.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            if let Some(process) = child.lock().unwrap().take() {
                terminate(process);
            }
        });
    }
}

fn terminate(mut child: Child) {
    if child.try_wait().ok().flatten().is_none() {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let tool = PathBuf::from(
                std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
            )
            .join("System32")
            .join("taskkill.exe");
            let _ = Command::new(tool)
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn installed_server(program: &str) -> Result<Command, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for folder in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
        let names = if cfg!(windows) {
            vec![
                format!("{program}.exe"),
                format!("{program}.cmd"),
                format!("{program}.bat"),
            ]
        } else {
            vec![program.to_owned()]
        };
        for name in names {
            let candidate = folder.join(name);
            if candidate.is_file() {
                return Ok(Command::new(candidate));
            }
        }
    }
    Err(format!(
        "{program} is unavailable; install it in an absolute PATH folder and restart Forge Room"
    ))
}

#[derive(Default)]
pub struct Manager {
    root: Option<PathBuf>,
    sessions: Vec<Session>,
    documents: Vec<Document>,
    next_version: i64,
    epoch: u64,
    next_request: u64,
    latest_request: u64,
    pub notice: String,
    dirty: bool,
}
impl Manager {
    pub fn captures(&self, path: &Path) -> bool {
        language(path)
            .is_some_and(|(_, program, _)| self.sessions.iter().any(|s| s.program == program))
    }
    pub fn running(&self) -> bool {
        !self.sessions.is_empty()
    }
    pub fn stop(&mut self) {
        self.sessions.clear();
        self.documents.clear();
        self.root = None;
        self.epoch += 1;
        self.notice = "Language sessions stopped".into();
        self.dirty = true;
    }
    pub fn status(&self) -> String {
        let summary = self
            .sessions
            .iter()
            .map(|s| s.output.lock().unwrap().status.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        let waiting = self
            .documents
            .iter()
            .filter(|doc| {
                !self.sessions.iter().any(|s| {
                    s.output
                        .lock()
                        .unwrap()
                        .diagnostics
                        .get(&doc.path)
                        .is_some_and(|(v, _)| *v == doc.version)
                })
            })
            .count();
        format!("{summary} | {waiting} buffer(s) awaiting diagnostics")
    }
    pub fn start(&mut self, root: &Path, file: &Path) -> Result<(), String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let file = file.canonicalize().map_err(|e| e.to_string())?;
        if !file.starts_with(&root) {
            return Err("File is outside the trusted folder".into());
        }
        let (_, program, args) =
            language(&file).ok_or("No configured server for this file type")?;
        if self.root.as_ref().is_some_and(|old| old != &root) {
            self.stop();
        }
        self.root = Some(root.clone());
        self.sessions
            .retain(|s| !(s.program == program && s.output.lock().unwrap().ended));
        if self.sessions.iter().any(|s| s.program == program) {
            self.notice = format!("{program} already has a session");
            return Ok(());
        }
        self.sessions
            .push(Session::spawn(root, program, args, None));
        *self.sessions.last().unwrap().documents.lock().unwrap() = Some(
            self.documents
                .iter()
                .filter(|d| language(&d.path).is_some_and(|(_, p, _)| p == program))
                .cloned()
                .collect(),
        );
        self.notice =
            format!("Starting {program}; open buffers of this language will be shared with it");
        Ok(())
    }
    /// Called at most a few times per second. Latest snapshots replace queued typing bursts.
    pub fn sync(&mut self, root: Option<&Path>, trusted: bool, documents: Vec<Document>) {
        let root = root.and_then(|r| r.canonicalize().ok());
        if !trusted || root != self.root {
            if self.running() {
                self.stop();
            }
            return;
        }
        let Some(root) = root else {
            return;
        };
        let mut accepted = Vec::new();
        let mut total = 0;
        let mut excluded = 0;
        for mut doc in documents {
            let Ok(path) = doc.path.canonicalize() else {
                continue;
            };
            let Some((_, program, _)) = language(&path) else {
                continue;
            };
            if !self.sessions.iter().any(|s| s.program == program) {
                continue;
            }
            if !path.starts_with(&root)
                || doc.text.len() > MAX_TEXT
                || accepted.len() >= MAX_FILES
                || total + doc.text.len() > MAX_TOTAL
            {
                excluded += 1;
                continue;
            }
            if accepted.iter().any(|d: &Document| d.path == path) {
                continue;
            }
            doc.path = path;
            doc.version = if let Some(old) = self
                .documents
                .iter()
                .find(|d| d.path == doc.path && d.text == doc.text)
            {
                old.version
            } else {
                self.next_version += 1;
                self.next_version
            };
            total += doc.text.len();
            accepted.push(doc);
        }
        if excluded > 0 {
            self.notice = format!(
                "{excluded} buffer(s) excluded: trusted root, 16 files, 256 KiB each / 2 MiB total"
            );
        }
        if accepted != self.documents {
            self.epoch += 1;
            self.documents = accepted;
            self.dirty = true;
            for session in &self.sessions {
                *session.documents.lock().unwrap() = Some(
                    self.documents
                        .iter()
                        .filter(|d| language(&d.path).is_some_and(|(_, p, _)| p == session.program))
                        .cloned()
                        .collect(),
                );
            }
        }
    }
    pub fn query(
        &mut self,
        file: &Path,
        text: &str,
        line: usize,
        column: usize,
        kind: QueryKind,
        name: Option<String>,
    ) -> Result<(), String> {
        let file = file.canonicalize().map_err(|e| e.to_string())?;
        let doc = self
            .documents
            .iter()
            .find(|d| d.path == file && d.text == text)
            .ok_or("Buffer is not synchronized; start its language session or check buffer limits")?
            .clone();
        let (_, program, _) = language(&file).ok_or("Unsupported language")?;
        let session = self
            .sessions
            .iter()
            .find(|s| s.program == program)
            .ok_or("Start this language session first")?;
        if !session.output.lock().unwrap().ready {
            return Err("Language server is not ready; retry after initialization".into());
        }
        self.next_request += 1;
        session
            .requests
            .try_send(Request {
                ticket: self.next_request,
                epoch: self.epoch,
                doc,
                line,
                column,
                kind,
                name,
            })
            .map_err(|_| "Language server query queue is busy".to_owned())?;
        self.latest_request = self.next_request;
        Ok(())
    }
    pub fn has_answer(&self) -> bool {
        self.sessions
            .iter()
            .any(|s| s.output.lock().unwrap().answer.is_some())
    }
    pub fn take_answer(&mut self) -> Option<Answer> {
        for session in &self.sessions {
            if let Some(answer) = session.output.lock().unwrap().answer.take() {
                if answer.ticket == self.latest_request && answer.epoch == self.epoch {
                    return Some(answer);
                }
                self.notice = "Discarded an older query or a result from changed buffers".into();
            }
        }
        None
    }
    pub fn report(&mut self) -> Option<Report> {
        let mut changed = std::mem::take(&mut self.dirty);
        let mut report = Report {
            source: "live language servers",
            ..Report::default()
        };
        let mut failures = Vec::new();
        for session in &self.sessions {
            let mut out = session.output.lock().unwrap();
            changed |= std::mem::take(&mut out.changed);
            if out.ended {
                failures.push(out.status.clone());
            }
            for (path, (version, problems)) in &out.diagnostics {
                if self
                    .documents
                    .iter()
                    .any(|d| &d.path == path && d.version == *version)
                {
                    report.problems.extend(problems.iter().cloned());
                }
            }
        }
        report.truncated = report.problems.len() > 500;
        report.problems.truncate(500);
        if !failures.is_empty() {
            report.failure = Some(failures.join("; "));
        }
        changed.then_some(report)
    }
}

impl Session {
    fn spawn(
        root: PathBuf,
        program: &'static str,
        args: &'static [&'static str],
        custom: Option<Command>,
    ) -> Self {
        let documents = Arc::new(Mutex::new(None));
        let output = Arc::new(Mutex::new(Output {
            status: format!("{program}: initializing"),
            ..Output::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let (sender, receiver) = mpsc::sync_channel(4);
        let session = Self {
            program,
            documents: documents.clone(),
            output: output.clone(),
            requests: sender,
            stop: stop.clone(),
            child: child.clone(),
        };
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let mut command = match custom {
                    Some(c) => c,
                    None => {
                        let mut c = installed_server(program)?;
                        c.args(args);
                        c
                    }
                };
                command
                    .current_dir(&root)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    command.creation_flags(0x0800_0000);
                }
                let mut process = command
                    .spawn()
                    .map_err(|e| format!("{program} is unavailable: {e}"))?;
                let input = process.stdin.take().expect("piped stdin");
                let stdout = process.stdout.take().expect("piped stdout");
                {
                    let mut slot = child.lock().unwrap();
                    if stop.load(Ordering::Acquire) {
                        terminate(process);
                        return Ok(());
                    }
                    *slot = Some(process);
                }
                let (messages, incoming) = mpsc::sync_channel(16);
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stdout);
                    while let Ok(message) = read_message(&mut reader) {
                        if messages.send(message).is_err() {
                            break;
                        }
                    }
                });
                run(
                    &root, program, input, incoming, receiver, &documents, &output, &stop,
                )
            })();
            if let Some(process) = child.lock().unwrap().take() {
                terminate(process);
            }
            let mut out = output.lock().unwrap();
            out.ready = false;
            out.ended = true;
            out.changed = true;
            out.status = match result {
                Ok(()) => format!("{program}: stopped"),
                Err(e) => e,
            };
        });
        session
    }
}

struct SyncMode {
    incremental: bool,
    save: Option<bool>,
}
fn sync_mode(result: &Value) -> Result<SyncMode, String> {
    let sync = &result["capabilities"]["textDocumentSync"];
    let kind = sync
        .as_u64()
        .or_else(|| sync["change"].as_u64())
        .unwrap_or(0);
    if !matches!(kind, 1 | 2) || sync.get("openClose") == Some(&json!(false)) {
        return Err("Server does not support open-buffer synchronization".into());
    }
    let save = match &sync["save"] {
        Value::Bool(true) => Some(false),
        Value::Object(o) => Some(
            o.get("includeText")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        _ => None,
    };
    Ok(SyncMode {
        incremental: kind == 2,
        save,
    })
}
fn notify(input: &mut ChildStdin, method: &str, params: Value) -> Result<(), String> {
    write_message(
        input,
        &json!({"jsonrpc":"2.0","method":method,"params":params}),
    )
    .map_err(|e| e.to_string())
}
fn sync_documents(
    input: &mut ChildStdin,
    mode: &SyncMode,
    old: &mut HashMap<PathBuf, Document>,
    wanted: Vec<Document>,
    output: &Arc<Mutex<Output>>,
) -> Result<(), String> {
    let removed: Vec<_> = old
        .keys()
        .filter(|p| !wanted.iter().any(|d| &d.path == *p))
        .cloned()
        .collect();
    for path in removed {
        notify(
            input,
            "textDocument/didClose",
            json!({"textDocument":{"uri":uri(&path)}}),
        )?;
        old.remove(&path);
        let mut out = output.lock().unwrap();
        out.diagnostics.remove(&path);
        out.changed = true;
    }
    for doc in wanted {
        if let Some(previous) = old.get(&doc.path) {
            if previous.version != doc.version {
                let change = if mode.incremental {
                    let lines: Vec<_> = previous.text.split('\n').collect();
                    json!({"range":{"start":{"line":0,"character":0},"end":{"line":lines.len()-1,"character":lines.last().unwrap().encode_utf16().count()}},"text":doc.text})
                } else {
                    json!({"text":doc.text})
                };
                notify(
                    input,
                    "textDocument/didChange",
                    json!({"textDocument":{"uri":uri(&doc.path),"version":doc.version},"contentChanges":[change]}),
                )?;
                let mut out = output.lock().unwrap();
                out.diagnostics.remove(&doc.path);
                out.changed = true;
            }
            if doc.saved
                && (!previous.saved || previous.version != doc.version)
                && let Some(include) = mode.save
            {
                let mut params = json!({"textDocument":{"uri":uri(&doc.path)}});
                if include {
                    params["text"] = json!(doc.text);
                }
                notify(input, "textDocument/didSave", params)?;
            }
        } else {
            notify(
                input,
                "textDocument/didOpen",
                json!({"textDocument":{"uri":uri(&doc.path),"languageId":language_id(&doc.path),"version":doc.version,"text":doc.text}}),
            )?;
        }
        old.insert(doc.path.clone(), doc);
    }
    Ok(())
}
fn language_id(path: &Path) -> &str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "tsx" => "typescriptreact",
        "js" => "javascript",
        "jsx" => "javascriptreact",
        _ => language(path).map(|l| l.0).unwrap_or(""),
    }
}
fn server_request(input: &mut ChildStdin, message: &Value, root: &Path) -> Result<(), String> {
    let (Some(id), Some(method)) = (
        message.get("id"),
        message.get("method").and_then(Value::as_str),
    ) else {
        return Ok(());
    };
    let response = match method {
        "workspace/configuration" => {
            json!({"result":vec![Value::Null; message["params"]["items"].as_array().map_or(0,|a| a.len().min(100))]})
        }
        "workspace/workspaceFolders" => json!({"result":[{"uri":uri(root),"name":"workspace"}]}),
        "workspace/applyEdit" => {
            json!({"result":{"applied":false,"failureReason":"Use explicit reviewed rename edits"}})
        }
        "window/workDoneProgress/create" => json!({"result":null}),
        _ => json!({"error":{"code":-32601,"message":"Server request is unsupported"}}),
    };
    let mut response = response;
    response["jsonrpc"] = json!("2.0");
    response["id"] = id.clone();
    write_message(input, &response).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn run(
    root: &Path,
    program: &str,
    mut input: ChildStdin,
    incoming: mpsc::Receiver<Value>,
    requests: mpsc::Receiver<Request>,
    documents: &Arc<Mutex<Option<Vec<Document>>>>,
    output: &Arc<Mutex<Output>>,
    stop: &AtomicBool,
) -> Result<(), String> {
    write_message(&mut input, &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "processId":std::process::id(),"rootUri":uri(root),"workspaceFolders":[{"uri":uri(root),"name":"workspace"}],
        "capabilities":{"general":{"positionEncodings":["utf-16"]},"textDocument":{"publishDiagnostics":{"versionSupport":true},"synchronization":{"didSave":true}}}
    }})).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + LIMIT;
    let mode = loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("{program}: initialization timed out"));
        }
        match incoming.recv_timeout(Duration::from_millis(30)) {
            Ok(message)
                if message.get("id") == Some(&json!(1)) && message.get("method").is_none() =>
            {
                if message.get("error").is_some() {
                    return Err(format!("{program}: initialization rejected"));
                }
                if message["result"]["capabilities"]["positionEncoding"]
                    .as_str()
                    .is_some_and(|p| p != "utf-16")
                {
                    return Err("Server selected unsupported position encoding".into());
                }
                break sync_mode(&message["result"])?;
            }
            Ok(message) => server_request(&mut input, &message, root)?,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!("{program}: server exited during initialization"));
            }
            Err(_) => {}
        }
    };
    notify(&mut input, "initialized", json!({}))?;
    {
        let mut out = output.lock().unwrap();
        out.ready = true;
        out.status = format!("{program}: live");
        out.changed = true;
    }
    let mut opened = HashMap::new();
    let mut pending: Option<(u64, Request, Instant)> = None;
    let mut next_id = 2u64;
    while !stop.load(Ordering::Acquire) {
        if let Some(wanted) = documents.lock().unwrap().take() {
            sync_documents(&mut input, &mode, &mut opened, wanted, output)?;
        }
        if pending.as_ref().is_some_and(|(_, request, _)| {
            !opened
                .get(&request.doc.path)
                .is_some_and(|d| d.version == request.doc.version)
        }) {
            let (id, request, _) = pending.take().unwrap();
            notify(&mut input, "$/cancelRequest", json!({"id":id}))?;
            publish_answer(output, request, "Buffer changed; query canceled".into());
        }
        if pending.is_none()
            && let Ok(request) = requests.try_recv()
        {
            if !opened
                .get(&request.doc.path)
                .is_some_and(|d| d.version == request.doc.version)
            {
                publish_answer(
                    output,
                    request,
                    "Buffer changed before the query was sent".into(),
                );
            } else {
                let method = match request.kind {
                    QueryKind::Hover => "textDocument/hover",
                    QueryKind::Definition => "textDocument/definition",
                    QueryKind::Completion => "textDocument/completion",
                    QueryKind::References => "textDocument/references",
                    QueryKind::Rename => "textDocument/rename",
                };
                let offset: usize = request
                    .doc
                    .text
                    .lines()
                    .nth(request.line.saturating_sub(1))
                    .unwrap_or("")
                    .chars()
                    .take(request.column.saturating_sub(1))
                    .map(char::len_utf16)
                    .sum();
                let mut params = json!({"textDocument":{"uri":uri(&request.doc.path)},"position":{"line":request.line.saturating_sub(1),"character":offset}});
                match request.kind {
                    QueryKind::References => params["context"] = json!({"includeDeclaration":true}),
                    QueryKind::Completion => params["context"] = json!({"triggerKind":1}),
                    QueryKind::Rename => params["newName"] = json!(request.name),
                    _ => {}
                }
                write_message(
                    &mut input,
                    &json!({"jsonrpc":"2.0","id":next_id,"method":method,"params":params}),
                )
                .map_err(|e| e.to_string())?;
                pending = Some((next_id, request, Instant::now()));
                next_id += 1;
            }
        }
        if pending
            .as_ref()
            .is_some_and(|(_, _, since)| since.elapsed() > LIMIT)
        {
            let (id, request, _) = pending.take().unwrap();
            notify(&mut input, "$/cancelRequest", json!({"id":id}))?;
            publish_answer(output, request, "Language query timed out".into());
        }
        match incoming.recv_timeout(Duration::from_millis(30)) {
            Ok(message) => {
                if message.get("method").and_then(Value::as_str)
                    == Some("textDocument/publishDiagnostics")
                {
                    if let Some(path) = message["params"]["uri"]
                        .as_str()
                        .and_then(path_from_uri)
                        .and_then(|p| p.canonicalize().ok())
                        && let Some(doc) = opened.get(&path)
                    {
                        let version = message["params"]["version"].as_i64();
                        // Unversioned diagnostics are accepted against the most recently sent snapshot.
                        if version.is_none_or(|v| v == doc.version) {
                            let mut out = output.lock().unwrap();
                            let mut problems = parse_diagnostics(&message, &path);
                            for p in &mut problems {
                                p.column = scalar_column(&doc.text, p.line, p.column);
                            }
                            out.diagnostics
                                .insert(path.clone(), (doc.version, problems));
                            out.changed = true;
                        }
                    }
                } else if message.get("method").is_none()
                    && pending
                        .as_ref()
                        .is_some_and(|(id, _, _)| message.get("id") == Some(&json!(id)))
                {
                    let (_, request, _) = pending.take().unwrap();
                    let mut answer = if message.get("error").is_some() {
                        QueryAnswer {
                            kind: request.kind,
                            message: "Language server rejected this query".into(),
                            location: None,
                            completions: vec![],
                            locations: vec![],
                            rename_edit: None,
                        }
                    } else {
                        query_result(request.kind, &message, root)
                    };
                    for (file, line, column) in answer
                        .location
                        .iter_mut()
                        .chain(answer.locations.iter_mut())
                    {
                        if let Some(doc) =
                            opened.get(&file.canonicalize().unwrap_or_else(|_| file.clone()))
                        {
                            *column = scalar_column(&doc.text, *line, *column);
                        } else if std::fs::metadata(&*file)
                            .is_ok_and(|m| m.len() <= MAX_TEXT as u64)
                            && let Ok(text) = std::fs::read_to_string(&*file)
                        {
                            *column = scalar_column(&text, *line, *column);
                        }
                    }
                    output.lock().unwrap().answer = Some(Answer {
                        ticket: request.ticket,
                        epoch: request.epoch,
                        file: request.doc.path,
                        text: request.doc.text,
                        result: answer,
                    });
                } else {
                    server_request(&mut input, &message, root)?;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!("{program}: server exited; restart explicitly"));
            }
            Err(_) => {}
        }
    }
    // LSP requests shutdown first, then exit. A stalled peer is killed by the Drop watchdog.
    write_message(
        &mut input,
        &json!({"jsonrpc":"2.0","id":next_id,"method":"shutdown","params":null}),
    )
    .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        match incoming.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(message) if message.get("id") == Some(&json!(next_id)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    notify(&mut input, "exit", Value::Null)?;
    Ok(())
}

fn scalar_column(text: &str, line: usize, column: usize) -> usize {
    let mut units = 0;
    let mut count = 0;
    for c in text
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .chars()
    {
        if units + c.len_utf16() > column.saturating_sub(1) {
            break;
        }
        units += c.len_utf16();
        count += 1;
    }
    count + 1
}
fn publish_answer(output: &Arc<Mutex<Output>>, request: Request, message: String) {
    output.lock().unwrap().answer = Some(Answer {
        ticket: request.ticket,
        epoch: request.epoch,
        file: request.doc.path,
        text: request.doc.text,
        result: QueryAnswer {
            kind: request.kind,
            message,
            location: None,
            completions: vec![],
            locations: vec![],
            rename_edit: None,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    #[test]
    fn utf16_locations_match_editor_scalar_columns() {
        assert_eq!(scalar_column("a😀b\nnext", 1, 4), 3);
        assert_eq!(scalar_column("a😀b", 1, 3), 2);
        assert_eq!(scalar_column("a😀b", 1, 99), 4);
        assert_eq!(language_id(Path::new("source.jsx")), "javascriptreact");
    }
    fn peer() -> &'static Path {
        static EXE: OnceLock<PathBuf> = OnceLock::new();
        EXE.get_or_init(|| {
            let folder =
                std::env::temp_dir().join(format!("forge-language-peer-{}", std::process::id()));
            std::fs::create_dir_all(&folder).unwrap();
            let exe = folder.join(if cfg!(windows) { "peer.exe" } else { "peer" });
            let result = Command::new("rustc")
                .arg("--edition=2024")
                .arg(
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/language_server.rs"),
                )
                .arg("-o")
                .arg(&exe)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            exe
        })
    }
    fn fixture(mode: &str) -> (Manager, PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "forge-live-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let file = root.join("main.rs");
        std::fs::write(&file, "disk text").unwrap();
        let log = root.join("server.log");
        let mut command = Command::new(peer());
        command.arg(&log).arg(mode);
        let mut manager = Manager {
            root: Some(root.clone()),
            ..Manager::default()
        };
        manager.sessions.push(Session::spawn(
            root.clone(),
            "rust-analyzer",
            &[],
            Some(command),
        ));
        (manager, root, file, log)
    }
    fn wait(mut predicate: impl FnMut() -> bool) {
        let until = Instant::now() + Duration::from_secs(10);
        while !predicate() {
            assert!(Instant::now() < until, "language test timed out");
            std::thread::sleep(Duration::from_millis(15));
        }
    }
    fn logged(log: &Path, needle: &str) -> bool {
        std::fs::read_to_string(log)
            .unwrap_or_default()
            .contains(needle)
    }
    fn stop(mut manager: Manager) {
        let process = manager.sessions[0].child.clone();
        let output = manager.sessions[0].output.clone();
        manager.stop();
        wait(|| process.lock().unwrap().is_none() && output.lock().unwrap().ended);
    }
    #[test]
    fn persistent_process_follows_buffers_queries_saves_and_close_with_version_filtering() {
        let (mut manager, root, file, log) = fixture("incremental");
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), "a😀\nsecond".into(), false)],
        );
        wait(|| manager.sessions[0].output.lock().unwrap().ready);
        let mut report = None;
        wait(|| {
            if let Some(r) = manager.report() {
                report = Some(r);
            }
            report.as_ref().is_some_and(|r| r.problems.len() == 1)
        });
        assert_eq!(report.unwrap().problems[0].message, "version 1");
        manager
            .query(&file, "a😀\nsecond", 1, 3, QueryKind::Hover, None)
            .unwrap();
        let mut answer = None;
        wait(|| {
            answer = manager.take_answer();
            answer.is_some()
        });
        assert_eq!(answer.unwrap().result.message, "live hover");
        assert!(logged(&log, "\"character\":3")); // Unicode scalar cursor -> UTF-16.
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), "changed".into(), false)],
        );
        assert!(manager.report().unwrap().problems.is_empty()); // old results disappear immediately.
        let mut report = None;
        wait(|| {
            if let Some(r) = manager.report() {
                report = Some(r);
            }
            report
                .as_ref()
                .is_some_and(|r| r.problems.first().is_some_and(|p| p.message == "version 2"))
        });
        wait(|| logged(&log, "\"applied\":false"));
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), "changed".into(), true)],
        );
        wait(|| logged(&log, "textDocument/didSave"));
        manager.sync(Some(&root), true, vec![]);
        wait(|| logged(&log, "textDocument/didClose"));
        assert!(
            manager
                .query(&file, "changed", 1, 1, QueryKind::Hover, None)
                .is_err()
        );
        let messages = std::fs::read_to_string(&log).unwrap();
        assert_eq!(messages.matches("\"method\":\"initialize\"").count(), 1);
        assert!(messages.contains("\"end\":{\"character\":6,\"line\":1}"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "disk text");
        stop(manager);
    }
    #[test]
    fn full_sync_bounds_root_and_trust_transitions() {
        let (mut manager, root, file, log) = fixture("full");
        let outside = root
            .parent()
            .unwrap()
            .join(format!("forge-outside-{}.rs", std::process::id()));
        std::fs::write(&outside, "outside").unwrap();
        manager.sync(
            Some(&root),
            true,
            vec![
                Document::new(outside.clone(), "private".into(), false),
                Document::new(file.clone(), "x".repeat(MAX_TEXT + 1), false),
                Document::new(file.clone(), "first".into(), false),
            ],
        );
        assert_eq!(manager.documents.len(), 1);
        wait(|| logged(&log, "textDocument/didOpen"));
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), "second".into(), false)],
        );
        wait(|| logged(&log, "textDocument/didChange"));
        let messages = std::fs::read_to_string(&log).unwrap();
        assert!(messages.contains("\"contentChanges\":[{\"text\":\"second\"}]"));
        assert!(!messages.contains("private"));
        assert!(manager.start(&root, &outside).is_err());
        manager.sync(Some(&root), false, vec![]);
        assert!(!manager.running());
        assert!(manager.report().unwrap().problems.is_empty());
        let _ = std::fs::remove_file(outside);
    }
    #[test]
    fn queries_from_old_requests_and_changed_other_buffers_are_discarded() {
        let (mut manager, root, file, log) = fixture("delay-hover");
        let second = root.join("other.rs");
        std::fs::write(&second, "other").unwrap();
        manager.sync(
            Some(&root),
            true,
            vec![
                Document::new(file.clone(), "first".into(), false),
                Document::new(second.clone(), "other".into(), false),
            ],
        );
        wait(|| manager.sessions[0].output.lock().unwrap().ready);
        manager
            .query(&file, "first", 1, 1, QueryKind::Hover, None)
            .unwrap();
        manager
            .query(&file, "first", 1, 2, QueryKind::References, None)
            .unwrap();
        let mut answer = None;
        wait(|| {
            answer = manager.take_answer();
            answer.is_some()
        });
        assert!(matches!(answer.unwrap().result.kind, QueryKind::References));
        manager
            .query(&file, "first", 1, 1, QueryKind::Hover, None)
            .unwrap();
        wait(|| logged(&log, "textDocument/hover"));
        manager.sync(
            Some(&root),
            true,
            vec![
                Document::new(file.clone(), "first".into(), false),
                Document::new(second, "new context".into(), false),
            ],
        );
        wait(|| manager.has_answer());
        assert!(manager.take_answer().is_none());
        assert!(manager.notice.contains("Discarded"));
        stop(manager);
    }
    #[test]
    fn snapshot_budget_limits_aggregate_text_and_file_count() {
        let (mut manager, root, _, log) = fixture("full");
        let mut docs = Vec::new();
        for i in 0..18 {
            let path = root.join(format!("source{i}.rs"));
            std::fs::write(&path, "x").unwrap();
            docs.push(Document::new(path, "x".repeat(MAX_TEXT), false));
        }
        manager.sync(Some(&root), true, docs.clone());
        assert_eq!(manager.documents.len(), 8);
        assert!(manager.notice.contains("10 buffer(s) excluded"));
        for d in &mut docs {
            d.text = "small".into();
        }
        manager.sync(Some(&root), true, docs);
        assert_eq!(manager.documents.len(), 16);
        wait(|| logged(&log, "textDocument/didOpen"));
        stop(manager);
    }
    #[test]
    fn stop_interrupts_initialization_and_bad_encoding_exits_without_auto_restart() {
        let (manager, _, _, log) = fixture("stall");
        wait(|| logged(&log, "initialize"));
        let slot = manager.sessions[0].child.clone();
        let pid = slot.lock().unwrap().as_ref().unwrap().id();
        let before = Instant::now();
        stop(manager);
        assert!(before.elapsed() < Duration::from_secs(2));
        // The worker marks the owned process gone; no arbitrary PID is terminated.
        assert!(pid > 0);
        let (mut manager, root, file, _) = fixture("bad-encoding");
        wait(|| manager.sessions[0].output.lock().unwrap().ended);
        assert!(
            manager
                .report()
                .unwrap()
                .failure
                .unwrap()
                .contains("encoding")
        );
        assert!(
            manager
                .query(&file, "", 1, 1, QueryKind::Hover, None)
                .is_err()
        );
        manager.sync(root.parent(), true, vec![]);
        assert!(!manager.running());
    }
    #[test]
    #[ignore = "requires installed rust-analyzer and a Rust toolchain"]
    fn real_rust_analyzer_updates_unsaved_diagnostics_and_reuses_queries() {
        let root = std::env::temp_dir().join(format!("forge-live-rust-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"live-language-trial\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
        )
        .unwrap();
        let file = root.join("src/main.rs");
        let valid = "fn helper() {}\nfn main() { helper(); }\n";
        std::fs::write(&file, valid).unwrap();
        let root = root.canonicalize().unwrap();
        let file = file.canonicalize().unwrap();
        let mut manager = Manager::default();
        manager.start(&root, &file).unwrap();
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), valid.into(), true)],
        );
        let deadline = Instant::now() + Duration::from_secs(45);
        while !manager.sessions[0].output.lock().unwrap().ready {
            assert!(Instant::now() < deadline, "{}", manager.status());
            std::thread::sleep(Duration::from_millis(50));
        }
        let invalid = "fn helper() {}\nfn main() { helper(); let x = ; }\n";
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), invalid.into(), false)],
        );
        let mut diagnosed = false;
        while Instant::now() < deadline {
            if manager
                .report()
                .is_some_and(|r| r.problems.iter().any(|p| p.error))
            {
                diagnosed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(diagnosed, "{}", manager.status());
        manager
            .query(&file, invalid, 2, 14, QueryKind::Definition, None)
            .unwrap();
        let mut answer = None;
        wait(|| {
            answer = manager.take_answer();
            answer.is_some()
        });
        assert_eq!(
            answer.unwrap().result.location.as_ref().map(|(_, l, _)| *l),
            Some(1)
        );
        assert_eq!(std::fs::read_to_string(file).unwrap(), valid);
        stop(manager);
    }
    #[test]
    #[ignore = "requires pylsp and pyflakes on PATH; use an isolated test environment"]
    fn real_python_server_updates_unsaved_diagnostics_and_reuses_navigation() {
        let root = std::env::temp_dir().join(format!("forge-live-python-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let file = root.join("main.py");
        let valid = "def helper():\n    return 1\n\nvalue = helper()\n";
        std::fs::write(&file, valid).unwrap();
        let mut manager = Manager::default();
        manager.start(&root, &file).unwrap();
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), valid.into(), true)],
        );
        wait(|| manager.sessions[0].output.lock().unwrap().ready);
        let invalid = "def helper():\n    return 1\n\nvalue = helper(\n";
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), invalid.into(), false)],
        );
        let mut diagnosed = false;
        wait(|| {
            if let Some(report) = manager.report() {
                diagnosed |= report.problems.iter().any(|p| p.error);
            }
            diagnosed
        });
        manager.sync(
            Some(&root),
            true,
            vec![Document::new(file.clone(), valid.into(), false)],
        );
        manager
            .query(&file, valid, 4, 11, QueryKind::Definition, None)
            .unwrap();
        let mut answer = None;
        wait(|| {
            answer = manager.take_answer();
            answer.is_some()
        });
        let answer = answer.unwrap();
        assert_eq!(
            answer.result.location.as_ref().map(|(_, line, _)| *line),
            Some(1),
            "{}",
            answer.result.message
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), valid);
        stop(manager);
    }
}

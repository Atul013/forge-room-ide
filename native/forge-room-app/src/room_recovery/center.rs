use super::storage::{Lease, Reservation, Store};
use crate::background;
use eframe::egui;
use forge_room_core::transport::{
    Host, RecoveryComparison, RecoveryFileStamp, RecoveryKind, RecoveryPreview, RoomLifecycle,
    inspect_recovery, recovery_file_stamp,
};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    path::{Path, PathBuf},
};
mod view;
pub struct Restored {
    pub host: Option<Host>,
    pub lease: Option<Lease>,
    pub journal: Option<PathBuf>,
    pub notice: String,
}
impl Drop for Restored {
    fn drop(&mut self) {
        if let Some(host) = self.host.take() {
            let lease = self.lease.take();
            std::thread::spawn(move || {
                drop(host);
                drop(lease);
            });
        }
    }
}
pub struct Creation {
    pub reservation: Reservation,
    pub room: String,
    pub project: Option<String>,
}
struct Inspection {
    result: Result<RecoveryPreview, String>,
    stamp: Option<RecoveryFileStamp>,
    store: Option<Box<Store>>,
    warning: String,
}
enum Job {
    Loaded(Result<Box<Store>, String>),
    Inspected(Box<Inspection>),
    Compared(Result<Box<(RecoveryPreview, RecoveryComparison)>, String>),
    Reserved(Result<Box<Creation>, String>),
    Restored(Result<Box<Restored>, String>),
    Updated(Result<(Box<Store>, String, Option<PathBuf>), String>),
    Discovered(Result<Vec<PathBuf>, String>),
    Exported(Result<String, String>),
}
fn catalog_at(root: Option<&Path>) -> Result<Store, String> {
    match root {
        Some(root) => Store::open_at(root),
        None => Store::open(),
    }
}
fn load(root: Option<PathBuf>) -> Job {
    Job::Loaded(catalog_at(root.as_deref()).and_then(|mut store| {
        store.reload()?;
        Ok(Box::new(store))
    }))
}
#[derive(Default)]
pub struct State {
    pub open: bool,
    pub ready: Option<Box<Restored>>,
    pub create: Option<Box<Creation>>,
    store: Option<Box<Store>>,
    job: background::Latest<Job>,
    started: bool,
    selected: String,
    page: usize,
    query: String,
    filter: usize,
    preview: Option<RecoveryPreview>,
    stamp: Option<RecoveryFileStamp>,
    source_path: Option<PathBuf>,
    source_error: String,
    notice: String,
    comparison: Option<Box<(RecoveryPreview, RecoveryComparison)>>,
    compare_path: String,
    restore: bool,
    restore_bind: String,
    fresh: bool,
    confirm: Option<(String, String, String, String)>,
    pending: VecDeque<(PathBuf, Option<String>)>,
    discovered: Vec<PathBuf>,
    project: Option<String>,
    include_path: bool,
    acknowledge_tail: bool,
    confirm_delete: bool,
    associate_project: bool,
    discard_restore: bool,
    catalog_root: Option<PathBuf>,
    quiet_reload: bool,
}
fn inspect(path: PathBuf, project: Option<String>, catalog_root: Option<PathBuf>) -> Job {
    let result = inspect_recovery(&path);
    let stamp = match &result {
        Ok(preview) => Some(preview.file_stamp()),
        Err(_) => recovery_file_stamp(&path).ok(),
    };
    let mut warning = String::new();
    let store = match catalog_at(catalog_root.as_deref()) {
        Ok(mut store) => {
            let update = match &result {
                Ok(preview) => store.register(preview, project),
                Err(error) => match &stamp {
                    Some(stamp) => store.track_problem(stamp, error.clone(), project),
                    None => {
                        let id = Store::id(&path);
                        if store.index.entries.contains_key(&id) {
                            store.issue(&id, error.clone())
                        } else {
                            Ok(())
                        }
                    }
                },
            };
            if let Err(error) = update {
                warning = format!("Inspection finished; catalog was not updated: {error}");
            }
            Some(Box::new(store))
        }
        Err(error) => {
            warning = format!("File inspection is available; private catalog is closed: {error}");
            None
        }
    };
    Job::Inspected(Box::new(Inspection {
        result,
        stamp,
        store,
        warning,
    }))
}
fn restored(
    preview: RecoveryPreview,
    bind: String,
    fresh: bool,
    project: Option<String>,
    catalog_root: Option<PathBuf>,
) -> Job {
    Job::Restored((|| {
        let bind: SocketAddr = bind
            .parse()
            .map_err(|_| "Enter a specific loopback or LAN IP:port")?;
        // A managed source must not be a currently hosted journal in another native window.
        let _source_lease = Store::lease_for(catalog_root.as_deref(), preview.source())?;
        let mut store = catalog_at(catalog_root.as_deref()).ok();
        preview.recheck()?;
        let reservation = if fresh {
            Some(store.as_mut().ok_or("Open an intact private catalog or turn off managed journaling and choose an external journal afterward")?.reserve(RecoveryKind::Journal,&preview.summary().room,project.clone())?)
        } else {
            None
        };
        let host = Host::recover_preview(bind, &preview)?;
        let mut notice="Recovered public Room paused; archived Rooms stay archived. New invitations/owner claims are required.".to_string();
        let (lease, journal) = if let Some(reservation) = reservation {
            host.enable_journal(&reservation.path)?;
            let new_preview = inspect_recovery(&reservation.path)?;
            if let Some(store) = store.as_mut()
                && let Err(error) = store.register(&new_preview, project)
            {
                notice.push_str(&format!(
                    " New journal is durable; catalog update failed: {error}"
                ));
            }
            (Some(reservation.lease), Some(reservation.path))
        } else {
            if preview.summary().kind == RecoveryKind::Journal {
                notice.push_str(" Start a fresh journal before resuming.");
            }
            (None, None)
        };
        Ok(Box::new(Restored {
            host: Some(host),
            lease,
            journal,
            notice,
        }))
    })())
}
impl State {
    pub fn inspect_selected(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.open = true;
        self.request_inspect(ctx, path);
    }
    pub fn busy(&self) -> bool {
        self.job.busy() || self.create.is_some() || self.ready.is_some()
    }
    pub fn tick(&mut self, ctx: &egui::Context, project: Option<&Path>) {
        self.project = project.map(|path| path.to_string_lossy().into_owned());
        if !self.started && (self.open || !self.pending.is_empty()) {
            self.started = true;
            self.request_reload(ctx);
        }
        if let Some(job) = self.job.poll() {
            match job {
                Job::Loaded(result) => match result {
                    Ok(store) => {
                        self.store = Some(store);
                        if !self.quiet_reload {
                            self.notice="Private recovery catalog loaded. Cached summaries are not a fresh inspection.".into();
                        }
                        self.quiet_reload = false;
                    }
                    Err(error) => {
                        self.quiet_reload = false;
                        self.store = None;
                        self.notice = format!("Catalog closed; source files preserved: {error}");
                    }
                },
                Job::Inspected(inspection) => {
                    let Inspection {
                        result,
                        stamp,
                        store,
                        warning,
                    } = *inspection;
                    self.store = store;
                    if let Some(stamp) = &stamp {
                        self.selected = Store::id(stamp.source());
                        self.source_path = Some(stamp.source().into());
                    }
                    self.stamp = stamp;
                    self.comparison = None;
                    self.restore = false;
                    match result {
                        Ok(preview) => {
                            self.selected = Store::id(preview.source());
                            self.preview = Some(preview);
                            self.source_error.clear();
                            self.notice="Source inspected without starting a host. Review the saved public state before restoring.".into();
                        }
                        Err(error) => {
                            self.preview = None;
                            self.source_error = error;
                            self.notice="Source is unavailable or invalid. Original bytes were not changed.".into();
                        }
                    }
                    if !warning.is_empty() {
                        self.notice.push_str(&format!(" {warning}"));
                    }
                    self.page = 1;
                }
                Job::Compared(result) => match result {
                    Ok(value) => self.comparison = Some(value),
                    Err(error) => self.notice = error,
                },
                Job::Reserved(result) => match result {
                    Ok(creation) => self.create = Some(creation),
                    Err(error) => self.notice = error,
                },
                Job::Restored(result) => match result {
                    Ok(restored) => {
                        if self.discard_restore {
                            drop(restored);
                            self.notice="Restore adoption cancelled; temporary host stopped. Original source and any new managed copy remain preserved".into();
                        } else {
                            self.ready = Some(restored);
                        }
                        self.restore = false;
                        self.request_reload(ctx);
                        self.quiet_reload = true;
                    }
                    Err(error) => self.notice = error,
                },
                Job::Updated(result) => match result {
                    Ok((store, notice, path)) => {
                        self.store = Some(store);
                        self.notice = notice;
                        if let Some(path) = path {
                            self.pending.push_front((path, self.project.clone()));
                        }
                    }
                    Err(error) => self.notice = error,
                },
                Job::Discovered(result) => match result {
                    Ok(paths) => {
                        self.discovered = paths;
                        self.notice = format!(
                            "{} unindexed owned recovery files found; inspect/register explicitly",
                            self.discovered.len()
                        );
                    }
                    Err(error) => self.notice = error,
                },
                Job::Exported(result) => self.notice = result.unwrap_or_else(|error| error),
            }
        }
        if self.job.take_failed() {
            self.notice="Recovery worker ended without a result; no automatic restore or source overwrite occurred".into();
        }
        if !self.job.busy()
            && self.create.is_none()
            && self.ready.is_none()
            && let Some((path, project)) = self.pending.pop_front()
        {
            let root = self.catalog_root.clone();
            self.started = true;
            self.job.request(ctx, move || inspect(path, project, root));
        }
    }
    pub fn remember(&mut self, ctx: &egui::Context, path: PathBuf, project: Option<&Path>) {
        if self.pending.len() >= 16 {
            self.notice="Saved source remains durable, but the catalog registration queue is full; inspect the file later".into();
            return;
        }
        self.pending.push_back((
            path,
            project.map(|path| path.to_string_lossy().into_owned()),
        ));
        ctx.request_repaint();
    }
    fn request_inspect(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.busy() {
            self.notice =
                "Wait for the current recovery operation before selecting another source".into();
            return;
        }
        self.selected = Store::id(&path);
        self.source_path = Some(path.clone());
        let project = self
            .associate_project
            .then(|| self.project.clone())
            .flatten();
        let root = self.catalog_root.clone();
        self.started = true;
        self.job.request(ctx, move || inspect(path, project, root));
    }
    pub fn reserve_managed(
        &mut self,
        ctx: &egui::Context,
        kind: RecoveryKind,
        room: String,
        label: String,
        project: Option<&Path>,
    ) {
        if self.busy() {
            self.notice = "Finish the current recovery operation first".into();
            return;
        }
        let project = project.map(|path| path.to_string_lossy().into_owned());
        let root = self.catalog_root.clone();
        self.job.request(ctx, move || {
            Job::Reserved((|| {
                let reservation =
                    catalog_at(root.as_deref())?.reserve(kind, &label, project.clone())?;
                Ok(Box::new(Creation {
                    reservation,
                    room,
                    project,
                }))
            })())
        });
    }
    pub fn external_destination(&self, path: &Path) -> Result<(), String> {
        if Store::owned_directory(self.catalog_root.as_deref(), path)? {
            return Err("Use the managed-vault controls for private recovery storage; choose an external destination here".into());
        }
        Ok(())
    }
    fn request_reload(&mut self, ctx: &egui::Context) {
        let root = self.catalog_root.clone();
        self.job.request(ctx, move || load(root));
    }
    #[cfg(test)]
    pub fn at(root: &Path) -> Self {
        Self {
            catalog_root: Some(root.into()),
            ..Self::default()
        }
    }
    pub fn request_restore(
        &mut self,
        ctx: &egui::Context,
        preview: RecoveryPreview,
        bind: String,
        fresh: bool,
    ) {
        if self.busy() {
            self.notice = "Finish the current recovery operation before restoring".into();
            return;
        }
        self.discard_restore = false;
        let project = self.project.clone();
        let root = self.catalog_root.clone();
        self.job
            .request(ctx, move || restored(preview, bind, fresh, project, root));
        self.notice =
            "Restoring exactly reviewed state on a worker; no automatic resume or owner claim"
                .into();
    }
    pub fn operation_notice(&mut self, notice: String) {
        self.notice = notice;
    }
    fn managed_copy(&mut self, ctx: &egui::Context, preview: RecoveryPreview) {
        if self.busy() {
            return;
        }
        let root = self.catalog_root.clone();
        let project = self.project.clone();
        self.job.request(ctx,move||Job::Updated((||{preview.recheck()?;let mut store=catalog_at(root.as_deref())?;let reservation=store.reserve(RecoveryKind::Checkpoint,&preview.summary().room,project.clone())?;preview.export_checkpoint(&reservation.path)?;let copy=inspect_recovery(&reservation.path)?;store.register(&copy,project)?;let path=reservation.path;drop(reservation.lease);Ok((Box::new(store),"Sanitized checkpoint copied into the managed vault; original source unchanged, no host started".into(),Some(path)))})()));
    }
    fn update(
        &mut self,
        ctx: &egui::Context,
        kind: String,
        id: String,
        seal: String,
        digest: String,
    ) {
        if self.busy() {
            return;
        }
        let root = self.catalog_root.clone();
        self.job.request(ctx, move || {
            Job::Updated((|| {
                let mut store = catalog_at(root.as_deref())?;
                let mut path = None;
                let notice = match kind.as_str() {
                    "pin" => {
                        store.pin(&id, true, &seal)?;
                        "Recovery source pinned"
                    }
                    "unpin" => {
                        store.pin(&id, false, &seal)?;
                        "Recovery source unpinned"
                    }
                    "forget" => {
                        store.forget(&id, &seal)?;
                        "Catalog entry forgotten; original files retained"
                    }
                    "quarantine" => {
                        path = Some(store.quarantine(&id, &seal, &digest)?);
                        "Inactive managed source preserved in private quarantine"
                    }
                    "purge" => {
                        store.purge_quarantine(&id, &seal, &digest)?;
                        "Exactly reviewed inactive owned quarantine file removed"
                    }
                    _ => return Err("Unknown recovery catalog action".into()),
                };
                Ok((Box::new(store), notice.into(), path))
            })())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, Instant},
    };
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "forge-recovery-center-{}",
                forge_room_core::workflows::new_id().unwrap()
            ));
            fs::create_dir(&root).unwrap();
            Self(root.canonicalize().unwrap())
        }
        fn catalog(&self) -> PathBuf {
            self.0.join("catalog")
        }
        fn checkpoint(&self) -> PathBuf {
            let host = Host::start("127.0.0.1:0".parse().unwrap(), "Center trial").unwrap();
            let path = self.0.join("source.json");
            host.save_checkpoint(&path).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert!(
                self.0
                    .starts_with(std::env::temp_dir().canonicalize().unwrap())
            );
            assert!(
                self.0
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-recovery-center-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn drive(state: &mut State, predicate: impl Fn(&State) -> bool) {
        let context = egui::Context::default();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !predicate(state) {
            state.tick(&context, None);
            assert!(
                Instant::now() < deadline,
                "Recovery center did not settle: {}",
                state.notice
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn asynchronous_inspection_never_hosts_and_restore_explicitly_starts_paused_with_fresh_managed_durability()
     {
        let fixture = Fixture::new();
        let source = fixture.checkpoint();
        let original = fs::read(&source).unwrap();
        let mut state = State::at(&fixture.catalog());
        state.inspect_selected(&egui::Context::default(), source.clone());
        drive(&mut state, |s| s.preview.is_some() && !s.job.busy());
        assert!(state.ready.is_none());
        assert!(state.create.is_none());
        assert_eq!(state.store.as_ref().unwrap().index.entries.len(), 1);
        let preview = state.preview.clone().unwrap();
        state.request_restore(
            &egui::Context::default(),
            preview,
            "127.0.0.1:0".into(),
            true,
        );
        drive(&mut state, |s| s.ready.is_some());
        let ready = state.ready.take().unwrap();
        let host = ready.host.as_ref().unwrap();
        assert_eq!(host.lifecycle().unwrap(), RoomLifecycle::Paused);
        assert!(host.journal_bytes().unwrap().unwrap() > 5);
        let journal = ready.journal.clone().unwrap();
        assert!(Store::lease_for(Some(&fixture.catalog()), &journal).is_err());
        assert_eq!(fs::read(&source).unwrap(), original);
        drop(ready);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Store::lease_for(Some(&fixture.catalog()), &journal).is_err() {
            assert!(
                Instant::now() < deadline,
                "Closed host retained its managed lease"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        drive(&mut state, |s| !s.job.busy());
        assert_eq!(state.store.as_ref().unwrap().index.entries.len(), 2);
        assert!(journal.exists());
        assert_eq!(fs::read(source).unwrap(), original);
    }
    #[test]
    fn cancelled_adoption_and_changed_sources_do_not_leave_a_ready_host_or_overwrite_the_source() {
        let fixture = Fixture::new();
        let source = fixture.checkpoint();
        let mut state = State::at(&fixture.catalog());
        state.inspect_selected(&egui::Context::default(), source.clone());
        drive(&mut state, |s| s.preview.is_some() && !s.job.busy());
        let preview = state.preview.clone().unwrap();
        state.request_restore(
            &egui::Context::default(),
            preview.clone(),
            "127.0.0.1:0".into(),
            false,
        );
        state.discard_restore = true;
        drive(&mut state, |s| !s.job.busy());
        assert!(state.ready.is_none());
        assert!(state.notice.contains("cancelled"));
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        value["snapshot"]["server_time_epoch_seconds"] = serde_json::json!(1);
        fs::write(&source, serde_json::to_vec(&value).unwrap()).unwrap();
        let changed = fs::read(&source).unwrap();
        state.request_restore(
            &egui::Context::default(),
            preview,
            "127.0.0.1:0".into(),
            true,
        );
        drive(&mut state, |s| !s.job.busy());
        assert!(state.ready.is_none());
        assert!(state.notice.contains("changed"));
        assert_eq!(fs::read(&source).unwrap(), changed);
    }
    #[test]
    fn corruption_closes_only_the_catalog_while_selected_file_inspection_and_unjournaled_checkpoint_restore_still_work()
     {
        let fixture = Fixture::new();
        let source = fixture.checkpoint();
        let mut store = Store::open_at(&fixture.catalog()).unwrap();
        let reservation = store
            .reserve(RecoveryKind::Checkpoint, "reserved", None)
            .unwrap();
        drop(reservation);
        let statefile = fs::read_dir(store.root.join("states"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(statefile, b"FRC1\n{}\nDONE\n").unwrap();
        let inspection = inspect(source.clone(), None, Some(fixture.catalog()));
        let Job::Inspected(inspection) = inspection else {
            panic!("inspection job missing")
        };
        assert!(inspection.result.is_ok());
        assert!(inspection.store.is_none());
        assert!(inspection.warning.contains("catalog is closed"));
        let Job::Restored(result) = restored(
            inspection.result.unwrap(),
            "127.0.0.1:0".into(),
            false,
            None,
            Some(fixture.catalog()),
        ) else {
            panic!("restore job missing")
        };
        let host = result.unwrap();
        assert_eq!(
            host.host.as_ref().unwrap().lifecycle().unwrap(),
            RoomLifecycle::Paused
        );
        drop(host);
    }
    #[test]
    fn all_recovery_pages_and_restore_management_reviews_build_without_claiming_desktop_acceptance()
    {
        let fixture = Fixture::new();
        let source = fixture.checkpoint();
        let preview = inspect_recovery(&source).unwrap();
        let mut store = Store::open_at(&fixture.catalog()).unwrap();
        store.register(&preview, None).unwrap();
        let mut state = State {
            open: true,
            started: true,
            selected: Store::id(preview.source()),
            stamp: Some(preview.file_stamp()),
            preview: Some(preview.clone()),
            store: Some(Box::new(store)),
            ..State::at(&fixture.catalog())
        };
        let context = egui::Context::default();
        for width in [1280.0, 700.0] {
            for page in 0..3 {
                state.page = page;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ui| state.show(ui.ctx(), true, "127.0.0.1:0", None),
                );
                output.textures_delta.clear();
                assert!(!output.shapes.is_empty());
            }
        }
        state.restore = true;
        state.restore_bind = "127.0.0.1:0".into();
        state.acknowledge_tail = true;
        let mut output = context.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), true, "127.0.0.1:0", None)
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert!(state.ready.is_none());
        assert!(!state.job.busy());
        state.restore = false;
        state.confirm = Some((
            "forget".into(),
            state.selected.clone(),
            state.store.as_ref().unwrap().index.seal().unwrap(),
            String::new(),
        ));
        let mut output = context.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), true, "127.0.0.1:0", None)
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert!(source.exists());
    }
}

#[cfg(test)]
mod copy_tests {
    use super::*;
    #[test]
    fn sanitized_vault_copy_stays_non_hosting_and_manual_destinations_cannot_bypass_the_private_namespace()
     {
        let root = std::env::temp_dir().join(format!(
            "forge-recovery-copy-{}",
            forge_room_core::workflows::new_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let catalog = root.join("catalog");
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Copy trial").unwrap();
        let source = root.join("source.json");
        host.save_checkpoint(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let preview = inspect_recovery(&source).unwrap();
        let mut state = State::at(&catalog);
        let context = egui::Context::default();
        state.managed_copy(&context, preview);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while state.job.busy() || !state.pending.is_empty() {
            state.tick(&context, None);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(state.ready.is_none());
        assert!(state.preview.is_some());
        let copied = state.preview.as_ref().unwrap().source();
        assert!(copied.starts_with(catalog.canonicalize().unwrap().join("vault")));
        assert_ne!(copied, source);
        assert!(
            state
                .external_destination(&catalog.join("vault/new.json"))
                .is_err()
        );
        assert!(
            state
                .external_destination(&catalog.join("states/new.state"))
                .is_err()
        );
        assert!(
            state
                .external_destination(&root.join("outside.json"))
                .is_ok()
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("forge-recovery-copy-")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

mod storage;
mod view;
use crate::background::Latest;
use forge_room_core::transport::{Client, Host, Invite, RevocationReview, Snapshot};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
};
use storage::{Edit, Profile, Profiles, Store};

pub struct Joined {
    pub client: Option<Client>,
    pub snapshot: Snapshot,
    profile: Profile,
    project: Option<String>,
    expected: Option<Profile>,
    requested_name: String,
}
impl Drop for Joined {
    fn drop(&mut self) {
        if let Some(mut client) = self.client.take() {
            std::thread::spawn(move || {
                let _ = client.leave();
            });
        }
    }
}
struct Review {
    invite: Invite,
    name: String,
    project: Option<String>,
    expected: Option<Profile>,
}
enum Job {
    Catalog(Result<Store, String>),
    Profiles(Result<Profiles, String>),
    Export(Result<(), String>),
    Joined(Result<Box<Joined>, String>),
}
pub struct State {
    pub open: bool,
    pub detach_member: bool,
    pub ready: Option<Box<Joined>>,
    worker: Latest<Job>,
    store: Option<Store>,
    loaded: bool,
    invite: String,
    name: String,
    show_invite: bool,
    review: Option<Review>,
    pending: Option<Box<Joined>>,
    joining: bool,
    cancelled: bool,
    page: usize,
    selected: String,
    label: String,
    query: String,
    notice: String,
    import: Option<(Profiles, Option<String>)>,
    forget: Option<(String, String)>,
    allow_changed: bool,
    lifetime: u64,
    grant_selection: BTreeSet<String>,
    revocation: Option<RevocationReview>,
    fresh_invite: Option<Invite>,
    catalog_root: Option<PathBuf>,
    last_joined: Option<(Profile, Option<String>, u64)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            detach_member: false,
            ready: None,
            worker: Default::default(),
            store: None,
            loaded: false,
            invite: String::new(),
            name: crate::room_state::local_user_name(),
            show_invite: false,
            review: None,
            pending: None,
            joining: false,
            cancelled: false,
            page: 0,
            selected: String::new(),
            label: String::new(),
            query: String::new(),
            notice: String::new(),
            import: None,
            forget: None,
            allow_changed: false,
            lifetime: 300,
            grant_selection: Default::default(),
            revocation: None,
            fresh_invite: None,
            catalog_root: None,
            last_joined: None,
        }
    }
}
fn open(root: Option<PathBuf>) -> Result<Store, String> {
    Store::open(&root.map(Ok).unwrap_or_else(Store::default_root)?)
}
fn update(root: Option<PathBuf>, expected: Option<String>, edit: Edit) -> Job {
    Job::Catalog(open(root).and_then(|mut s| {
        s.edit(expected.as_deref(), edit)?;
        Ok(s)
    }))
}
fn profiles(path: &Path) -> Result<Profiles, String> {
    crate::local_records::file(path)?;
    let mut raw = vec![];
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(storage::MAX_PROFILE as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    Profiles::parse(&raw)
}
impl State {
    pub fn open_join(&mut self) {
        self.open = true;
        self.page = 0;
    }
    pub fn open_host(&mut self) {
        self.open = true;
        self.page = 2;
    }
    pub fn operation_notice(&mut self, text: String) {
        self.notice = text;
    }
    pub fn busy(&self) -> bool {
        self.worker.busy() || self.pending.is_some() || self.ready.is_some()
    }
    fn expected(&self) -> Option<Profile> {
        self.store
            .as_ref()?
            .index
            .entries
            .iter()
            .find(|e| e.profile.id().ok().as_deref() == Some(&self.selected))
            .map(|e| e.profile.clone())
    }
    fn catalog_edit(&mut self, ctx: &eframe::egui::Context, edit: Edit) {
        if self.worker.busy() {
            return;
        }
        let Some(seal) = self.store.as_ref().and_then(|s| s.index.seal().ok()) else {
            self.notice = "Catalog unavailable; refresh before editing metadata".into();
            return;
        };
        let root = self.catalog_root.clone();
        self.worker
            .request(ctx, move || update(root, Some(seal), edit));
    }
    pub fn remember(&mut self, ctx: &eframe::egui::Context, joined: &Joined) {
        self.last_joined = Some((
            joined.profile.clone(),
            joined.project.clone(),
            forge_room_core::now_epoch_seconds(),
        ));
        self.save_last_joined(ctx);
    }
    fn save_last_joined(&mut self, ctx: &eframe::egui::Context) {
        if self.worker.busy() {
            return;
        }
        if let Some((profile, project, at)) = self.last_joined.clone() {
            let root = self.catalog_root.clone();
            self.worker.request(ctx, move || {
                update(root, None, Edit::Visited(profile, project, at))
            });
        }
    }
    pub fn tick(&mut self) {
        if let Some(job) = self.worker.poll() {
            match job {
            Job::Catalog(result)=>match result{Ok(store)=>{self.notice=if store.warning.is_empty(){"Connection metadata refreshed; saved entries do not prove a host is online".into()}else{store.warning.clone()};self.store=Some(store);},Err(e)=>{self.notice=format!("Connection metadata unavailable: {e}. Private invite joining remains available.");self.store=None;}},
            Job::Profiles(result)=>match result{Ok(value)=>{let seal=self.store.as_ref().and_then(|s|s.index.seal().ok());self.import=Some((value,seal));self.notice="Import inspected; review the public host metadata before adding it".into();},Err(e)=>self.notice=e},
            Job::Export(result)=>self.notice=match result{Ok(())=>"Public bookmark metadata exported to a new file; no project paths or authentication included".into(),Err(e)=>e},
            Job::Joined(result)=>{self.joining=false;match result{Err(e)=>self.notice=format!("Join failed: {e}"),Ok(joined)=>{
                if self.cancelled{drop(joined);self.notice="Join adoption cancelled; completed connections are closed with leave on a worker".into();}
                else if joined.expected.as_ref().is_some_and(|p|p.room_id!=joined.profile.room_id) || joined.snapshot.shared.tasks.iter().any(|t|t.owner_id==joined.snapshot.you) || joined.snapshot.members.iter().find(|m|m.id==joined.snapshot.you).is_some_and(|m|m.name!=joined.requested_name){self.pending=Some(joined);self.notice="The joined Room/member identity or existing task ownership requires review before local adoption".into();}
                else{self.ready=Some(joined);}
            }}self.cancelled=false;}
        }
        }
        if self.worker.take_failed() {
            self.joining = false;
            self.review = None;
            self.notice =
                "Access worker stopped without a result; no connection was adopted".into();
        }
    }
    fn freeze(&mut self, project: Option<&Path>) -> Result<(), String> {
        let invite = Invite::decode(&self.invite)?;
        if self.name.trim().is_empty()
            || self.name.len() > 64
            || self.name.chars().any(char::is_control)
        {
            return Err(
                "Use a non-empty display name up to 64 bytes without control characters".into(),
            );
        }
        if !self.selected.is_empty() && self.expected().is_none() {
            return Err("Selected bookmark is unavailable; refresh or explicitly clear the selection before reviewing another host".into());
        }
        self.review = Some(Review {
            invite,
            name: self.name.trim().into(),
            project: project.map(|p| p.to_string_lossy().into_owned()),
            expected: self.expected(),
        });
        self.allow_changed = false;
        Ok(())
    }
    fn connect(&mut self, ctx: &eframe::egui::Context) {
        let Some(review) = self.review.take() else {
            return;
        };
        self.invite.clear();
        self.joining = true;
        self.cancelled = false;
        self.worker.request(ctx, move || {
            Job::Joined((|| {
                let address = review.invite.address();
                let fingerprint = review.invite.certificate_fingerprint();
                let (client, snapshot) = Client::join(review.invite, &review.name)?;
                let profile = Profile {
                    room: snapshot.room.clone(),
                    room_id: snapshot.shared.room_id.clone(),
                    address,
                    fingerprint,
                    label: String::new(),
                };
                let joined = Box::new(Joined {
                    client: Some(client),
                    snapshot,
                    profile,
                    project: review.project,
                    expected: review.expected,
                    requested_name: review.name,
                });
                joined.profile.validate()?;
                Ok(joined)
            })())
        });
    }
    pub fn show(
        &mut self,
        ctx: &eframe::egui::Context,
        offline: bool,
        host: Option<&Host>,
        project: Option<&Path>,
    ) {
        self.view(ctx, offline, host, project);
    }
    #[cfg(test)]
    pub(crate) fn at(root: PathBuf) -> Self {
        Self {
            catalog_root: Some(root),
            ..Default::default()
        }
    }
    #[cfg(test)]
    pub(crate) fn join_for_test(
        &mut self,
        ctx: &eframe::egui::Context,
        invite: Invite,
        name: &str,
    ) {
        self.invite = invite.encode();
        self.name = name.into();
        self.freeze(None).unwrap();
        self.connect(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "forge-access-ui-{}",
                forge_room_core::workflows::new_id().unwrap()
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let root = self.0.canonicalize().unwrap();
            assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
            assert!(
                root.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("forge-access-ui-")
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    fn finish(state: &mut State) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while state.worker.busy() {
            state.tick();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn successful_join_metadata_never_serializes_private_invites_or_member_sessions() {
        let fixture = Fixture::new();
        let ctx = eframe::egui::Context::default();
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        let invite = host.invite(Duration::from_secs(60)).unwrap();
        let secret = serde_json::from_str::<serde_json::Value>(&invite.encode()).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();
        let mut state = State::at(fixture.0.clone());
        state.loaded = true;
        state.name = "Ada".into();
        state.invite = invite.encode();
        state.freeze(Some(Path::new("C:/private-project"))).unwrap();
        state.connect(&ctx);
        assert!(state.invite.is_empty());
        finish(&mut state);
        let joined = state.ready.take().expect("joined");
        let identity = joined.snapshot.you.clone();
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(fixture.0.join(".writer"))
            .unwrap();
        lock.try_lock().unwrap();
        state.remember(&ctx, &joined);
        finish(&mut state);
        assert!(state.store.is_none());
        assert!(
            host.snapshot()
                .unwrap()
                .members
                .iter()
                .any(|m| m.id == identity)
        );
        drop(lock);
        state.save_last_joined(&ctx);
        finish(&mut state);
        let store = state.store.as_ref().unwrap();
        assert_eq!(store.index.entries.len(), 1);
        assert!(store.index.entries[0].joined_at.is_some());
        let raw = serde_json::to_string(&store.index).unwrap();
        assert!(!raw.contains(&secret));
        assert!(!raw.contains(&identity));
        let exported = String::from_utf8(store.index.public().encode().unwrap()).unwrap();
        assert!(!exported.contains("private-project"));
        assert!(!exported.contains("certificate\""));
        drop(joined);
    }
    #[test]
    fn cancellation_leaves_and_mismatched_room_identity_requires_post_join_review() {
        let fixture = Fixture::new();
        let ctx = eframe::egui::Context::default();
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        let mut state = State::at(fixture.0.clone());
        state.loaded = true;
        state.name = "Ada".into();
        state.invite = host.invite(Duration::from_secs(60)).unwrap().encode();
        state.freeze(None).unwrap();
        state.connect(&ctx);
        state.cancelled = true;
        finish(&mut state);
        assert!(state.ready.is_none() && state.pending.is_none());
        let deadline = Instant::now() + Duration::from_secs(15);
        while !host.snapshot().unwrap().members.is_empty() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        let invite = host.invite(Duration::from_secs(60)).unwrap();
        let profile = Profile {
            room: "Access".into(),
            room_id: "f".repeat(64),
            address: host.address(),
            fingerprint: invite.certificate_fingerprint(),
            label: "Old public identity".into(),
        };
        let mut store = Store::open(&fixture.0).unwrap();
        store
            .edit(
                None,
                Edit::Import(Profiles {
                    version: 1,
                    entries: vec![profile.clone()],
                }),
            )
            .unwrap();
        state.store = Some(store);
        state.selected = profile.id().unwrap();
        state.invite = invite.encode();
        state.freeze(None).unwrap();
        state.connect(&ctx);
        finish(&mut state);
        assert!(state.ready.is_none());
        assert!(state.pending.is_some());
        state.open = true;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), true, None, None)
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        state.open = false;
        state.pending.take();
        state.store = None;
        state.invite = host.invite(Duration::from_secs(60)).unwrap().encode();
        assert!(state.freeze(None).is_err());
    }
    #[test]
    fn native_access_pages_and_frozen_reviews_render_without_surface_acceptance() {
        let fixture = Fixture::new();
        let ctx = eframe::egui::Context::default();
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        let invite = host.invite(Duration::from_secs(60)).unwrap();
        let mut state = State::at(fixture.0.clone());
        state.open = true;
        state.loaded = true;
        let mut store = Store::open(&fixture.0).unwrap();
        let profile = Profile {
            room: "Room with a long public name for narrow-window inspection".into(),
            room_id: "a".repeat(64),
            address: host.address(),
            fingerprint: invite.certificate_fingerprint(),
            label: "Long bookmark label ".repeat(8),
        };
        state.selected = profile.id().unwrap();
        state.label = profile.label.clone();
        store
            .edit(
                None,
                Edit::Import(Profiles {
                    version: 1,
                    entries: vec![profile],
                }),
            )
            .unwrap();
        state.store = Some(store);
        for width in [1280.0, 650.0] {
            for page in 0..3 {
                state.page = page;
                let mut output = ctx.run_ui(
                    eframe::egui::RawInput {
                        screen_rect: Some(eframe::egui::Rect::from_min_size(
                            eframe::egui::Pos2::ZERO,
                            eframe::egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ui| state.show(ui.ctx(), true, Some(&host), None),
                );
                output.textures_delta.clear();
                assert!(!output.shapes.is_empty());
            }
        }
        state.page = 0;
        state.invite = invite.encode();
        state.freeze(None).unwrap();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), true, Some(&host), None)
        });
        output.textures_delta.clear();
        state.review = None;
        state.page = 2;
        state.revocation = Some(host.review_revocation(vec![invite.public_id()]).unwrap());
        let mut output = ctx.run_ui(Default::default(), |ui| {
            state.show(ui.ctx(), false, Some(&host), None)
        });
        output.textures_delta.clear();
    }
}

//! Host-local invitation inventory and exact reviewed revocation. No credential is exported.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteSummary {
    pub id: String,
    pub remaining_seconds: u64,
    pub expired: bool,
    pub used: bool,
    pub revoked: bool,
    pub reclaim: Option<Member>,
    pub sessions: Vec<Member>,
}
pub struct RevocationReview {
    room: String,
    ids: Vec<String>,
    seal: String,
    grants: Vec<InviteSummary>,
}
impl RevocationReview {
    pub fn grants(&self) -> &[InviteSummary] {
        &self.grants
    }
    pub fn digest(&self) -> &str {
        &self.seal
    }
}
impl Invite {
    /// Public certificate fingerprint, not an authentication credential or account identity.
    pub fn certificate_fingerprint(&self) -> String {
        crate::workflows::digest(&self.certificate)
    }
    pub fn public_id(&self) -> String {
        crate::workflows::digest(self.token.as_bytes())
    }
}
fn summaries(state: &Authority) -> Vec<InviteSummary> {
    let now = Instant::now();
    let mut rows = state
        .grants
        .iter()
        .map(|(token, g)| {
            let mut sessions = state
                .sessions
                .values()
                .filter(|s| &s.invite == token)
                .map(|s| Member {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    online: s.expires > now && now.duration_since(s.seen) < LEASE,
                })
                .collect::<Vec<_>>();
            sessions.sort_by(|a, b| a.id.cmp(&b.id));
            InviteSummary {
                id: crate::workflows::digest(token.as_bytes()),
                remaining_seconds: g.expires.saturating_duration_since(now).as_secs(),
                expired: g.expires <= now,
                used: g.used,
                revoked: g.revoked,
                reclaim: g.reclaim.clone(),
                sessions,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}
fn selected(state: &Authority, ids: &[String]) -> Result<Vec<InviteSummary>> {
    if ids.is_empty() || ids.len() > 128 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err("Select 1-128 distinct invitations".into());
    }
    let all = summaries(state);
    ids.iter()
        .map(|id| {
            all.iter()
                .find(|g| &g.id == id)
                .cloned()
                .ok_or("Selected invitation no longer exists".into())
        })
        .collect()
}
fn seal(state: &Authority, rows: &[InviteSummary]) -> Result<String> {
    // Time and leased presence naturally change; authority, scope and affected identities must not.
    let values=rows.iter().map(|g|serde_json::json!({"id":g.id,"used":g.used,"revoked":g.revoked,"reclaim":g.reclaim.as_ref().map(|m|(&m.id,&m.name)),"sessions":g.sessions.iter().map(|m|(&m.id,&m.name)).collect::<Vec<_>>()})).collect::<Vec<_>>();
    crate::workflows::seal(&(
        state.shared.state.room_id.clone(),
        state.shared.state.host_id.clone(),
        values,
    ))
}
impl Authority {
    /// Reclaimed identities and departed task owners return offline. Authentication never survives.
    pub(super) fn retire_sessions(&mut self, remove: impl Fn(&Session) -> bool) {
        let ids = self
            .sessions
            .iter()
            .filter(|(_, s)| remove(s))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in ids {
            if let Some(session) = self.sessions.remove(&key) {
                self.shared.retire_presence(&session.id);
                if let Some(mut member) = self
                    .grants
                    .get(&session.invite)
                    .and_then(|g| g.reclaim.clone())
                    .or_else(|| {
                        self.shared
                            .state
                            .tasks
                            .iter()
                            .any(|t| t.owner_id == session.id)
                            .then(|| Member {
                                id: session.id.clone(),
                                name: session.name.clone(),
                                online: false,
                            })
                    })
                {
                    member.online = false;
                    self.recovered.insert(member.id.clone(), member);
                }
            }
        }
    }
}
impl Host {
    pub fn invitation_summaries(&self) -> Result<Vec<InviteSummary>> {
        let state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        Ok(summaries(&state))
    }
    pub fn review_revocation(&self, mut ids: Vec<String>) -> Result<RevocationReview> {
        ids.sort();
        let state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        let grants = selected(&state, &ids)?;
        Ok(RevocationReview {
            room: state.shared.state.room_id.clone(),
            seal: seal(&state, &grants)?,
            ids,
            grants,
        })
    }
    /// One local-host mutation. New joined sessions or changed scopes invalidate the whole review.
    pub fn revoke_reviewed(&self, review: &RevocationReview) -> Result<usize> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_mutation()?;
        let rows = selected(&state, &review.ids)?;
        if review.room != state.shared.state.room_id || seal(&state, &rows)? != review.seal {
            return Err("Invitation scope or affected members changed; review again".into());
        }
        let tokens = state
            .grants
            .keys()
            .filter(|token| {
                review
                    .ids
                    .contains(&crate::workflows::digest(token.as_bytes()))
            })
            .cloned()
            .collect::<HashSet<_>>();
        let affected = state
            .sessions
            .values()
            .filter(|s| tokens.contains(&s.invite))
            .count();
        for token in &tokens {
            state.grants.get_mut(token).expect("reviewed grant").revoked = true;
        }
        state.retire_sessions(|s| tokens.contains(&s.invite));
        state.sync_members();
        let actor = state.shared.state.host_id.clone();
        state.shared.audit(
            &actor,
            "revoked reviewed invitations and their joined sessions",
        );
        state.persist()?;
        Ok(affected)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn departed_fresh_task_owners_remain_claimable_without_retaining_guest_sessions() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Ownership").unwrap();
        let invite = host.invite(Duration::from_secs(60)).unwrap();
        let (mut owner, _) = Client::join(invite.clone(), "Owner").unwrap();
        let id = owner.identity.clone();
        host.control(Action::CreateTask {
            title: "Keep ownership".into(),
            owner: id.clone(),
            criteria: vec![],
            dependencies: vec![],
            priority: crate::TaskPriority::Normal,
            due_hours: None,
        })
        .unwrap();
        let (mut guest, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Guest").unwrap();
        guest.leave().unwrap();
        assert!(host.recovered_members().unwrap().is_empty());
        let review = host.review_revocation(vec![invite.public_id()]).unwrap();
        host.revoke_reviewed(&review).unwrap();
        assert!(owner.poll().is_err());
        assert_eq!(host.recovered_members().unwrap()[0].id, id);
        let (mut reclaimed, _) = Client::join(
            host.reclaim_invite(&id, Duration::from_secs(60)).unwrap(),
            "Not reassigned",
        )
        .unwrap();
        assert_eq!(reclaimed.identity, id);
        assert_eq!(host.snapshot().unwrap().shared.tasks[0].owner_id, id);
        reclaimed.leave().unwrap();
        assert_eq!(host.recovered_members().unwrap()[0].id, id);
    }
    #[test]
    fn reclaimed_owners_return_offline_after_leave_expiry_and_both_revocation_paths() {
        let path = std::env::temp_dir().join(format!(
            "forge-access-reclaim-{}.json",
            crate::workflows::new_id().unwrap()
        ));
        let original = Host::start("127.0.0.1:0".parse().unwrap(), "Reclaim").unwrap();
        let (client, _) = Client::join(
            original.invite(Duration::from_secs(60)).unwrap(),
            "Saved owner",
        )
        .unwrap();
        let owner = client.identity.clone();
        original.save_checkpoint(&path).unwrap();
        drop(client);
        drop(original);
        let host = Host::recover("127.0.0.1:0".parse().unwrap(), &path).unwrap();
        host.resume().unwrap();
        for mode in ["leave", "expiry", "legacy revoke", "reviewed revoke"] {
            let invite = host
                .reclaim_invite(&owner, Duration::from_secs(60))
                .unwrap();
            let (mut client, snapshot) = Client::join(invite.clone(), "Ignored new name").unwrap();
            assert_eq!(client.identity, owner);
            assert_eq!(
                snapshot
                    .members
                    .iter()
                    .find(|m| m.id == owner)
                    .unwrap()
                    .name,
                "Saved owner"
            );
            assert!(host.recovered_members().unwrap().is_empty());
            match mode {
                "leave" => client.leave().unwrap(),
                "expiry" => {
                    host.authority
                        .lock()
                        .unwrap()
                        .sessions
                        .values_mut()
                        .find(|s| s.id == owner)
                        .unwrap()
                        .expires = Instant::now() - Duration::from_secs(1);
                    host.authority
                        .lock()
                        .unwrap()
                        .grants
                        .get_mut(&invite.token)
                        .unwrap()
                        .expires = Instant::now() - Duration::from_secs(1);
                    host.invite(Duration::from_secs(60)).unwrap();
                    assert!(client.poll().is_err());
                }
                "legacy revoke" => host.revoke(&invite).unwrap(),
                _ => {
                    let review = host.review_revocation(vec![invite.public_id()]).unwrap();
                    host.revoke_reviewed(&review).unwrap();
                }
            }
            assert!(client.poll().is_err());
            let members = host.recovered_members().unwrap();
            assert_eq!(members.len(), 1, "{mode}");
            assert_eq!(members[0].id, owner);
            assert!(!members[0].online);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn inventory_has_no_credentials_and_review_rejects_new_sessions_atomically() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        let a = host.invite(Duration::from_secs(60)).unwrap();
        let b = host.invite(Duration::from_secs(60)).unwrap();
        let frozen = host
            .review_revocation(vec![a.public_id(), b.public_id()])
            .unwrap();
        let (mut client, _) = Client::join(a.clone(), "Ada").unwrap();
        assert!(host.revoke_reviewed(&frozen).is_err());
        assert!(client.poll().is_ok());
        let rows = host.invitation_summaries().unwrap();
        let raw = serde_json::to_string(&rows).unwrap();
        let token = serde_json::from_str::<serde_json::Value>(&a.encode()).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!raw.contains(&token));
        assert!(!raw.contains("certificate"));
        let review = host
            .review_revocation(vec![a.public_id(), b.public_id()])
            .unwrap();
        assert_eq!(
            review
                .grants()
                .iter()
                .map(|g| g.sessions.len())
                .sum::<usize>(),
            1
        );
        assert_eq!(host.revoke_reviewed(&review).unwrap(), 1);
        assert!(client.poll().is_err());
        assert!(Client::join(b, "Denied").is_err());
    }
    #[test]
    fn foreign_host_reviews_are_denied_and_expiry_does_not_silently_revoke_a_joined_member() {
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        let invite = host.invite(Duration::from_secs(60)).unwrap();
        let (mut client, _) = Client::join(invite.clone(), "Ada").unwrap();
        host.authority
            .lock()
            .unwrap()
            .grants
            .get_mut(&invite.token)
            .unwrap()
            .expires = Instant::now() - Duration::from_secs(1);
        assert!(client.poll().is_ok());
        let row = host.invitation_summaries().unwrap().remove(0);
        assert!(row.expired && row.used);
        assert_eq!(row.sessions.len(), 1);
        let review = host.review_revocation(vec![invite.public_id()]).unwrap();
        let other = Host::start("127.0.0.1:0".parse().unwrap(), "Access").unwrap();
        assert!(other.revoke_reviewed(&review).is_err());
        host.pause().unwrap();
        assert_eq!(host.revoke_reviewed(&review).unwrap(), 1);
        assert!(client.poll().is_err());
    }
}

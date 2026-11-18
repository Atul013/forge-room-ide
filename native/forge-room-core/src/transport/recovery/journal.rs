use super::*;

const MAX_JOURNAL: u64 = 64 * 1024 * 1024;
pub(in crate::transport) struct Journal {
    file: std::fs::File,
    bytes: u64,
    last: Vec<u8>,
}
impl Journal {
    fn append(&mut self, bytes: Vec<u8>) -> Result<()> {
        if bytes == self.last {
            return Ok(());
        }
        if bytes.len() > MAX_FRAME || self.bytes + bytes.len() as u64 + 8 > MAX_JOURNAL {
            return Err(
                "Public journal limit reached; recover the last saved record into a new journal"
                    .into(),
            );
        }
        self.file
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .and_then(|_| self.file.write_all(&bytes))
            .and_then(|_| self.file.write_all(b"DONE"))
            .and_then(|_| self.file.sync_all())
            .map_err(|e| format!("Public journal write failed: {e}"))?;
        self.bytes += bytes.len() as u64 + 8;
        self.last = bytes;
        Ok(())
    }
}
impl Authority {
    pub(in crate::transport) fn check_storage(&self) -> Result<()> {
        match &self.storage_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    pub(in crate::transport) fn persist(&mut self) -> Result<()> {
        self.check_storage()?;
        if self.journal.is_none() {
            return Ok(());
        }
        let checkpoint = Checkpoint::capture(self);
        let result = checkpoint
            .validate()
            .and_then(|_| serde_json::to_vec(&checkpoint).map_err(|e| e.to_string()))
            .and_then(|bytes| {
                self.journal
                    .as_mut()
                    .ok_or("Journal unavailable")?
                    .append(bytes)
            });
        if let Err(error) = result {
            let error = format!("Room stopped after storage failure: {error}. Restart from the journal before continuing.");
            self.storage_error = Some(error.clone());
            return Err(error);
        }
        Ok(())
    }
}
impl Host {
    /// Explicit opt-in/rotation. A fresh file is flushed before replacing an existing journal.
    pub fn enable_journal(&self, path: &Path) -> Result<()> {
        let mut state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        let checkpoint = Checkpoint::capture(&state);
        checkpoint.validate()?;
        let bytes = serde_json::to_vec(&checkpoint).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(b"FRJ1\n").map_err(|e| e.to_string())?;
        let mut journal = Journal {
            file,
            bytes: 5,
            last: Vec::new(),
        };
        journal.append(bytes)?;
        state.journal = Some(journal);
        state.journal_required = true;
        Ok(())
    }
    pub fn journal_bytes(&self) -> Result<Option<u64>> {
        let state = self.authority.lock().map_err(|_| "Room unavailable")?;
        state.check_storage()?;
        Ok(state.journal.as_ref().map(|journal| journal.bytes))
    }
    /// Preserves the original journal, ignores an incomplete final record, rejects complete corruption.
    /// The recovered host is paused and requires an explicit fresh journal before future durability.
    pub fn recover_journal(bind: SocketAddr, path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Choose a regular journal file".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_JOURNAL + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_JOURNAL || !bytes.starts_with(b"FRJ1\n") {
            return Err("Invalid or oversized public journal".into());
        }
        let (mut offset, mut last) = (5usize, None);
        while bytes.len().saturating_sub(offset) >= 4 {
            let length = u32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .map_err(|_| "Invalid journal header")?,
            ) as usize;
            if length > MAX_FRAME {
                return Err("Invalid journal record length".into());
            }
            let end = offset + 4 + length;
            if bytes.len() < end + 4 {
                break;
            }
            if &bytes[end..end + 4] != b"DONE" {
                return Err("Corrupt complete journal record".into());
            }
            let record = &bytes[offset + 4..end];
            let checkpoint: Checkpoint =
                serde_json::from_slice(record).map_err(|_| "Corrupt complete journal JSON")?;
            checkpoint.validate()?;
            last = Some(record);
            offset = end + 4;
        }
        let host = Self::recover_bytes(bind, last.ok_or("Journal has no complete saved record")?)?;
        host.authority
            .lock()
            .map_err(|_| "Room unavailable")?
            .journal_required = true;
        Ok(host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acknowledged_public_mutations_survive_torn_tail_and_private_text_is_excluded() {
        let path = std::env::temp_dir().join(format!("forge-journal-{}", secret().unwrap()));
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "journal").unwrap();
        host.enable_journal(&path).unwrap();
        let (mut a, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        let (b, _) = Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Grace").unwrap();
        a.post("durable public message").unwrap();
        a.execute(Action::Message {
            text: "private not on disk".into(),
            recipient: Some(b.identity),
            task: None,
            decision: false,
            parent: None,
        })
        .unwrap();
        host.pause().unwrap();
        drop(host);
        let saved = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&saved).contains("private not on disk"));
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&100u32.to_be_bytes()).unwrap();
        file.write_all(b"{torn").unwrap();
        drop(file);
        let recovered = Host::recover_journal("127.0.0.1:0".parse().unwrap(), &path).unwrap();
        assert_eq!(recovered.lifecycle().unwrap(), RoomLifecycle::Paused);
        assert!(recovered
            .snapshot()
            .unwrap()
            .events
            .iter()
            .any(|event| event.text == "durable public message"));
        assert_eq!(recovered.recovered_members().unwrap().len(), 2);
        assert!(hostless_corruption(&path, saved));
        std::fs::remove_file(path).unwrap();
    }
    fn hostless_corruption(path: &Path, mut saved: Vec<u8>) -> bool {
        *saved.last_mut().unwrap() = b'X';
        std::fs::write(path, &saved).unwrap();
        Host::recover_journal("127.0.0.1:0".parse().unwrap(), path).is_err()
    }
    #[test]
    fn storage_failure_denies_acknowledgement_and_subsequent_state() {
        let path = std::env::temp_dir().join(format!("forge-journal-full-{}", secret().unwrap()));
        let host = Host::start("127.0.0.1:0".parse().unwrap(), "full").unwrap();
        host.enable_journal(&path).unwrap();
        let (mut client, _) =
            Client::join(host.invite(Duration::from_secs(60)).unwrap(), "Ada").unwrap();
        host.authority
            .lock()
            .unwrap()
            .journal
            .as_mut()
            .unwrap()
            .bytes = MAX_JOURNAL;
        assert!(client.post("not acknowledged").is_err());
        assert!(client.poll().is_err());
        assert!(host.resume().is_err());
        assert!(host.snapshot().is_err());
        drop(host);
        let recovered = Host::recover_journal("127.0.0.1:0".parse().unwrap(), &path).unwrap();
        assert!(!recovered
            .snapshot()
            .unwrap()
            .events
            .iter()
            .any(|event| event.text == "not acknowledged"));
        std::fs::remove_file(path).unwrap();
    }
}

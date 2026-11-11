//! Auto Save: a file's tab is saved once its text has stopped changing for a moment. This only
//! decides *when*; the save itself is the ordinary one, which refuses to overwrite a file that
//! changed on disk. A tab whose save failed is left alone until its text changes again.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long the text must stay unchanged before it is saved.
pub const DELAY: Duration = Duration::from_millis(1200);

#[derive(Default)]
pub struct Tracker {
    /// Per tab: the fingerprint of its text, when that text was first seen, and whether a save of
    /// exactly this text already failed.
    seen: HashMap<u64, (u64, Instant, bool)>,
}

impl Tracker {
    /// Whether tab `id`, which holds unsaved text with fingerprint `hash`, should be saved now.
    pub fn due(&mut self, id: u64, hash: u64, now: Instant) -> bool {
        let entry = self.seen.entry(id).or_insert((hash, now, false));
        if entry.0 != hash {
            *entry = (hash, now, false);
            return false;
        }
        !entry.2 && now.duration_since(entry.1) >= DELAY
    }

    /// Text that is already there and must not be saved on its own (tabs brought back by crash
    /// recovery): it waits for the user's next edit.
    pub fn hold(&mut self, id: u64, hash: u64, now: Instant) {
        self.seen.insert(id, (hash, now, true));
    }

    /// The save of the text last reported by [`Tracker::due`] failed: do not try it again.
    pub fn failed(&mut self, id: u64) {
        if let Some(entry) = self.seen.get_mut(&id) {
            entry.2 = true;
        }
    }

    /// Forget tabs that are clean or closed.
    pub fn keep_only(&mut self, live: &[u64]) {
        self.seen.retain(|id, _| live.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_is_saved_after_its_text_rests_and_a_failed_one_waits_for_new_text() {
        let mut tracker = Tracker::default();
        let start = Instant::now();
        assert!(!tracker.due(1, 10, start), "just seen");
        assert!(!tracker.due(1, 10, start + DELAY / 2));
        assert!(tracker.due(1, 10, start + DELAY));
        // Typing again restarts the wait.
        assert!(!tracker.due(1, 11, start + DELAY * 2));
        assert!(!tracker.due(1, 11, start + DELAY * 2 + DELAY / 2));
        assert!(tracker.due(1, 11, start + DELAY * 3));
        // A failure is not retried for the same text, but new text is.
        tracker.failed(1);
        assert!(!tracker.due(1, 11, start + DELAY * 9));
        assert!(!tracker.due(1, 12, start + DELAY * 9));
        assert!(tracker.due(1, 12, start + DELAY * 11));
        // Held text is not saved until it changes.
        tracker.hold(2, 5, start);
        assert!(!tracker.due(2, 5, start + DELAY * 30));
        assert!(!tracker.due(2, 6, start + DELAY * 30));
        assert!(tracker.due(2, 6, start + DELAY * 32));
        // Clean or closed tabs are forgotten, so a later edit starts a new wait.
        tracker.keep_only(&[]);
        assert!(!tracker.due(1, 12, start + DELAY * 20));
    }
}

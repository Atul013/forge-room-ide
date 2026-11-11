//! Work that must not freeze the window: run something on another thread and pick up its result on
//! a later frame. [`Latest`] runs one job at a time and remembers when another was asked for
//! meanwhile, so a burst of requests costs at most one extra run.

use eframe::egui;
use std::sync::mpsc;

pub struct Latest<T> {
    receiver: Option<mpsc::Receiver<T>>,
    again: bool,
}

impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self {
            receiver: None,
            again: false,
        }
    }
}

impl<T: Send + 'static> Latest<T> {
    /// Whether a job is running.
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }

    /// Start `work` on a new thread. While a job is already running it is not started: the request
    /// is remembered instead (see [`Latest::take_again`]) and false is returned.
    pub fn request(
        &mut self,
        context: &egui::Context,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> bool {
        if self.receiver.is_some() {
            self.again = true;
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work());
            context.request_repaint();
        });
        self.receiver = Some(receiver);
        true
    }

    /// The finished result, once. Also notices a job that died without answering.
    pub fn poll(&mut self) -> Option<T> {
        let receiver = self.receiver.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                None
            }
        }
    }

    /// True once if another job was requested while the last one ran (and clears that memory).
    pub fn take_again(&mut self) -> bool {
        std::mem::take(&mut self.again)
    }

    /// Forget a running job's result (it is for something that is no longer shown).
    pub fn discard(&mut self) {
        self.receiver = None;
        self.again = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait<T: Send + 'static>(latest: &mut Latest<T>) -> Option<T> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(result) = latest.poll() {
                return Some(result);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn a_result_arrives_once_and_a_second_request_is_remembered() {
        let context = egui::Context::default();
        let mut latest: Latest<u32> = Latest::default();
        assert!(latest.poll().is_none());
        assert!(latest.request(&context, || {
            std::thread::sleep(Duration::from_millis(60));
            7
        }));
        assert!(latest.busy());
        // Asked again while running: not started, but remembered exactly once.
        assert!(!latest.request(&context, || 8));
        assert!(!latest.request(&context, || 9));
        assert_eq!(wait(&mut latest), Some(7));
        assert!(latest.poll().is_none(), "a result is delivered only once");
        assert!(!latest.busy());
        assert!(latest.take_again());
        assert!(!latest.take_again());
        // Free again: the next job runs.
        assert!(latest.request(&context, || 10));
        assert_eq!(wait(&mut latest), Some(10));
    }

    #[test]
    fn a_job_that_panics_frees_the_slot() {
        let context = egui::Context::default();
        let mut latest: Latest<u32> = Latest::default();
        assert!(latest.request(&context, || panic!("work failed")));
        // The sender is dropped by the unwinding thread: polling notices and frees the slot.
        let deadline = Instant::now() + Duration::from_secs(5);
        while latest.busy() && Instant::now() < deadline {
            assert!(latest.poll().is_none());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!latest.busy());
        assert!(latest.request(&context, || 1));
        assert_eq!(wait(&mut latest), Some(1));
    }

    #[test]
    fn discard_forgets_the_running_job_and_the_pending_request() {
        let context = egui::Context::default();
        let mut latest: Latest<u32> = Latest::default();
        latest.request(&context, || 1);
        latest.request(&context, || 2);
        latest.discard();
        assert!(!latest.busy());
        assert!(!latest.take_again());
    }
}

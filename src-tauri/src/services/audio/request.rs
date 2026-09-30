//! Request/reply plumbing between the command layer and the audio thread.
//!
//! Every request carries its own reply channel. The previous design shared one response queue
//! between all callers, which paired replies with requests only by arrival order: two calls that
//! crossed could read each other's reply, and a reply arriving after the 5 s timeout was consumed
//! by the next caller, leaving every later call one reply behind for good.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crate::error::{CrateError, Result};

/// A request on its way to the worker thread, with the channel to answer on.
pub(super) type Envelope<Req, Resp> = (Req, Sender<Resp>);

/// The caller's side: sends a request and waits for *its* reply.
pub(super) struct Requester<Req, Resp> {
    tx: Sender<Envelope<Req, Resp>>,
}

// Not derived: `#[derive(Clone)]` would demand `Req: Clone` and `Resp: Clone`, which a cloned
// `Sender` does not need.
impl<Req, Resp> Clone for Requester<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<Req, Resp> Requester<Req, Resp> {
    /// A requester and the receiving end the worker thread loops on.
    pub(super) fn channel() -> (Self, Receiver<Envelope<Req, Resp>>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    /// Sends `request` and waits at most `timeout` for the worker's answer to it. A late answer
    /// is sent to a channel nobody listens to any more and is dropped: it can never be mistaken
    /// for the reply to a later request.
    pub(super) fn call(&self, request: Req, timeout: Duration) -> Result<Resp> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send((request, reply_tx))
            .map_err(|e| CrateError::Audio(format!("Failed to send command: {e}")))?;
        reply_rx
            .recv_timeout(timeout)
            .map_err(|e| CrateError::Audio(format!("Failed to receive response: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    /// Worker that answers `n` with `n * 10` after `delay(n)`.
    fn spawn_worker(
        rx: Receiver<Envelope<u32, u32>>,
        delay: impl Fn(u32) -> Duration + Send + 'static,
    ) {
        thread::spawn(move || {
            while let Ok((n, reply)) = rx.recv() {
                thread::sleep(delay(n));
                let _ = reply.send(n * 10);
            }
        });
    }

    #[test]
    fn a_caller_gets_the_reply_to_its_own_request() {
        let (requester, rx) = Requester::channel();
        spawn_worker(rx, |_| Duration::ZERO);
        assert_eq!(requester.call(7, Duration::from_secs(1)).unwrap(), 70);
    }

    #[test]
    fn concurrent_callers_never_read_each_others_reply() {
        let (requester, rx) = Requester::channel();
        let requester = Arc::new(requester);
        spawn_worker(rx, |_| Duration::from_millis(5));

        let handles: Vec<_> = (1..=16u32)
            .map(|n| {
                let requester = requester.clone();
                thread::spawn(move || (n, requester.call(n, Duration::from_secs(2)).unwrap()))
            })
            .collect();
        for handle in handles {
            let (n, reply) = handle.join().unwrap();
            assert_eq!(reply, n * 10, "request {n} must get its own reply");
        }
    }

    #[test]
    fn a_late_reply_is_not_given_to_the_next_request() {
        let (requester, rx) = Requester::channel();
        // Request 1 is answered after the caller gave up; request 2 is answered at once.
        spawn_worker(rx, |n| {
            if n == 1 {
                Duration::from_millis(120)
            } else {
                Duration::ZERO
            }
        });

        assert!(requester.call(1, Duration::from_millis(20)).is_err());
        // Before the fix this returned the late reply (10) instead of 20.
        assert_eq!(requester.call(2, Duration::from_secs(2)).unwrap(), 20);
        assert_eq!(requester.call(3, Duration::from_secs(2)).unwrap(), 30);
    }

    #[test]
    fn a_closed_worker_is_an_error_not_a_hang() {
        let (requester, rx) = Requester::<u32, u32>::channel();
        drop(rx);
        assert!(requester.call(1, Duration::from_millis(50)).is_err());
    }
}

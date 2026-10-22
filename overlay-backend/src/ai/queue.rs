//! Admission of provider requests: two at a time. A request the user asked
//! for is admitted before background work (auto tiles, session names,
//! summaries, memory), and background work holds at most one of the two slots,
//! so a wanted answer waits behind at most one other request. A newer auto-tile
//! question replaces an older one that has not started yet: on a call only the
//! latest question is worth answering.
//!
//! The priority travels with the task: [`with_priority`] wraps the future that
//! makes the request, and every request made inside it is admitted with that
//! priority. Anything outside such a scope is interactive.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Mutex, MutexGuard};
use tokio::sync::oneshot;

/// Requests the provider serves at once.
const SLOTS: usize = 2;
/// Of those, the most that background work may hold.
const BACKGROUND_SLOTS: usize = 1;

/// The finish reason of a stream whose auto-tile question a newer one replaced
/// before it started.
pub const SUPERSEDED_REASON: &str = "superseded";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiPriority {
    /// Asked by the user: F9, typed and spoken questions, Vision, follow-ups.
    Interactive,
    /// Work nobody waits on: session names, summaries, debriefs, memory.
    Background,
    /// A background auto-tile answer; a newer one replaces it while it waits.
    AutoTile,
}

tokio::task_local! {
    static PRIORITY: AiPriority;
}

/// Run `future` with every AI request inside it admitted at `priority`.
pub async fn with_priority<F: Future>(priority: AiPriority, future: F) -> F::Output {
    PRIORITY.scope(priority, future).await
}

pub(crate) fn current_priority() -> AiPriority {
    PRIORITY
        .try_with(|priority| *priority)
        .unwrap_or(AiPriority::Interactive)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Interactive,
    Background,
    /// All slots, for a local server switch.
    Exclusive,
}

struct Waiter {
    id: u64,
    kind: Kind,
    auto_tile: bool,
    granted: oneshot::Sender<bool>,
}

struct State {
    interactive: usize,
    background: usize,
    exclusive: bool,
    next_id: u64,
    waiting: VecDeque<Waiter>,
}

pub(crate) struct Queue {
    state: Mutex<State>,
}

static QUEUE: Queue = Queue::new();

impl Queue {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                interactive: 0,
                background: 0,
                exclusive: false,
                next_id: 0,
                waiting: VecDeque::new(),
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    async fn acquire_kind(
        &'static self,
        kind: Kind,
        auto_tile: bool,
    ) -> Result<AiPermit, Superseded> {
        let (granted, receiver) = oneshot::channel();
        let id = {
            let mut state = self.state();
            if auto_tile {
                let mut kept = VecDeque::with_capacity(state.waiting.len());
                for waiter in std::mem::take(&mut state.waiting) {
                    if waiter.auto_tile {
                        let _ = waiter.granted.send(false);
                    } else {
                        kept.push_back(waiter);
                    }
                }
                state.waiting = kept;
            }
            let id = state.next_id;
            state.next_id += 1;
            state.waiting.push_back(Waiter {
                id,
                kind,
                auto_tile,
                granted,
            });
            state.admit();
            id
        };
        let mut pending = Pending {
            queue: self,
            id,
            kind,
            granted: receiver,
            finished: false,
        };
        let granted = (&mut pending.granted).await;
        pending.finished = true;
        match granted {
            Ok(true) => Ok(AiPermit { queue: self, kind }),
            _ => Err(Superseded),
        }
    }

    pub(crate) async fn acquire_at(
        &'static self,
        priority: AiPriority,
    ) -> Result<AiPermit, Superseded> {
        match priority {
            AiPriority::Interactive => self.acquire_kind(Kind::Interactive, false).await,
            AiPriority::Background => self.acquire_kind(Kind::Background, false).await,
            AiPriority::AutoTile => self.acquire_kind(Kind::Background, true).await,
        }
    }

    pub(crate) async fn acquire_exclusive(&'static self) -> Result<AiPermit, Superseded> {
        self.acquire_kind(Kind::Exclusive, false).await
    }

    pub(crate) fn try_acquire_idle(&'static self) -> Option<AiPermit> {
        let mut state = self.state();
        if state.exclusive || state.interactive + state.background > 0 || !state.waiting.is_empty()
        {
            return None;
        }
        state.hold(Kind::Background);
        Some(AiPermit {
            queue: self,
            kind: Kind::Background,
        })
    }

    #[cfg(test)]
    pub(crate) fn snapshot(&self) -> (usize, usize, bool, usize) {
        let state = self.state();
        (
            state.interactive,
            state.background,
            state.exclusive,
            state.waiting.len(),
        )
    }
}

impl State {
    fn hold(&mut self, kind: Kind) {
        match kind {
            Kind::Interactive => self.interactive += 1,
            Kind::Background => self.background += 1,
            Kind::Exclusive => self.exclusive = true,
        }
    }

    fn release(&mut self, kind: Kind) {
        match kind {
            Kind::Interactive => self.interactive = self.interactive.saturating_sub(1),
            Kind::Background => self.background = self.background.saturating_sub(1),
            Kind::Exclusive => self.exclusive = false,
        }
        self.admit();
    }

    fn next_waiter(&self) -> Option<usize> {
        if self.exclusive {
            return None;
        }
        let active = self.interactive + self.background;
        // A server switch drains everything first and blocks new requests.
        if let Some(index) = self.waiting.iter().position(|w| w.kind == Kind::Exclusive) {
            return (active == 0).then_some(index);
        }
        if active >= SLOTS {
            return None;
        }
        self.waiting
            .iter()
            .position(|w| w.kind == Kind::Interactive)
            .or_else(|| {
                (self.background < BACKGROUND_SLOTS)
                    .then(|| self.waiting.iter().position(|w| w.kind == Kind::Background))
                    .flatten()
            })
    }

    fn admit(&mut self) {
        while let Some(index) = self.next_waiter() {
            let Some(waiter) = self.waiting.remove(index) else {
                return;
            };
            // The receiver lives as long as the entry is queued (see `Pending`).
            if waiter.granted.send(true).is_ok() {
                self.hold(waiter.kind);
            }
        }
    }
}

/// A held slot; dropping it admits the next request.
pub struct AiPermit {
    queue: &'static Queue,
    kind: Kind,
}

impl Drop for AiPermit {
    fn drop(&mut self) {
        self.queue.state().release(self.kind);
    }
}

/// A request still waiting; dropping it leaves the queue, or returns a slot
/// granted after the waiting future was abandoned.
struct Pending {
    queue: &'static Queue,
    id: u64,
    kind: Kind,
    granted: oneshot::Receiver<bool>,
    finished: bool,
}

impl Drop for Pending {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut state = self.queue.state();
        if let Some(index) = state.waiting.iter().position(|w| w.id == self.id) {
            state.waiting.remove(index);
        } else if let Ok(true) = self.granted.try_recv() {
            state.release(self.kind);
        }
    }
}

/// Why a waiting request was not admitted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Superseded;

/// Wait for a slot at the current task's priority.
pub(crate) async fn acquire() -> Result<AiPermit, Superseded> {
    QUEUE.acquire_at(current_priority()).await
}

/// Wait until no request runs and hold every slot.
pub(crate) async fn acquire_exclusive() -> Result<AiPermit, Superseded> {
    QUEUE.acquire_exclusive().await
}

/// A background slot now, only if nothing runs or waits.
pub(crate) fn try_acquire_idle() -> Option<AiPermit> {
    QUEUE.try_acquire_idle()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::time::Duration;

    fn queue() -> &'static Queue {
        Box::leak(Box::new(Queue::new()))
    }

    /// Start waiting at `priority`; the handle resolves once admitted.
    fn wait(
        queue: &'static Queue,
        priority: AiPriority,
    ) -> tokio::task::JoinHandle<Result<AiPermit, Superseded>> {
        tokio::spawn(queue.acquire_at(priority))
    }

    async fn settle() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn a_wanted_question_goes_before_waiting_background_work() {
        let q = queue();
        let first = q.acquire_at(AiPriority::Interactive).await.unwrap();
        let second = q.acquire_at(AiPriority::Interactive).await.unwrap();
        let background = wait(q, AiPriority::Background);
        settle().await;
        let wanted = wait(q, AiPriority::Interactive);
        settle().await;
        assert_eq!(q.snapshot(), (2, 0, false, 2));

        drop(first);
        let wanted = wanted.await.unwrap().unwrap();
        assert!(!background.is_finished());
        drop(second);
        let background = background.await.unwrap().unwrap();
        assert_eq!(q.snapshot(), (1, 1, false, 0));
        drop((wanted, background));
        assert_eq!(q.snapshot(), (0, 0, false, 0));
    }

    #[tokio::test]
    async fn background_work_keeps_one_slot_free_for_the_user() {
        let q = queue();
        let auto = q.acquire_at(AiPriority::AutoTile).await.unwrap();
        let namer = wait(q, AiPriority::Background);
        settle().await;
        assert!(!namer.is_finished(), "background took the second slot");
        let wanted = tokio::time::timeout(
            Duration::from_secs(1),
            q.acquire_at(AiPriority::Interactive),
        )
        .await
        .expect("a wanted question waited behind background work")
        .unwrap();
        drop(auto);
        namer.await.unwrap().unwrap();
        drop(wanted);
    }

    #[tokio::test]
    async fn a_newer_auto_tile_question_replaces_the_waiting_one() {
        let q = queue();
        let running = q.acquire_at(AiPriority::AutoTile).await.unwrap();
        let older = wait(q, AiPriority::AutoTile);
        settle().await;
        let summary = wait(q, AiPriority::Background);
        settle().await;
        let newer = wait(q, AiPriority::AutoTile);
        settle().await;
        assert_eq!(older.await.unwrap().err(), Some(Superseded));
        assert_eq!(q.snapshot(), (0, 1, false, 2));

        // Other background work keeps its place before the newer question.
        drop(running);
        let summary = summary.await.unwrap().unwrap();
        assert!(!newer.is_finished());
        drop(summary);
        drop(newer.await.unwrap().unwrap());
        assert_eq!(q.snapshot(), (0, 0, false, 0));
    }

    #[tokio::test]
    async fn an_abandoned_wait_leaves_the_queue_and_returns_its_slot() {
        let q = queue();
        let first = q.acquire_at(AiPriority::Interactive).await.unwrap();
        let second = q.acquire_at(AiPriority::Interactive).await.unwrap();
        let closed = wait(q, AiPriority::Interactive);
        settle().await;
        closed.abort();
        settle().await;
        assert_eq!(q.snapshot(), (2, 0, false, 0));

        // A slot granted to a future that is dropped before it sees the grant.
        let mut late = Box::pin(q.acquire_at(AiPriority::Interactive));
        assert!(futures_poll_once(&mut late).await.is_none());
        drop(first);
        assert_eq!(q.snapshot(), (2, 0, false, 0));
        drop(late);
        assert_eq!(q.snapshot(), (1, 0, false, 0));
        drop(second);
        assert_eq!(q.snapshot(), (0, 0, false, 0));
    }

    #[tokio::test]
    async fn a_server_switch_drains_every_request_and_holds_new_ones() {
        let q = queue();
        let running = q.acquire_at(AiPriority::Background).await.unwrap();
        let switch = tokio::spawn(q.acquire_exclusive());
        settle().await;
        let wanted = wait(q, AiPriority::Interactive);
        settle().await;
        assert!(!switch.is_finished());
        assert!(!wanted.is_finished(), "a request started during a switch");
        assert!(q.try_acquire_idle().is_none());

        drop(running);
        let switch = switch.await.unwrap().unwrap();
        settle().await;
        assert!(!wanted.is_finished());
        drop(switch);
        drop(wanted.await.unwrap().unwrap());
    }

    #[tokio::test]
    async fn the_idle_prime_runs_only_when_nothing_runs_or_waits() {
        let q = queue();
        let prime = q.try_acquire_idle().unwrap();
        assert!(q.try_acquire_idle().is_none());
        let wanted = q.acquire_at(AiPriority::Interactive).await.unwrap();
        drop((prime, wanted));
        assert_eq!(q.snapshot(), (0, 0, false, 0));
    }

    #[tokio::test]
    async fn the_priority_follows_the_task_that_makes_the_request() {
        assert_eq!(current_priority(), AiPriority::Interactive);
        let inner = with_priority(AiPriority::AutoTile, async { current_priority() }).await;
        assert_eq!(inner, AiPriority::AutoTile);
    }

    /// Poll a future once; `None` while it is still pending.
    async fn futures_poll_once<F: Future + Unpin>(future: &mut F) -> Option<F::Output> {
        std::future::poll_fn(|cx| {
            std::task::Poll::Ready(match std::pin::Pin::new(&mut *future).poll(cx) {
                std::task::Poll::Ready(output) => Some(output),
                std::task::Poll::Pending => None,
            })
        })
        .await
    }
}

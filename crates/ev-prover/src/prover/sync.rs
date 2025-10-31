use std::{
    fmt,
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;

use super::RangeProofCommitted;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageProofState {
    Idle,
    Running,
}

#[derive(Debug)]
pub struct MessageProofSync {
    state: Mutex<MessageProofState>,
    notify: Notify,
}

impl MessageProofSync {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(MessageProofState::Idle),
            notify: Notify::new(),
        }
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub async fn wait_for_idle(self: &Arc<Self>) {
        loop {
            if matches!(*self.state.lock().unwrap(), MessageProofState::Idle) {
                return;
            }
            self.notify.notified().await;
        }
    }

    pub async fn begin(self: &Arc<Self>) -> MessageProofPermit {
        self.wait_for_idle().await;
        {
            let mut state = self.state.lock().unwrap();
            *state = MessageProofState::Running;
        }
        MessageProofPermit { sync: Arc::clone(self) }
    }

    fn mark_idle(&self) {
        {
            let mut state = self.state.lock().unwrap();
            *state = MessageProofState::Idle;
        }
        self.notify.notify_waiters();
    }
}

#[derive(Debug)]
pub struct MessageProofPermit {
    sync: Arc<MessageProofSync>,
}

impl Drop for MessageProofPermit {
    fn drop(&mut self) {
        self.sync.mark_idle();
    }
}

pub struct MessageProofRequest {
    pub commit: RangeProofCommitted,
    pub permit: Option<MessageProofPermit>,
}

impl fmt::Debug for MessageProofRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageProofRequest")
            .field("commit", &self.commit)
            .field("has_permit", &self.permit.is_some())
            .finish()
    }
}

impl MessageProofRequest {
    pub fn new(commit: RangeProofCommitted) -> Self {
        Self { commit, permit: None }
    }

    pub fn with_permit(commit: RangeProofCommitted, permit: MessageProofPermit) -> Self {
        Self {
            commit,
            permit: Some(permit),
        }
    }
}

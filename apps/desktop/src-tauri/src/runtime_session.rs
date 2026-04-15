use runtime_core::{RuntimeMode, SessionContext, SessionEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct SessionController {
    next_sequence: AtomicU64,
    active_session: Mutex<Option<SessionContext>>,
}

impl SessionController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_session(
        &self,
        active_pack: String,
        runtime_mode: RuntimeMode,
        assigned_agent_id: String,
    ) -> (SessionContext, SessionEvent) {
        let session_id = self.create_session_id();
        let session_context = SessionContext {
            session_id: session_id.clone(),
            active_pack: active_pack.clone(),
            runtime_mode,
            assigned_agent_id,
        };

        if let Ok(mut locked_session) = self.active_session.lock() {
            *locked_session = Some(session_context.clone());
        }

        (
            session_context.clone(),
            SessionEvent::SessionStarted {
                session_id,
                active_pack,
            },
        )
    }

    pub fn stop_session(&self) -> Option<SessionEvent> {
        let mut locked_session = self.active_session.lock().ok()?;
        let previous_session = locked_session.clone()?;
        *locked_session = None;

        Some(SessionEvent::SessionStopped {
            session_id: previous_session.session_id,
            active_pack: previous_session.active_pack,
            reason: "stopped_by_user".to_string(),
        })
    }

    pub fn heartbeat_event(&self) -> Option<SessionEvent> {
        let session = self.active_session.lock().ok()?.clone()?;

        Some(SessionEvent::RuntimeHeartbeat {
            session_id: session.session_id,
            active_pack: session.active_pack,
            status: "ok".to_string(),
        })
    }

    pub fn current_session(&self) -> Option<SessionContext> {
        self.active_session.lock().ok()?.clone()
    }

    fn create_session_id(&self) -> String {
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let unix_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);

        format!("session-{unix_millis}-{sequence}")
    }
}

pub fn runtime_mode_label(runtime_mode: RuntimeMode) -> String {
    match runtime_mode {
        RuntimeMode::Local => "local",
        RuntimeMode::Cloud => "cloud",
        RuntimeMode::Hybrid => "hybrid",
    }
    .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatConfig {
    pub interval: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_started_then_heartbeat_then_stopped() {
        let controller = SessionController::new();

        let (session, started_event) = controller.start_session(
            "companion".to_string(),
            RuntimeMode::Hybrid,
            "companion-agent".to_string(),
        );
        let heartbeat_event = controller
            .heartbeat_event()
            .expect("heartbeat should exist while session is active");
        let stopped_event = controller
            .stop_session()
            .expect("stop should emit event for active session");

        match started_event {
            SessionEvent::SessionStarted { session_id, .. } => {
                assert_eq!(session_id, session.session_id);
            }
            _ => panic!("expected session_started event"),
        }

        match heartbeat_event {
            SessionEvent::RuntimeHeartbeat { session_id, .. } => {
                assert_eq!(session_id, session.session_id);
            }
            _ => panic!("expected runtime_heartbeat event"),
        }

        match stopped_event {
            SessionEvent::SessionStopped { session_id, .. } => {
                assert_eq!(session_id, session.session_id);
            }
            _ => panic!("expected session_stopped event"),
        }
    }

    #[test]
    fn generates_unique_session_ids() {
        let controller = SessionController::new();
        let (session_a, _) = controller.start_session(
            "companion".to_string(),
            RuntimeMode::Hybrid,
            "companion-agent".to_string(),
        );
        let _ = controller.stop_session();
        let (session_b, _) = controller.start_session(
            "coding".to_string(),
            RuntimeMode::Hybrid,
            "coding-agent".to_string(),
        );

        assert_ne!(session_a.session_id, session_b.session_id);
    }
}

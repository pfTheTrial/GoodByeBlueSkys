use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Local,
    Cloud,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportKind {
    Cli,
    Mcp,
    Bridge,
    Api,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivacyLevel {
    LocalOnly,
    LocalFirst,
    Hybrid,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupMode {
    Cold,
    Warm,
    Hot,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SupportedCapabilities {
    pub chat: bool,
    pub streaming: bool,
    pub vision: bool,
    pub screen_reasoning: bool,
    pub ui_pointing: bool,
    pub tool_use: bool,
    pub mcp: bool,
    pub local_execution: bool,
    pub medical_vision: bool,
    pub tts: bool,
    pub stt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub id: String,
    pub transport: TransportKind,
    pub supports: SupportedCapabilities,
    pub privacy_level: PrivacyLevel,
    pub startup_mode: StartupMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePolicy {
    pub mode: RuntimeMode,
    pub minimum_privacy_level: PrivacyLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentEvent {
    Token { text: String },
    FinalText { text: String },
    PointOnScreen {
        x: f32,
        y: f32,
        label: Option<String>,
        screen: Option<u8>,
    },
    HighlightRegion {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        label: Option<String>,
        screen: Option<u8>,
    },
    Confidence {
        level: ConfidenceLevel,
        reason: Option<String>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub active_pack: String,
    pub runtime_mode: RuntimeMode,
    pub assigned_agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSessionContextPayload {
    pub session_id: String,
    pub active_pack: String,
    pub runtime_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceSessionConfig {
    pub session_id: String,
    pub input_device_id: Option<String>,
    pub output_device_id: Option<String>,
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum VoiceSessionEvent {
    VoiceSessionStarted {
        session_id: String,
        locale: String,
    },
    VoiceInputChunkAccepted {
        session_id: String,
        chunk_size_bytes: usize,
    },
    VoiceOutputChunkReady {
        session_id: String,
        mime_type: String,
        chunk_size_bytes: usize,
    },
    VoiceSessionStopped {
        session_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionStarted { session_id: String, active_pack: String },
    RuntimeHeartbeat {
        session_id: String,
        active_pack: String,
        status: String,
    },
    SessionStopped {
        session_id: String,
        active_pack: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_voice_session_events_with_stable_tag() {
        let event = VoiceSessionEvent::VoiceSessionStarted {
            session_id: "session-voice-1".to_string(),
            locale: "pt-BR".to_string(),
        };

        let value = serde_json::to_value(event).expect("voice event should serialize");
        assert_eq!(value["event_type"], "voice_session_started");
        assert_eq!(value["session_id"], "session-voice-1");
        assert_eq!(value["locale"], "pt-BR");
    }
}

use runtime_core::{SessionContext, VoiceInputChunkPayload, VoiceOutputChunkPayload};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub fn sidecar_health_check() -> Result<String, String> {
    let sidecar_bin = configured_sidecar_bin();

    let output = Command::new(&sidecar_bin)
        .arg("--health")
        .output()
        .map_err(|error| format!("failed to spawn sidecar binary '{sidecar_bin}': {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "sidecar health check failed (status={}): {stderr}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err("sidecar health check returned empty output".to_string());
    }

    Ok(stdout)
}

pub fn sidecar_enabled_from_env() -> bool {
    std::env::var("COMPANION_ENABLE_SIDECAR")
        .ok()
        .map(|raw_value| {
            matches!(
                raw_value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn configured_sidecar_bin() -> String {
    std::env::var("COMPANION_SIDECAR_BIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "runtime-sidecar".to_string())
}

pub struct RuntimeSidecarSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarTelemetryEvent {
    pub command: String,
    pub response_kind: String,
    pub session_id: Option<String>,
    pub detail: Option<String>,
}

impl RuntimeSidecarSession {
    pub fn spawn() -> Result<Self, String> {
        let sidecar_bin = configured_sidecar_bin();
        Self::spawn_with_binary(&sidecar_bin)
    }

    pub fn spawn_with_binary(sidecar_bin: &str) -> Result<Self, String> {
        let mut child = Command::new(&sidecar_bin)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn sidecar binary '{sidecar_bin}': {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "sidecar stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "sidecar stdout unavailable".to_string())?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub fn send_session_started(&mut self, session: &SessionContext) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::SessionStarted {
            session_id: session.session_id.clone(),
            active_pack: session.active_pack.clone(),
            runtime_mode: runtime_mode_label(session),
            assigned_agent_id: session.assigned_agent_id.clone(),
        })
    }

    pub fn send_runtime_heartbeat(&mut self, session_id: &str) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::RuntimeHeartbeat {
            session_id: session_id.to_string(),
            status: "ok".to_string(),
        })
    }

    pub fn send_session_stopped(&mut self, session_id: &str, reason: &str) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::SessionStopped {
            session_id: session_id.to_string(),
            reason: reason.to_string(),
        })
    }

    pub fn shutdown(mut self) -> Result<SidecarTelemetryEvent, String> {
        let event = self.send_request(SidecarRequest::Shutdown)?;
        self.child
            .wait()
            .map_err(|error| format!("failed to wait for sidecar shutdown: {error}"))?;
        Ok(event)
    }

    pub fn send_voice_session_started(
        &mut self,
        session_id: &str,
        locale: &str,
    ) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::VoiceSessionStarted {
            session_id: session_id.to_string(),
            locale: locale.to_string(),
        })
    }

    pub fn send_voice_session_stopped(
        &mut self,
        session_id: &str,
        reason: &str,
    ) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::VoiceSessionStopped {
            session_id: session_id.to_string(),
            reason: reason.to_string(),
        })
    }

    pub fn send_voice_input_chunk(
        &mut self,
        payload: &VoiceInputChunkPayload,
    ) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::VoiceInputChunk {
            session_id: payload.session_id.clone(),
            chunk_size_bytes: payload.chunk_size_bytes,
        })
    }

    pub fn send_voice_output_chunk(
        &mut self,
        payload: &VoiceOutputChunkPayload,
    ) -> Result<SidecarTelemetryEvent, String> {
        self.send_request(SidecarRequest::VoiceOutputChunk {
            session_id: payload.session_id.clone(),
            mime_type: payload.mime_type.clone(),
            chunk_size_bytes: payload.chunk_size_bytes,
        })
    }

    fn send_request(&mut self, request: SidecarRequest) -> Result<SidecarTelemetryEvent, String> {
        let command_name = request.command_name().to_string();
        let request_session_id = request.session_id().map(ToString::to_string);
        let serialized = serde_json::to_string(&request)
            .map_err(|error| format!("failed to serialize sidecar request: {error}"))?;
        self.send_line(&serialized)?;
        let response = self.read_response()?;
        let response_kind_label = response.kind.to_label().to_string();
        match response.kind {
            SidecarResponseKind::Ack | SidecarResponseKind::Bye => Ok(SidecarTelemetryEvent {
                command: command_name,
                response_kind: response_kind_label,
                session_id: request_session_id,
                detail: response.message,
            }),
            SidecarResponseKind::Error => Err(format!(
                "sidecar returned error: {}",
                response
                    .message
                    .unwrap_or_else(|| "unknown sidecar error".to_string())
            )),
        }
    }

    fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .map_err(|error| format!("failed to write to sidecar stdin: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("failed to flush sidecar stdin: {error}"))
    }

    fn read_response(&mut self) -> Result<SidecarResponse, String> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|error| format!("failed to read sidecar response: {error}"))?;
        serde_json::from_str(line.trim())
            .map_err(|error| format!("failed to parse sidecar response: {error}"))
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SidecarRequest {
    SessionStarted {
        session_id: String,
        active_pack: String,
        runtime_mode: String,
        assigned_agent_id: String,
    },
    RuntimeHeartbeat {
        session_id: String,
        status: String,
    },
    SessionStopped {
        session_id: String,
        reason: String,
    },
    VoiceSessionStarted {
        session_id: String,
        locale: String,
    },
    VoiceSessionStopped {
        session_id: String,
        reason: String,
    },
    VoiceInputChunk {
        session_id: String,
        chunk_size_bytes: usize,
    },
    VoiceOutputChunk {
        session_id: String,
        mime_type: String,
        chunk_size_bytes: usize,
    },
    Shutdown,
}

impl SidecarRequest {
    fn command_name(&self) -> &'static str {
        match self {
            SidecarRequest::SessionStarted { .. } => "session_started",
            SidecarRequest::RuntimeHeartbeat { .. } => "runtime_heartbeat",
            SidecarRequest::SessionStopped { .. } => "session_stopped",
            SidecarRequest::VoiceSessionStarted { .. } => "voice_session_started",
            SidecarRequest::VoiceSessionStopped { .. } => "voice_session_stopped",
            SidecarRequest::VoiceInputChunk { .. } => "voice_input_chunk",
            SidecarRequest::VoiceOutputChunk { .. } => "voice_output_chunk",
            SidecarRequest::Shutdown => "shutdown",
        }
    }

    fn session_id(&self) -> Option<&str> {
        match self {
            SidecarRequest::SessionStarted { session_id, .. } => Some(session_id),
            SidecarRequest::RuntimeHeartbeat { session_id, .. } => Some(session_id),
            SidecarRequest::SessionStopped { session_id, .. } => Some(session_id),
            SidecarRequest::VoiceSessionStarted { session_id, .. } => Some(session_id),
            SidecarRequest::VoiceSessionStopped { session_id, .. } => Some(session_id),
            SidecarRequest::VoiceInputChunk { session_id, .. } => Some(session_id),
            SidecarRequest::VoiceOutputChunk { session_id, .. } => Some(session_id),
            SidecarRequest::Shutdown => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SidecarResponse {
    kind: SidecarResponseKind,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SidecarResponseKind {
    Ack,
    Error,
    Bye,
}

impl SidecarResponseKind {
    fn to_label(&self) -> &'static str {
        match self {
            SidecarResponseKind::Ack => "ack",
            SidecarResponseKind::Error => "error",
            SidecarResponseKind::Bye => "bye",
        }
    }
}

fn runtime_mode_label(session: &SessionContext) -> String {
    match session.runtime_mode {
        runtime_core::RuntimeMode::Local => "local",
        runtime_core::RuntimeMode::Cloud => "cloud",
        runtime_core::RuntimeMode::Hybrid => "hybrid",
    }
    .to_string()
}

impl Drop for RuntimeSidecarSession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_errors_when_binary_is_missing() {
        std::env::set_var("COMPANION_SIDECAR_BIN", "__missing_sidecar_binary__");
        let result = sidecar_health_check();
        std::env::remove_var("COMPANION_SIDECAR_BIN");

        assert!(result.is_err());
    }

    #[test]
    fn sidecar_enablement_defaults_to_false() {
        std::env::remove_var("COMPANION_ENABLE_SIDECAR");
        assert!(!sidecar_enabled_from_env());
    }

    #[test]
    fn sidecar_request_serialization_uses_protocol_kind() {
        let serialized = serde_json::to_string(&SidecarRequest::RuntimeHeartbeat {
            session_id: "s1".to_string(),
            status: "ok".to_string(),
        })
        .expect("request should serialize");
        assert!(serialized.contains("\"kind\":\"runtime_heartbeat\""));
    }
}

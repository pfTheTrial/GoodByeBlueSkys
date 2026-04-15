use agent_supervisor::AgentSupervisor;
use policy_engine::{BackendPolicyEngine, UploadDataType, WorkspacePolicy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use runtime_core::{
    CapabilityManifest, CapabilityRegistry, PrivacyLevel, RuntimeMode, StartupMode,
    RuntimePolicy, RuntimeSessionContextPayload, SessionEvent, SupportedCapabilities, TransportKind,
    VoiceInputChunkPayload, VoiceOutputChunkPayload, VoiceSessionConfig, VoiceSessionEvent,
};

use crate::runtime_session::{
    runtime_mode_label, HeartbeatConfig, SessionController,
};
use crate::runtime_sidecar::{sidecar_enabled_from_env, RuntimeSidecarSession, SidecarTelemetryEvent};

pub struct RuntimeState {
    pub registry: CapabilityRegistry,
    pub session_controller: Arc<SessionController>,
    pub agent_supervisor: AgentSupervisor,
    pub heartbeat_config: HeartbeatConfig,
    pub default_runtime_policy: RuntimePolicy,
    pub workspace_policy: WorkspacePolicy,
    pub sidecar_enabled: bool,
    pub sidecar_session: Option<RuntimeSidecarSession>,
    pub pending_sidecar_events: Vec<SidecarTelemetryEvent>,
    pub voice_session: Option<VoiceSessionConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCapabilityManifestPayload {
    pub id: String,
    pub transport: String,
    pub privacy_level: String,
}

pub fn runtime_health() -> String {
    "ok:warm".to_string()
}

pub fn start_session(
    runtime_state: &mut RuntimeState,
    active_pack: Option<String>,
    runtime_mode: Option<RuntimeMode>,
) -> Result<(RuntimeSessionContextPayload, SessionEvent), String> {
    let effective_runtime_mode = runtime_mode.unwrap_or(runtime_state.default_runtime_policy.mode);
    let effective_runtime_policy = RuntimePolicy {
        mode: effective_runtime_mode,
        minimum_privacy_level: runtime_state.default_runtime_policy.minimum_privacy_level,
    };

    if !BackendPolicyEngine::upload_is_allowed(
        UploadDataType::Screen,
        runtime_state.workspace_policy,
        effective_runtime_policy,
    ) {
        return Err("workspace policy blocks screen upload in current mode".to_string());
    }

    let pack_name = active_pack.unwrap_or_else(|| "companion".to_string());
    let agent_id = agent_id_for_pack(&pack_name);

    if runtime_state.sidecar_enabled {
        if runtime_state.sidecar_session.is_none() {
            runtime_state.sidecar_session = Some(RuntimeSidecarSession::spawn()?);
        }
    }

    runtime_state
        .agent_supervisor
        .register_agent(agent_id.clone());
    runtime_state
        .agent_supervisor
        .set_hot(&agent_id)
        .map_err(|error| format!("agent supervisor error: {error:?}"))?;

    let _selected_backend = runtime_state
        .registry
        .choose_backend_with_filter(true, |manifest| {
            BackendPolicyEngine::manifest_is_allowed(manifest, effective_runtime_policy)
        })
        .map_err(|error| format!("backend selection failed: {error}"))?;

    let (session_context, started_event) =
        runtime_state
            .session_controller
            .start_session(pack_name, effective_runtime_mode, agent_id);

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        let telemetry_event = sidecar_session.send_session_started(&session_context)?;
        runtime_state.pending_sidecar_events.push(telemetry_event);
    }

    Ok((
        RuntimeSessionContextPayload {
            session_id: session_context.session_id,
            active_pack: session_context.active_pack,
            runtime_mode: runtime_mode_label(session_context.runtime_mode),
        },
        started_event,
    ))
}

pub fn stop_session(runtime_state: &mut RuntimeState) -> Option<SessionEvent> {
    let current_session = runtime_state
        .session_controller
        .current_session();

    if let Some(agent_id) = current_session
        .as_ref()
        .map(|session| session.assigned_agent_id.clone())
    {
        let _ = runtime_state.agent_supervisor.set_cold(&agent_id);
    }

    if let (Some(session), Some(sidecar_session)) =
        (current_session.as_ref(), runtime_state.sidecar_session.as_mut())
    {
        if let Ok(telemetry_event) =
            sidecar_session.send_session_stopped(&session.session_id, "stopped_by_user")
        {
            runtime_state.pending_sidecar_events.push(telemetry_event);
        }
    }

    if let Some(sidecar_session) = runtime_state.sidecar_session.take() {
        if let Ok(telemetry_event) = sidecar_session.shutdown() {
            runtime_state.pending_sidecar_events.push(telemetry_event);
        }
    }

    runtime_state.session_controller.stop_session()
}

pub fn start_voice_session(
    runtime_state: &mut RuntimeState,
    locale: Option<String>,
) -> Result<VoiceSessionEvent, String> {
    let current_session = runtime_state
        .session_controller
        .current_session()
        .ok_or_else(|| "cannot start voice session without active runtime session".to_string())?;

    if runtime_state.voice_session.is_some() {
        return Err("voice session already active".to_string());
    }

    if runtime_state.workspace_policy.no_audio_upload {
        return Err("workspace policy blocks audio upload".to_string());
    }

    let voice_locale = locale
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "pt-BR".to_string());

    let voice_session = VoiceSessionConfig {
        session_id: current_session.session_id.clone(),
        input_device_id: None,
        output_device_id: None,
        locale: voice_locale.clone(),
    };

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        let telemetry_event =
            sidecar_session.send_voice_session_started(&voice_session.session_id, &voice_locale)?;
        runtime_state.pending_sidecar_events.push(telemetry_event);
    }

    runtime_state.voice_session = Some(voice_session.clone());

    Ok(VoiceSessionEvent::VoiceSessionStarted {
        session_id: voice_session.session_id,
        locale: voice_locale,
    })
}

pub fn stop_voice_session(
    runtime_state: &mut RuntimeState,
    reason: &str,
) -> Option<VoiceSessionEvent> {
    let voice_session = runtime_state.voice_session.take()?;

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        if let Ok(telemetry_event) =
            sidecar_session.send_voice_session_stopped(&voice_session.session_id, reason)
        {
            runtime_state.pending_sidecar_events.push(telemetry_event);
        }
    }

    Some(VoiceSessionEvent::VoiceSessionStopped {
        session_id: voice_session.session_id,
        reason: reason.to_string(),
    })
}

pub fn submit_voice_input_chunk(
    runtime_state: &mut RuntimeState,
    chunk_size_bytes: usize,
) -> Result<VoiceSessionEvent, String> {
    let voice_session = runtime_state
        .voice_session
        .as_ref()
        .ok_or_else(|| "voice session is not active".to_string())?;

    if chunk_size_bytes == 0 {
        return Err("voice input chunk must be greater than zero".to_string());
    }

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        let payload = VoiceInputChunkPayload {
            session_id: voice_session.session_id.clone(),
            chunk_size_bytes,
        };
        let telemetry_event = sidecar_session.send_voice_input_chunk(&payload)?;
        runtime_state.pending_sidecar_events.push(telemetry_event);
    }

    Ok(VoiceSessionEvent::VoiceInputChunkAccepted {
        session_id: voice_session.session_id.clone(),
        chunk_size_bytes,
    })
}

pub fn publish_voice_output_chunk(
    runtime_state: &mut RuntimeState,
    mime_type: Option<String>,
    chunk_size_bytes: usize,
) -> Result<VoiceSessionEvent, String> {
    let voice_session = runtime_state
        .voice_session
        .as_ref()
        .ok_or_else(|| "voice session is not active".to_string())?;

    if chunk_size_bytes == 0 {
        return Err("voice output chunk must be greater than zero".to_string());
    }

    let resolved_mime_type = mime_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "audio/pcm".to_string());

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        let payload = VoiceOutputChunkPayload {
            session_id: voice_session.session_id.clone(),
            mime_type: resolved_mime_type.clone(),
            chunk_size_bytes,
        };
        let telemetry_event = sidecar_session.send_voice_output_chunk(&payload)?;
        runtime_state.pending_sidecar_events.push(telemetry_event);
    }

    Ok(VoiceSessionEvent::VoiceOutputChunkReady {
        session_id: voice_session.session_id.clone(),
        mime_type: resolved_mime_type,
        chunk_size_bytes,
    })
}

pub fn forward_heartbeat_to_sidecar(runtime_state: &mut RuntimeState) -> Result<(), String> {
    if !runtime_state.sidecar_enabled {
        return Ok(());
    }

    let session_id = match runtime_state
        .session_controller
        .current_session()
        .map(|session| session.session_id)
    {
        Some(value) => value,
        None => return Ok(()),
    };

    if let Some(sidecar_session) = runtime_state.sidecar_session.as_mut() {
        let telemetry_event = sidecar_session.send_runtime_heartbeat(&session_id)?;
        runtime_state.pending_sidecar_events.push(telemetry_event);
    }

    Ok(())
}

pub fn take_pending_sidecar_events(runtime_state: &mut RuntimeState) -> Vec<SidecarTelemetryEvent> {
    std::mem::take(&mut runtime_state.pending_sidecar_events)
}

pub fn runtime_capabilities(
    runtime_state: &RuntimeState,
) -> Vec<RuntimeCapabilityManifestPayload> {
    runtime_state
        .registry
        .list()
        .into_iter()
        .map(|manifest| RuntimeCapabilityManifestPayload {
            id: manifest.id,
            transport: transport_to_label(manifest.transport),
            privacy_level: privacy_level_to_label(manifest.privacy_level),
        })
        .collect::<Vec<_>>()
}

pub fn build_runtime_state() -> RuntimeState {
    build_runtime_state_with_config(heartbeat_config_from_env())
}

pub fn build_runtime_state_with_config(heartbeat_config: HeartbeatConfig) -> RuntimeState {
    let mut registry = CapabilityRegistry::new();

    let local_cli_manifest = CapabilityManifest {
        id: "local-cli-bootstrap".to_string(),
        transport: TransportKind::Cli,
        supports: SupportedCapabilities {
            chat: true,
            streaming: true,
            screen_reasoning: true,
            tool_use: true,
            local_execution: true,
            ..Default::default()
        },
        privacy_level: PrivacyLevel::LocalFirst,
        startup_mode: StartupMode::Warm,
    };

    let api_manifest = CapabilityManifest {
        id: "cloud-api-bootstrap".to_string(),
        transport: TransportKind::Api,
        supports: SupportedCapabilities {
            chat: true,
            streaming: true,
            vision: true,
            ..Default::default()
        },
        privacy_level: PrivacyLevel::Cloud,
        startup_mode: StartupMode::Cold,
    };

    registry
        .register(local_cli_manifest)
        .expect("bootstrap manifest must register");
    registry
        .register(api_manifest)
        .expect("bootstrap manifest must register");

    RuntimeState {
        registry,
        session_controller: Arc::new(SessionController::new()),
        agent_supervisor: AgentSupervisor::new(),
        heartbeat_config,
        default_runtime_policy: RuntimePolicy {
            mode: RuntimeMode::Hybrid,
            minimum_privacy_level: PrivacyLevel::Cloud,
        },
        workspace_policy: workspace_policy_from_env(),
        sidecar_enabled: sidecar_enabled_from_env(),
        sidecar_session: None,
        pending_sidecar_events: Vec::new(),
        voice_session: None,
    }
}

pub fn heartbeat_config_from_env() -> HeartbeatConfig {
    let configured_interval_millis = std::env::var("COMPANION_HEARTBEAT_INTERVAL_MS")
        .ok()
        .and_then(|raw_value| raw_value.parse::<u64>().ok())
        .filter(|value| *value >= 100);

    match configured_interval_millis {
        Some(interval_millis) => HeartbeatConfig {
            interval: Duration::from_millis(interval_millis),
        },
        None => HeartbeatConfig::default(),
    }
}

pub fn workspace_policy_from_env() -> WorkspacePolicy {
    WorkspacePolicy {
        no_screen_upload: env_flag_is_truthy("COMPANION_POLICY_NO_SCREEN_UPLOAD"),
        no_audio_upload: env_flag_is_truthy("COMPANION_POLICY_NO_AUDIO_UPLOAD"),
        local_only: env_flag_is_truthy("COMPANION_POLICY_LOCAL_ONLY"),
    }
}

fn env_flag_is_truthy(var_name: &str) -> bool {
    std::env::var(var_name)
        .ok()
        .map(|raw_value| {
            matches!(
                raw_value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn transport_to_label(transport: TransportKind) -> String {
    match transport {
        TransportKind::Cli => "cli",
        TransportKind::Mcp => "mcp",
        TransportKind::Bridge => "bridge",
        TransportKind::Api => "api",
        TransportKind::Local => "local",
    }
    .to_string()
}

fn privacy_level_to_label(privacy_level: PrivacyLevel) -> String {
    match privacy_level {
        PrivacyLevel::LocalOnly => "local-only",
        PrivacyLevel::LocalFirst => "local-first",
        PrivacyLevel::Hybrid => "hybrid",
        PrivacyLevel::Cloud => "cloud",
    }
    .to_string()
}

fn agent_id_for_pack(active_pack: &str) -> String {
    format!("{active_pack}-agent")
}

pub fn adaptive_heartbeat_interval(runtime_state: &RuntimeState) -> Duration {
    let base_interval = runtime_state.heartbeat_config.interval;
    let active_runtime_mode = runtime_state
        .session_controller
        .current_session()
        .map(|session| session.runtime_mode)
        .unwrap_or(runtime_state.default_runtime_policy.mode);

    let base_millis = base_interval.as_millis() as u64;
    let adjusted_millis = match active_runtime_mode {
        RuntimeMode::Local => base_millis.saturating_sub(base_millis / 3).max(100),
        RuntimeMode::Hybrid => base_millis.max(100),
        RuntimeMode::Cloud => (base_millis + (base_millis / 2)).max(100),
    };

    Duration::from_millis(adjusted_millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_supervisor::AgentWarmState;
    use std::fs;
    use std::path::{Path, PathBuf};
    use runtime_core::SessionEvent;

    #[test]
    fn runtime_app_start_stop_updates_agent_state_and_emits_events() {
        let mut runtime_state = build_runtime_state_with_config(HeartbeatConfig {
            interval: Duration::from_millis(250),
        });

        let (session_payload, started_event) =
            start_session(&mut runtime_state, Some("coding".to_string()), None)
                .expect("start session should succeed");

        let expected_agent_id = "coding-agent".to_string();
        assert_eq!(
            runtime_state.agent_supervisor.get_state(&expected_agent_id),
            Some(AgentWarmState::Hot)
        );
        assert!(runtime_state.sidecar_session.is_none());
        assert_eq!(session_payload.active_pack, "coding");
        assert_eq!(runtime_state.heartbeat_config.interval, Duration::from_millis(250));

        match started_event {
            SessionEvent::SessionStarted { session_id, active_pack } => {
                assert_eq!(session_id, session_payload.session_id);
                assert_eq!(active_pack, "coding");
            }
            _ => panic!("expected session_started"),
        }

        let stopped_event =
            stop_session(&mut runtime_state).expect("stopped event should be generated");

        assert_eq!(
            runtime_state.agent_supervisor.get_state(&expected_agent_id),
            Some(AgentWarmState::Cold)
        );
        assert!(runtime_state.sidecar_session.is_none());

        match stopped_event {
            SessionEvent::SessionStopped {
                session_id,
                active_pack,
                ..
            } => {
                assert_eq!(session_id, session_payload.session_id);
                assert_eq!(active_pack, "coding");
            }
            _ => panic!("expected session_stopped"),
        }
    }

    #[test]
    fn adaptive_heartbeat_changes_by_mode() {
        let mut runtime_state = build_runtime_state_with_config(HeartbeatConfig {
            interval: Duration::from_millis(300),
        });

        runtime_state.default_runtime_policy.mode = RuntimeMode::Cloud;
        assert_eq!(
            adaptive_heartbeat_interval(&runtime_state),
            Duration::from_millis(450)
        );

        runtime_state.default_runtime_policy.mode = RuntimeMode::Local;
        assert_eq!(
            adaptive_heartbeat_interval(&runtime_state),
            Duration::from_millis(200)
        );
    }

    #[test]
    fn start_session_fails_when_workspace_policy_blocks_screen_upload() {
        let mut runtime_state = build_runtime_state_with_config(HeartbeatConfig {
            interval: Duration::from_millis(200),
        });
        runtime_state.workspace_policy = WorkspacePolicy {
            no_screen_upload: true,
            no_audio_upload: false,
            local_only: false,
        };

        let result = start_session(&mut runtime_state, Some("companion".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn start_session_accepts_explicit_runtime_mode() {
        let mut runtime_state = build_runtime_state_with_config(HeartbeatConfig {
            interval: Duration::from_millis(250),
        });

        let (session_payload, _) = start_session(
            &mut runtime_state,
            Some("companion".to_string()),
            Some(RuntimeMode::Local),
        )
        .expect("session should start in local mode");

        assert_eq!(session_payload.runtime_mode, "local");
        assert_eq!(
            runtime_state
                .session_controller
                .current_session()
                .expect("session should exist")
                .runtime_mode,
            RuntimeMode::Local
        );
    }

    #[test]
    fn workspace_policy_is_loaded_from_environment() {
        std::env::set_var("COMPANION_POLICY_NO_SCREEN_UPLOAD", "true");
        std::env::set_var("COMPANION_POLICY_NO_AUDIO_UPLOAD", "1");
        std::env::set_var("COMPANION_POLICY_LOCAL_ONLY", "yes");

        let policy = workspace_policy_from_env();
        assert!(policy.no_screen_upload);
        assert!(policy.no_audio_upload);
        assert!(policy.local_only);

        std::env::remove_var("COMPANION_POLICY_NO_SCREEN_UPLOAD");
        std::env::remove_var("COMPANION_POLICY_NO_AUDIO_UPLOAD");
        std::env::remove_var("COMPANION_POLICY_LOCAL_ONLY");
    }

    #[test]
    fn sidecar_is_disabled_by_default_for_safe_rollout() {
        std::env::remove_var("COMPANION_ENABLE_SIDECAR");
        let runtime_state = build_runtime_state_with_config(HeartbeatConfig::default());
        assert!(!runtime_state.sidecar_enabled);
    }

    #[test]
    fn sidecar_real_binary_roundtrip_for_start_heartbeat_stop_when_available() {
        let Some(sidecar_binary_path) = locate_prepared_sidecar_binary() else {
            return;
        };

        let mut runtime_state = build_runtime_state_with_config(HeartbeatConfig {
            interval: Duration::from_millis(150),
        });
        runtime_state.sidecar_enabled = true;
        runtime_state.sidecar_session = Some(
            RuntimeSidecarSession::spawn_with_binary(
                &sidecar_binary_path.to_string_lossy(),
            )
            .expect("sidecar should spawn for integration roundtrip"),
        );

        let _ = start_session(&mut runtime_state, Some("companion".to_string()), Some(RuntimeMode::Hybrid))
            .expect("start session should succeed with sidecar");
        let voice_started =
            start_voice_session(&mut runtime_state, Some("pt-BR".to_string()))
                .expect("voice session should start with sidecar");
        match voice_started {
            VoiceSessionEvent::VoiceSessionStarted { .. } => {}
            _ => panic!("expected voice session started event"),
        }
        let input_chunk = submit_voice_input_chunk(&mut runtime_state, 512)
            .expect("voice input chunk should be accepted");
        match input_chunk {
            VoiceSessionEvent::VoiceInputChunkAccepted { chunk_size_bytes, .. } => {
                assert_eq!(chunk_size_bytes, 512);
            }
            _ => panic!("expected voice input chunk accepted event"),
        }
        let output_chunk = publish_voice_output_chunk(
            &mut runtime_state,
            Some("audio/pcm".to_string()),
            1024,
        )
        .expect("voice output chunk should be published");
        match output_chunk {
            VoiceSessionEvent::VoiceOutputChunkReady {
                chunk_size_bytes,
                mime_type,
                ..
            } => {
                assert_eq!(chunk_size_bytes, 1024);
                assert_eq!(mime_type, "audio/pcm");
            }
            _ => panic!("expected voice output chunk ready event"),
        }
        let voice_stopped =
            stop_voice_session(&mut runtime_state, "voice_stopped_by_test")
                .expect("voice session should stop with sidecar");
        match voice_stopped {
            VoiceSessionEvent::VoiceSessionStopped { .. } => {}
            _ => panic!("expected voice session stopped event"),
        }
        forward_heartbeat_to_sidecar(&mut runtime_state)
            .expect("heartbeat forwarding should succeed with sidecar");
        let stopped = stop_session(&mut runtime_state).expect("stop should emit runtime event");
        match stopped {
            SessionEvent::SessionStopped { .. } => {}
            _ => panic!("expected session stopped event"),
        }

        let telemetry_events = take_pending_sidecar_events(&mut runtime_state);
        let commands = telemetry_events
            .iter()
            .map(|event| (event.command.as_str(), event.response_kind.as_str()))
            .collect::<Vec<_>>();

        assert!(commands.contains(&("session_started", "ack")));
        assert!(commands.contains(&("voice_session_started", "ack")));
        assert!(commands.contains(&("voice_input_chunk", "ack")));
        assert!(commands.contains(&("voice_output_chunk", "ack")));
        assert!(commands.contains(&("voice_session_stopped", "ack")));
        assert!(commands.contains(&("runtime_heartbeat", "ack")));
        assert!(commands.contains(&("session_stopped", "ack")));
        assert!(commands.contains(&("shutdown", "bye")));
    }

    fn locate_prepared_sidecar_binary() -> Option<PathBuf> {
        let binaries_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
        if !binaries_dir.exists() {
            return None;
        }

        let expected_extension = if cfg!(windows) { ".exe" } else { "" };
        let entries = fs::read_dir(&binaries_dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy();
            if !file_name.starts_with("runtime-sidecar-") {
                continue;
            }
            if cfg!(windows) && !file_name.ends_with(".exe") {
                continue;
            }
            if !cfg!(windows) && file_name.ends_with(".exe") {
                continue;
            }

            let metadata = fs::metadata(&path).ok()?;
            if metadata.len() == 0 {
                continue;
            }

            if expected_extension.is_empty() || file_name.ends_with(expected_extension) {
                return Some(path);
            }
        }

        None
    }
}

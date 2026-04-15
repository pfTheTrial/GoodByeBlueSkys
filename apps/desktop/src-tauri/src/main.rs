mod runtime_app;
mod runtime_sidecar;
mod runtime_session;

use runtime_app::{
    adaptive_heartbeat_interval, forward_heartbeat_to_sidecar, take_pending_sidecar_events,
    build_runtime_state, runtime_capabilities as runtime_capabilities_from_state,
    publish_voice_output_chunk as runtime_publish_voice_output_chunk_in_state,
    runtime_health as runtime_health_from_state, start_session as runtime_start_session_in_state,
    submit_voice_input_chunk as runtime_submit_voice_input_chunk_in_state,
    start_voice_session as runtime_start_voice_session_in_state,
    stop_session as runtime_stop_session_in_state,
    stop_voice_session as runtime_stop_voice_session_in_state,
    RuntimeCapabilityManifestPayload, RuntimeState,
};
use runtime_core::{RuntimeMode, RuntimeSessionContextPayload, SessionEvent, VoiceSessionEvent};
use std::sync::Mutex;
use std::thread;
use tauri::Emitter;
use tauri::Manager;

#[tauri::command]
fn runtime_health() -> String {
    runtime_health_from_state()
}

#[tauri::command]
fn runtime_start_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
    active_pack: Option<String>,
    runtime_mode: Option<String>,
) -> Result<RuntimeSessionContextPayload, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let parsed_runtime_mode = runtime_mode
        .as_deref()
        .map(parse_runtime_mode)
        .transpose()?;
    let (runtime_session_payload, started_event) = runtime_start_session_command(
        &mut locked_state,
        active_pack,
        parsed_runtime_mode,
    )?;
    let _ = app.emit("runtime://session_event", started_event);
    emit_sidecar_events(&app, &mut locked_state);

    Ok(runtime_session_payload)
}

#[tauri::command]
fn runtime_stop_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
) -> Result<(), String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    emit_voice_events(
        &app,
        runtime_stop_voice_session_command(&mut locked_state, "runtime_session_stopped"),
    );
    if let Some(stopped_event_payload) = runtime_stop_session_command(&mut locked_state) {
        let _ = app.emit("runtime://session_event", stopped_event_payload);
    }
    emit_sidecar_events(&app, &mut locked_state);

    Ok(())
}

#[tauri::command]
fn runtime_voice_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
    locale: Option<String>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let voice_event = runtime_start_voice_session_command(&mut locked_state, locale)?;
    emit_voice_events(&app, Some(voice_event.clone()));
    emit_sidecar_events(&app, &mut locked_state);
    Ok(voice_event)
}

#[tauri::command]
fn runtime_voice_stop(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
) -> Result<(), String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    emit_voice_events(
        &app,
        runtime_stop_voice_session_command(&mut locked_state, "stopped_by_user"),
    );
    emit_sidecar_events(&app, &mut locked_state);
    Ok(())
}

#[tauri::command]
fn runtime_voice_input_chunk(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
    chunk_size_bytes: Option<usize>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let voice_event = runtime_voice_input_chunk_command(
        &mut locked_state,
        chunk_size_bytes.unwrap_or(512),
    )?;
    emit_voice_events(&app, Some(voice_event.clone()));
    emit_sidecar_events(&app, &mut locked_state);
    Ok(voice_event)
}

#[tauri::command]
fn runtime_voice_output_chunk(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<RuntimeState>>,
    mime_type: Option<String>,
    chunk_size_bytes: Option<usize>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let voice_event = runtime_voice_output_chunk_command(
        &mut locked_state,
        mime_type,
        chunk_size_bytes.unwrap_or(1024),
    )?;
    emit_voice_events(&app, Some(voice_event.clone()));
    emit_sidecar_events(&app, &mut locked_state);
    Ok(voice_event)
}

#[cfg(test)]
#[tauri::command]
fn runtime_voice_start_test(
    state: tauri::State<'_, Mutex<RuntimeState>>,
    locale: Option<String>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    runtime_start_voice_session_command(&mut locked_state, locale)
}

#[cfg(test)]
#[tauri::command]
fn runtime_voice_stop_test(
    state: tauri::State<'_, Mutex<RuntimeState>>,
) -> Result<(), String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let _ = runtime_stop_voice_session_command(&mut locked_state, "stopped_by_test");
    Ok(())
}

#[cfg(test)]
#[tauri::command]
fn runtime_voice_input_chunk_test(
    state: tauri::State<'_, Mutex<RuntimeState>>,
    chunk_size_bytes: Option<usize>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    runtime_voice_input_chunk_command(&mut locked_state, chunk_size_bytes.unwrap_or(512))
}

#[cfg(test)]
#[tauri::command]
fn runtime_voice_output_chunk_test(
    state: tauri::State<'_, Mutex<RuntimeState>>,
    mime_type: Option<String>,
    chunk_size_bytes: Option<usize>,
) -> Result<VoiceSessionEvent, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    runtime_voice_output_chunk_command(
        &mut locked_state,
        mime_type,
        chunk_size_bytes.unwrap_or(1024),
    )
}

#[tauri::command]
fn runtime_capabilities(
    state: tauri::State<'_, Mutex<RuntimeState>>,
) -> Result<Vec<RuntimeCapabilityManifestPayload>, String> {
    state
        .lock()
        .map_err(|_| "state lock failed".to_string())
        .map(|locked| runtime_capabilities_command(&locked))
}

#[tauri::command]
fn runtime_sidecar_health() -> Result<String, String> {
    runtime_sidecar::sidecar_health_check()
}

#[cfg(test)]
#[tauri::command]
fn runtime_start_session_test(
    state: tauri::State<'_, Mutex<RuntimeState>>,
    active_pack: Option<String>,
    runtime_mode: Option<String>,
) -> Result<RuntimeSessionContextPayload, String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let parsed_runtime_mode = runtime_mode
        .as_deref()
        .map(parse_runtime_mode)
        .transpose()?;
    runtime_start_session_command(&mut locked_state, active_pack, parsed_runtime_mode)
        .map(|(payload, _)| payload)
}

#[cfg(test)]
#[tauri::command]
fn runtime_stop_session_test(state: tauri::State<'_, Mutex<RuntimeState>>) -> Result<(), String> {
    let mut locked_state = state
        .lock()
        .map_err(|_| "state lock failed".to_string())?;
    let _ = runtime_stop_session_command(&mut locked_state);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let shared_session_state = app
                .state::<Mutex<RuntimeState>>()
                .lock()
                .expect("runtime state should lock in setup")
                .session_controller
                .clone();
            thread::spawn(move || {
                loop {
                    let heartbeat_interval = match app_handle.state::<Mutex<RuntimeState>>().lock() {
                        Ok(mut locked_state) => {
                            let _ = forward_heartbeat_to_sidecar(&mut locked_state);
                            emit_sidecar_events(&app_handle, &mut locked_state);
                            adaptive_heartbeat_interval(&locked_state)
                        }
                        Err(_) => std::time::Duration::from_secs(2),
                    };

                    if let Some(payload) = shared_session_state.heartbeat_event() {
                        let _ = app_handle.emit("runtime://session_event", payload);
                    }
                    thread::sleep(heartbeat_interval);
                }
            });

            Ok(())
        })
        .manage(Mutex::new(build_runtime_state()))
        .invoke_handler(tauri::generate_handler![
            runtime_health,
            runtime_capabilities,
            runtime_sidecar_health,
            runtime_start_session,
            runtime_stop_session,
            runtime_voice_start,
            runtime_voice_stop,
            runtime_voice_input_chunk,
            runtime_voice_output_chunk
        ])
        .run(tauri::generate_context!())
        .expect("tauri application failed");
}

fn runtime_start_session_command(
    runtime_state: &mut RuntimeState,
    active_pack: Option<String>,
    runtime_mode: Option<RuntimeMode>,
) -> Result<(RuntimeSessionContextPayload, SessionEvent), String> {
    runtime_start_session_in_state(runtime_state, active_pack, runtime_mode)
}

fn runtime_stop_session_command(runtime_state: &mut RuntimeState) -> Option<SessionEvent> {
    runtime_stop_session_in_state(runtime_state)
}

fn runtime_start_voice_session_command(
    runtime_state: &mut RuntimeState,
    locale: Option<String>,
) -> Result<VoiceSessionEvent, String> {
    runtime_start_voice_session_in_state(runtime_state, locale)
}

fn runtime_stop_voice_session_command(
    runtime_state: &mut RuntimeState,
    reason: &str,
) -> Option<VoiceSessionEvent> {
    runtime_stop_voice_session_in_state(runtime_state, reason)
}

fn runtime_voice_input_chunk_command(
    runtime_state: &mut RuntimeState,
    chunk_size_bytes: usize,
) -> Result<VoiceSessionEvent, String> {
    runtime_submit_voice_input_chunk_in_state(runtime_state, chunk_size_bytes)
}

fn runtime_voice_output_chunk_command(
    runtime_state: &mut RuntimeState,
    mime_type: Option<String>,
    chunk_size_bytes: usize,
) -> Result<VoiceSessionEvent, String> {
    runtime_publish_voice_output_chunk_in_state(runtime_state, mime_type, chunk_size_bytes)
}

fn runtime_capabilities_command(
    runtime_state: &RuntimeState,
) -> Vec<RuntimeCapabilityManifestPayload> {
    runtime_capabilities_from_state(runtime_state)
}

fn parse_runtime_mode(raw_runtime_mode: &str) -> Result<RuntimeMode, String> {
    match raw_runtime_mode.trim().to_ascii_lowercase().as_str() {
        "local" => Ok(RuntimeMode::Local),
        "cloud" => Ok(RuntimeMode::Cloud),
        "hybrid" => Ok(RuntimeMode::Hybrid),
        _ => Err("invalid runtime_mode, expected local|cloud|hybrid".to_string()),
    }
}

fn emit_sidecar_events(app: &tauri::AppHandle, runtime_state: &mut RuntimeState) {
    for sidecar_event in take_pending_sidecar_events(runtime_state) {
        let _ = app.emit("runtime://sidecar_event", sidecar_event);
    }
}

fn emit_voice_events(app: &tauri::AppHandle, voice_event: Option<VoiceSessionEvent>) {
    if let Some(event) = voice_event {
        let _ = app.emit("runtime://voice_event", event);
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use crate::runtime_session::HeartbeatConfig;

    #[test]
    fn command_helpers_cover_start_capabilities_stop_flow() {
        let mut runtime_state =
            build_runtime_state_with_command_defaults(HeartbeatConfig::default());

        let capability_payloads = runtime_capabilities_command(&runtime_state);
        assert!(!capability_payloads.is_empty());

        let (session_payload, started_event) =
            runtime_start_session_command(
                &mut runtime_state,
                Some("companion".to_string()),
                Some(RuntimeMode::Local),
            )
                .expect("session should start");
        assert_eq!(session_payload.active_pack, "companion");
        assert_eq!(session_payload.runtime_mode, "local");
        match started_event {
            SessionEvent::SessionStarted { .. } => {}
            _ => panic!("expected started event"),
        }

        let voice_started =
            runtime_start_voice_session_command(&mut runtime_state, Some("pt-BR".to_string()))
                .expect("voice should start");
        match voice_started {
            VoiceSessionEvent::VoiceSessionStarted { .. } => {}
            _ => panic!("expected voice started event"),
        }

        let voice_stopped =
            runtime_stop_voice_session_command(&mut runtime_state, "stopped_by_test")
                .expect("voice should stop");
        match voice_stopped {
            VoiceSessionEvent::VoiceSessionStopped { .. } => {}
            _ => panic!("expected voice stopped event"),
        }

        let voice_input = runtime_voice_input_chunk_command(&mut runtime_state, 512);
        assert!(voice_input.is_err());

        let stopped_event =
            runtime_stop_session_command(&mut runtime_state).expect("session should stop");
        match stopped_event {
            SessionEvent::SessionStopped { .. } => {}
            _ => panic!("expected stopped event"),
        }
    }

    #[test]
    fn runtime_sidecar_health_command_errors_when_sidecar_is_not_available() {
        std::env::set_var("COMPANION_SIDECAR_BIN", "__missing_sidecar_binary__");
        let result = runtime_sidecar_health();
        std::env::remove_var("COMPANION_SIDECAR_BIN");
        assert!(result.is_err());
    }

    fn build_runtime_state_with_command_defaults(heartbeat: HeartbeatConfig) -> RuntimeState {
        runtime_app::build_runtime_state_with_config(heartbeat)
    }

    #[test]
    fn parse_runtime_mode_accepts_supported_values() {
        assert_eq!(
            parse_runtime_mode("local").expect("local should parse"),
            RuntimeMode::Local
        );
        assert_eq!(
            parse_runtime_mode("cloud").expect("cloud should parse"),
            RuntimeMode::Cloud
        );
        assert_eq!(
            parse_runtime_mode("hybrid").expect("hybrid should parse"),
            RuntimeMode::Hybrid
        );
        assert!(parse_runtime_mode("unsupported").is_err());
    }
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::webview::InvokeRequest;

    #[test]
    fn tauri_invoke_harness_covers_health_capabilities_and_start_session() {
        let mut runtime_state =
            runtime_app::build_runtime_state_with_config(crate::runtime_session::HeartbeatConfig::default());
        runtime_state.workspace_policy = policy_engine::WorkspacePolicy::default();

        let app = tauri::test::mock_builder()
            .manage(Mutex::new(runtime_state))
            .invoke_handler(tauri::generate_handler![
                runtime_health,
                runtime_capabilities,
                runtime_sidecar_health,
                runtime_start_session_test,
                runtime_stop_session_test,
                runtime_voice_start_test,
                runtime_voice_stop_test,
                runtime_voice_input_chunk_test,
                runtime_voice_output_chunk_test
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app should build");
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("webview should build");

        let health_response = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_health", InvokeBody::default()),
        )
        .expect("runtime_health should succeed")
        .deserialize::<String>()
        .expect("health payload should deserialize");
        assert_eq!(health_response, "ok:warm");

        let capability_payloads = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_capabilities", InvokeBody::default()),
        )
        .expect("runtime_capabilities should succeed")
        .deserialize::<Vec<RuntimeCapabilityManifestPayload>>()
        .expect("capabilities payload should deserialize");
        assert!(!capability_payloads.is_empty());

        let start_body = serde_json::json!({
            "activePack": "companion",
            "runtimeMode": "local"
        });
        let session_payload = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_start_session_test", InvokeBody::Json(start_body)),
        )
        .expect("runtime_start_session should succeed")
        .deserialize::<RuntimeSessionContextPayload>()
        .expect("session payload should deserialize");

        assert_eq!(session_payload.active_pack, "companion");
        assert_eq!(session_payload.runtime_mode, "local");

        let voice_start_body = serde_json::json!({
            "locale": "pt-BR"
        });
        let voice_event = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_voice_start_test", InvokeBody::Json(voice_start_body)),
        )
        .expect("runtime_voice_start_test should succeed")
        .deserialize::<VoiceSessionEvent>()
        .expect("voice event payload should deserialize");
        match voice_event {
            VoiceSessionEvent::VoiceSessionStarted { locale, .. } => {
                assert_eq!(locale, "pt-BR");
            }
            _ => panic!("expected voice session started event"),
        }

        tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_voice_stop_test", InvokeBody::default()),
        )
        .expect("runtime_voice_stop_test should succeed");

        let voice_start_body_again = serde_json::json!({
            "locale": "pt-BR"
        });
        let _ = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_voice_start_test", InvokeBody::Json(voice_start_body_again)),
        )
        .expect("runtime_voice_start_test should succeed again");

        let voice_input_body = serde_json::json!({
            "chunkSizeBytes": 256
        });
        let input_event = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_voice_input_chunk_test", InvokeBody::Json(voice_input_body)),
        )
        .expect("runtime_voice_input_chunk_test should succeed")
        .deserialize::<VoiceSessionEvent>()
        .expect("voice input event should deserialize");
        match input_event {
            VoiceSessionEvent::VoiceInputChunkAccepted { chunk_size_bytes, .. } => {
                assert_eq!(chunk_size_bytes, 256);
            }
            _ => panic!("expected voice input chunk accepted event"),
        }

        let voice_output_body = serde_json::json!({
            "mimeType": "audio/pcm",
            "chunkSizeBytes": 512
        });
        let output_event = tauri::test::get_ipc_response(
            &webview,
            invoke_request("runtime_voice_output_chunk_test", InvokeBody::Json(voice_output_body)),
        )
        .expect("runtime_voice_output_chunk_test should succeed")
        .deserialize::<VoiceSessionEvent>()
        .expect("voice output event should deserialize");
        match output_event {
            VoiceSessionEvent::VoiceOutputChunkReady {
                chunk_size_bytes,
                mime_type,
                ..
            } => {
                assert_eq!(chunk_size_bytes, 512);
                assert_eq!(mime_type, "audio/pcm");
            }
            _ => panic!("expected voice output chunk ready event"),
        }
    }

    fn invoke_request(command: &str, body: InvokeBody) -> InvokeRequest {
        InvokeRequest {
            cmd: command.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().expect("url should parse"),
            body,
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        }
    }
}

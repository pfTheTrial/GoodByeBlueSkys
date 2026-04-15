use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--health") => {
            println!("ok:runtime-sidecar");
        }
        Some("--stdio") => run_stdio_loop(),
        _ => {
            eprintln!("usage: runtime-sidecar [--health|--stdio]");
            std::process::exit(2);
        }
    }
}

fn run_stdio_loop() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let input_line = match line {
            Ok(value) => value,
            Err(_) => break,
        };

        let response = handle_protocol_line(input_line.trim());

        let serialized = match serde_json::to_string(&response) {
            Ok(value) => value,
            Err(_) => break,
        };

        if writeln!(stdout, "{serialized}").is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }

        if matches!(response.kind, SidecarResponseKind::Bye) {
            break;
        }
    }
}

fn handle_protocol_line(raw_line: &str) -> SidecarResponse {
    let request = match serde_json::from_str::<SidecarRequest>(raw_line) {
        Ok(value) => value,
        Err(error) => {
            return SidecarResponse {
                kind: SidecarResponseKind::Error,
                message: Some(format!("invalid_request:{error}")),
            };
        }
    };

    match request {
        SidecarRequest::SessionStarted { .. } => SidecarResponse::ack(),
        SidecarRequest::RuntimeHeartbeat { .. } => SidecarResponse::ack(),
        SidecarRequest::SessionStopped { .. } => SidecarResponse::ack(),
        SidecarRequest::Shutdown => SidecarResponse::bye(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)]
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
    Shutdown,
}

#[derive(Debug, Serialize)]
struct SidecarResponse {
    kind: SidecarResponseKind,
    message: Option<String>,
}

impl SidecarResponse {
    fn ack() -> Self {
        Self {
            kind: SidecarResponseKind::Ack,
            message: None,
        }
    }

    fn bye() -> Self {
        Self {
            kind: SidecarResponseKind::Bye,
            message: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SidecarResponseKind {
    Ack,
    Error,
    Bye,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_handles_session_started_and_shutdown() {
        let started = r#"{"kind":"session_started","session_id":"s1","active_pack":"companion","runtime_mode":"hybrid","assigned_agent_id":"companion-agent"}"#;
        let started_response = handle_protocol_line(started);
        assert!(matches!(started_response.kind, SidecarResponseKind::Ack));

        let shutdown = r#"{"kind":"shutdown"}"#;
        let shutdown_response = handle_protocol_line(shutdown);
        assert!(matches!(shutdown_response.kind, SidecarResponseKind::Bye));
    }

    #[test]
    fn protocol_returns_error_for_invalid_payload() {
        let invalid_response = handle_protocol_line("not-json");
        assert!(matches!(invalid_response.kind, SidecarResponseKind::Error));
    }
}

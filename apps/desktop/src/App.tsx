import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type CapabilityManifest = {
  id: string;
  transport: string;
  privacy_level: string;
};

type RuntimeSessionContext = {
  session_id: string;
  active_pack: string;
  runtime_mode: string;
};

type RuntimeSessionEvent = {
  event_type: string;
  session_id: string;
  active_pack: string;
  status?: string;
  reason?: string;
};

type SidecarTelemetryEvent = {
  command: string;
  response_kind: string;
  session_id?: string;
  detail?: string;
};

type VoiceSessionEvent = {
  event_type: string;
  session_id: string;
  locale?: string;
  reason?: string;
};

type RuntimeMode = "local" | "cloud" | "hybrid";

const STORAGE_KEYS = {
  runtimeMode: "companion.runtime_mode",
  eventLimit: "companion.event_limit",
  runtimeAutoScroll: "companion.runtime_autoscroll",
  sidecarAutoScroll: "companion.sidecar_autoscroll"
} as const;

function readStoredRuntimeMode(): RuntimeMode {
  const candidate = localStorage.getItem(STORAGE_KEYS.runtimeMode);
  if (candidate === "local" || candidate === "cloud" || candidate === "hybrid") {
    return candidate;
  }
  return "hybrid";
}

function readStoredEventLimit(): number {
  const candidate = Number(localStorage.getItem(STORAGE_KEYS.eventLimit));
  if (!Number.isFinite(candidate)) {
    return 12;
  }
  return Math.min(200, Math.max(5, Math.round(candidate)));
}

function readStoredBoolean(key: string, fallback: boolean): boolean {
  const candidate = localStorage.getItem(key);
  if (candidate === "true") {
    return true;
  }
  if (candidate === "false") {
    return false;
  }
  return fallback;
}

export function App() {
  const [health, setHealth] = useState<string>("loading");
  const [sidecarHealth, setSidecarHealth] = useState<string>("unknown");
  const [capabilities, setCapabilities] = useState<CapabilityManifest[]>([]);
  const [session, setSession] = useState<RuntimeSessionContext | null>(null);
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>(readStoredRuntimeMode);
  const [runtimeEvents, setRuntimeEvents] = useState<RuntimeSessionEvent[]>([]);
  const [sidecarEvents, setSidecarEvents] = useState<SidecarTelemetryEvent[]>([]);
  const [eventLimit, setEventLimit] = useState(readStoredEventLimit);
  const [runtimeFilter, setRuntimeFilter] = useState("");
  const [sidecarFilter, setSidecarFilter] = useState("");
  const [runtimeAutoScroll, setRuntimeAutoScroll] = useState(() =>
    readStoredBoolean(STORAGE_KEYS.runtimeAutoScroll, true)
  );
  const [sidecarAutoScroll, setSidecarAutoScroll] = useState(() =>
    readStoredBoolean(STORAGE_KEYS.sidecarAutoScroll, true)
  );
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [voiceStatus, setVoiceStatus] = useState("inativa");
  const [isVoiceStarting, setIsVoiceStarting] = useState(false);
  const [isVoiceStopping, setIsVoiceStopping] = useState(false);
  const runtimeLogRef = useRef<HTMLUListElement | null>(null);
  const sidecarLogRef = useRef<HTMLUListElement | null>(null);

  const refreshSidecarHealth = () => {
    void invoke<string>("runtime_sidecar_health")
      .then((status) => {
        setSidecarHealth(status);
        setErrorMessage(null);
      })
      .catch((error: unknown) => {
        setSidecarHealth(`error: ${String(error)}`);
      });
  };

  const startSession = () => {
    setIsStarting(true);
    setErrorMessage(null);
    void invoke<RuntimeSessionContext>("runtime_start_session", {
      activePack: "companion",
      runtimeMode
    })
      .then((nextSession) => {
        setSession(nextSession);
      })
      .catch((error: unknown) => {
        setSession(null);
        setErrorMessage(`Falha ao iniciar sessao: ${String(error)}`);
      })
      .finally(() => setIsStarting(false));
  };

  const stopSession = () => {
    setIsStopping(true);
    setErrorMessage(null);
    void invoke("runtime_stop_session")
      .then(() => setSession(null))
      .then(() => setVoiceStatus("inativa"))
      .catch((error: unknown) => {
        setErrorMessage(`Falha ao parar sessao: ${String(error)}`);
      })
      .finally(() => setIsStopping(false));
  };

  const startVoiceSession = () => {
    setIsVoiceStarting(true);
    setErrorMessage(null);
    void invoke<VoiceSessionEvent>("runtime_voice_start", { locale: "pt-BR" })
      .then((event) => {
        setVoiceStatus(`ativa (${event.locale ?? "n/a"})`);
      })
      .catch((error: unknown) => {
        setErrorMessage(`Falha ao iniciar voz: ${String(error)}`);
      })
      .finally(() => setIsVoiceStarting(false));
  };

  const stopVoiceSession = () => {
    setIsVoiceStopping(true);
    setErrorMessage(null);
    void invoke("runtime_voice_stop")
      .then(() => setVoiceStatus("inativa"))
      .catch((error: unknown) => {
        setErrorMessage(`Falha ao parar voz: ${String(error)}`);
      })
      .finally(() => setIsVoiceStopping(false));
  };

  const filteredRuntimeEvents = runtimeEvents.filter((event) => {
    const searchable = `${event.event_type} ${event.session_id} ${event.status ?? ""} ${event.reason ?? ""}`
      .toLowerCase();
    return searchable.includes(runtimeFilter.toLowerCase());
  });

  const filteredSidecarEvents = sidecarEvents.filter((event) => {
    const searchable =
      `${event.command} ${event.response_kind} ${event.session_id ?? ""} ${event.detail ?? ""}`
        .toLowerCase();
    return searchable.includes(sidecarFilter.toLowerCase());
  });

  useEffect(() => {
    if (!runtimeAutoScroll) {
      return;
    }
    runtimeLogRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [runtimeAutoScroll, filteredRuntimeEvents.length]);

  useEffect(() => {
    if (!sidecarAutoScroll) {
      return;
    }
    sidecarLogRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  }, [sidecarAutoScroll, filteredSidecarEvents.length]);

  const exportLogs = (kind: "runtime" | "sidecar") => {
    const payload = kind === "runtime" ? runtimeEvents : sidecarEvents;
    const content = JSON.stringify(
      {
        kind,
        exported_at: new Date().toISOString(),
        items: payload
      },
      null,
      2
    );
    const blob = new Blob([content], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `companion-${kind}-events.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.runtimeMode, runtimeMode);
  }, [runtimeMode]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.eventLimit, String(eventLimit));
  }, [eventLimit]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.runtimeAutoScroll, String(runtimeAutoScroll));
  }, [runtimeAutoScroll]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.sidecarAutoScroll, String(sidecarAutoScroll));
  }, [sidecarAutoScroll]);

  useEffect(() => {
    void invoke<string>("runtime_health")
      .then(setHealth)
      .catch((error: unknown) => setHealth(`error: ${String(error)}`));

    void invoke<CapabilityManifest[]>("runtime_capabilities")
      .then(setCapabilities)
      .catch(() => setCapabilities([]));

    refreshSidecarHealth();

    const runtimeUnlistenPromise = listen<RuntimeSessionEvent>(
      "runtime://session_event",
      (event) => {
        setRuntimeEvents((previousEvents) => [event.payload, ...previousEvents].slice(0, eventLimit));
      }
    );

    const sidecarUnlistenPromise = listen<SidecarTelemetryEvent>(
      "runtime://sidecar_event",
      (event) => {
        setSidecarEvents((previousEvents) => [event.payload, ...previousEvents].slice(0, eventLimit));
      }
    );

    const voiceUnlistenPromise = listen<VoiceSessionEvent>(
      "runtime://voice_event",
      (event) => {
        if (event.payload.event_type === "voice_session_started") {
          setVoiceStatus(`ativa (${event.payload.locale ?? "n/a"})`);
        } else if (event.payload.event_type === "voice_session_stopped") {
          setVoiceStatus("inativa");
        }
      }
    );

    return () => {
      void runtimeUnlistenPromise.then((unlisten) => unlisten());
      void sidecarUnlistenPromise.then((unlisten) => unlisten());
      void voiceUnlistenPromise.then((unlisten) => unlisten());
    };
  }, [eventLimit]);

  return (
    <main className="screen">
      <section className="card shell">
        <header className="hero">
          <h1>Companion Platform - Prototipo Manual</h1>
          <p className="subtle">Runtime + Sidecar observabilidade em tempo real</p>
        </header>

        <section className="grid">
          <article className="panel">
            <h2>Estado</h2>
            <p>Status runtime: {health}</p>
            <p>Status sidecar: {sidecarHealth}</p>
            <p>Status voz: {voiceStatus}</p>
            <p>
              Sessao ativa:{" "}
              {session
                ? `${session.session_id} | ${session.active_pack} | ${session.runtime_mode}`
                : "sem sessao ativa"}
            </p>
            {errorMessage ? <p className="error">{errorMessage}</p> : null}
          </article>

          <article className="panel">
            <h2>Controles</h2>
            <label className="field">
              Runtime mode
              <select
                value={runtimeMode}
                onChange={(event) => setRuntimeMode(event.target.value as RuntimeMode)}
                disabled={Boolean(session)}
              >
                <option value="local">local</option>
                <option value="cloud">cloud</option>
                <option value="hybrid">hybrid</option>
              </select>
            </label>
            <label className="field">
              Limite de eventos
              <input
                type="number"
                min={5}
                max={200}
                value={eventLimit}
                onChange={(event) => {
                  const nextValue = Number(event.target.value);
                  if (Number.isFinite(nextValue)) {
                    setEventLimit(Math.min(200, Math.max(5, nextValue)));
                  }
                }}
              />
            </label>
            <div className="actions">
              <button onClick={startSession} disabled={isStarting || Boolean(session)}>
                {isStarting ? "Iniciando..." : "Iniciar sessao"}
              </button>
              <button onClick={stopSession} disabled={isStopping || !session}>
                {isStopping ? "Parando..." : "Parar sessao"}
              </button>
              <button
                onClick={startVoiceSession}
                disabled={isVoiceStarting || !session || voiceStatus !== "inativa"}
              >
                {isVoiceStarting ? "Iniciando voz..." : "Iniciar voz"}
              </button>
              <button
                onClick={stopVoiceSession}
                disabled={isVoiceStopping || voiceStatus === "inativa"}
              >
                {isVoiceStopping ? "Parando voz..." : "Parar voz"}
              </button>
              <button onClick={refreshSidecarHealth}>Health sidecar</button>
            </div>
          </article>
        </section>

        <section className="grid">
          <article className="panel">
            <h2>Capability Registry</h2>
            <ul className="log">
              {capabilities.map((manifest) => (
                <li key={manifest.id}>
                  {manifest.id} | {manifest.transport} | {manifest.privacy_level}
                </li>
              ))}
            </ul>
          </article>

          <article className="panel">
            <h2>Eventos runtime</h2>
            <div className="panel-controls">
              <input
                className="filter-input"
                placeholder="Filtrar runtime events"
                value={runtimeFilter}
                onChange={(event) => setRuntimeFilter(event.target.value)}
              />
              <label className="toggle-inline">
                <input
                  type="checkbox"
                  checked={runtimeAutoScroll}
                  onChange={(event) => setRuntimeAutoScroll(event.target.checked)}
                />
                Auto-scroll
              </label>
              <button onClick={() => setRuntimeEvents([])}>Limpar runtime</button>
              <button onClick={() => exportLogs("runtime")}>Export runtime</button>
            </div>
            <ul className="log" ref={runtimeLogRef}>
              {filteredRuntimeEvents.map((event, index) => (
                <li key={`${event.session_id}-${index}`}>
                  {event.event_type} | {event.session_id} |{" "}
                  {event.status ?? event.reason ?? "n/a"}
                </li>
              ))}
            </ul>
          </article>

          <article className="panel">
            <h2>Eventos sidecar</h2>
            <div className="panel-controls">
              <input
                className="filter-input"
                placeholder="Filtrar sidecar events"
                value={sidecarFilter}
                onChange={(event) => setSidecarFilter(event.target.value)}
              />
              <label className="toggle-inline">
                <input
                  type="checkbox"
                  checked={sidecarAutoScroll}
                  onChange={(event) => setSidecarAutoScroll(event.target.checked)}
                />
                Auto-scroll
              </label>
              <button onClick={() => setSidecarEvents([])}>Limpar sidecar</button>
              <button onClick={() => exportLogs("sidecar")}>Export sidecar</button>
            </div>
            <ul className="log" ref={sidecarLogRef}>
              {filteredSidecarEvents.map((event, index) => (
                <li key={`${event.command}-${event.session_id ?? "none"}-${index}`}>
                  {event.command} | {event.response_kind} | {event.session_id ?? "n/a"} |{" "}
                  {event.detail ?? "n/a"}
                </li>
              ))}
            </ul>
          </article>
        </section>
      </section>
    </main>
  );
}

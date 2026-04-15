import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

type ListenerCallback = (event: { payload: unknown }) => void;

const listeners = new Map<string, ListenerCallback>();
const invokeMock = vi.fn();
const listenMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: unknown) => invokeMock(command, args)
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (eventName: string, callback: ListenerCallback) => listenMock(eventName, callback)
}));

describe("App integration", () => {
  const scrollToMock = vi.fn();

  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    listeners.clear();
    invokeMock.mockReset();
    listenMock.mockReset();
    scrollToMock.mockReset();
    localStorage.clear();
    Object.defineProperty(HTMLElement.prototype, "scrollTo", {
      configurable: true,
      value: scrollToMock
    });

    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "runtime_health":
          return Promise.resolve("ok:warm");
        case "runtime_capabilities":
          return Promise.resolve([
            {
              id: "local-cli-bootstrap",
              transport: "cli",
              privacy_level: "local-first"
            }
          ]);
        case "runtime_sidecar_health":
          return Promise.resolve("ok:runtime-sidecar");
        case "runtime_start_session":
          return Promise.resolve({
            session_id: "session-1",
            active_pack: "companion",
            runtime_mode: "hybrid"
          });
        case "runtime_stop_session":
          return Promise.resolve(null);
        case "runtime_voice_start":
          return Promise.resolve({
            event_type: "voice_session_started",
            session_id: "session-1",
            locale: "pt-BR"
          });
        case "runtime_voice_stop":
          return Promise.resolve(null);
        case "runtime_voice_input_chunk":
          return Promise.resolve({
            event_type: "voice_input_chunk_accepted",
            session_id: "session-1",
            chunk_size_bytes: 512
          });
        case "runtime_voice_output_chunk":
          return Promise.resolve({
            event_type: "voice_output_chunk_ready",
            session_id: "session-1",
            chunk_size_bytes: 1024,
            mime_type: "audio/pcm"
          });
        default:
          return Promise.reject(new Error(`unexpected command: ${command}`));
      }
    });

    listenMock.mockImplementation((eventName: string, callback: ListenerCallback) => {
      listeners.set(eventName, callback);
      return Promise.resolve(() => {
        listeners.delete(eventName);
      });
    });
  });

  it("renders runtime state and processes start/stop with sidecar events", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText(/Status runtime: ok:warm/i)).toBeInTheDocument();
    expect(screen.getByText(/Status sidecar: ok:runtime-sidecar/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /iniciar sessao/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_start_session", {
        activePack: "companion",
        runtimeMode: "hybrid"
      });
    });

    emitEvent("runtime://session_event", {
      event_type: "session_started",
      session_id: "session-1",
      active_pack: "companion"
    });
    emitEvent("runtime://session_event", {
      event_type: "runtime_heartbeat",
      session_id: "session-1",
      active_pack: "companion",
      status: "ok"
    });
    emitEvent("runtime://sidecar_event", {
      command: "session_started",
      response_kind: "ack",
      session_id: "session-1"
    });

    expect(await screen.findByText(/session_started \| session-1/i)).toBeInTheDocument();
    expect(await screen.findByText(/session_started \| ack \| session-1/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /parar sessao/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_stop_session", undefined);
    });

    emitEvent("runtime://sidecar_event", {
      command: "shutdown",
      response_kind: "bye",
      session_id: undefined
    });
    expect(await screen.findByText(/shutdown \| bye \| n\/a/i)).toBeInTheDocument();
  });

  it("sends selected runtime mode in start command", async () => {
    const user = userEvent.setup();
    render(<App />);

    const modeSelect = await screen.findByLabelText(/runtime mode/i);
    await user.selectOptions(modeSelect, "local");
    await user.click(screen.getByRole("button", { name: /iniciar sessao/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_start_session", {
        activePack: "companion",
        runtimeMode: "local"
      });
    });
  });

  it("filters runtime and sidecar event lists", async () => {
    const user = userEvent.setup();
    render(<App />);

    emitEvent("runtime://session_event", {
      event_type: "runtime_heartbeat",
      session_id: "s-1",
      active_pack: "companion",
      status: "ok"
    });
    emitEvent("runtime://session_event", {
      event_type: "session_started",
      session_id: "s-2",
      active_pack: "companion"
    });
    emitEvent("runtime://sidecar_event", {
      command: "runtime_heartbeat",
      response_kind: "ack",
      session_id: "s-1"
    });
    emitEvent("runtime://sidecar_event", {
      command: "shutdown",
      response_kind: "bye"
    });

    await user.type(screen.getByPlaceholderText(/filtrar runtime events/i), "heartbeat");
    expect(screen.getByText(/runtime_heartbeat \| s-1/i)).toBeInTheDocument();
    expect(screen.queryByText(/session_started \| s-2/i)).not.toBeInTheDocument();

    await user.type(screen.getByPlaceholderText(/filtrar sidecar events/i), "shutdown");
    expect(screen.getByText(/shutdown \| bye/i)).toBeInTheDocument();
    expect(screen.queryByText(/runtime_heartbeat \| ack \| s-1/i)).not.toBeInTheDocument();
  });

  it("clears runtime and sidecar logs via panel actions", async () => {
    const user = userEvent.setup();
    render(<App />);

    emitEvent("runtime://session_event", {
      event_type: "runtime_heartbeat",
      session_id: "s-clear",
      active_pack: "companion",
      status: "ok"
    });
    emitEvent("runtime://sidecar_event", {
      command: "runtime_heartbeat",
      response_kind: "ack",
      session_id: "s-clear"
    });

    expect(await screen.findByText(/runtime_heartbeat \| s-clear/i)).toBeInTheDocument();
    expect(await screen.findByText(/runtime_heartbeat \| ack \| s-clear/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /limpar runtime/i }));
    await user.click(screen.getByRole("button", { name: /limpar sidecar/i }));

    expect(screen.queryByText(/runtime_heartbeat \| s-clear/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/runtime_heartbeat \| ack \| s-clear/i)).not.toBeInTheDocument();
  });

  it("allows disabling auto-scroll for runtime and sidecar logs", async () => {
    const user = userEvent.setup();
    render(<App />);

    await screen.findByText(/Status runtime: ok:warm/i);

    const initialScrollCalls = scrollToMock.mock.calls.length;

    emitEvent("runtime://session_event", {
      event_type: "runtime_heartbeat",
      session_id: "s-auto",
      active_pack: "companion",
      status: "ok"
    });
    emitEvent("runtime://sidecar_event", {
      command: "runtime_heartbeat",
      response_kind: "ack",
      session_id: "s-auto"
    });

    await waitFor(() => {
      expect(scrollToMock.mock.calls.length).toBeGreaterThan(initialScrollCalls);
    });

    const autoScrollToggles = screen.getAllByRole("checkbox");
    await user.click(autoScrollToggles[0]);
    await user.click(autoScrollToggles[1]);

    const disabledCalls = scrollToMock.mock.calls.length;

    emitEvent("runtime://session_event", {
      event_type: "runtime_heartbeat",
      session_id: "s-auto-2",
      active_pack: "companion",
      status: "ok"
    });
    emitEvent("runtime://sidecar_event", {
      command: "runtime_heartbeat",
      response_kind: "ack",
      session_id: "s-auto-2"
    });

    await waitFor(() => {
      expect(screen.getByText(/runtime_heartbeat \| s-auto-2/i)).toBeInTheDocument();
      expect(screen.getByText(/runtime_heartbeat \| ack \| s-auto-2/i)).toBeInTheDocument();
    });
    expect(scrollToMock.mock.calls.length).toBe(disabledCalls);
  });

  it("restores persisted panel preferences from localStorage", async () => {
    localStorage.setItem("companion.runtime_mode", "local");
    localStorage.setItem("companion.event_limit", "25");
    localStorage.setItem("companion.runtime_autoscroll", "false");
    localStorage.setItem("companion.sidecar_autoscroll", "false");

    render(<App />);

    const modeSelect = (await screen.findByLabelText(/runtime mode/i)) as HTMLSelectElement;
    const limitInput = screen.getByLabelText(/limite de eventos/i) as HTMLInputElement;
    const toggles = screen.getAllByRole("checkbox") as HTMLInputElement[];

    expect(modeSelect.value).toBe("local");
    expect(limitInput.value).toBe("25");
    expect(toggles[0].checked).toBe(false);
    expect(toggles[1].checked).toBe(false);
  });

  it("starts and stops voice session through tauri commands", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /iniciar sessao/i }));
    await user.click(screen.getByRole("button", { name: /iniciar voz/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_voice_start", {
        locale: "pt-BR"
      });
    });
    expect(screen.getByText(/Status voz: ativa \(pt-BR\)/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /parar voz/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_voice_stop", undefined);
    });
    expect(screen.getByText(/Status voz: inativa/i)).toBeInTheDocument();
  });

  it("sends voice input and output chunks while voice session is active", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /iniciar sessao/i }));
    await user.click(screen.getByRole("button", { name: /iniciar voz/i }));

    await user.click(screen.getByRole("button", { name: /enviar chunk input/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_voice_input_chunk", {
        chunkSizeBytes: 512
      });
    });
    await waitFor(() => {
      expect(screen.getByText(/Ultimo evento voz: input:512 bytes/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /publicar chunk output/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("runtime_voice_output_chunk", {
        mimeType: "audio/pcm",
        chunkSizeBytes: 1024
      });
    });
    await waitFor(() => {
      expect(
        screen.getByText(/Ultimo evento voz: output:1024 bytes \(audio\/pcm\)/i)
      ).toBeInTheDocument();
    });
  });
});

function emitEvent(eventName: string, payload: unknown) {
  const listener = listeners.get(eventName);
  if (!listener) {
    throw new Error(`listener not registered for ${eventName}`);
  }
  listener({ payload });
}

# Prototype Manual Test Checklist

## Objetivo

Validar o prototipo desktop com observabilidade runtime+sidecar usando comandos reais do backend Tauri.

## Pre-condicoes

1. Build sidecar preparado:
   - `powershell -ExecutionPolicy Bypass -File tools/scripts/prepare-sidecar.ps1`
2. Sidecar habilitado:
   - PowerShell: `$env:COMPANION_ENABLE_SIDECAR='true'`
3. Executar app:
   - `npm run tauri:dev --workspace @companion/desktop`

## Fluxo de teste

1. Abrir painel e confirmar:
   - `Status runtime` nao esta em erro.
   - `Status sidecar` responde `ok:runtime-sidecar` ao clicar `Health sidecar`.
2. Selecionar `runtime mode` (`local`, `cloud`, `hybrid`) e iniciar sessao.
3. Validar bloco `Sessao ativa` com `session_id`, `active_pack`, `runtime_mode`.
4. Validar `Eventos runtime`:
   - `session_started`
   - `runtime_heartbeat`
   - `session_stopped` apos parar.
5. Validar `Eventos sidecar`:
   - `session_started | ack`
   - `runtime_heartbeat | ack`
   - `session_stopped | ack`
   - `shutdown | bye`

## Resultado esperado

- Sem erro de start/stop de sessao no painel.
- Eventos runtime e sidecar atualizam em tempo real.
- Sidecar fecha corretamente apos `Parar sessao`.

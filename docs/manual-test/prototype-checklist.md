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
6. Validar sessao de voz:
   - Clicar `Iniciar voz` e confirmar `Status voz: ativa (pt-BR)`.
   - Clicar `Enviar chunk input` e validar `Ultimo evento voz: input:<bytes>`.
   - Clicar `Publicar chunk output` e validar `Ultimo evento voz: output:<bytes> (<mime>)`.
   - Clicar `Parar voz` e confirmar `Status voz: inativa`.
7. Validar configuracao de chunk no painel:
   - Ajustar `Chunk input (bytes)`, `Chunk output (bytes)` e `MIME output`.
   - Repetir envio/publicacao e confirmar que o valor refletido no `Ultimo evento voz` muda conforme configurado.

## Resultado esperado

- Sem erro de start/stop de sessao no painel.
- Eventos runtime e sidecar atualizam em tempo real.
- Sidecar fecha corretamente apos `Parar sessao`.
- Fluxo de voz (start/chunk/chunk/stop) funciona sem erro.

# Backlog Tecnico - Fase 1

## Objetivo da fase

Subir base tecnica com contratos, registry e desktop shell inicial.

## Entregas desta iteracao

1. Monorepo inicial
2. `runtime-core` com contratos e capability registry
3. App desktop base (React + Tauri)
4. ADR da fundacao
5. `SessionContext` e eventos runtime->UI (`session_started`, `runtime_heartbeat`, `session_stopped`)
6. Policy de score separada em modulo dedicado (`runtime-core::policy`)
7. Fixtures de capability manifests por modo em `tools/fixtures/capability-manifests/*`
8. Crate inicial `agent-supervisor` com estados `cold/warm/hot`
9. `SessionId` unico por sessao
10. Runtime app extraido para modulo dedicado (`runtime_app.rs`)
11. Integracao de lifecycle de sessao com `agent-supervisor` (hot/cold)
12. Teste de sequencia `session_started -> runtime_heartbeat -> session_stopped`
13. Eventos de sessao unificados via tipo compartilhado (`runtime-core::SessionEvent`)
14. Heartbeat configuravel por ambiente (`COMPANION_HEARTBEAT_INTERVAL_MS`)
15. Teste de integracao de runtime app cobrindo start/stop + estado de agente
16. Crate `policy-engine` iniciado e integrado ao runtime para filtro de backend por politica
17. `RuntimeSessionContextPayload` movido para tipo compartilhado no `runtime-core`
18. Fluxo command-level coberto por testes no boundary de comandos do backend desktop
19. Heartbeat adaptativo por modo (`local/cloud/hybrid`) no runtime loop
20. Regras de bloqueio de upload por workspace/policy no `policy-engine`
21. Cobertura IPC end-to-end com harness de teste Tauri (`mock_builder` + invokes reais)
22. Inicializacao de `workspace_policy` por ambiente (`COMPANION_POLICY_*`)
23. Runtime mode por sessao exposto no comando `runtime_start_session`
24. Sidecar separado iniciado via crate `runtime-sidecar` + health check de processo no desktop
25. Lifecycle de sessao conectado ao sidecar `--stdio` com rollout seguro por `COMPANION_ENABLE_SIDECAR`
26. Bundle/config Tauri com `externalBin` estabilizado via `build.rs` (build+copia do sidecar por target)
27. Script de preparo de artefato sidecar para release (`tools/scripts/prepare-sidecar.ps1`)
28. Protocolo sidecar `--stdio` evoluido para eventos de sessao em JSON (`session_started/heartbeat/session_stopped/shutdown`)
29. Smoke test de bundle sidecar por target adicionado (`tools/scripts/smoke-sidecar-bundle.ps1` + workflow CI)
30. Smoke de protocolo sidecar com processo real validando respostas `ack/bye`
31. Eventos de telemetria sidecar acoplados ao backend UI (`runtime://sidecar_event`)
32. Workflow de smoke sidecar expandido para matriz Windows/Linux/macOS
33. Prototipo desktop manual com painel de observabilidade runtime+sidecar
34. Checklist de teste manual do prototipo (`docs/manual-test/prototype-checklist.md`)
35. Teste automatizado de integracao UI para fluxo start/stop com sidecar (`apps/desktop/src/App.integration.test.tsx`)
36. Teste UI com assertiva de encerramento sidecar (`shutdown|bye`) no fluxo start/stop
37. UX do painel evoluida com filtro/busca e exportacao JSON de logs runtime/sidecar
38. Painel com limite configuravel de eventos e acoes de limpeza de logs
39. Teste de roundtrip com sidecar real no backend (`session_started -> heartbeat -> session_stopped -> shutdown`)
40. Toggle de auto-scroll adicionado para logs runtime/sidecar com controle por painel
41. Fluxo E2E de janela Tauri real adicionado via WebdriverIO + `tauri-driver` (gated por env)
42. Smoke E2E real conectado ao CI Windows com provisionamento de `tauri-driver` + `EdgeDriver`
43. Script operacional para branch protection GitHub com checks obrigatorios do desktop

## Itens imediatos (proximo commit)

1. Executar `tools/scripts/apply-branch-protection.ps1` no repositorio remoto alvo (ORG/REPO)

## Riscos em aberto

- build desktop depende de setup completo Tauri no ambiente
- tempo de execucao do smoke E2E real no CI Windows pode impactar tempo total de pipeline
- gate de branch protection ainda depende de aplicacao no repositorio remoto (fora deste workspace)

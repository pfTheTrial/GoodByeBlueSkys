# Tauri E2E Smoke (janela real)

## Objetivo

Validar fluxo UI com janela Tauri real (sem mocks no frontend) para o caminho minimo `Iniciar sessao -> Parar sessao`.

## Pre-requisitos

1. Toolchain Rust + Node instalados
2. Driver Tauri instalado:
   - `cargo install tauri-driver`
3. Windows: Edge WebDriver instalado:
   - `winget install --id Microsoft.EdgeDriver --silent`
4. (Opcional) caminho custom do driver:
   - `TAURI_DRIVER_PATH=<caminho-do-tauri-driver>`
5. (Opcional) caminho custom do native driver:
   - `EDGE_WEBDRIVER_PATH=<caminho-do-msedgedriver>`

## Execucao

1. No root do repositorio:
   - PowerShell:
     - `$env:COMPANION_RUN_TAURI_E2E='1'`
     - `npm run desktop:test-e2e`
2. Sem a variavel `COMPANION_RUN_TAURI_E2E=1`, o comando apenas faz skip seguro.

## CI

1. Workflow dedicado: `.github/workflows/tauri-e2e-smoke.yml`
2. Runner: `windows-latest`
3. Provisionamento no job:
   - `Microsoft.EdgeDriver` (quando ausente)
   - `tauri-driver` (quando ausente)

## O que o smoke valida

1. Janela abre com titulo `Companion Platform`
2. Header principal renderiza
3. Botao `Iniciar sessao` ativa sessao
4. Botao `Parar sessao` retorna para `sem sessao ativa`

## Notas de risco

1. Se o `tauri-driver` ou `msedgedriver` nao existir no ambiente, o fluxo falha com erro explicito.
2. O smoke depende de build debug do app (`tauri build --debug --no-bundle`), portanto pode demorar mais que testes de unidade.

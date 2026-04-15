# Arquivos para publicar no GitHub

## Objetivo

Garantir que apenas código e documentação necessários para o app sejam publicados.

## Entram no repositório

1. `apps/` (código do desktop e configuração Tauri)
2. `crates/` (runtime/core/policy/supervisor/sidecar)
3. `docs/` (arquitetura, testes, operação)
4. `tools/` (scripts de build/test/release)
5. `.github/workflows/` (pipelines CI)
6. Arquivos raiz de build e governança:
   - `Cargo.toml`, `Cargo.lock`
   - `package.json`, `package-lock.json`
   - `README.md`, `AGENTS.md`, `RULES.md`, `WORKFLOW.md`
   - `.gitignore`

## Não entram no repositório

1. Segredos e configuração local (`.env*`, certificados/chaves)
2. Artefatos gerados (`node_modules`, `target`, `dist`, `coverage`)
3. Logs temporários (`*.log`, `tauri-dev.*.log`)
4. Binários gerados do sidecar em `apps/desktop/src-tauri/binaries/*` (mantém apenas `README.md`)
5. Pastas locais de tooling/assistente (`.agents/`, `.codex/`, `_caveman-src/`)

## Checklist pré-push

1. Validar `.gitignore` atualizado.
2. Confirmar que nenhum arquivo de segredo aparece em staging.
3. Confirmar que não há logs/binários temporários no commit.

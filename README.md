# Companion Platform

Base inicial da Fase 1.

## Objetivo

Criar fundacao tecnica para companion desktop Windows-first com:

- monorepo organizado
- runtime sidecar/core em Rust
- shell desktop em Tauri 2
- frontend React + TypeScript
- contratos centrais e capability registry

## Estrutura

- `apps/desktop`: app desktop (UI + shell)
- `crates/runtime-core`: contratos e roteamento por capacidade
- `providers/`: adapters por provider (placeholder)
- `packs/`: packs de comportamento (placeholder)
- `tools/`: utilitarios e fixtures (placeholder)
- `docs/architecture`: ADRs e backlog tecnico

## Validacao executada na Fase 1

- `cargo check -p runtime-core`
- checklist estrutural de diretorios e arquivos base

## Proximo passo

Fase 1.1: iniciar fluxo runtime->UI via evento stream e comandos Tauri para sessao ativa.

# GoodByeBlueSkys

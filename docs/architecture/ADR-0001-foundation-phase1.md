# ADR-0001 - Foundation Phase 1

## Status

Accepted (2026-04-14)

## Contexto

A plataforma precisa nascer Windows-first, agnostica de provider e com baixo acoplamento entre UI e runtime.

## Decisao

1. Monorepo com separacao clara:
   - `apps/*` para aplicacoes
   - `crates/*` para core/runtime
   - `providers/*` para adapters
   - `packs/*` para comportamento
2. Runtime em Rust com contratos centrais em `runtime-core`.
3. Desktop shell em Tauri 2 com frontend React + TypeScript.
4. Capability registry como mecanismo base para roteamento por politica/capacidade.

## Consequencias

- Positivo: base desacoplada para local/cloud/hibrido.
- Positivo: facilita testes de core sem depender de UI.
- Negativo: setup inicial mais amplo que app unico.
- Risco: bridge runtime->UI ainda minimalista na Fase 1.

## Validacao obrigatoria

- `cargo check -p runtime-core`
- comandos Tauri expostos para `runtime_health` e `runtime_capabilities`
- contratos centrais versionados em crate dedicado


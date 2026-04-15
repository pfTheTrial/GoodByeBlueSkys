# Baseline Arquitetural - Companion Platform

Documento base derivado da proposta tecnica.

## Stack alvo

- Runtime/Core: Rust
- Desktop shell: Tauri 2
- UI: React + TypeScript
- Estilo UI: Tailwind + shadcn/ui
- Storage local: SQLite

## Diretrizes de arquitetura

- Windows-first
- provider-agnostic
- agent-agnostic
- runtime sidecar separado da UI
- roteamento por capacidade e politica
- modos local, cloud e hibrido

## Packs planejados

- companion
- tutor
- coding
- medical (separado do core)
- gov-training

## Roadmap tecnico (macro)

1. Fundacao: monorepo, app desktop, runtime sidecar, contratos centrais
2. Voz: STT/TTS multiprovider (local e cloud)
3. Providers: CLI, MCP, bridge, API, Ollama
4. Screen understanding: acessibilidade + parser visual
5. Packs: companion/tutor/coding, depois medical e gov-training

## Riscos principais

- acoplamento precoce a provider
- expansao de escopo na fase de fundacao
- regressao por falta de gates de validacao
- indefinicao de politica local/cloud para dados sensiveis

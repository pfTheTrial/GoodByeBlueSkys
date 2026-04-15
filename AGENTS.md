# AGENTS.md - Companion Platform (Base)

Fonte única para operação de agentes neste repositório.

## Objetivo

Construir base Windows-first de companion multimodal com:

- runtime sidecar em Rust
- shell desktop em Tauri 2
- frontend em React + TypeScript
- roteamento por capacidade (provider-agnostic e agent-agnostic)

## Ordem de precedência

1. `AGENTS.md`
2. `RULES.md`
3. `WORKFLOW.md`
4. `docs/architecture/*`
5. arquivos de skill em `.agents/skills/*`

## Estrutura operacional

- Fluxo de trabalho: `WORKFLOW.md`
- Regras de engenharia: `RULES.md`
- Perfis de agentes: `agents/*`
- Skills de execução: `skills/*`

Nota de ambiente: neste workspace, escrita em `.agents/*` esta bloqueada por permissao. A base operacional foi criada em paths locais do repositorio.

## Baseline de escopo (fase atual)

- Esta entrega cobre base de governança e execução.
- Não cobre scaffold completo de app/runtime.
- Próxima entrega deve iniciar monorepo e contratos centrais.

## Princípios

- Clareza > esperteza
- Passos pequenos e verificáveis
- Risco/regressão antes de velocidade
- Validação objetiva antes de concluir

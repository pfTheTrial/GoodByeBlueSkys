# RULES.md

Regras obrigatórias para engenharia neste repositório.

## Escopo e mudança

- Escopo pequeno por ticket.
- Um comportamento principal por commit.
- Não incluir melhorias oportunistas fora do objetivo.

## Segurança e configuração

- Não hardcodar segredos.
- Segredos via ambiente/secret store.
- Dados sensíveis mascarados em logs.

## Arquitetura

- Core agnóstico de provider e agente.
- Roteamento por capacidade e política.
- Runtime sidecar separado da UI.
- Pack medical isolado do core.

## Qualidade

- Nomes explícitos e sem abreviações ambíguas.
- Código claro antes de compacto.
- Comentar somente decisões não óbvias.

## Gates de bloqueio

Bloquear merge quando:

- escopo do ticket expandiu sem aprovação
- validação obrigatória não executada
- risco de regressão não avaliado
- política de privacidade/segurança quebrada

## Definição de pronto (base)

- documentação atualizada (`AGENTS.md`, `WORKFLOW.md`, `RULES.md`)
- handoff para Builder anexado quando houver mudança de código
- validação executada ou pendente declarada explicitamente


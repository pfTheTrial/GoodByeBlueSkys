# WORKFLOW.md

Workflow padrão para tickets do projeto.

## 0. Entrada

Todo ticket deve declarar:

- objetivo
- escopo
- fora de escopo
- critérios de aceite
- riscos esperados

Sem isso: bloquear início.

## 1. Planejamento curto

Saída obrigatória:

- plano em passos pequenos
- hipótese de risco/regressão
- validação objetiva por passo

Gate:

- escopo pequeno e verificável
- no máximo 1 comportamento novo por commit

## 2. Implementação

Regras:

- não misturar mudanças não relacionadas
- manter interfaces explícitas e desacopladas de provider
- evitar refatoração paralela sem necessidade do ticket

Gate:

- código compila
- comportamento alvo implementado

## 3. Validação

Sempre registrar:

- validação executada
- validação pendente
- riscos residuais

Checklist mínima:

- testes de unidade (quando aplicável)
- teste manual do fluxo alterado
- checagem de regressão básica no caminho crítico

Sem validação registrada: não concluir entrega.

## 4. Handoff para Builder

Quando houver implementação:

- contexto curto do ticket
- decisões técnicas tomadas
- arquivos alterados
- passos para validar localmente
- próximos passos priorizados

Formato recomendado:

1. Objetivo implementado
2. Escopo entregue
3. Fora de escopo mantido
4. Riscos e lacunas de teste
5. Como validar
6. Próximo commit sugerido


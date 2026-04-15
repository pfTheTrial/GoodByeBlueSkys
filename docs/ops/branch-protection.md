# Branch Protection (GitHub)

## Objetivo

Exigir checks de CI desktop antes de merge em `main`/`master`.

## Pre-requisitos

1. `gh` instalado e autenticado (`gh auth login`)
2. Permissao de admin/maintainer no repositório

## Script

Arquivo: `tools/scripts/apply-branch-protection.ps1`

## Exemplo de execucao

PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File tools/scripts/apply-branch-protection.ps1 `
  -Repo "ORG/REPO" `
  -Branch "main" `
  -RequiredChecks @("tauri-e2e-smoke","sidecar-bundle-smoke")
```

## Resultado esperado

1. Branch protegida com:
   - checks obrigatorios (`tauri-e2e-smoke`, `sidecar-bundle-smoke`)
   - `strict` para status checks
   - 1 aprovacao minima em PR
   - resolucao obrigatoria de conversas

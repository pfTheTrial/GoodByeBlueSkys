param(
  [Parameter(Mandatory = $true)]
  [string]$Repo,
  [string]$Branch = "main",
  [string[]]$RequiredChecks = @("tauri-e2e-smoke", "sidecar-bundle-smoke"),
  [switch]$IncludeAdmins
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
  $PSNativeCommandUseErrorActionPreference = $false
}

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "gh CLI nao encontrado. Instale em https://cli.github.com/"
}

$authStatus = $null
try {
  $authStatus = & gh auth status 2>$null
}
catch {
  $authStatus = $null
}
if ($LASTEXITCODE -ne 0) {
  throw "gh CLI sem autenticacao valida. Execute: gh auth login"
}

$branchLookup = $null
try {
  $branchLookup = & gh api "/repos/$Repo/branches/$Branch" 2>$null
}
catch {
  $branchLookup = $null
}
if ($LASTEXITCODE -ne 0) {
  throw "Branch '$Branch' nao encontrada em '$Repo'. Crie/push da branch antes de aplicar branch protection."
}

$normalizedChecks = @()
foreach ($check in $RequiredChecks) {
  if ($null -eq $check) {
    continue
  }
  $parts = $check -split ","
  foreach ($part in $parts) {
    if ($null -eq $part) {
      continue
    }
    $trimmed = $part.Trim()
    $trimmed = $trimmed -replace '\\\"', ''
    $trimmed = $trimmed.Trim().Trim('"').Trim("'")
    if ($trimmed.Length -gt 0) {
      $normalizedChecks += $trimmed
    }
  }
}

if ($normalizedChecks.Count -eq 0) {
  throw "RequiredChecks vazio. Informe ao menos 1 check."
}

$payload = @{
  required_status_checks = @{
    strict = $true
    contexts = $normalizedChecks
  }
  enforce_admins = [bool]$IncludeAdmins
  required_pull_request_reviews = @{
    required_approving_review_count = 1
    dismiss_stale_reviews = $true
    require_code_owner_reviews = $false
  }
  restrictions = $null
  required_linear_history = $false
  allow_force_pushes = $false
  allow_deletions = $false
  block_creations = $false
  required_conversation_resolution = $true
  lock_branch = $false
  allow_fork_syncing = $false
}

$payloadJson = $payload | ConvertTo-Json -Depth 10 -Compress

Write-Host "Aplicando branch protection em ${Repo}:${Branch} ..."

$ghOutput = $null
try {
  $ghOutput = $payloadJson | & gh api --method PUT "/repos/$Repo/branches/$Branch/protection" `
    --header "Accept: application/vnd.github+json" `
    --input - 2>&1
}
catch {
  $ghOutput = $_.Exception.Message
}
if ($LASTEXITCODE -ne 0) {
  throw "Falha ao aplicar branch protection: $ghOutput"
}

Write-Host "Branch protection aplicada com sucesso."
Write-Host "Checks exigidos: $($normalizedChecks -join ', ')"

param(
    [string]$Target = "",
    [switch]$Release = $true
)

$ErrorActionPreference = "Stop"

$workspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$srcTauriDir = Join-Path $workspaceRoot "apps\desktop\src-tauri"
$binariesDir = Join-Path $srcTauriDir "binaries"

$resolvedTarget = $Target
if ([string]::IsNullOrWhiteSpace($resolvedTarget)) {
    $resolvedTarget = (& rustc -vV | Select-String "^host:\s+" | ForEach-Object {
        $_.Line.Split(":")[1].Trim()
    })
    if ([string]::IsNullOrWhiteSpace($resolvedTarget)) {
        throw "failed to resolve host target from rustc -vV"
    }
}

$cargoArgs = @("build", "-p", "runtime-sidecar", "--target", $resolvedTarget)
if ($Release) {
    $cargoArgs += "--release"
}

Push-Location $workspaceRoot
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build runtime-sidecar failed"
    }
} finally {
    Pop-Location
}

$profileDir = if ($Release) { "release" } else { "debug" }
$extension = if ($resolvedTarget -like "*windows*") { ".exe" } else { "" }
$sidecarName = "runtime-sidecar$extension"
$sourceBinary = Join-Path $workspaceRoot "target\$resolvedTarget\$profileDir\$sidecarName"
$targetBinary = Join-Path $binariesDir "runtime-sidecar-$resolvedTarget$extension"

if (-not (Test-Path $sourceBinary)) {
    throw "sidecar binary not found at: $sourceBinary"
}

New-Item -Path $binariesDir -ItemType Directory -Force | Out-Null
Copy-Item -Path $sourceBinary -Destination $targetBinary -Force

Write-Output "Prepared sidecar binary: $targetBinary"

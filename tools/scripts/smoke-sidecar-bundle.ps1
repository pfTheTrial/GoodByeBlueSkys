param(
    [string]$Target = ""
)

$ErrorActionPreference = "Stop"

$workspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$prepareScript = Join-Path $workspaceRoot "tools\scripts\prepare-sidecar.ps1"

$resolvedTarget = $Target
if ([string]::IsNullOrWhiteSpace($resolvedTarget)) {
    $resolvedTarget = (& rustc -vV | Select-String "^host:\s+" | ForEach-Object {
        $_.Line.Split(":")[1].Trim()
    })
    if ([string]::IsNullOrWhiteSpace($resolvedTarget)) {
        throw "failed to resolve host target from rustc -vV"
    }
}

$extension = if ($resolvedTarget -like "*windows*") { ".exe" } else { "" }
$binaryPath = Join-Path $workspaceRoot "apps\desktop\src-tauri\binaries\runtime-sidecar-$resolvedTarget$extension"

& $prepareScript -Target $resolvedTarget
if ($LASTEXITCODE -ne 0) {
    throw "prepare-sidecar script failed"
}

Push-Location $workspaceRoot
try {
    & cargo check -p runtime-sidecar
    if ($LASTEXITCODE -ne 0) {
        throw "cargo check -p runtime-sidecar failed during sidecar smoke test"
    }
} finally {
    Pop-Location
}

if (-not (Test-Path $binaryPath)) {
    throw "expected sidecar binary not found at: $binaryPath"
}

function Invoke-SidecarProtocolSmoke {
    param(
        [string]$SidecarBinPath
    )

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = New-Object System.Diagnostics.ProcessStartInfo
    $process.StartInfo.FileName = $SidecarBinPath
    $process.StartInfo.Arguments = "--stdio"
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardInput = $true
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    $process.StartInfo.CreateNoWindow = $true

    if (-not $process.Start()) {
        throw "failed to start sidecar process for protocol smoke test"
    }

    try {
        $messages = @(
            '{"kind":"session_started","session_id":"smoke-session","active_pack":"companion","runtime_mode":"hybrid","assigned_agent_id":"companion-agent"}',
            '{"kind":"runtime_heartbeat","session_id":"smoke-session","status":"ok"}',
            '{"kind":"session_stopped","session_id":"smoke-session","reason":"smoke_test"}',
            '{"kind":"shutdown"}'
        )

        foreach ($message in $messages) {
            $process.StandardInput.WriteLine($message)
            $process.StandardInput.Flush()
            $responseLine = $process.StandardOutput.ReadLine()
            if ([string]::IsNullOrWhiteSpace($responseLine)) {
                throw "empty response from sidecar protocol smoke test"
            }

            $response = $responseLine | ConvertFrom-Json
            if ($message -eq '{"kind":"shutdown"}') {
                if ($response.kind -ne "bye") {
                    throw "expected sidecar bye response on shutdown, got: $responseLine"
                }
            }
            elseif ($response.kind -ne "ack") {
                throw "expected sidecar ack response, got: $responseLine"
            }
        }
    }
    finally {
        if (-not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
        $process.Dispose()
    }
}

Invoke-SidecarProtocolSmoke -SidecarBinPath $binaryPath

Write-Output "Sidecar bundle smoke test passed for target: $resolvedTarget"

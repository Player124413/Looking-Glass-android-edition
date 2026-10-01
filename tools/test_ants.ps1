# Run in the background desktop. Art and restart fixtures stay private.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$antRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $antRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/ants | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('ant-check', 'ant-render-check', 'ant-save-write', 'ant-save-read', 'pool-check', 'potears2-check')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/ants/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/ants/report.json
        if ($code -ne 0) { throw "Ant regression failed: $check ($code). See private/ants/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }

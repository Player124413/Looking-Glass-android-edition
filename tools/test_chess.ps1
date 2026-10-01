# Native checks must run in the background desktop. Never uses player saves.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$chessRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $chessRoot
try {
    $Executable = (Resolve-Path -LiteralPath $Executable).Path
    New-Item -ItemType Directory -Force -Path private/chess | Out-Null
    $report = [ordered]@{
        executable = $Executable
        sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
        checks = @()
    }
    foreach ($check in @('chess-check', 'chess-render-check', 'chess-save-write', 'chess-save-read')) {
        $ErrorActionPreference = 'Continue'
        & $Executable "--$check" --no-audio *> "private/chess/$check.log"
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        $report.checks += [ordered]@{ name = $check; exit_code = $code }
        $report | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 private/chess/report.json
        if ($code -ne 0) { throw "Chess regression failed: $check ($code). See private/chess/$check.log" }
        Write-Host "PASS $check"
    }
}
finally { Pop-Location }

@echo off
setlocal DisableDelayedExpansion
set "LG_SETUP_FILE=%~f0"
set "LG_SETUP_SOURCE=%~1"
set "LG_SETUP_NONINTERACTIVE=%~2"
if "%~1"=="" if not "%~2"=="--no-pause" if exist "%~dp0tools\windows_setup.ps1" (
    powershell.exe -NoLogo -NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File "%~dp0tools\windows_setup.ps1"
    exit /b
)
powershell.exe -NoLogo -NoProfile -Command "$text = [IO.File]::ReadAllText($env:LG_SETUP_FILE); & ([ScriptBlock]::Create(($text -split '# POWERSHELL_START\r?\n', 2)[1]))"
set "LG_SETUP_EXIT=%ERRORLEVEL%"
if not "%~2"=="--no-pause" pause
exit /b %LG_SETUP_EXIT%
# POWERSHELL_START
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetDirectoryName($env:LG_SETUP_FILE)
$requiredPacks = @('pak0.pk3', 'pak1_large.pk3', 'pak2.pk3', 'pak3.pk3', 'pak4_english.pk3', 'pak5_mod.pk3')
$private = Join-Path $root 'private'
$config = Join-Path $private 'data-path.txt'
$receipt = Join-Path $private 'setup.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Test-Cancelled {
    if ($env:LG_SETUP_CANCEL -and (Test-Path -LiteralPath $env:LG_SETUP_CANCEL)) {
        throw 'Setup cancelled. Your previous game-data setting and saves are unchanged.'
    }
}

function Set-Progress([string]$message, [int]$percent = -1) {
    Write-Host $message
    if ($env:LG_SETUP_STATUS) {
        Save-Text $env:LG_SETUP_STATUS ((@{ message = $message; percent = $percent } | ConvertTo-Json -Compress))
    }
    Test-Cancelled
}

function Find-Base([string]$folder) {
    foreach ($suffix in @('', 'bin\base', 'Alice1\bin\base', 'alice_202106\Alice1\bin\base')) {
        $candidate = if ($suffix) { Join-Path $folder $suffix } else { $folder }
        if (Test-Path -LiteralPath (Join-Path $candidate 'pak0.pk3') -PathType Leaf) {
            return [IO.Path]::GetFullPath($candidate)
        }
    }
    return $null
}

function Test-Base([string]$folder) {
    $maps = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $foundVillage = $false
    foreach ($name in $requiredPacks) {
        Test-Cancelled
        $path = Join-Path $folder $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Missing $name. Select the English 2011 vanilla data or Alice1_2011_vanilla.7z."
        }
        $zip = [IO.Compression.ZipFile]::OpenRead($path)
        try {
            if ($zip.Entries.Count -eq 0) { throw "Empty game archive: $name" }
            foreach ($entry in $zip.Entries) {
                $entryName = $entry.FullName.Replace('\', '/').ToLowerInvariant()
                if ($entryName -match '^maps/[^/]+\.bsp$') { [void]$maps.Add($entryName) }
                if ($entryName -eq 'maps/gvillage.bsp') {
                    $stream = $entry.Open()
                    try {
                        $header = New-Object byte[] 8
                        $read = 0
                        while ($read -lt 8) {
                            $count = $stream.Read($header, $read, 8 - $read)
                            if ($count -eq 0) { throw 'Truncated village map.' }
                            $read += $count
                        }
                        if ([Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'FAKK' -or [BitConverter]::ToInt32($header, 4) -ne 42) {
                            throw 'The village map is not the supported FAKK version 42.'
                        }
                        $foundVillage = $true
                    } finally { $stream.Dispose() }
                }
            }
        } finally { $zip.Dispose() }
    }
    if (-not $foundVillage -or $maps.Count -ne 36) {
        throw "Expected the 36-map English 2011 data set; found $($maps.Count) maps."
    }
    Write-Host 'Validated six game packs and 36 map entries.'
}

function Find-Extractor {
    $bundled = Join-Path $root 'setup\7zr.exe'
    if (Test-Path -LiteralPath $bundled -PathType Leaf) { return $bundled }
    foreach ($name in @('7z.exe', '7zz.exe')) {
        $command = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue
        if ($command) { return $command.Source }
    }
    foreach ($parent in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
        if ($parent) {
            $candidate = Join-Path $parent '7-Zip\7z.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
        }
    }
    throw 'To extract .7z, install 7-Zip from https://www.7-zip.org/ and run Setup.cmd again. Alternatively, extract the archive yourself and select its Alice1 folder.'
}

function Save-Text([string]$path, [string]$text) {
    $temporary = "$path.$([Guid]::NewGuid().ToString('N')).tmp"
    [IO.File]::WriteAllText($temporary, $text, $utf8)
    if (Test-Path -LiteralPath $path) {
        [IO.File]::Replace($temporary, $path, "$path.bak")
    } else {
        [IO.File]::Move($temporary, $path)
    }
}

try {
    Write-Host 'Looking Glass - local game-data setup'
    Write-Host 'Use an existing Alice1 folder or the English 2011 vanilla .7z download.'
    Write-Host 'Download page: https://archive.org/details/alice_202106'
    Write-Host 'File: Alice1_2011_vanilla.7z (use game data you are entitled to use).'
    Write-Host ''
    $source = $env:LG_SETUP_SOURCE
    if ([string]::IsNullOrWhiteSpace($source)) {
        $source = Find-Base $root
        if (-not $source -and (Test-Path -LiteralPath $config)) {
            $saved = [IO.File]::ReadAllText($config).Trim()
            if ($saved) {
                $candidate = if ([IO.Path]::IsPathRooted($saved)) { $saved } else { Join-Path $root $saved }
                $source = Find-Base $candidate
            }
        }
        if (-not $source) {
            foreach ($candidate in @((Join-Path $root 'Alice1_2011_vanilla.7z'), (Join-Path $root 'alice_202106\Alice1_2011_vanilla.7z'))) {
                if (Test-Path -LiteralPath $candidate -PathType Leaf) { $source = $candidate; break }
            }
        }
        if (-not $source) {
            if ($env:LG_SETUP_NONINTERACTIVE -eq '--no-pause') { throw 'No game data found. Pass an archive or folder as the first argument.' }
            $source = Read-Host 'Paste the downloaded .7z path or extracted Alice1 folder (blank cancels)'
        }
    }
    $source = ([string]$source).Trim().Trim('"')
    if (-not $source) { throw 'Setup cancelled; existing configuration was kept.' }
    $source = (Get-Item -LiteralPath $source).FullName
    $archiveHash = $null
    $base = $null
    if (Test-Path -LiteralPath $source -PathType Container) {
        $base = Find-Base $source
        if (-not $base) { throw 'No bin\base\pak0.pk3 found in this folder. Select Alice1, its parent, or bin\base.' }
    } else {
        if ([IO.Path]::GetExtension($source) -ine '.7z') { throw 'Select the .7z download or an extracted game folder.' }
        $extractor = Find-Extractor
        Set-Progress 'Checking the selected archive...'
        $hasher = [Security.Cryptography.SHA256]::Create()
        $archiveStream = [IO.File]::OpenRead($source)
        try {
            $buffer = New-Object byte[] (1024 * 1024)
            while (($count = $archiveStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                Test-Cancelled
                [void]$hasher.TransformBlock($buffer, 0, $count, $null, 0)
            }
            [void]$hasher.TransformFinalBlock($buffer, 0, 0)
            $archiveHash = ([BitConverter]::ToString($hasher.Hash)).Replace('-', '').ToLowerInvariant()
        }
        finally { $archiveStream.Dispose(); $hasher.Dispose() }
        if (Test-Path -LiteralPath $receipt) {
            try {
                $previous = [IO.File]::ReadAllText($receipt) | ConvertFrom-Json
                if ($previous.archiveSha256 -eq $archiveHash) {
                    $candidate = Join-Path $root $previous.dataPath
                    if (Test-Path -LiteralPath $candidate -PathType Container) {
                        Test-Base $candidate
                        $base = $candidate
                        Write-Host 'Reusing the already imported copy of this download.'
                    }
                }
            } catch { Write-Host 'The previous import cannot be reused; making a fresh import.' }
        }
        if (-not $base) {
            # Exact entry names plus flat extraction exclude all original executables,
            # scripts, traversal paths and language subfolders. Never overwrite input.
            $relative = 'private\game-data\import-' + [Guid]::NewGuid().ToString('N') + '\base'
            $base = Join-Path $root $relative
            [void][IO.Directory]::CreateDirectory($base)
            Set-Progress 'Unpacking game files. This may take a few minutes...'
            $entries = @($requiredPacks | ForEach-Object { "Alice1\bin\base\$_" })
            $start = New-Object Diagnostics.ProcessStartInfo
            $start.FileName = $extractor
            $start.UseShellExecute = $false
            $start.CreateNoWindow = $true
            # Paths are literal Windows paths, which cannot contain double quotes.
            $start.Arguments = (@('e', '-y', '-bso0', '-bsp0', "-o$base", '--', $source) + $entries | ForEach-Object { '"' + $_ + '"' }) -join ' '
            $extract = [Diagnostics.Process]::Start($start)
            try {
                while (-not $extract.WaitForExit(200)) { Test-Cancelled }
                if ($extract.ExitCode -ne 0) { throw 'Unpacking failed. Check the archive and free disk space, then try again.' }
            } finally {
                if (-not $extract.HasExited) { $extract.Kill(); $extract.WaitForExit() }
                $extract.Dispose()
            }
        }
    }
    Set-Progress 'Validating game files...'
    Test-Base $base
    Test-Cancelled
    [void][IO.Directory]::CreateDirectory($private)
    $rootPrefix = [IO.Path]::GetFullPath($root).TrimEnd('\') + '\'
    $savedPath = if ($base.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) { $base.Substring($rootPrefix.Length) } else { $base }
    # Publish the data pointer only after every validation succeeds. Saves are untouched.
    Save-Text $config ($savedPath + [Environment]::NewLine)
    Save-Text $receipt (([ordered]@{ schema = 1; source = $source; archiveSha256 = $archiveHash; dataPath = $savedPath } | ConvertTo-Json) + [Environment]::NewLine)
    Write-Host ''
    Write-Host "Game data configured: $base"
    Write-Host 'Ready. Double-click Launch.cmd to play.'
    exit 0
} catch {
    Write-Host ''
    Write-Host ("Setup could not finish: " + $_.Exception.Message) -ForegroundColor Red
    exit 1
}

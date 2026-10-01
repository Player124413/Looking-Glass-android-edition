param(
    [ValidateSet('Setup', 'Play', 'Worker')][string]$Mode = 'Setup',
    [string]$JobFile
)
# Windows PowerShell 5.1; no installed modules, administrator rights or Python.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$utf8 = New-Object Text.UTF8Encoding($false)
$archiveName = 'Alice1_2011_vanilla.7z'
$downloadUrl = 'https://archive.org/download/alice_202106/Alice1_2011_vanilla.7z'
$downloadBytes = 977811158L
$downloadHash = 'b253bb2c9c875f1838a711d0fe4f12f4bc2c6207db3baaE6431af962eca8705d'.ToLowerInvariant()
$packs = @('pak0.pk3', 'pak1_large.pk3', 'pak2.pk3', 'pak3.pk3', 'pak4_english.pk3', 'pak5_mod.pk3')

function Save-State([string]$path, [object]$value) {
    $temp = $path + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
    [IO.File]::WriteAllText($temp, ($value | ConvertTo-Json -Compress), $utf8)
    if (Test-Path -LiteralPath $path) { [IO.File]::Replace($temp, $path, ($path + '.bak')) }
    else { [IO.File]::Move($temp, $path) }
}

function Find-Data([string]$folder) {
    if (-not $folder) { return $null }
    foreach ($suffix in @('', 'bin\base', 'Alice1\bin\base', 'alice_202106\Alice1\bin\base')) {
        $candidate = if ($suffix) { Join-Path $folder $suffix } else { $folder }
        if (@($packs | Where-Object { -not (Test-Path -LiteralPath (Join-Path $candidate $_) -PathType Leaf) }).Count -eq 0) { return $candidate }
    }
    return $null
}

function Read-ConfiguredData {
    $config = Join-Path $root 'private\data-path.txt'
    if (Test-Path -LiteralPath $config) {
        $value = [IO.File]::ReadAllText($config).Trim()
        if ($value) {
            if (-not [IO.Path]::IsPathRooted($value)) { $value = Join-Path $root $value }
            return Find-Data $value
        }
    }
    return $null
}

function Start-Game {
    $exe = Join-Path $root 'looking-glass.exe'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
        throw 'Game files are ready. Close setup and use Launch.cmd to start your development build.'
    }
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $exe
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    [void][Diagnostics.Process]::Start($start)
}

if ($Mode -eq 'Worker') {
    $job = [IO.File]::ReadAllText($JobFile) | ConvertFrom-Json
    $statusPath = Join-Path $job.directory 'status.json'
    $cancelPath = Join-Path $job.directory 'cancel'
    function Check-Cancel {
        if (Test-Path -LiteralPath $cancelPath) { throw 'Setup cancelled. Previous settings and saves were kept.' }
    }
    function Report([string]$message, [int]$percent = -1) {
        Save-State $statusPath @{ message = $message; percent = $percent }
        Check-Cancel
    }
    function Download-Hash([string]$path) {
        $stream = [IO.File]::OpenRead($path)
        $hash = [Security.Cryptography.SHA256]::Create()
        try {
            $buffer = New-Object byte[] (1024 * 1024)
            while (($count = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                Check-Cancel
                [void]$hash.TransformBlock($buffer, 0, $count, $null, 0)
            }
            [void]$hash.TransformFinalBlock($buffer, 0, 0)
            return [BitConverter]::ToString($hash.Hash).Replace('-', '').ToLowerInvariant()
        } finally { $stream.Dispose(); $hash.Dispose() }
    }
    try {
        Check-Cancel
        $source = $job.source
        if ($job.download) {
            $downloads = Join-Path $root 'private\downloads'
            [void][IO.Directory]::CreateDirectory($downloads)
            $source = Join-Path $downloads $archiveName
            if (-not (Test-Path -LiteralPath $source)) {
                $partial = Join-Path $downloads ([Guid]::NewGuid().ToString('N') + '.partial')
                Report 'Connecting to Internet Archive...'
                [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
                $request = [Net.HttpWebRequest]::Create($downloadUrl)
                $request.UserAgent = 'LookingGlass-Setup/1.0'
                $request.Timeout = 20000
                $request.ReadWriteTimeout = 15000
                $response = $request.GetResponse()
                try {
                    if ($response.ResponseUri.Scheme -ne 'https') { throw 'Download was redirected away from HTTPS.' }
                    $input = $response.GetResponseStream()
                    $output = [IO.File]::Create($partial)
                    try {
                        $buffer = New-Object byte[] (256 * 1024)
                        $received = 0L
                        $clock = [Diagnostics.Stopwatch]::StartNew()
                        while (($count = $input.Read($buffer, 0, $buffer.Length)) -gt 0) {
                            Check-Cancel
                            $output.Write($buffer, 0, $count)
                            $received += $count
                            if ($received -gt $downloadBytes) { throw 'The download has changed. Use Choose archive or Choose folder instead.' }
                            if ($clock.ElapsedMilliseconds -gt 300) {
                                Report ('Downloading game files: {0:N0} of 933 MB' -f ($received / 1MB)) ([Math]::Min(100, [int](100 * $received / $downloadBytes)))
                                $clock.Restart()
                            }
                        }
                    } finally { $output.Dispose(); $input.Dispose() }
                } finally { $response.Close() }
                Check-Cancel
                if ((Get-Item -LiteralPath $partial).Length -ne $downloadBytes) { throw 'The download is incomplete. Please try again, or choose a local archive.' }
                Report 'Checking the downloaded file...'
                if ((Download-Hash $partial) -ne $downloadHash) { throw 'The download did not pass its integrity check. It has not been installed.' }
                Check-Cancel
                [IO.File]::Move($partial, $source)
            } else {
                Report 'Checking the previously downloaded file...'
                if ((Download-Hash $source) -ne $downloadHash) { throw 'The saved download did not pass its integrity check. Choose a different archive or folder.' }
            }
        }
        Check-Cancel
        $env:LG_SETUP_FILE = Join-Path $root 'Setup.cmd'
        $env:LG_SETUP_SOURCE = $source
        $env:LG_SETUP_NONINTERACTIVE = '--no-pause'
        $env:LG_SETUP_STATUS = $statusPath
        $env:LG_SETUP_CANCEL = $cancelPath
        $backend = ([IO.File]::ReadAllText($env:LG_SETUP_FILE) -split '# POWERSHELL_START\r?\n', 2)[1]
        & ([ScriptBlock]::Create($backend)) *>&1 | Out-File -LiteralPath (Join-Path $job.directory 'setup.log') -Encoding utf8
        exit $LASTEXITCODE
    } catch {
        Save-State $statusPath @{ message = $_.Exception.Message; percent = 0; failed = $true }
        exit 1
    } finally {
        # Only the partial file created by this worker is removed; never the user's archive.
        if ($partial -and (Test-Path -LiteralPath $partial)) { [IO.File]::Delete($partial) }
    }
}

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[Windows.Forms.Application]::EnableVisualStyles()

# Serialize setup per installation, including a shortcut opened twice.
$rootHash = [Security.Cryptography.SHA256]::Create()
try { $key = [BitConverter]::ToString($rootHash.ComputeHash([Text.Encoding]::UTF8.GetBytes($root.ToLowerInvariant()))).Replace('-', '') }
finally { $rootHash.Dispose() }
$mutex = New-Object Threading.Mutex($false, ('Local\LookingGlassSetup-' + $key))
try { $owned = $mutex.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned = $true }
if (-not $owned) {
    [void][Windows.Forms.MessageBox]::Show('Setup is already open for this game folder.', 'Looking Glass')
    exit 0
}

try {
    if ($Mode -eq 'Play' -and (Read-ConfiguredData)) { Start-Game; exit 0 }
    $form = New-Object Windows.Forms.Form
    $form.Text = 'Looking Glass - Setup'
    $form.ClientSize = New-Object Drawing.Size(660, 530)
    $form.StartPosition = 'CenterScreen'
    $form.FormBorderStyle = 'FixedDialog'
    $form.MaximizeBox = $false
    $form.Font = New-Object Drawing.Font('Segoe UI', 10)
    $form.AutoScaleMode = 'Dpi'

    function Label([string]$text, [int]$x, [int]$y, [int]$w, [int]$h) {
        $c = New-Object Windows.Forms.Label
        $c.Text = $text; $c.SetBounds($x, $y, $w, $h); $form.Controls.Add($c); return $c
    }
    function Button([string]$text, [int]$x, [int]$y, [int]$w) {
        $c = New-Object Windows.Forms.Button
        $c.Text = $text; $c.SetBounds($x, $y, $w, 38); $form.Controls.Add($c); return $c
    }
    $heading = Label 'Welcome to Looking Glass' 24 20 610 40
    $heading.Font = New-Object Drawing.Font('Segoe UI', 20, [Drawing.FontStyle]::Bold)
    $intro = Label 'One-time setup connects the original game files. After that, just open Launch to play.' 26 70 610 45
    $local = New-Object Windows.Forms.RadioButton
    $local.Text = 'Use game files already on this computer'; $local.SetBounds(26, 120, 605, 28)
    $form.Controls.Add($local)
    $choices = New-Object Windows.Forms.ComboBox
    $choices.DropDownStyle = 'DropDownList'; $choices.SetBounds(44, 155, 570, 30)
    $choices.AccessibleName = 'Game file location'
    $choices.DropDownWidth = 700; $form.Controls.Add($choices)
    $archiveButton = Button 'Choose archive...' 44 193 170
    $folderButton = Button 'Choose folder...' 224 193 170
    $download = New-Object Windows.Forms.RadioButton
    $download.Text = 'Download compatible game files (933 MB)'; $download.SetBounds(26, 244, 605, 28)
    $form.Controls.Add($download)
    $sourceNote = Label 'From Internet Archive. Use files you are entitled to use. Allow 3 GB of free space for the download and unpacked game.' 44 276 570 46

    $desktop = New-Object Windows.Forms.CheckBox
    $desktop.Text = 'Create a desktop shortcut'; $desktop.SetBounds(26, 328, 290, 28)
    $form.Controls.Add($desktop)
    $playAfter = New-Object Windows.Forms.CheckBox
    $playAfter.Text = 'Play when setup finishes'; $playAfter.SetBounds(330, 328, 290, 28)
    $playAfter.Checked = Test-Path -LiteralPath (Join-Path $root 'looking-glass.exe')
    $form.Controls.Add($playAfter)
    $progress = New-Object Windows.Forms.ProgressBar
    $progress.AccessibleName = 'Setup progress'
    $progress.SetBounds(26, 370, 605, 20); $form.Controls.Add($progress)
    $statusLabel = Label 'Choose an option above, then select Set up.' 26 400 605 60
    $go = Button 'Set up' 406 477 120
    $cancel = Button 'Close' 536 477 95
    $form.AcceptButton = $go
    $form.CancelButton = $cancel
    $timer = New-Object Windows.Forms.Timer
    $timer.Interval = 250
    $script:worker = $null
    $script:closeWhenDone = $false
    $script:ready = $false
    $script:currentJob = $null

    function Add-Choice([string]$path) {
        if ($path -and (Test-Path -LiteralPath $path)) {
            $index = $choices.Items.IndexOf($path)
            if ($index -lt 0) { $index = $choices.Items.Add($path) }
            $choices.SelectedIndex = $index
        }
    }
    foreach ($folder in @($root, (Join-Path $env:USERPROFILE 'Downloads'))) {
        Add-Choice (Find-Data $folder)
        Add-Choice (Join-Path $folder $archiveName)
    }
    Add-Choice (Join-Path $root ('alice_202106\' + $archiveName))
    Add-Choice (Join-Path $root ('private\downloads\' + $archiveName))
    Add-Choice (Read-ConfiguredData)
    $local.Checked = $choices.Items.Count -gt 0
    $download.Checked = -not $local.Checked
    if ($local.Checked) { $statusLabel.Text = 'Found game files on this computer. Select Set up to connect them.' }

    function Refresh-Inputs {
        $busy = $null -ne $script:worker
        $local.Enabled = -not $busy; $download.Enabled = -not $busy
        $choices.Enabled = -not $busy -and $local.Checked
        $archiveButton.Enabled = -not $busy; $folderButton.Enabled = -not $busy
        $go.Enabled = -not $busy; $desktop.Enabled = -not $busy; $playAfter.Enabled = -not $busy
    }
    $local.Add_CheckedChanged({ Refresh-Inputs })
    $download.Add_CheckedChanged({ $script:ready = $false; $go.Text = 'Set up' })
    $choices.Add_SelectedIndexChanged({ $script:ready = $false; $go.Text = 'Set up' })
    $archiveButton.Add_Click({
        $dialog = New-Object Windows.Forms.OpenFileDialog
        $dialog.Title = 'Choose the downloaded game archive'
        $dialog.Filter = 'Game archive (*.7z)|*.7z'
        if ($dialog.ShowDialog($form) -eq 'OK') { Add-Choice $dialog.FileName; $local.Checked = $true; $script:ready = $false; $go.Text = 'Set up' }
        $dialog.Dispose()
    })
    $folderButton.Add_Click({
        $dialog = New-Object Windows.Forms.FolderBrowserDialog
        $dialog.Description = 'Choose Alice1, its parent folder, or bin\base.'
        $dialog.ShowNewFolderButton = $false
        if ($dialog.ShowDialog($form) -eq 'OK') { Add-Choice $dialog.SelectedPath; $local.Checked = $true; $script:ready = $false; $go.Text = 'Set up' }
        $dialog.Dispose()
    })
    $go.Add_Click({
        try {
            if ($script:ready) { Start-Game; $form.Close(); return }
            if ($local.Checked -and $choices.SelectedIndex -lt 0) { throw 'Choose an archive or folder first.' }
            $script:currentJob = Join-Path $root ('private\setup-jobs\' + [Guid]::NewGuid().ToString('N'))
            [void][IO.Directory]::CreateDirectory($script:currentJob)
            $jobPath = Join-Path $script:currentJob 'job.json'
            Save-State $jobPath @{ directory = $script:currentJob; source = [string]$choices.SelectedItem; download = $download.Checked }
            $start = New-Object Diagnostics.ProcessStartInfo
            $start.FileName = Join-Path $PSHOME 'powershell.exe'
            $start.Arguments = '-NoLogo -NoProfile -ExecutionPolicy Bypass -Command "& $env:LG_UI_SCRIPT -Mode Worker -JobFile $env:LG_UI_JOB"'
            $start.EnvironmentVariables['LG_UI_SCRIPT'] = $PSCommandPath
            $start.EnvironmentVariables['LG_UI_JOB'] = $jobPath
            $start.UseShellExecute = $false; $start.CreateNoWindow = $true; $start.WorkingDirectory = $root
            $start.RedirectStandardError = $true
            $script:worker = [Diagnostics.Process]::Start($start)
            $script:workerErrors = $script:worker.StandardError.ReadToEndAsync()
            $statusLabel.Text = 'Starting setup...'; $progress.Style = 'Marquee'; $cancel.Text = 'Cancel'
            Refresh-Inputs
            $timer.Start()
        } catch { [void][Windows.Forms.MessageBox]::Show($form, $_.Exception.Message, 'Looking Glass - Setup') }
    })
    function Request-Cancel {
        [IO.File]::WriteAllText((Join-Path $script:currentJob 'cancel'), 'cancel', $utf8)
        $statusLabel.Text = 'Cancelling safely. Waiting for the current file operation to stop...'
        $cancel.Enabled = $false
    }
    $cancel.Add_Click({ if ($script:worker) { Request-Cancel } else { $form.Close() } })
    $form.Add_FormClosing({
        if ($script:worker) { $_.Cancel = $true; $script:closeWhenDone = $true; Request-Cancel }
    })
    $timer.Add_Tick({
        $statePath = Join-Path $script:currentJob 'status.json'
        if (Test-Path -LiteralPath $statePath) {
            try {
                $state = [IO.File]::ReadAllText($statePath) | ConvertFrom-Json
                if ($cancel.Enabled) { $statusLabel.Text = $state.message }
                if ($state.percent -ge 0) { $progress.Style = 'Continuous'; $progress.Value = [Math]::Min(100, $state.percent) }
                else { $progress.Style = 'Marquee' }
            } catch { } # A short status-file race is retried on the next tick.
        }
        if ($script:worker.HasExited) {
            $timer.Stop()
            $code = $script:worker.ExitCode
            $workerError = $script:workerErrors.Result
            $script:worker.Dispose(); $script:worker = $null
            $progress.Style = 'Continuous'; $cancel.Enabled = $true; $cancel.Text = 'Close'
            Refresh-Inputs
            if ($code -eq 0) {
                $progress.Value = 100; $statusLabel.Text = 'Ready to play. Next time, just open Launch.'
                $script:ready = Test-Path -LiteralPath (Join-Path $root 'looking-glass.exe')
                if ($script:ready) { $go.Text = 'Play now' }
                if ($desktop.Checked) {
                    try {
                        $linkPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Looking Glass.lnk'
                        # Never replace an existing shortcut belonging to another installation.
                        if (Test-Path -LiteralPath $linkPath) { $linkPath = Join-Path ([Environment]::GetFolderPath('Desktop')) ('Looking Glass ' + [Guid]::NewGuid().ToString('N').Substring(0, 6) + '.lnk') }
                        $shell = New-Object -ComObject WScript.Shell
                        $link = $shell.CreateShortcut($linkPath)
                        $link.TargetPath = Join-Path $PSHOME 'powershell.exe'
                        $link.Arguments = '-NoLogo -NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" -Mode Play'
                        $link.WorkingDirectory = $root; $link.IconLocation = (Join-Path $root 'looking-glass.exe'); $link.Save()
                    } catch { $statusLabel.Text = 'Game files are ready. The desktop shortcut could not be created; use Launch.cmd.' }
                }
                if ($playAfter.Checked -and -not $script:closeWhenDone) {
                    try { Start-Game; $form.Close() }
                    catch { $statusLabel.Text = $_.Exception.Message }
                }
            } else {
                if ($workerError) { $statusLabel.Text = 'Setup could not finish. Please try again or choose an existing archive or folder.' }
                $log = Join-Path $script:currentJob 'setup.log'
                if (Test-Path -LiteralPath $log) {
                    $lines = @(Get-Content -LiteralPath $log | Where-Object { $_.Trim() })
                    if ($lines.Count) { $statusLabel.Text = $lines[-1] }
                }
                $progress.Value = 0
            }
            if ($script:closeWhenDone) { $form.Close() }
        }
    })
    Refresh-Inputs
    [void]$form.ShowDialog()
    $timer.Dispose(); $form.Dispose()
} catch {
    [void][Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Looking Glass - Setup')
    exit 1
} finally {
    if ($owned) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}

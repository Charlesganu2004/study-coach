[CmdletBinding()]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\StudyCoach'),
    [switch]$NoShortcut
)

$ErrorActionPreference = 'Stop'
$CoachPackage = [IO.Path]::GetFullPath($PSScriptRoot)
$CoachDestination = [IO.Path]::GetFullPath($InstallDir).TrimEnd('\', '/')
$CoachMarker = Join-Path $CoachDestination 'installation.json'
$CoachNames = @('study-coach-app.exe', 'StartStudyCoach.cmd', 'install.ps1', 'uninstall.ps1', 'README.txt')
$CoachPrevious = $null

if ($CoachDestination -eq [IO.Path]::GetPathRoot($CoachDestination) -or
    $CoachDestination -eq [IO.Path]::GetFullPath($env:USERPROFILE).TrimEnd('\', '/') -or
    $CoachDestination -eq $CoachPackage) {
    throw 'Choose a separate application folder for installation.'
}
if (Test-Path -LiteralPath $CoachDestination) {
    if ((Get-Item -LiteralPath $CoachDestination).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'The installation folder cannot be a symbolic link or junction.'
    }
    $CoachExisting = @(Get-ChildItem -LiteralPath $CoachDestination -Force)
    if ($CoachExisting.Count -gt 0) {
        if (-not (Test-Path -LiteralPath $CoachMarker)) {
            throw 'The destination is not empty and is not an existing Study Coach installation.'
        }
        $CoachPrevious = Get-Content -Raw -LiteralPath $CoachMarker | ConvertFrom-Json
        if ($CoachPrevious.application -ne 'study-coach-rust' -or $CoachPrevious.schema_version -ne 1) {
            throw 'The destination has an unrecognized installation marker.'
        }
    }
}

$CoachSumsPath = Join-Path $CoachPackage 'SHA256SUMS'
if (-not (Test-Path -LiteralPath $CoachSumsPath)) { throw 'SHA256SUMS is missing from the package.' }
$CoachVerified = @{}
foreach ($CoachLine in [IO.File]::ReadAllLines($CoachSumsPath)) {
    if ($CoachLine -notmatch '^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$') { throw 'The package checksum file has an invalid entry.' }
    $CoachHash = $Matches[1]
    $CoachName = $Matches[2]
    if ($CoachNames -notcontains $CoachName -or $CoachVerified.ContainsKey($CoachName)) {
        throw 'The package checksum file has an unexpected or duplicate entry.'
    }
    $CoachSource = Join-Path $CoachPackage $CoachName
    if (-not (Test-Path -LiteralPath $CoachSource -PathType Leaf)) { throw "Package file missing: $CoachName" }
    if ((Get-Item -LiteralPath $CoachSource).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'Package files cannot be symbolic links.'
    }
    if ((Get-FileHash -LiteralPath $CoachSource -Algorithm SHA256).Hash.ToLowerInvariant() -ne $CoachHash) {
        throw "Package checksum mismatch: $CoachName"
    }
    $CoachVerified[$CoachName] = $true
}
if ($CoachVerified.Count -ne $CoachNames.Count) { throw 'The package checksum file is incomplete.' }

$CoachShortcut = if ($CoachPrevious) { $CoachPrevious.shortcut } else { $null }
$CoachLink = $null
if (-not $NoShortcut) {
    $CoachPrograms = [Environment]::GetFolderPath('Programs')
    $CoachShortcut = Join-Path $CoachPrograms 'Study Coach.lnk'
    $CoachShell = New-Object -ComObject WScript.Shell
    $CoachLink = $CoachShell.CreateShortcut($CoachShortcut)
    if ((Test-Path -LiteralPath $CoachShortcut) -and $CoachLink.TargetPath -ne (Join-Path $CoachDestination 'study-coach-app.exe')) {
        throw 'A different Study Coach shortcut already exists. Rerun with -NoShortcut.'
    }
}
[IO.Directory]::CreateDirectory($CoachDestination) | Out-Null
foreach ($CoachName in @($CoachNames + 'SHA256SUMS')) {
    $CoachTarget = Join-Path $CoachDestination $CoachName
    if ((Test-Path -LiteralPath $CoachTarget) -and
        ((Get-Item -LiteralPath $CoachTarget).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'An installed file cannot be a symbolic link.'
    }
    Copy-Item -LiteralPath (Join-Path $CoachPackage $CoachName) -Destination $CoachTarget -Force
}

if ($CoachLink) {
    $CoachLink.TargetPath = Join-Path $CoachDestination 'study-coach-app.exe'
    $CoachLink.WorkingDirectory = $CoachDestination
    $CoachLink.Description = 'Study from your notes with exact source citations.'
    $CoachLink.Save()
}
$CoachInstall = [ordered]@{
    application = 'study-coach-rust'
    schema_version = 1
    installed_files = @($CoachNames + 'SHA256SUMS')
    shortcut = $CoachShortcut
}
[IO.File]::WriteAllText($CoachMarker, ($CoachInstall | ConvertTo-Json -Depth 3), (New-Object Text.UTF8Encoding($false)))
Write-Output "Installed Study Coach in $CoachDestination"
Write-Output 'Launch StartStudyCoach.cmd. First launch creates your local application data and blank key file.'
Write-Output 'No administrator account or Rust installation is required to run this package. Windows curl.exe is required.'

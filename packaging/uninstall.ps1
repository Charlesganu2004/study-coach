[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$CoachDirectory = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\', '/')
$CoachMarker = Join-Path $CoachDirectory 'installation.json'
if (-not (Test-Path -LiteralPath $CoachMarker -PathType Leaf)) {
    throw 'No installation marker was found. Remove a portable package folder manually.'
}
if ((Get-Item -LiteralPath $CoachDirectory).Attributes -band [IO.FileAttributes]::ReparsePoint) {
    throw 'The installation folder cannot be a symbolic link or junction.'
}
$CoachInstall = Get-Content -Raw -LiteralPath $CoachMarker | ConvertFrom-Json
$CoachAllowed = @('study-coach-app.exe', 'StartStudyCoach.cmd', 'install.ps1', 'uninstall.ps1', 'README.txt', 'SHA256SUMS')
if ($CoachInstall.application -ne 'study-coach-rust' -or $CoachInstall.schema_version -ne 1 -or
    @($CoachInstall.installed_files).Count -ne $CoachAllowed.Count) {
    throw 'The installation marker is not recognized.'
}
foreach ($CoachName in $CoachInstall.installed_files) {
    if ($CoachAllowed -notcontains $CoachName) { throw 'The installation marker contains an unexpected file.' }
}
if ($CoachInstall.shortcut) {
    $CoachExpectedShortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Study Coach.lnk'
    if ($CoachInstall.shortcut -eq $CoachExpectedShortcut -and (Test-Path -LiteralPath $CoachExpectedShortcut)) {
        $CoachShell = New-Object -ComObject WScript.Shell
        $CoachLink = $CoachShell.CreateShortcut($CoachExpectedShortcut)
        if ($CoachLink.TargetPath -eq (Join-Path $CoachDirectory 'study-coach-app.exe')) {
            Remove-Item -LiteralPath $CoachExpectedShortcut
        }
    }
}
foreach ($CoachName in $CoachAllowed) {
    $CoachTarget = [IO.Path]::GetFullPath((Join-Path $CoachDirectory $CoachName))
    if (-not $CoachTarget.StartsWith($CoachDirectory + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'A removal path escaped the installation folder.'
    }
    if (Test-Path -LiteralPath $CoachTarget) { Remove-Item -LiteralPath $CoachTarget }
}
Remove-Item -LiteralPath $CoachMarker
if (@(Get-ChildItem -LiteralPath $CoachDirectory -Force).Count -eq 0) {
    Remove-Item -LiteralPath $CoachDirectory
}
Write-Output 'Study Coach program files removed. Your notes, history, settings, and API keys were preserved in the application data folder.'

param(
    [Parameter(Mandatory)] [string] $MsiPath
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$installDir = Join-Path $env:LOCALAPPDATA 'NorthStar'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\NorthStar\NorthStar Prototype.lnk'
$uninstallRoots = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
)
function Get-NorthStarInstallation {
    foreach ($root in $uninstallRoots) {
        Get-ChildItem $root -ErrorAction SilentlyContinue |
            Get-ItemProperty |
            Where-Object DisplayName -eq 'NorthStar Prototype'
    }
}
if ((Test-Path -LiteralPath $installDir) -or (Get-NorthStarInstallation)) {
    throw 'NorthStar is already installed or its install directory exists; refusing to disturb it'
}

$installed = $false
try {
    $log = Join-Path $repo 'target\msi-install.log'
    $process = Start-Process msiexec.exe -ArgumentList "/i `"$MsiPath`" /qn /norestart /l*v `"$log`"" -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "MSI install failed with code $($process.ExitCode); see $log" }
    $installed = $true

    $files = @('northstar-desktop.exe', 'LICENSE.txt', 'THIRD_PARTY_FONTS.md',
               'IBM-Plex-OFL.txt', 'WINDOWS_PROTOTYPE.md', 'RECOVERY.md', 'NAVIGATION.md', 'WORD_NAVIGATION.md', 'FIND.md', 'DELETION.md', 'BACKSPACE_DELETE.md', 'ERASURE.md', 'WORD_ERASURE.md')
    foreach ($file in $files) {
        if (-not (Test-Path -LiteralPath (Join-Path $installDir $file) -PathType Leaf)) {
            throw "Installed file missing: $file"
        }
    }
    if (-not (Test-Path -LiteralPath $shortcut -PathType Leaf)) { throw 'Start menu shortcut is missing' }
    if (-not (Get-NorthStarInstallation)) { throw 'Windows uninstall entry is missing' }
    $builtExe = Join-Path $repo 'target\release\northstar-desktop.exe'
    if ((Get-FileHash -LiteralPath (Join-Path $installDir 'northstar-desktop.exe')).Hash -ne
        (Get-FileHash -LiteralPath $builtExe).Hash) {
        throw 'Installed executable differs from the release build'
    }
    Write-Output 'MSI installation and payload verified'
} finally {
    if ($installed) {
        $log = Join-Path $repo 'target\msi-uninstall.log'
        $process = Start-Process msiexec.exe -ArgumentList "/x `"$MsiPath`" /qn /norestart /l*v `"$log`"" -Wait -PassThru -WindowStyle Hidden
        if ($process.ExitCode -ne 0) { throw "MSI uninstall failed with code $($process.ExitCode); see $log" }
        if ((Test-Path -LiteralPath $installDir) -or (Test-Path -LiteralPath $shortcut) -or
            (Get-NorthStarInstallation)) {
            throw 'MSI uninstall left installed files, shortcut, or uninstall entry'
        }
        Write-Output 'MSI uninstallation verified'
    }
}

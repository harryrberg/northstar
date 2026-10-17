param(
    [Parameter(Mandatory)] [string] $MsiPath
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$installDir = Join-Path $env:LOCALAPPDATA 'OpenStar'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\OpenStar\OpenStar Prototype.lnk'
$uninstallRoots = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
)
function Get-OpenStarInstallation {
    foreach ($root in $uninstallRoots) {
        Get-ChildItem $root -ErrorAction SilentlyContinue |
            Get-ItemProperty |
            Where-Object DisplayName -eq 'OpenStar Prototype'
    }
}
function Get-PreviousInstallation {
    foreach ($root in $uninstallRoots) {
        Get-ChildItem $root -ErrorAction SilentlyContinue |
            Get-ItemProperty |
            Where-Object DisplayName -eq 'NorthStar Prototype'
    }
}
if ((Test-Path -LiteralPath $installDir) -or (Get-OpenStarInstallation) -or (Get-PreviousInstallation)) {
    throw 'OpenStar or NorthStar is already installed, or the OpenStar install directory exists; refusing to disturb it'
}

$installed = $false
try {
    $log = Join-Path $repo 'target\msi-install.log'
    $process = Start-Process msiexec.exe -ArgumentList "/i `"$MsiPath`" /qn /norestart /l*v `"$log`"" -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "MSI install failed with code $($process.ExitCode); see $log" }
    $installed = $true

    $files = @('openstar-desktop.exe', 'LICENSE.txt', 'THIRD_PARTY_FONTS.md',
               'IBM-Plex-OFL.txt', 'WINDOWS_PROTOTYPE.md', 'MENUS.md', 'FILE_OPERATIONS.md', 'RECOVERY.md', 'NAVIGATION.md', 'HARD_BREAK.md', 'LINE_BOUNDARIES.md', 'QUICK_JUMPS.md', 'WORD_NAVIGATION.md', 'FIND.md', 'REPLACEMENT.md', 'DELETION.md', 'BACKSPACE_DELETE.md', 'BLOCK_COPY.md', 'BLOCK_MOVE_DELETE.md', 'BLOCK_WRITE.md', 'ERASURE.md', 'WORD_ERASURE.md')
    foreach ($file in $files) {
        if (-not (Test-Path -LiteralPath (Join-Path $installDir $file) -PathType Leaf)) {
            throw "Installed file missing: $file"
        }
    }
    if (-not (Test-Path -LiteralPath $shortcut -PathType Leaf)) { throw 'Start menu shortcut is missing' }
    if (-not (Get-OpenStarInstallation)) { throw 'Windows uninstall entry is missing' }
    $builtExe = Join-Path $repo 'target\release\openstar-desktop.exe'
    if ((Get-FileHash -LiteralPath (Join-Path $installDir 'openstar-desktop.exe')).Hash -ne
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
            (Get-OpenStarInstallation)) {
            throw 'MSI uninstall left installed files, shortcut, or uninstall entry'
        }
        Write-Output 'MSI uninstallation verified'
    }
}

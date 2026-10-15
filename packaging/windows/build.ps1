param(
    [string] $OutputPath,
    [switch] $SkipRustBuild
)

$ErrorActionPreference = 'Stop'
$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $repo
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
    $cargo = $cargoCommand.Source
} else {
    $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (-not (Test-Path -LiteralPath $cargo -PathType Leaf)) {
        throw 'Rust Cargo is not installed or on PATH'
    }
}

$metadata = & $cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$version = ($metadata.packages | Where-Object name -eq 'northstar-desktop').version
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Desktop version $version is not a three-part MSI version"
}
if (-not $OutputPath) {
    $OutputPath = Join-Path $repo "releases\windows\NorthStar-Prototype-$version-x64.msi"
}
$OutputPath = [System.IO.Path]::GetFullPath($OutputPath)

if (-not $SkipRustBuild) {
    & $cargo build --release -p northstar-desktop --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed' }
}
$desktop = Join-Path $repo 'target\release\northstar-desktop.exe'
if (-not (Test-Path -LiteralPath $desktop -PathType Leaf)) {
    throw "Desktop executable is missing: $desktop"
}

$wixVersion = '5.0.2'
$wix = Join-Path $repo 'target\tools\wix\wix.exe'
if (-not (Test-Path -LiteralPath $wix -PathType Leaf)) {
    $dotnet = Join-Path $repo 'target\tools\dotnet\dotnet.exe'
    if (-not (Test-Path -LiteralPath $dotnet -PathType Leaf)) {
        $dotnetCommand = Get-Command dotnet -ErrorAction SilentlyContinue
        if (-not $dotnetCommand -or -not (& $dotnetCommand.Source --list-sdks)) {
            throw 'Install .NET SDK 10.0.100, then rerun this script'
        }
        $dotnet = $dotnetCommand.Source
    }
    & $dotnet tool install wix --tool-path (Split-Path $wix) --version $wixVersion
    if ($LASTEXITCODE -ne 0) { throw 'WiX installation failed' }
}
if ((& $wix --version) -notmatch "^$([regex]::Escape($wixVersion))\+") {
    throw "Expected WiX $wixVersion at $wix"
}

New-Item -ItemType Directory -Force (Split-Path $OutputPath) | Out-Null
& $wix build (Join-Path $PSScriptRoot 'NorthStar.wxs') `
    -arch x64 `
    -d "ProductVersion=$version" `
    -d "DesktopExecutable=$desktop" `
    -d "RepoRoot=$repo" `
    -pdb (Join-Path $repo 'target\NorthStar.wixpdb') `
    -out $OutputPath
if ($LASTEXITCODE -ne 0) { throw 'WiX build failed' }

$hash = (Get-FileHash -LiteralPath $OutputPath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Output "$OutputPath  SHA-256 $hash"

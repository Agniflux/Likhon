# Builds Desktop\Product\Setup\Likhon-Setup-<version>.exe from an already built (and signed) likhon.exe.
#   .\build-installer.ps1                          uses ..\Product\Portable\likhon.exe
#   .\build-installer.ps1 -Exe D:\path\likhon.exe  another build (for example the signed one from CI)
# Writes SHA256SUMS.txt next to the installer. Signing (SignPath "installer" config, then the Ed25519 .sig) comes after.
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\Product\Portable\likhon.exe'),
    [string]$Version = '',
    [string]$Sub = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Version) {
    $toml = Get-Content (Join-Path $PSScriptRoot '..\Source\Cargo.toml') -Raw
    $Version = [regex]::Match($toml, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
}
$iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $iscc) { $iscc = Get-Item "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" -ErrorAction SilentlyContinue }
if (-not $iscc) { $iscc = Get-Item 'D:\Program Files\Inno Setup 7\ISCC.exe', "${env:ProgramFiles}\Inno Setup 7\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe" -ErrorAction SilentlyContinue | Select-Object -First 1 }
if (-not $iscc) { throw 'Inno Setup 6 or 7 is not installed (winget install JRSoftware.InnoSetup).' }
if (-not (Test-Path $Exe)) { throw "likhon.exe not found: $Exe" }
$exeFull = (Resolve-Path $Exe).Path
$out = Join-Path $PSScriptRoot '..\Product\Setup'
if ($Sub) {
    if ($Sub -notmatch '^[a-z0-9-]+$') { throw '-Sub must be a plain folder name' }
    $out = Join-Path $out $Sub
}
New-Item -ItemType Directory -Force $out | Out-Null
$isccPath = if ($iscc.Source) { $iscc.Source } else { $iscc.FullName }
& $isccPath "/DAppVersion=$Version" "/DExePath=$exeFull" "/DOutDir=$((Resolve-Path $out).Path)" (Join-Path $PSScriptRoot 'likhon.iss')
if ($LASTEXITCODE -ne 0) { throw 'ISCC failed' }
$setup = Get-ChildItem $out -Filter "Likhon-Setup-$Version.exe" | Select-Object -First 1
$hash = (Get-FileHash $setup.FullName -Algorithm SHA256).Hash.ToLower()
"$hash  $($setup.Name)" | Set-Content (Join-Path $out 'SHA256SUMS.txt') -Encoding ascii
Write-Host "Built $($setup.Name): $([math]::Round($setup.Length/1MB,2)) MB, sha256 $hash"

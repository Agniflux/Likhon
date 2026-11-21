# Fills Desktop/Product/Portable from a release build: likhon.exe and the dictionary folder beside it.
#   cargo build --release -p likhon
#   powershell -ExecutionPolicy Bypass -File scripts/make-portable.ps1
# A copy for another site (its address is built in; the copy goes to Portable\<Sub>):
#   powershell -ExecutionPolicy Bypass -File scripts/make-portable.ps1 -Site https://example.org -Sub other
# With -Site the script builds likhon.exe itself with LIKHON_SITE set (settings::DEFAULT_SITE), copies it to
# Portable\<Sub>, then builds again without it, so target\release is the normal build afterwards.
# Refuses while Likhon runs from that folder (a running exe cannot be replaced); exit it from the tray first.
param([string]$Site = '', [string]$Sub = '')
$ErrorActionPreference = 'Stop'
$src = Resolve-Path (Join-Path $PSScriptRoot '..')
$out = Join-Path $src '..\Product\Portable'
if ($Sub) {
    if ($Sub -notmatch '^[a-z0-9-]+$') { throw "-Sub must be a plain folder name" }
    $out = Join-Path $out $Sub
}
if ($Site -and $Site -notmatch '^https://[a-z0-9.-]+(:[0-9]+)?$') { throw "-Site must be an https address with no path, like https://likhon.app" }
$running = Get-CimInstance Win32_Process -Filter "Name='likhon.exe'" | Where-Object { $_.ExecutablePath -and (Split-Path $_.ExecutablePath) -eq (New-Item -ItemType Directory -Force $out).FullName }
if ($running) { "Likhon is running from $out (PID $($running.ProcessId -join ', ')). Exit it from the tray, then run this again."; exit 1 }
if ($Site) {
    Push-Location $src
    try {
        $env:LIKHON_SITE = $Site
        cargo build --release -p likhon
        if ($LASTEXITCODE) { throw "build for $Site failed" }
    } finally { Remove-Item Env:LIKHON_SITE -ErrorAction SilentlyContinue; Pop-Location }
}
New-Item -ItemType Directory -Force (Join-Path $out 'dict') | Out-Null
Copy-Item (Join-Path $src 'target\release\likhon.exe') $out -Force
# The dictionary (owner decision 2026-10-22: a separate dict folder), with its MPL-2.0 licence and source note.
# words.full.sqlite (the owner's enlarged list, never in git) wins over the public Avro-derived words.sqlite when it exists.
$full = Join-Path $src 'assets\dict\mpl\words.full.sqlite'
Copy-Item $(if (Test-Path $full) { $full } else { Join-Path $src 'assets\dict\mpl\words.sqlite' }) (Join-Path $out 'dict\words.sqlite') -Force
foreach ($f in 'LICENSE', 'SOURCE.md') { Copy-Item (Join-Path $src "assets\dict\mpl\$f") (Join-Path $out "dict\$f") -Force }
if ($Site) {
    Push-Location $src
    try { cargo build --release -p likhon; if ($LASTEXITCODE) { throw 'rebuilding the normal likhon.exe failed' } } finally { Pop-Location }
}
@(Get-ChildItem $out -File) + @(Get-ChildItem (Join-Path $out 'dict') -File) | Where-Object { $_.Name -ne '.gitkeep' } | ForEach-Object { "{0,12:N0}  {1}" -f $_.Length, $_.FullName.Substring((Resolve-Path $out).Path.Length + 1) }

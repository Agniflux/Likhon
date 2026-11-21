# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Adds Likhon to Windows' keyboards (Settings > Time & language > Bangla (Bangladesh) > Keyboards, and Win+Space):
# copies likhon_tsf.dll (64-bit and 32-bit builds) to Program Files\Likhon, registers both with regsvr32, and adds
# the keyboard to this user's Bangla (Bangladesh) language. Undo: unregister-tsf.ps1. Run in an elevated PowerShell:
#   powershell -ExecutionPolicy Bypass -File register-tsf.ps1 [-Data <folder with settings.json>]
# -Data defaults to Desktop\Product\Portable (the folder likhon.exe runs from), so the keyboard uses your layout and
# toggle key. Store apps cannot read that folder and type with the default (phonetic) settings.
param([string]$Data = (Resolve-Path (Join-Path $PSScriptRoot '..\..\Product\Portable')).Path)
$ErrorActionPreference = 'Stop'
$log = Join-Path $env:ProgramData 'Likhon\tsf.log'
New-Item -ItemType Directory -Force (Split-Path $log) | Out-Null
function Log($m) { Add-Content $log "$(Get-Date -Format s) register: $m"; $m }

$src64 = Join-Path $PSScriptRoot '..\target\release\likhon_tsf.dll'
$src86 = Join-Path $PSScriptRoot '..\target\i686-pc-windows-msvc\release\likhon_tsf.dll'
foreach ($s in @($src64, $src86)) { if (-not (Test-Path $s)) { throw "Build first: $s is missing (see Desktop/Source/README.md)" } }

$dst64 = Join-Path $env:ProgramFiles 'Likhon'
$dst86 = Join-Path ${env:ProgramFiles(x86)} 'Likhon'
foreach ($pair in @(@($src64, $dst64), @($src86, $dst86))) {
    New-Item -ItemType Directory -Force $pair[1] | Out-Null
    Copy-Item $pair[0] (Join-Path $pair[1] 'likhon_tsf.dll') -Force
    Set-Content (Join-Path $pair[1] 'datadir.txt') $Data -Encoding UTF8
    Log "copied to $($pair[1]) (settings from $Data)"
}
& "$env:WINDIR\System32\regsvr32.exe" /s (Join-Path $dst64 'likhon_tsf.dll'); Log "regsvr32 x64: exit $LASTEXITCODE"
if ($LASTEXITCODE) { throw 'regsvr32 (64-bit) failed; run unregister-tsf.ps1' }
& "$env:WINDIR\SysWOW64\regsvr32.exe" /s (Join-Path $dst86 'likhon_tsf.dll'); Log "regsvr32 x86: exit $LASTEXITCODE"
if ($LASTEXITCODE) { throw 'regsvr32 (32-bit) failed; run unregister-tsf.ps1' }

# This user's keyboard list: Bangla (Bangladesh) with the Likhon keyboard.
$tip = '0845:{C772DC88-B930-41B1-A45A-C800A07FAD7C}{1A047B60-A777-42B7-B08B-2329083DCFE3}'
$list = Get-WinUserLanguageList
$bn = $list | Where-Object { $_.LanguageTag -ieq 'bn-BD' } | Select-Object -First 1
if (-not $bn) { $list.Add('bn-BD'); $bn = $list | Where-Object { $_.LanguageTag -ieq 'bn-BD' } | Select-Object -First 1 }
if (-not ($bn.InputMethodTips -icontains $tip)) { $bn.InputMethodTips.Add($tip) }
Set-WinUserLanguageList $list -Force
Log 'added to Bangla (Bangladesh)'
'Likhon is now in your keyboards: press Win+Space and pick "লিখন (Likhon)".'

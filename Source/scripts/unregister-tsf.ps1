# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Undo for register-tsf.ps1: takes the Likhon keyboard out of this user's keyboard list, unregisters the text service
# (64-bit and 32-bit) and deletes its files. Run in an elevated PowerShell:
#   powershell -ExecutionPolicy Bypass -File unregister-tsf.ps1
# A DLL still loaded by a running app cannot be deleted: the script says so; sign out and run it again.
$ErrorActionPreference = 'Continue'
$log = Join-Path $env:ProgramData 'Likhon\tsf.log'
New-Item -ItemType Directory -Force (Split-Path $log) | Out-Null
function Log($m) { Add-Content $log "$(Get-Date -Format s) unregister: $m"; $m }

$tip = '0845:{C772DC88-B930-41B1-A45A-C800A07FAD7C}{1A047B60-A777-42B7-B08B-2329083DCFE3}'
$list = Get-WinUserLanguageList
foreach ($l in $list) {
    $t = $l.InputMethodTips | Where-Object { $_ -ieq $tip }
    if ($t) { [void]$l.InputMethodTips.Remove($t); Log "removed from $($l.LanguageTag)" }
}
Set-WinUserLanguageList $list -Force

$x64 = Join-Path $env:ProgramFiles 'Likhon\likhon_tsf.dll'
$x86 = Join-Path ${env:ProgramFiles(x86)} 'Likhon\likhon_tsf.dll'
if (Test-Path $x64) { & "$env:WINDIR\System32\regsvr32.exe" /s /u $x64; Log "regsvr32 /u x64: exit $LASTEXITCODE" }
if (Test-Path $x86) { & "$env:WINDIR\SysWOW64\regsvr32.exe" /s /u $x86; Log "regsvr32 /u x86: exit $LASTEXITCODE" }
foreach ($f in @($x64, $x86)) {
    $dir = Split-Path $f
    if (Test-Path $f) {
        try { Remove-Item $f -ErrorAction Stop; Log "deleted $f" } catch { Log "still in use: $f (sign out, then run this again)" }
    }
    Remove-Item (Join-Path $dir 'datadir.txt') -ErrorAction SilentlyContinue
    if ((Test-Path $dir) -and -not (Get-ChildItem $dir)) { Remove-Item $dir }
}
'Likhon keyboard removed.'

# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Undo for the Default Bangla font changer (app/src/fontdefault.rs), by hand: puts Windows' font-link lists back
# exactly as they were before Likhon first changed them, from the backup the changer saved, and removes a font file
# the changer copied into the Windows fonts folder. Run in an elevated PowerShell:
#   powershell -ExecutionPolicy Bypass -File restore-default-font.ps1
# The same as Settings > Fonts > Restore Windows default (likhon.exe --font-default-restore). Sign out and in after.
$ErrorActionPreference = 'Stop'
$backup = Join-Path $env:ProgramData 'Likhon\font-default-backup.json'
$log = Join-Path $env:ProgramData 'Likhon\font-default.log'
if (-not (Test-Path $backup)) { "No backup at $backup: Likhon has not changed the default font."; exit 0 }
$b = Get-Content $backup -Raw -Encoding UTF8 | ConvertFrom-Json
$key = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontLink\SystemLink'
foreach ($p in $b.values.PSObject.Properties) {
    if ($null -eq $p.Value) {
        Remove-ItemProperty -Path $key -Name $p.Name -ErrorAction SilentlyContinue
    } else {
        Set-ItemProperty -Path $key -Name $p.Name -Value ([string[]]$p.Value) -Type MultiString
    }
    Add-Content $log "$(Get-Date -Format s) restore (script): $($p.Name)"
}
if ($b.copiedFont) {
    Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts' -Name $b.copiedFont.regName -ErrorAction SilentlyContinue
    Remove-Item $b.copiedFont.file -ErrorAction SilentlyContinue
    Add-Content $log "$(Get-Date -Format s) restore (script): removed $($b.copiedFont.file)"
}
Remove-Item $backup
"Restored. Sign out and back in to see it everywhere."

# Stops every running likhon.exe. Use it if a development build ever swallows keys or breaks typing
# (root CLAUDE.md, "Dev safety"). Needs no admin rights for your own processes.
#
#   powershell -ExecutionPolicy Bypass -File Desktop\Source\scripts\kill-likhon.ps1

$procs = Get-Process -Name 'likhon' -ErrorAction SilentlyContinue
if (-not $procs) {
    Write-Host 'No likhon.exe is running.'
    exit 0
}
foreach ($p in $procs) {
    Write-Host ("Stopping likhon.exe (PID {0}) {1}" -f $p.Id, $p.Path)
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Milliseconds 300
if (Get-Process -Name 'likhon' -ErrorAction SilentlyContinue) {
    Write-Host 'Some likhon.exe processes are still running. They may belong to another user or run as administrator.'
    exit 1
}
Write-Host 'Done. Keyboard input is back to normal.'

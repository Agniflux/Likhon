# Likhon: reports which development tools are installed. It changes nothing.
# Run from this folder:  ./check-toolchain.ps1

$ErrorActionPreference = 'SilentlyContinue'
$missing = New-Object System.Collections.Generic.List[string]

function Show($label, $ok, $detail) {
    $mark = if ($ok) { '[ok]     ' } else { '[MISSING]' }
    Write-Host ('{0} {1,-30} {2}' -f $mark, $label, $detail)
    if (-not $ok) { $missing.Add($label) }
}

function Ver($label, $exe, $arg) {
    $cmd = Get-Command $exe -ErrorAction SilentlyContinue
    if ($cmd) {
        $out = (& $exe $arg 2>&1 | Select-Object -First 1)
        Show $label $true "$out"
    } else {
        Show $label $false 'not found on PATH'
    }
}

Write-Host ''
Write-Host 'Likhon toolchain check'
Write-Host '----------------------'

# Windows and elevation
$os = [System.Environment]::OSVersion.Version
Write-Host ("Windows version: {0}   (Windows 10 or 11 expected)" -f $os)
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host ("Running as administrator: {0}   (not needed for this check)" -f $isAdmin)
Write-Host ''

# Core tools
Ver 'Git' 'git' '--version'
Ver 'Rust compiler' 'rustc' '--version'
Ver 'Cargo' 'cargo' '--version'
Ver 'Node.js' 'node' '--version'
Ver 'npm' 'npm' '--version'
Ver 'PHP' 'php' '--version'
Ver 'Composer' 'composer' '--version'

# Rust targets (the text-service DLL needs both)
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $targets = (rustup target list --installed 2>&1) -join ' '
    Show 'Rust target x86_64 (64-bit)' ($targets -match 'x86_64-pc-windows-msvc') 'x86_64-pc-windows-msvc'
    Show 'Rust target i686 (32-bit)' ($targets -match 'i686-pc-windows-msvc') 'i686-pc-windows-msvc'
    Show 'Rust target wasm32' ($targets -match 'wasm32-unknown-unknown') 'wasm32-unknown-unknown (for the website engine)'
} else {
    Show 'rustup' $false 'not found on PATH'
}

# Optional Rust helpers
$tauri = (cargo tauri --version 2>&1 | Select-Object -First 1)
Show 'cargo tauri (Tauri CLI)' ($tauri -match 'tauri') "$tauri"
Ver 'wasm-pack (or wasm-bindgen)' 'wasm-pack' '--version'

# C++ build tools (needed for Rust on Windows and for the text-service DLL)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path $vswhere) {
    $vs = & $vswhere -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>&1 | Select-Object -First 1
    Show 'Visual Studio C++ build tools' ([bool]$vs) "$vs"
} else {
    Show 'Visual Studio C++ build tools' $false 'vswhere not found: install Build Tools for Visual Studio with the C++ workload and a Windows SDK'
}

# Installer tools (one will be chosen: see Desktop/CLAUDE.md section 9)
$iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $iscc) { $iscc = Get-Item "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" -ErrorAction SilentlyContinue }
Show 'Inno Setup (optional)' ([bool]$iscc) 'candidate installer tool'
Show 'NSIS (optional)' ([bool](Get-Command makensis -ErrorAction SilentlyContinue)) 'candidate installer tool'

# WebView2 runtime (the Settings window needs it; typing does not)
$wv = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
if (-not $wv) { $wv = Get-ItemProperty 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue }
Show 'WebView2 runtime' ([bool]$wv) $(if ($wv) { "version $($wv.pv)" } else { 'not detected (Settings window will not open without it)' })

Write-Host ''
if ($missing.Count -eq 0) {
    Write-Host 'Everything needed was found.'
} else {
    Write-Host ('Missing or not detected: ' + ($missing -join ', '))
    Write-Host 'Install what is missing, open a new terminal, and run this again.'
}

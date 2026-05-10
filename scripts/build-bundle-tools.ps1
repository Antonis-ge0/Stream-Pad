$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$bootstrapperDir = Join-Path $root "installer-bootstrapper"
$sourceExe = Join-Path $bootstrapperDir "target\release\stream_pad_installer.exe"
$targetExe = Join-Path $root "src-tauri\installer\stream-pad-maintenance-x86_64-pc-windows-msvc.exe"

Push-Location $bootstrapperDir
try {
  cargo build --release
}
finally {
  Pop-Location
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $targetExe) | Out-Null
Copy-Item -Force $sourceExe $targetExe

$ErrorActionPreference = "Stop"

$vswhereCommand = Get-Command "vswhere.exe" -ErrorAction SilentlyContinue
if ($vswhereCommand) {
  $vswherePath = $vswhereCommand.Source
} else {
  $programFilesX86 = [Environment]::GetFolderPath(
    [Environment+SpecialFolder]::ProgramFilesX86
  )
  $vswherePath = Join-Path $programFilesX86 "Microsoft Visual Studio\Installer\vswhere.exe"
}

if (-not (Test-Path -LiteralPath $vswherePath)) {
  throw "vswhere.exe was not found. Install Visual Studio Build Tools with Desktop development with C++."
}

$installationPath = & $vswherePath `
  -latest `
  -products "*" `
  -requires "Microsoft.VisualStudio.Component.VC.Tools.x86.x64" `
  -property installationPath |
  Select-Object -First 1

if ([string]::IsNullOrWhiteSpace($installationPath)) {
  throw "No complete MSVC x64 toolchain was found. Install Desktop development with C++."
}

$vcvarsPath = Join-Path $installationPath.Trim() "VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path -LiteralPath $vcvarsPath)) {
  throw "vcvars64.bat was not found in the selected Visual Studio installation."
}

$command = 'call "{0}" >nul && npm.cmd run tauri dev' -f $vcvarsPath
& $env:ComSpec /d /c $command
exit $LASTEXITCODE

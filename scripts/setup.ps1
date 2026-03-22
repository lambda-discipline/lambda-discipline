# This script sets up the project which includes building the CMake project.
# Should be run from within the PROJECT ROOT.

# --- CMake Project Setup ---
$BuildDir = "cmake-build-debug"
if (!(Test-Path $BuildDir))
{
    New-Item -ItemType Directory -Path $BuildDir | Out-Null
}
Set-Location $BuildDir
cmake ..
cmake --build .
Set-Location ..

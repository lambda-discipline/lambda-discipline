# This script sets up the project which includes building the CMake project, and
# creating python environment for development.
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

# --- Python Environment Setup ---
$VenvDir = "venv"
if (!(Test-Path $VenvDir))
{
    python -m venv $VenvDir
}
. .\$VenvDir\Scripts\Activate.ps1
python -m pip install --upgrade pip
pip install -r requirements.txt
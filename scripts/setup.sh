#!/usr/bin/env bash
# This script sets up the project which includes building the CMake project,
# and creating a Python environment for development.
# Should be run from within the PROJECT ROOT.

set -e

# --- CMake Project Setup ---
BuildDir="cmake-build-debug"
if [[ ! -d "$BuildDir" ]]; then
    mkdir "$BuildDir"
fi
cd "$BuildDir"
cmake ..
cmake --build .
cd ..

# --- Python Environment Setup ---
VenvDir="venv"
if [[ ! -d "$VenvDir" ]]; then
    python3 -m venv "$VenvDir"
fi
source "$VenvDir/bin/activate"
python -m pip install --upgrade pip
if [[ -f "requirements.txt" ]]; then
    pip install -r requirements.txt
fi

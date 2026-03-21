#!/usr/bin/env bash
# This script sets up the project which includes building the CMake project,
# and creating a Python environment for development.
# Should be run from within the PROJECT ROOT.

set -eu

SCRIPTS_DIR_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_PATH="$(cd "$SCRIPTS_DIR_PATH/.." && pwd)"
BUILD_DIR="$ROOT_PATH/cmake-build-debug"
VENV_DIR="$ROOT_PATH/venv"

# Install dependencies.
sudo bash "$SCRIPTS_DIR_PATH/install.sh"

# CMake Project Setup.
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"
cmake "$ROOT_PATH"
cmake --build "$BUILD_DIR"

# Python Environment Setup.
if [[ ! -d "$VENV_DIR" ]]; then
    python3 -m venv "$VENV_DIR"
fi

source "$VENV_DIR/bin/activate"
python -m pip install --upgrade pip
if [[ -f "requirements.txt" ]]; then
    pip install -r requirements.txt
fi

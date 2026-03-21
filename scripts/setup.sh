#!/usr/bin/env bash
# This script sets up the project which includes building the CMake project,
# and creating a Python environment for development.
# Should be run from within the PROJECT ROOT.

set -eu

SCRIPTS_DIR_PATH=$(dirname "${BASH_SOURCE[0]}")
ROOT_PATH=$(dirname "$SCRIPTS_DIR_PATH")
BUILD_DIR="$ROOT_PATH/cmake-build-debug"

# Install dependencies.
# TODO: Support installing dependencies in a cross platform manner.
sudo apt-get install -y libreadline-dev

# CMake Project Setup.
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"
cmake "$ROOT_PATH"
cmake --build "$BUILD_DIR"

# Python Environment Setup.
cd "$ROOT_PATH"

VENV_DIR="venv"
if [[ ! -d "$VENV_DIR" ]]; then
    python3 -m venv "$VENV_DIR"
fi

source "$VENV_DIR/bin/activate"
python -m pip install --upgrade pip
if [[ -f "requirements.txt" ]]; then
    pip install -r requirements.txt
fi

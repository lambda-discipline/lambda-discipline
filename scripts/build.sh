#!/usr/bin/env bash
set -xeu

# This script sets up the project which includes building the CMake project.

SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPTS_DIR/.." && pwd)"
BUILD_DIR="$ROOT_DIR/cmake-build-debug"

source "$ROOT_DIR/scripts/commons.sh"

# Usage: ./build.sh [Debug|Release|RelWithDebInfo|MinSizeRel]
BUILD_TYPE="${1:-Debug}"
case "$BUILD_TYPE" in
  Debug|Release|RelWithDebInfo|MinSizeRel) ;;
  *) log_fatal "Invalid build type \"$BUILD_TYPE\"."; exit 1 ;;
esac

# Install dependencies.
log_info "Installing dependencies."
sudo apt-get update
sudo apt-get install -y  cmake libreadline-dev

# CMake Project Setup.
mkdir -p "$BUILD_DIR"
cmake -S "$ROOT_DIR" -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE="$BUILD_TYPE"
log_info "Building cmake project."
cmake --build "$BUILD_DIR" --config "$BUILD_TYPE"

#!/usr/bin/env bash
set -eu

# TODO: Support installing dependencies in a cross platform manner.
sudo apt-get update
sudo apt-get install -y  cmake libreadline-dev
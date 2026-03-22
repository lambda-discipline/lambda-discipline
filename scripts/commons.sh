#!/usr/bin/env bash
set -eu

# Enable colored output, when inside TTY mode.
if [ -t 1 ]; then
    RED="\033[0;31m"
    GREEN="\033[1;32m"
    BLUE="\033[0;34m"
    GRAY="\033[0;90m"
    RESET="\033[0m"
else
    RED=""
    GREEN=""
    BLUE=""
    GRAY=""
    RESET=""
fi

function log_info() { echo -e "[${BLUE}INFO${RESET}] $*"; }
function log_pass() { echo -e "[${GREEN}PASS${RESET}] $*"; }
function log_note() { echo -e "[${GRAY}NOTE${RESET}] $*";  }
function log_fail() { echo -e "[${RED}FAIL${RESET}] $*" >&2; }
function log_error() { echo -e "[${RED}ERROR${RESET}] $*" >&2; }
function log_fatal() { echo -e "[${RED}FATAL${RESET}] $*" >&2; exit 1; }

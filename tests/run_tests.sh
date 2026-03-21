#!/usr/bin/env bash
set -eu

LBD="cmake-build-debug/lbd"
TESTS_DIR_PATH=$(dirname "${BASH_SOURCE[0]}")
IGNORE_FILE_PATH="$TESTS_DIR_PATH/ignore_tests.list"

# Enable colored output, when inside TTY mode.
if [ -t 1 ]; then
    RED="\033[0;31m"
    GREEN="\033[1;32m"
    YELLOW="\033[1;33m"
    BLUE="\033[0;34m"
    CYAN="\033[0;36m"
    GRAY="\033[0;90m"
    RESET="\033[0m"
else
    RED=""
    GREEN=""
    YELLOW=""
    BLUE=""
    CYAN=""
    GRAY=""
    RESET=""
fi

function log_info() { echo -e "[${BLUE}INFO${RESET}] $*"; }
function log_pass() { echo -e "[${GREEN}PASS${RESET}] $*"; }
function log_note() { echo -e "[${GRAY}NOTE${RESET}] $*";  }
function log_fail() { echo -e "[${RED}FAIL${RESET}] $*" >&2; }
function log_error() { echo -e "[${RED}ERROR${RESET}] $*" >&2; }

passed=0
failed=0
skipped=0
total=0

while read -r source; do
    total=$((total+1))

    # If file is present inside ignore list; then continue.
    if [ -f "$IGNORE_FILE_PATH" ] && grep -Fxq "$source" "$IGNORE_FILE_PATH"; then
        log_note "Skipping tests for \"$source\"."
        skipped=$((skipped+1))
        continue
    fi

    rel_test_file_path="${source#examples/}"
    out_file_path="examples/out/${rel_test_file_path%.lbd}.out.txt"

    if [ ! -f "$out_file_path" ]; then
        log_error "Missing output file \"$out_file_path\"."
        failed=$((failed+1))
        continue
    fi

    start=$(date +%s%N)
    if diff -u <("$LBD" -f "$source") "$out_file_path"; then
        end=$(date +%s%N)
        elapsed=$(( (end - start) / 1000000 ))
        log_pass "Test \"$source\" passed. (${elapsed} ms)"
        passed=$((passed+1))
    else
        end=$(date +%s%N)
        elapsed=$(( (end - start) / 1000000 ))
        log_fail "Test \"$source\" failed. (${elapsed} ms)"
        failed=$((failed+1))
    fi
done < <(find examples -name "*lbd")

log_info "===================="
log_info "Total: $total"
log_info "Passed: $passed"
log_info "Failed: $failed"
log_info "Skipped: $skipped"
log_info "===================="

if [ "$failed" -ne 0 ]; then
    exit 1
fi

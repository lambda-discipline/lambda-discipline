#!/usr/bin/env bash
set -eu

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$TESTS_DIR/.." && pwd)"
LBD="$ROOT_DIR/cmake-build-debug/lbd"
IGNORE_FILE="$TESTS_DIR/ignore_tests.list"

source "$ROOT_DIR/scripts/commons.sh"

GENERATE=0
if [[ "${1:-}" == "--generate" ]]; then
    GENERATE=1
    log_info "Golden output generation mode enabled."
fi

passed=0
failed=0
skipped=0
total=0

while read -r source; do
    total=$((total+1))

    # If file is present inside ignore list; then continue.
    if [ -f "$IGNORE_FILE" ] && grep -Fxq "$source" "$IGNORE_FILE"; then
        log_note "Skipping tests for \"$source\"."
        skipped=$((skipped+1))
        continue
    fi

    out_file_path="${source%.lbd}.out"

    if [ "$GENERATE" -eq 1 ]; then
        if ! "$LBD" -f "$source" > "$out_file_path" 2>&1; then
            log_note "Generated (with errors) \"$out_file_path\"."
        else
            log_note "Generated \"$out_file_path\"."
        fi
        continue
    fi

    if [ ! -f "$out_file_path" ]; then
        log_error "Missing output file \"$out_file_path\"."
        failed=$((failed+1))
        continue
    fi

    start=$(date +%s%N)
    if diff -u <("$LBD" -f "$source" 2>&1 || true) "$out_file_path"; then
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
done < <(find tests -name "*lbd")

if [ "$GENERATE" -eq 1 ]; then
    exit 0
fi

log_info "===================="
log_info "Total: $total"
log_info "Passed: $passed"
log_info "Failed: $failed"
log_info "Skipped: $skipped"
log_info "===================="

if [ "$failed" -ne 0 ]; then
    exit 1
fi

#!/usr/bin/env bash
set -eu

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$TESTS_DIR/.." && pwd)"
LBD="$ROOT_DIR/cmake-build-debug/lbd"
IGNORE_FILE="$TESTS_DIR/ignore_tests.list"

source "$ROOT_DIR/scripts/commons.sh"

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

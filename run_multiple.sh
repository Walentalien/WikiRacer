#!/usr/bin/env bash
set -euo pipefail

# Build the project once in release mode so subsequent runs are fast
# cargo build --release

BIN="$(pwd)/target/release/WikiRacer"
LOG_DIR="run_logs"
mkdir -p "$LOG_DIR"

# Define start and target link pairs separated by |
CASES=(
    "https://en.wikipedia.org/wiki/Rust_(programming_language)|https://en.wikipedia.org/wiki/Mozilla"
    "https://en.wikipedia.org/wiki/Python_(programming_language)|https://en.wikipedia.org/wiki/Guido_van_Rossum"
    "https://en.wikipedia.org/wiki/Java_(programming_language)|https://en.wikipedia.org/wiki/James_Gosling"
)

run_id=1
for case in "${CASES[@]}"; do
    start_url="${case%%|*}"
    target_url="${case##*|}"
    timestamp="$(date +%Y-%m-%d_%H-%M-%S)"
    log_file="$LOG_DIR/run_${run_id}_${timestamp}.log"

    echo "Run $run_id: $start_url -> $target_url"
    start_sec=$(date +%s)
    "$BIN" --start-url "$start_url" --target-url "$target_url" > "$log_file" 2>&1
    end_sec=$(date +%s)
    duration=$((end_sec - start_sec))
    echo "Duration: ${duration}s" >> "$log_file"
    echo "Finished run $run_id in ${duration}s. Log saved to $log_file"
    ((run_id++))
    echo
done
#!/bin/bash

# File to check (pass it as an argument)
FILE="$1"

if [[ ! -f "$FILE" ]]; then
    echo "Usage: $0 <file>"
    exit 1
fi

# Count total duplicate lines
total_lines=$(sort "$FILE" | wc -l)
unique_lines=$(sort "$FILE" | uniq | wc -l)
duplicate_lines=$((total_lines - unique_lines))

echo "Total line count: $total_lines"
echo "Unique line count: $unique_lines"
echo "Total duplicates count: $duplicate_lines"


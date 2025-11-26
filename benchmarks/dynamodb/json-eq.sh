#!/bin/sh

# Semantic JSON comparison using jq
# Usage: json-eq file1.json file2.json

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 file1.json file2.json" >&2
    exit 1
fi

# Create temporary files
tmp1=$(mktemp)
tmp2=$(mktemp)

# Ensure cleanup on exit
trap 'rm -f "$tmp1" "$tmp2"' EXIT

# Sort keys and normalize JSON
jq -S . "$1" > "$tmp1"
jq -S . "$2" > "$tmp2"

# Compare normalized JSON
diff -q "$tmp1" "$tmp2"

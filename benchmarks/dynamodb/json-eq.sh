#!/bin/sh

# Semantic JSON comparison using jq
# Usage: json-eq.sh [--keep] file1.json file2.json
#   --keep    Keep temporary files (print paths to stderr)

KEEP_TEMP=0

# Parse options
if [ "$1" = "--keep" ]; then
    KEEP_TEMP=1
    shift
fi

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 [--keep] file1.json file2.json" >&2
    exit 1
fi

# Create temporary files
tmp1=$(mktemp)
tmp2=$(mktemp)

# Ensure cleanup on exit (unless --keep flag is set)
if [ "$KEEP_TEMP" -eq 0 ]; then
    trap 'rm -f "$tmp1" "$tmp2"' EXIT
else
    echo "Temporary files: $tmp1 $tmp2" >&2
fi

# Sort keys and normalize JSON
jq -S . "$1" > "$tmp1"
jq -S . "$2" > "$tmp2"

# Compare normalized JSON
diff -q "$tmp1" "$tmp2"

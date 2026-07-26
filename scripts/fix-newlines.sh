#!/bin/bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Fix literal \n artifacts in manifest and README files.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Checking for literal \\n artifacts in manifest and README files..."

# Find all .a2ml and README.adoc files
mapfile -t FILES < <(find "$REPO_ROOT" -type f \( -name "*.a2ml" -o -name "README.adoc" \) ! -path "*/.git/*")

TOTAL_COUNT=${#FILES[@]}
FIXED_COUNT=0

echo "Checking $TOTAL_COUNT files..."

# First pass: fix files
for filepath in "${FILES[@]}"; do
    if grep -q '\\\\n' "$filepath" 2>/dev/null; then
        # Create a temporary file
        tmpfile=$(mktemp)
        # Replace literal \n with actual newlines
        sed 's/\\\\n/\n/g' "$filepath" > "$tmpfile"
        # Check if the file was changed
        if ! cmp -s "$filepath" "$tmpfile"; then
            mv "$tmpfile" "$filepath"
            echo "Fixed: $filepath"
            ((FIXED_COUNT++))
        else
            rm "$tmpfile"
        fi
    fi
done

echo ""
echo "Fixed $FIXED_COUNT files out of $TOTAL_COUNT checked."

# Second pass: verify no more artifacts
REMANING=()
for filepath in "${FILES[@]}"; do
    if grep -q '\\\\n' "$filepath" 2>/dev/null; then
        REMANING+=("$filepath")
    fi
done

if [ ${#REMANING[@]} -gt 0 ]; then
    echo ""
    echo "WARNING: ${#REMANING[@]} files still have \\n artifacts:"
    for f in "${REMANING[@]:0:10}"; do
        echo "  $f"
    done
    if [ ${#REMANING[@]} -gt 10 ]; then
        echo "  ... and $(( ${#REMANING[@]} - 10 )) more"
    fi
    exit 1
else
    echo ""
    echo "All files verified - no more \\n artifacts found!"
    exit 0
fi

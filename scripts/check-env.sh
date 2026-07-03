#!/bin/bash
set -euo pipefail

missing=()

if ! command -v tsc &> /dev/null; then
  missing+=("tsc (TypeScript) — install with: npm install -g typescript")
fi

if ! command -v sass &> /dev/null; then
  missing+=("sass (Dart Sass) — install with: npm install -g sass")
fi

if [ ${#missing[@]} -gt 0 ]; then
  echo "ERROR: Missing required build tools:"
  for m in "${missing[@]}"; do
    echo "  • $m"
  done
  exit 1
fi

echo "✓ All build tools available"

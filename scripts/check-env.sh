#!/bin/bash
set -euo pipefail

missing=()

if ! command -v pnpm &> /dev/null; then
  missing+=("pnpm — install pnpm 11.17.0 or run through Corepack")
else
  if ! pnpm exec tsc --version &> /dev/null; then
    missing+=("tsc (TypeScript) — run: pnpm install")
  fi

  if ! pnpm exec sass --version &> /dev/null; then
    missing+=("sass (Dart Sass) — run: pnpm install")
  fi
fi

if [ ${#missing[@]} -gt 0 ]; then
  echo "ERROR: Missing required build tools:"
  for m in "${missing[@]}"; do
    echo "  • $m"
  done
  exit 1
fi

echo "✓ All build tools available"

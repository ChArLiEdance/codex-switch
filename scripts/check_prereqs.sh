#!/usr/bin/env sh
set -eu

printf 'node: '
node --version
printf 'npm: '
npm --version

if command -v cargo >/dev/null 2>&1; then
  printf 'cargo: '
  cargo --version
else
  echo 'cargo: missing'
fi

if command -v rustc >/dev/null 2>&1; then
  printf 'rustc: '
  rustc --version
else
  echo 'rustc: missing'
fi


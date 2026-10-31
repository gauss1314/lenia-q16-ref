#!/bin/sh
# Fails if the WASM module contains any float instruction or float-typed value
# (spec §7.3 item 1, D-037). Needs wasm2wat (wabt).
set -e
wat=$(wasm2wat "$1")
hits=$(printf '%s\n' "$wat" | grep -nE '\b(f32|f64)(\.| |\))' || true)
if [ -n "$hits" ]; then
  echo "float instructions found in $1:"; printf '%s\n' "$hits" | head -20; exit 1
fi
echo "no float instructions in $1"

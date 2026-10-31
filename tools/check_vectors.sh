#!/bin/sh
# Runs public vectors natively (and, with --wasm, through the WASM build in Node)
# and compares chain and final-state hashes with vectors/v0/vectors.tsv.
set -e
cd "$(dirname "$0")/.."
L=./target/release/lq16
W=target/wasm32-unknown-unknown/release/lq16_wasm.wasm
tmp=$(mktemp -d)
fail=0
grep -v '^#' vectors/v0/vectors.tsv | while IFS="$(printf '\t')" read -r name params n sz amp seed steps chain final; do
  $L soup --out "$tmp/$name.state" --n "$n" --size "$sz" --amp "$amp" --seed "$seed" >/dev/null
  $L info "$params" "$tmp/$name" >/dev/null
  out=$($L run --params "$params" --state "$tmp/$name.state" --steps "$steps" --threads 2)
  got=$(echo "$out" | awk '/^chain/{print $2}'); gotf=$(echo "$out" | awk '/^final/{print $2}')
  if [ "$got" = "$chain" ] && [ "$gotf" = "$final" ]; then echo "native $name ok"; else echo "native $name MISMATCH $got"; exit 1; fi
  if [ "$1" = "--wasm" ]; then
    node tools/wasm_run.mjs "$W" "$tmp/$name/params.lq16" "$tmp/$name.state" "$steps" "$chain" "$final" "$name"
  fi
done

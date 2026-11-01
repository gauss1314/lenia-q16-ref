#!/bin/sh
# Re-runs the public benchmark structures (bench/v0) and compares chains.
# Native uses the NTT path; with --wasm also through the WASM build (NTT) in Node.
set -e
cd "$(dirname "$0")/.."
L=./target/release/lq16
W=target/wasm32-unknown-unknown/release/lq16_wasm.wasm
tmp=$(mktemp -d)
grep -v '^#' bench/v0/benchmarks.tsv | while IFS="$(printf '\t')" read -r name params state steps chain rest; do
  out=$($L run --params "$params" --state "bench/v0/$state" --steps "$steps" --algo ntt)
  got=$(echo "$out" | awk '/^chain/{print $2}')
  if [ "$got" = "$chain" ]; then echo "native bench $name ok"; else echo "native bench $name MISMATCH $got"; exit 1; fi
  if [ "$1" = "--wasm" ]; then
    final=$(echo "$out" | awk '/^final/{print $2}')
    $L info "$params" "$tmp/$name" >/dev/null
    node tools/wasm_run.mjs "$W" "$tmp/$name/params.lq16" "bench/v0/$state" "$steps" "$chain" "$final" "$name" 1
  fi
done

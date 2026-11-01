#!/bin/sh
# Writes params and soup records for every public vector into vec-tmp/ (git-ignored).
set -e
cd "$(dirname "$0")/.."
L=./target/release/lq16
mkdir -p vec-tmp
grep -v '^#' vectors/v0/vectors.tsv | while IFS="$(printf '\t')" read -r name params n sz amp seed steps chain final; do
  $L soup --out "vec-tmp/$name.state" --n "$n" --size "$sz" --amp "$amp" --seed "$seed" >/dev/null
  $L info "$params" "vec-tmp/$name" >/dev/null
done
grep -v '^#' bench/v0/benchmarks.tsv | while IFS="$(printf '\t')" read -r name params rest; do
  $L info "$params" "vec-tmp/bench-$name" >/dev/null
done

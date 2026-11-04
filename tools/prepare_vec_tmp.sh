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
# full G1 vectors (vectors/g1/g1.tsv, written by tools/check_g1.sh --write)
if [ -f vectors/g1/g1.tsv ]; then
  grep -v '^#' vectors/g1/g1.tsv | while IFS="$(printf '\t')" read -r name params src rest; do
    $L info "$params" "vec-tmp/g1-$name" >/dev/null
    case "$src" in
      soup:*) $L soup --out "vec-tmp/g1-$name.state" --n 256 --size "$(echo "$src" | cut -d: -f2)" --amp 65535 --seed "$(echo "$src" | cut -d: -f3)" >/dev/null ;;
      *) cp "$src" "vec-tmp/g1-$name.state" ;;
    esac
  done
fi

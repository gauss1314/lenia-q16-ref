#!/bin/sh
# Full G1 matrix (phase 0 plan §6.1): the 8 self-found benchmarks and 2 random soups,
# 2000 TU each (20000 steps at T = 10), NTT. Compares chains with vectors/g1/g1.tsv.
#   tools/check_g1.sh            check
#   tools/check_g1.sh --write    (re)compute the chains and write vectors/g1/g1.tsv
set -e
cd "$(dirname "$0")/.."
L=./target/release/lq16
tmp=$(mktemp -d)
items() {
  grep -v '^#' bench/v0/benchmarks.tsv | while IFS="$(printf '\t')" read -r name params state rest; do
    printf 'bench-%s\t%s\tbench/v0/%s\n' "$name" "$params" "$state"; done
  # soups: self-found parameters, SHA-256 counter PRNG (lq16 soup), N = 256
  printf 'soup-1\tR=13 T=10 mu=1923 sigma=277 beta=1/2,1/1\tsoup:128:11\n'
  printf 'soup-2\tR=13 T=10 mu=1833 sigma=251 beta=1/3,1/1,1/2\tsoup:256:12\n'
}
state_of() {
  case "$1" in
    soup:*) sz=$(echo "$1" | cut -d: -f2); sd=$(echo "$1" | cut -d: -f3)
            $L soup --out "$tmp/s.lq16" --n 256 --size "$sz" --amp 65535 --seed "$sd" >/dev/null; echo "$tmp/s.lq16" ;;
    *) echo "$1" ;;
  esac
}
if [ "$1" = "--write" ]; then
  out=vectors/g1/g1.tsv
  printf '# Full G1 vectors (spec %s): 2000 TU each, NTT. Self-found parameters only.\n# name\tparams\tsource\tsteps\tchain\tfinal\n' "$(grep -o 'lenia-q16/[^"]*' src/params.rs)" > "$out"
  items | while IFS="$(printf '\t')" read -r name params src; do
    T=$(echo "$params" | sed 's/.*T=\([0-9]*\).*/\1/'); steps=$((2000 * T))
    r=$($L run --params "$params" --state "$(state_of "$src")" --steps "$steps" --algo ntt)
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$params" "$src" "$steps" "$(echo "$r" | awk '/^chain/{print $2}')" "$(echo "$r" | awk '/^final/{print $2}')" >> "$out"
    echo "wrote $name"
  done
  exit 0
fi
grep -v '^#' vectors/g1/g1.tsv | while IFS="$(printf '\t')" read -r name params src steps chain final; do
  r=$($L run --params "$params" --state "$(state_of "$src")" --steps "$steps" --algo ntt)
  got=$(echo "$r" | awk '/^chain/{print $2}')
  if [ "$got" = "$chain" ]; then echo "g1 $name ok"; else echo "g1 $name MISMATCH $got"; exit 1; fi
done

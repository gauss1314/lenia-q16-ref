"""G2 verdict under the D-067 rule (Phase 0, not normative).

  python3 g2_verdict.py results.jsonl        (lines from compare_g2.py: 1 int + 4 noise seeds per state)

Per item the tolerance is the larger of the table value (measurement report §1) and twice the
largest natural fluctuation over the noise seeds. The class is compared only when every noise
seed gives the float64 class; otherwise the item is "not compared" and the numbers decide.
Survival is not judged when any noise seed dies or grows without bound. Prints a Markdown table.
"""
import json, sys
from collections import defaultdict

TABLE = dict(speed_rel=0.05, direction_deg=5.0, omega_rel=0.05, freq_rel=0.05, mass_rel=0.02)
IOU_MIN = 0.9
ITEMS = ['alive', 'cls', 'speed_rel', 'direction_deg', 'omega_rel', 'freq_rel', 'mass_rel', 'morph_iou']


def verdict(intr, noise):
    out, ok = {}, True
    unstable = any(not n['other']['alive'] for n in noise)
    if unstable:
        out['alive'] = 'not judged (noise seed not bounded)'
    else:
        good = intr['diff']['alive_same']
        out['alive'] = 'ok' if good else 'FAIL'
        ok &= good
    f64cls = intr['f64']['cls']
    if all(n['other']['cls'] == f64cls for n in noise):
        good = intr['other']['cls'] == f64cls
        out['cls'] = 'ok' if good else f"FAIL ({intr['other']['cls']} vs {f64cls})"
        ok &= good
    else:
        out['cls'] = 'not compared (seeds: ' + '/'.join(sorted({n['other']['cls'] for n in noise})) + ')'
    for k, base in TABLE.items():
        v = intr['diff'][k]
        nv = [n['diff'][k] for n in noise if n['diff'][k] is not None and n['other']['alive']]
        tol = max([base] + [2 * x for x in nv])
        if v is None:
            out[k] = '—'
            continue
        good = v <= tol
        out[k] = f"{'ok' if good else 'FAIL'} {v:.3g}≤{tol:.3g}" if good else f"FAIL {v:.3g}>{tol:.3g}"
        ok &= good
    v = intr['diff']['morph_iou']
    nd = [1 - n['diff']['morph_iou'] for n in noise if n['other']['alive']]
    need = min([IOU_MIN] + [1 - 2 * d for d in nd])
    good = v >= need
    out['morph_iou'] = f"{'ok' if good else 'FAIL'} {v:.3f}≥{need:.3f}" if good else f"FAIL {v:.3f}<{need:.3f}"
    ok &= good
    return ok, out


def main():
    by = defaultdict(lambda: {'int': None, 'noise': []})
    for line in open(sys.argv[1]):
        d = json.loads(line)
        if d['mode'] == 'int':
            by[d['state']]['int'] = d
        else:
            by[d['state']]['noise'].append(d)
    print('| structure | pass | ' + ' | '.join(ITEMS) + ' |')
    print('|' + '---|' * (len(ITEMS) + 2))
    for st in sorted(by):
        r = by[st]
        if r['int'] is None or len(r['noise']) < 4:
            print(f"| {st} | incomplete ({len(r['noise'])} seeds) |" + ' |' * len(ITEMS))
            continue
        ok, out = verdict(r['int'], r['noise'])
        print(f"| {st.split('.')[0]} | {'PASS' if ok else 'FAIL'} | " + ' | '.join(out[k] for k in ITEMS) + ' |')


if __name__ == '__main__':
    main()

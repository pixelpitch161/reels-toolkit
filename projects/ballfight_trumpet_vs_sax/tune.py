"""Rule tuning for #29 (v3: combo vs flail) (once, then frozen in timeline.py / fighters.py): win share, median K.O., lead changes and fit
rate over seeds, for a grid of roster / rule overrides.

    python tune.py 40 T.dmg=3.5,4 S.dmg=1.2 S.curve=90,110 scale=0.8
        T.<key> / S.<key>      TRUMPET / SAX weapon fields;  T.u1.<key>, S.low.<key> ...  ability fields
        scale                  the damage scale (timeline.DMG_SCALE)"""
import itertools
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sim  # noqa: E402  (sets up the engine path)
import fighters as FT  # noqa: E402
import timeline as T  # noqa: E402
from fight import Fight, Rules, lead_changes  # noqa: E402

WHO = {'T': 'TRUMPET', 'S': 'SAX'}
SLOT = {'u1': 'unlock1', 'u2': 'unlock2', 'low': 'low'}


def apply(cell):
    sc = T.DMG_SCALE
    for key, val in cell:
        if key == 'scale':
            sc = val
            continue
        parts = key.split('.')
        cfg = FT.FIGHTERS[WHO[parts[0]]]
        if len(parts) == 2:
            cfg['weapon'][parts[1]] = val
        else:
            cfg['abilities'][SLOT[parts[1]]][parts[2]] = val
    return sc


def one(args):
    cell, seed = args
    sc = apply(cell)
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=Rules(bpm=T.BPM, t_fight=T.T_FIGHT, dmg_scale=sc),
              record=False).run()
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    lead15 = 0
    n_hits = [0, 0]
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
        if e[0] <= 15.0:
            lead15 = np.sign(hp[0] - hp[1])
    caps = len(sim.captions(r['events']))
    shots = sum(1 for e in r['events'] if e[1] == 'punch')
    landed = sum(1 for e in r['events'] if e[1] == 'hit' and e[7] == 'piston')
    wraps = sum(1 for e in r['events'] if e[1] == 'wrap')
    hc = sum(1 for e in r['events'] if e[1] == 'hit' and e[7] == 'beam')
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), lead15, caps, n_hits, landed / max(1, shots), wraps, hc)


if __name__ == '__main__':
    n = int(sys.argv[1])
    axes = []
    for a in sys.argv[2:]:
        k, v = a.split('=')
        axes.append([(k, float(x)) for x in v.split(',')])
    cells = list(itertools.product(*axes)) or [()]
    jobs = [(c, s) for c in cells for s in range(1000, 1000 + n)]
    with Pool(2) as P:
        res = P.map(one, jobs, chunksize=4)
    for cell in cells:
        r = [x for x in res if x[0] == cell]
        g = np.mean([x[2] == 0 for x in r if x[2] is not None]) if any(x[2] is not None for x in r) else float('nan')
        tko = np.array([x[3] for x in r])
        lc = np.mean([x[4] for x in r])
        whp = np.median([x[5] for x in r])
        fit = np.mean([(49 <= x[3] <= 53) and x[4] >= 2 and x[5] <= 30 and x[7] >= 5 for x in r])
        caps = np.mean([x[7] for x in r])
        hits = np.mean([x[8] for x in r], 0)
        acc = np.mean([x[9] for x in r])
        t15 = np.mean([x[6] > 0 for x in r])
        wr = np.mean([x[10] for x in r])
        hc = np.mean([x[11] for x in r])
        lab = ' '.join(f'{k}={v:g}' for k, v in cell)
        print(f'{lab}: trumpet win {100 * g:3.0f}%  KO med {np.median(tko):5.1f} ({np.percentile(tko, 10):4.1f}-'
              f'{np.percentile(tko, 90):4.1f})  lc {lc:.2f}  whp {whp:3.0f}  caps {caps:.1f}  fit {100 * fit:3.1f}%  '
              f'hits T/S {hits[0]:.0f}/{hits[1]:.0f}  punch acc {100 * acc:.0f}%  wraps {wr:.1f}  HIGH C hit {100 * hc:.0f}%  T@15 {100 * t15:.0f}%', flush=True)

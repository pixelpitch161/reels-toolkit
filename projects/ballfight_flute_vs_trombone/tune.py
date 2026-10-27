"""Handicap tuning for #41 (once, then frozen in timeline.DMG_SIDE). Crossover rule: both fighters fight exactly as built
in fighters.py; the ONLY lever is the damage each one deals. Reports the win share, median K.O., lead changes, fit rate
and the longest stretch without a hit over seeds, for a grid of handicaps.

    python tune.py 60 hf=1.0,1.2 ht=0.4,0.5         # hf = FLUTE's damage scale, ht = TROMBONE's

Seeds 1000.. (the honest search later uses 1..), so the tuning never picks the episode's run."""
import itertools
import os
import sys
from multiprocessing import Pool
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import sim  # noqa: E402  (sets up the engine path)
import timeline as T  # noqa: E402
from fight import Fight, lead_changes  # noqa: E402


def one(args):
    cell, seed = args
    c = dict(cell)
    hs = (c.get('hf', T.DMG_SIDE[0]), c.get('ht', T.DMG_SIDE[1]))
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=sim.rules(hs), record=False).run()
    st = sim.story(r)
    fits, _ = sim.score(st)
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    n_hits = [0, 0]
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), len(st['caps']), n_hits, st['max_gap'], fits,
            len(st['lows']))


if __name__ == '__main__':
    n = int(sys.argv[1])
    axes = []
    for a in sys.argv[2:]:
        k, v = a.split('=')
        axes.append([(k, float(x)) for x in v.split(',')])
    cells = list(itertools.product(*axes))
    jobs = [(c, s) for c in cells for s in range(1000, 1000 + n)]
    with Pool(2) as P:
        res = P.map(one, jobs, chunksize=4)
    for cell in cells:
        r = [x for x in res if x[0] == cell]
        dec = [x for x in r if x[2] is not None and x[3] < 99]
        g = np.mean([x[2] == 0 for x in dec]) if dec else 0.0
        tko = np.array([x[3] for x in r])
        lab = ' '.join(f'{k}={v:g}' for k, v in cell)
        hits = np.mean([x[7] for x in r], 0)
        print(f'{lab}: FLUTE wins {100 * g:3.0f}% of {len(dec)}  KO med {np.median(tko):5.1f} '
              f'({np.percentile(tko, 10):4.1f}-{np.percentile(tko, 90):4.1f})  no-KO {100 * np.mean(tko >= 99):2.0f}%  '
              f'lc {np.mean([x[4] for x in r]):.2f}  whp {np.median([x[5] for x in r]):3.0f}  caps {np.mean([x[6] for x in r]):.1f}  '
              f'both lows {100 * np.mean([x[10] == 2 for x in r]):2.0f}%  gap med {np.median([x[8] for x in r]):.1f}  '
              f'fit {100 * np.mean([x[9] for x in r]):.1f}%  hits F/T {hits[0]:.0f}/{hits[1]:.0f}', flush=True)

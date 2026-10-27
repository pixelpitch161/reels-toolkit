"""Tuning for #42 (once, then frozen in fighters.py VIOLA + timeline.DMG_SIDE). The VIOLIN fights exactly as built in #28
(crossover rule: its only lever is its damage handicap); the VIOLA is new, so its weapon numbers are tuned here once,
then frozen in the roster like every new fighter's. Reports the win share, median K.O., lead changes, fit rate, the hit
counts and the longest stretch without a hit over seeds, for a grid.

    python tune.py 60 hv=1.0 hn=1.2,1.4 w.roll=1.8,2.0 w.dmg=6,7      # hv / hn = VIOLA / VIOLIN damage handicap
                                                                     # w.<key> = VIOLA weapon, u1. / u2. / lo.<key> = its abilities

Seeds 1000.. (the honest search later uses 1..), so the tuning never picks the episode's run."""
import copy
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
from fighters import FIGHTERS  # noqa: E402

BASE = copy.deepcopy(FIGHTERS['VIOLA'])
SLOT = {'u1': 'unlock1', 'u2': 'unlock2', 'lo': 'low'}


def patch(c):
    FIGHTERS['VIOLA'] = copy.deepcopy(BASE)
    for k, v in c.items():
        if k.startswith('w.'):
            FIGHTERS['VIOLA']['weapon'][k[2:]] = v
        elif k[:3] in ('u1.', 'u2.', 'lo.'):
            FIGHTERS['VIOLA']['abilities'][SLOT[k[:2]]][k[3:]] = v


def one(args):
    cell, seed = args
    c = dict(cell)
    patch(c)
    hs = (c.get('hv', T.DMG_SIDE[0]), c.get('hn', T.DMG_SIDE[1]))
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=sim.rules(hs), record=False).run()
    st = sim.story(r)
    fits, _ = sim.score(st)
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    n_hits = [0, 0]
    dmg = [0.0, 0.0]
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
            dmg[e[2]] += e[4]
    notes = sum(1 for e in r['events'] if e[1] == 'note')
    crush = sum(1 for e in r['events'] if e[1] == 'crush')
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), len(st['caps']), n_hits, st['max_gap'], fits,
            len(st['lows']), notes, crush)


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
        land = sum(x[12] for x in r) / max(1, sum(x[11] for x in r))
        print(f'{lab}: VIOLA wins {100 * g:3.0f}% of {len(dec)}  KO med {np.median(tko):5.1f} '
              f'({np.percentile(tko, 10):4.1f}-{np.percentile(tko, 90):4.1f})  no-KO {100 * np.mean(tko >= 99):2.0f}%  '
              f'lc {np.mean([x[4] for x in r]):.2f}  caps {np.mean([x[6] for x in r]):.1f}  '
              f'both lows {100 * np.mean([x[10] == 2 for x in r]):2.0f}%  gap med {np.median([x[8] for x in r]):.1f}  '
              f'fit {100 * np.mean([x[9] for x in r]):.1f}%  hits VA/VN {hits[0]:.0f}/{hits[1]:.0f}  notes land {100 * land:.0f}%',
              flush=True)

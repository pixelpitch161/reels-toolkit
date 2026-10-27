"""Tuning grid, run once per pairing, then frozen (in episode.py for a crossover's handicap, in the roster for a new
fighter's numbers). Reports win share, median K.O. (10th-90th), no-K.O. share, lead changes, winner HP, captions, both
desperation moves, the longest gap between hits, the fit rate and the hits each side lands, over seeds 1000.. (the
honest search uses 1.., so tuning never picks the episode's run).

Axes (each a comma list):
    FLUTE=1.8,2.0           a fighter's damage handicap (crossover rule: the only per-episode lever)
    scale=0.55,0.6          the shared damage scale
    CLARINET.weapon.jitter=28,40    a roster number of a NEW fighter (part: weapon / unlock1 / unlock2 / low)

    python -m ballfight tune <episode> 300 FLUTE=2.0 TROMBONE=0.40"""
import itertools
import multiprocessing as mp
import time

import numpy as np

from .episode import current
from . import story


def _apply(cell):
    """this cell's roster numbers (a new fighter's own levers), in this worker's copy of the roster"""
    from fighters import FIGHTERS
    for k, v in cell:
        if '.' in k:
            name, part, field = k.split('.')
            d = FIGHTERS[name]['weapon'] if part == 'weapon' else FIGHTERS[name]['abilities'][part]
            d[field] = int(v) if isinstance(d.get(field), int) and not isinstance(d.get(field), bool) else v


def one(args):
    cell, seed = args
    from fight import Fight, lead_changes
    T = current().T
    c = dict(cell)
    _apply(cell)
    side = list(T.DMG_SIDE) if T.DMG_SIDE is not None else [1.0, 1.0]
    use_side = T.DMG_SIDE is not None
    for j, n in enumerate(T.FIGHTERS):
        if n in c:
            side[j] = c[n]
            use_side = True
    rules = story.rules(dmg_side=tuple(side) if use_side else None, dmg_scale=c.get('scale'))
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=rules, record=False).run()
    st = story.story(r)
    fits, _ = story.score(st)
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    n_hits = [0, 0]
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), len(st['caps']), n_hits, st['max_gap'], fits,
            len(st['lows']))


def main(n, axes_args, procs=2):
    T = current().T
    t0 = time.time()
    axes = []
    for a in axes_args:
        k, v = a.split('=')
        axes.append([(k, float(x)) for x in v.split(',')])
    cells = list(itertools.product(*axes)) or [()]
    jobs = [(c, s) for c in cells for s in range(1000, 1000 + n)]
    with mp.get_context('fork').Pool(procs) as P:
        res = P.map(one, jobs, chunksize=4)
    a, b = T.FIGHTERS
    for cell in cells:
        r = [x for x in res if x[0] == cell]
        dec = [x for x in r if x[2] is not None and x[3] < 99]
        g = np.mean([x[2] == 0 for x in dec]) if dec else 0.0
        tko = np.array([x[3] for x in r])
        lab = ' '.join(f'{k}={v:g}' for k, v in cell) or 'as frozen'
        hits = np.mean([x[7] for x in r], 0)
        print(f'{lab}: {a} wins {100 * g:3.0f}% of {len(dec)}  KO med {np.median(tko):5.1f} '
              f'({np.percentile(tko, 10):4.1f}-{np.percentile(tko, 90):4.1f})  no-KO {100 * np.mean(tko >= 99):2.0f}%  '
              f'lc {np.mean([x[4] for x in r]):.2f}  whp {np.median([x[5] for x in r]):3.0f}  '
              f'caps {np.mean([x[6] for x in r]):.1f}  both lows {100 * np.mean([x[10] == 2 for x in r]):2.0f}%  '
              f'gap med {np.median([x[8] for x in r]):.1f}  fit {100 * np.mean([x[9] for x in r]):.1f}%  '
              f'hits {a[:4]}/{b[:4]} {hits[0]:.0f}/{hits[1]:.0f}', flush=True)
    print(f'{len(jobs)} runs in {time.time() - t0:.0f} s. Aim: 40-60 % wins, ~50 s median K.O.; freeze the cell in '
          f'episode.py (DMG_SIDE / DMG_SCALE) or, for a new fighter\'s numbers, in fighters.py (own commit).')

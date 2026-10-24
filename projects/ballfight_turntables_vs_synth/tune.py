"""Rule tuning for #31 (once, then frozen in timeline.py / fighters.py): win share, median K.O., lead changes and fit
rate over seeds, for a grid of roster / rule overrides.

    python tune.py 32 T.dmg=0.4,0.5 S.dmg=1.4,1.6 S.grow=0.5 scale=1.0
        T.<key> / S.<key>      TURNTABLES / SYNTH weapon fields;  T.u2.<key>, S.low.<key> ...  ability fields
        scale                  the damage scale (timeline.DMG_SCALE)

Also reports who leads at 15 s, how often the trailer comes back, and how many REWINDs fire."""
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

WHO = {'T': 'TURNTABLES', 'S': 'SYNTH'}
SLOT = {'u1': 'unlock1', 'u2': 'unlock2', 'low': 'low'}


def apply(cell):
    sc = 1.0
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
    n_rw = 0
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            n_hits[e[2]] += 1
        elif e[1] == 'heal':
            hp[e[2]] += e[3]
            n_rw += 1
        if e[0] <= 15.0:
            lead15 = np.sign(hp[0] - hp[1])
    caps = len(sim.captions(r['events']))
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), lead15, caps, n_hits, n_rw)


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
        g = np.mean([x[2] == 0 for x in r])
        tko = np.array([x[3] for x in r])
        lc = np.mean([x[4] for x in r])
        whp = np.median([x[5] for x in r])
        fit = np.mean([(49 <= x[3] <= 53) and x[4] >= 2 and x[5] <= 30 and x[7] >= 5 for x in r])
        d15 = np.mean([x[6] > 0 for x in r])
        claw = np.mean([x[2] == 1 for x in r if x[6] > 0]) if any(x[6] > 0 for x in r) else 0
        hits = np.mean([x[8] for x in r], 0)
        rw = np.mean([x[9] for x in r])
        dec = [x for x in r if x[3] < 99]
        g = np.mean([x[2] == 0 for x in dec]) if dec else 0
        lab = ' '.join(f'{k}={v:g}' for k, v in cell)
        print(f'{lab}: decks win {100 * g:3.0f}% ({len(dec)} K.O.s)  KO med {np.median(tko):5.1f} ({np.percentile(tko, 10):4.1f}-'
              f'{np.percentile(tko, 90):4.1f})  lc {lc:.1f}  whp {whp:3.0f}  fit {100 * fit:2.0f}%  T@15 {100 * d15:3.0f}%  '
              f'S-comeback {100 * claw:3.0f}%  hits T/S {hits[0]:.0f}/{hits[1]:.0f}  rewinds {rw:.1f}', flush=True)

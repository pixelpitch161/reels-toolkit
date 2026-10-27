"""Rule tuning for #39 (once; the result is frozen in projects/ballfight/fighters.py and timeline.DMG_SCALE). Both
fighters are new, so their own numbers are the levers. Reports the win share, median K.O., lead changes, fit rate,
caption count, both lows, the hit mix and how the bassoon's stomps land (backstab share) over seeds, for a grid.

    python tune.py 80 bd=1.0,1.2 bb=2.0 rk=1.5,2.0 sl=3,4 sc=0.4,0.5

Grid keys: bd = BASSOON stomp damage, bb = its backstab multiplier, bh = its backstab half-angle, br = its hop reach,
bs = its stomp radius, bj = its jitter, bt = its stagger, bl = its lead, bg = its stomp growth, kd = BOCAL HOOK damage, sd = STACCATO beats, ba = its air time (16ths), be = beats between hops, bm = MOUNTAIN KING's damage;
rd = BARI honk damage, rk = honk knock, rp = honk push, sl = sling damage, sk = sling knock, rs = side step, rr = ring
decay, gd = GROWL damage, qd = LOW A damage, ks = SKANK sub, rg = rattle growth, rx = its max, gs / qs = GROWL / LOW A speed; sc = the shared damage scale. Seeds 1000.. (the honest
search later uses 1..), so the tuning never picks the episode's run."""
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

KEYS = {'bd': ('BASSOON', 'weapon', 'dmg'), 'bb': ('BASSOON', 'weapon', 'back'), 'bh': ('BASSOON', 'weapon', 'back_half'),
        'br': ('BASSOON', 'weapon', 'base'), 'bs': ('BASSOON', 'weapon', 'stomp'), 'bj': ('BASSOON', 'weapon', 'jitter'),
        'bt': ('BASSOON', 'weapon', 'stagger'), 'bl': ('BASSOON', 'weapon', 'lead'), 'bg': ('BASSOON', 'weapon', 'stomp_grow'), 'rg': ('BARI SAX', 'weapon', 'grow'), 'rx': ('BARI SAX', 'weapon', 'max_reach'), 'gs': ('BARI SAX', 'unlock1', 'speed'), 'qs': ('BARI SAX', 'low', 'speed'), 'kd': ('BASSOON', 'unlock2', 'dmg'), 'sd': ('BASSOON', 'unlock1', 'dur_beats'), 'ba': ('BASSOON', 'weapon', 'air16'), 'be': ('BASSOON', 'weapon', 'every'), 'bm': ('BASSOON', 'low', 'dmg'),
        'rd': ('BARI SAX', 'weapon', 'dmg'), 'rk': ('BARI SAX', 'weapon', 'knock'), 'rp': ('BARI SAX', 'weapon', 'push'),
        'sl': ('BARI SAX', 'weapon', 'sling'), 'sk': ('BARI SAX', 'weapon', 'sling_knock'),
        'rs': ('BARI SAX', 'weapon', 'side_step'), 'rr': ('BARI SAX', 'weapon', 'ring'),
        'gd': ('BARI SAX', 'unlock1', 'dmg'), 'qd': ('BARI SAX', 'low', 'dmg'), 'ks': ('BARI SAX', 'unlock2', 'sub')}
INTS = ('air16', 'every')


def apply(cell):
    for k, v in cell:
        if k in KEYS:
            name, part, field = KEYS[k]
            d = FIGHTERS[name]['weapon'] if part == 'weapon' else FIGHTERS[name]['abilities'][part]
            d[field] = int(v) if field in INTS else v


def one(args):
    cell, seed = args
    apply(cell)
    c = dict(cell)
    r = Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=sim.rules(c.get('sc')), record=False).run()
    st = sim.story(r)
    fits, _ = sim.score(st)
    lc, _ = lead_changes(r['events'])
    hp = [100.0, 100.0]
    dmg = {}
    for e in r['events']:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            key = (e[2], e[7])
            dmg[key] = dmg.get(key, 0.0) + e[4]
    stomps = [e for e in r['events'] if e[1] == 'stomp']
    land = sum(e[5] for e in stomps)
    back = sum(e[6] for e in stomps)
    slings = sum(e[1] == 'sling' for e in r['events'])
    return (cell, seed, r['winner'], r['t_ko'] or 99, lc, max(hp), len(st['caps']), dmg, st['max_gap'], fits,
            len(st['lows']), len(stomps), land, back, slings)


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
        keys = sorted({k for x in r for k in x[7]})
        mix = ' '.join(f'{"BN" if k[0] == 0 else "BS"}:{k[1]} {np.mean([x[7].get(k, 0) for x in r]):.0f}' for k in keys)
        print(f'{lab}: BASSOON wins {100 * g:3.0f}% of {len(dec)}  KO med {np.median(tko):5.1f} '
              f'({np.percentile(tko, 10):4.1f}-{np.percentile(tko, 90):4.1f})  no-KO {100 * np.mean(tko >= 99):2.0f}%  '
              f'lc {np.mean([x[4] for x in r]):.2f}  caps {np.mean([x[6] for x in r]):.1f}  '
              f'both lows {100 * np.mean([x[10] == 2 for x in r]):2.0f}%  gap med {np.median([x[8] for x in r]):.1f}  '
              f'fit {100 * np.mean([x[9] for x in r]):.1f}%  stomps {np.mean([x[11] for x in r]):.0f} '
              f'land {100 * np.sum([x[12] for x in r]) / max(1, np.sum([x[11] for x in r])):.0f}% '
              f'back {100 * np.sum([x[13] for x in r]) / max(1, np.sum([x[12] for x in r])):.0f}%  '
              f'slings {np.mean([x[14] for x in r]):.1f}  dmg {mix}', flush=True)

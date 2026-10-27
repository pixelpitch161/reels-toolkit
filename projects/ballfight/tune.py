"""quick rule tuning: win share + median KO over seeds for a grid of (key dmg, blade dmg, dmg_scale)"""
import sys, itertools, numpy as np
sys.path.insert(0, __import__('os').path.dirname(__import__('os').path.abspath(__file__)))
from multiprocessing import Pool
import fighters as FT
import sim  # noqa: F401,E402  (freezes #27's PIANO v1 in the roster before the grid runs)

def one(args):
    kd, bd, sc, seed = args
    FT.FIGHTERS['PIANO']['weapon']['dmg'] = kd
    FT.FIGHTERS['PIANO']['abilities']['unlock2']['dmg'] = kd
    FT.FIGHTERS['GUITAR']['weapon']['dmg'] = bd
    from fight import Fight, Rules, lead_changes
    r = Fight('GUITAR', 'PIANO', seed=seed, rules=Rules(dmg_scale=sc), record=False).run()
    lc, _ = lead_changes(r['events'])
    hp = [100 - sum(e[4] for e in r['events'] if e[1] == 'hit' and e[3] == k) for k in (0, 1)]
    return (kd, bd, sc, seed, r['winner'], r['t_ko'] or 99, lc, max(hp))

if __name__ == '__main__':
    kds = [float(x) for x in sys.argv[1].split(',')]
    bds = [float(x) for x in sys.argv[2].split(',')]
    scs = [float(x) for x in sys.argv[3].split(',')]
    n = int(sys.argv[4])
    jobs = [(kd, bd, sc, s) for kd, bd, sc in itertools.product(kds, bds, scs) for s in range(1000, 1000 + n)]
    with Pool(2) as P:
        res = P.map(one, jobs, chunksize=4)
    for kd, bd, sc in itertools.product(kds, bds, scs):
        r = [x for x in res if x[:3] == (kd, bd, sc)]
        g = np.mean([x[4] == 0 for x in r])
        tko = np.array([x[5] for x in r])
        lc = np.mean([x[6] for x in r])
        whp = np.median([x[7] for x in r])
        fit = np.mean([(49 <= x[5] <= 53) and x[6] >= 2 and x[7] <= 30 for x in r])
        print(f'key {kd:.2f} blade {bd:.1f} scale {sc:.2f}: guitar wins {100*g:4.0f}%  KO median {np.median(tko):5.1f} '
              f'(10-90% {np.percentile(tko,10):4.1f}-{np.percentile(tko,90):4.1f})  lead changes {lc:.1f}  winner HP {whp:4.0f}  fit {100*fit:.0f}%', flush=True)

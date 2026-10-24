"""Run the chosen seed once (seed.txt, picked by seed_search.py) and cache what picture and sound need (sim.npz).

    python sim.py              # seed.txt
    python sim.py 1768         # another seed (for comparisons; doesn't change seed.txt)

Cached per video frame: ball positions, wall owners, tile counts, alive flags. Events: every wall bounce (time,
ball, wall angle, splash width in tiles, tiles repainted), every ball-ball collision, the pops, the winner.
Nothing is edited."""
import json
import os
import sys
import time
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import royale as R

NPZ = os.path.join(HERE, 'sim.npz')
SEED_FILE = os.path.join(HERE, 'seed.txt')


def chosen_seed():
    with open(SEED_FILE) as f:
        return int(f.read().split()[0])


def run(seed=None):
    seed = chosen_seed() if seed is None else seed
    t0 = time.time()
    rec = R.Run(seed, record=True).run()
    H = rec['hits']
    hit_t = np.array([h[0] for h in H], float)
    hit_q = np.array([h[1] for h in H], np.int8)
    hit_a = np.array([h[2] for h in H], float)
    hit_w = np.array([h[3] for h in H], float)
    hit_n = np.array([len(h[4]) for h in H], np.int16)
    flips = [(i, k, prev) for i, h in enumerate(H) for k, prev in h[4]]
    C = rec['colls']
    meta = dict(seed=seed, pops=[(float(t), int(q), [float(p[0]), float(p[1])]) for t, q, p in rec['pops']],
                winner=None if rec['winner'] is None else int(rec['winner']), t_win=rec['t_win'],
                t_launch=[float(x) for x in rec['t_launch']], params=R.PARAMS.__dict__)
    np.savez_compressed(NPZ, pos=rec['pos'], vel=rec['vel'], owner=rec['owner'], count=rec['count'], alive=rec['alive'],
                        hit_t=hit_t, hit_q=hit_q, hit_a=hit_a, hit_w=hit_w, hit_n=hit_n,
                        flip=np.array(flips, np.int32).reshape(-1, 3),
                        coll_t=np.array([c[0] for c in C], float), coll_ij=np.array([[c[1], c[2]] for c in C], np.int8).reshape(-1, 2),
                        coll_p=np.array([c[3] for c in C], float).reshape(-1, 2), meta=json.dumps(meta))
    print(f"seed {seed}: pops " + ', '.join(f'{R.NAMES[q]} {t:.2f}' for t, q, _ in rec['pops']) +
          f"; winner {R.NAMES[rec['winner']] if rec['winner'] is not None else '-'}; {len(H)} bounces, {len(C)} collisions "
          f"({time.time() - t0:.1f} s)")
    return rec


_CACHE = {}


def load():
    if not _CACHE:
        d = np.load(NPZ)
        _CACHE.update({k: d[k] for k in d.files if k != 'meta'})
        _CACHE.update(json.loads(str(d['meta'])))
    return _CACHE


if __name__ == '__main__':
    run(int(sys.argv[1]) if len(sys.argv) > 1 else None)

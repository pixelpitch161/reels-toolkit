"""Run the chosen seed once (episode.SEED) and cache everything picture and sound need (sim.npz).

    python sim.py              # episode.SEED (picked by seed_search.py)
    python sim.py 19           # another seed (for comparisons)

The cache holds, per video frame: the grain grid (material * 32 + tone), the box angle, lid / hole state,
team counts, landings, moves and drains; every grain that left the box (frame, substep, material, exit cell,
tone); the hopper order and rates; the turns, the knockouts and the winner. Nothing is edited."""
import os
import sys
import time
import json
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E

NPZ = os.path.join(HERE, 'sim.npz')
SEED_FILE = os.path.join(HERE, 'seed.txt')


def chosen_seed():
    with open(SEED_FILE) as f:
        return int(f.read().split()[0])


def run(seed=None):
    seed = chosen_seed() if seed is None else seed
    t0 = time.time()
    r = E.Run(seed, record=True)
    rec = r.run()
    meta = dict(seed=seed, order=[int(x) for x in rec['order']], n_each=int(rec['n_each']), hole_side=rec['hole_side'],
                hole_cells=[(int(a), int(b)) for a, b in rec['hole_cells']],
                outs=[(float(t), int(q)) for t, q in rec['outs']], t_win=rec['t_win'],
                winner=None if rec['winner'] is None else int(rec['winner']),
                turns=[(float(a), float(b), int(c)) for a, b, c in rec['turns']],
                roll=[(float(a), float(b), float(c)) for a, b, c in rec['roll']] if rec['roll'] else None,
                left=int(rec['left']))
    np.savez_compressed(NPZ, grids=rec['grids'], theta=rec['theta'], count=rec['count'], landed=rec['landed'],
                        moves=rec['moves'], drained=rec['drained'], lid=rec['lid'], hole=rec['hole'],
                        rates=r.rates, ex_f=rec['ex_f'], ex_s=rec['ex_s'], ex_m=rec['ex_m'], ex_r=rec['ex_r'],
                        ex_c=rec['ex_c'], ex_t=rec['ex_t'], meta=json.dumps(meta))
    print(f'seed {seed}: {E.summary(rec)}  ({time.time() - t0:.1f} s)')
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

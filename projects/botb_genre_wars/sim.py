"""Run the chosen seed once and cache everything picture and sound need (sim.npz + sim_state.pkl).

    python sim.py              # episode.SEED
    python sim.py 82           # another seed (for comparisons)

The cache holds, per frame (sim time k / 30 s, k = 0..1829): the owner grid, vine parent links, last-claim
times, team tile counts, and each team's process state (grains, ball, rings, tips); plus every claim and every
attempted move, the knockouts and the winner. Video time = sim time."""
import os
import pickle
import sys
import time
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E
from battle import summary

NPZ = os.path.join(HERE, 'sim.npz')
PKL = os.path.join(HERE, 'sim_state.pkl')


def run(seed=None):
    seed = E.SEED if seed is None else seed
    t0 = time.time()
    rec = E.make(seed).run(E.DUR)
    np.savez_compressed(NPZ, owner=rec['owner'], parent=rec['parent'], claim_t=rec['claim_t'], count=rec['count'],
                        frame_times=rec['frame_times'], ev_t=rec['ev_t'], ev_k=rec['ev_k'], ev_i=rec['ev_i'],
                        ev_j=rec['ev_j'], ev_prev=rec['ev_prev'], at_t=rec['at_t'], at_k=rec['at_k'], at_i=rec['at_i'],
                        at_j=rec['at_j'], seed=seed)
    meta = dict(state=rec['state'], outs=rec['outs'], winner=rec['winner'], t_win=rec['t_win'], t_fight=rec['t_fight'],
                ring_fields=rec['ring_fields'], names=rec['names'], kinds=rec['kinds'], colors=rec['colors'],
                ntiles=rec['ntiles'], seed=seed)
    with open(PKL, 'wb') as f:
        pickle.dump(meta, f, protocol=4)
    print(f'seed {seed}: {summary(rec)}  ({time.time() - t0:.1f} s)')
    return rec


_CACHE = {}


def load():
    if not _CACHE:
        d = np.load(NPZ)
        _CACHE.update({k: d[k] for k in d.files})
        with open(PKL, 'rb') as f:
            _CACHE.update(pickle.load(f))
    return _CACHE


if __name__ == '__main__':
    run(int(sys.argv[1]) if len(sys.argv) > 1 else None)

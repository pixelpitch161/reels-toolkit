"""Early-stopping seed search (#32's, with the 4 s no-hit window): honest fights under the frozen rules, ranked against
the beat sheet. Runs in parallel and stops once `stop` runs fit; the ranked fits go to check/seeds.txt, every run to
check/raw_<a>_<b>.jsonl (the next session can reuse them).

Hard windows (episode config / series defaults): K.O. in 49-53 s, >= 2 lead changes, winner HP <= 30 at the K.O., >= 5
ability captions >= 4 s apart, no stretch over 4 s without a hit. Soft score: closeness, extra / late lead changes,
caption timing vs the targets, both desperation moves, no long gaps. Also prints the balance over every decided run."""
import json
import multiprocessing as mp
import os
import time

from .episode import current
from . import story


def one(seed):
    run = story.run_seed(seed, record=False)
    st = story.story(run)
    fits, sc = story.score(st)
    st.update(fits=fits, score=sc)
    st['firsts'] = [(round(t, 2), w, n) for t, w, n in st['firsts']]
    st['caps'] = [(round(t, 2), w, n) for t, w, n in st['caps']]
    st['lc_t'] = [round(t, 2) for t in st['lc_t']]
    return st


def main(a, b, stop=5, procs=2):
    ep = current()
    T = ep.T
    t0 = time.time()
    raw = os.path.join(ep.check, f'raw_{a}_{b}.jsonl')
    fits, allr = [], []
    with mp.get_context('fork').Pool(procs) as P, open(raw, 'w') as fo:
        for st in P.imap_unordered(one, range(a, b), chunksize=2):
            fo.write(json.dumps(st) + '\n')
            allr.append(st)
            if st['fits']:
                fits.append(st)
                print(f"fit: seed {st['seed']}  score {st['score']}  winner {st['winner']}  K.O. {st['t_ko']:.2f}  "
                      f"HP {st['winner_hp']:.0f}  lead changes {st['lead_changes']} {st['lc_t']}  "
                      f"caps {[c[2] for c in st['caps']]}", flush=True)
                if len(fits) >= stop:
                    P.terminate()
                    break
    dec = [r for r in allr if r['winner'] is not None and r['t_ko']]
    g = sum(r['winner'] == 0 for r in dec)
    print(f'{len(allr)} runs in {time.time() - t0:.0f} s, {len(dec)} decided by K.O.: {T.FIGHTERS[0]} {g} '
          f'({100 * g / max(1, len(dec)):.0f}%), {T.FIGHTERS[1]} {len(dec) - g}; {len(fits)} fits')
    fits.sort(key=lambda r: -r['score'])
    path = os.path.join(ep.check, 'seeds.txt')
    with open(path, 'w') as f:
        for r in fits:
            f.write(f"{r['seed']} score {r['score']} winner {r['winner']} ko {r['t_ko']:.2f} hp {r['winner_hp']:.1f} "
                    f"lc {r['lead_changes']} {r['lc_t']} caps {r['caps']} lows {r['lows']}\n")
    print(open(path).read())
    if fits:
        print(f"best: SEED = {fits[0]['seed']}  (set it in episode.py)")
    return fits

"""#36 early-stopping seed search (#32's, with the 4 s no-hit window): honest fights under the frozen rules, ranked against the beat sheet.

    python seed_search.py 1 400 --stop 5          # -> check/seeds.txt (ranked fits) + check/raw_<a>_<b>.jsonl (every run)

Hard windows (timeline.py): K.O. in 49-53 s, >= 2 lead changes, winner HP <= 30 at the K.O., >= 5 ability captions
>= 4 s apart. Soft score: closeness, extra / late lead changes, caption timing vs the targets, both desperation moves,
no long stretch without a hit. Also prints the balance (who wins) over every decided run."""
import argparse
import json
import os
import sys
from multiprocessing import Pool

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)


def one(seed):
    import sim
    run = sim.run_seed(seed, record=False)
    st = sim.story(run)
    fits, sc = sim.score(st)
    st.update(fits=fits, score=sc)
    st['firsts'] = [(round(t, 2), w, n) for t, w, n in st['firsts']]
    st['caps'] = [(round(t, 2), w, n) for t, w, n in st['caps']]
    st['lc_t'] = [round(t, 2) for t in st['lc_t']]
    return st


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int)
    ap.add_argument('b', type=int)
    ap.add_argument('--stop', type=int, default=5)
    ap.add_argument('--procs', type=int, default=2)
    a = ap.parse_args()
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    raw = os.path.join(HERE, 'check', f'raw_{a.a}_{a.b}.jsonl')
    fits, allr = [], []
    with Pool(a.procs) as P, open(raw, 'w') as fo:
        for st in P.imap_unordered(one, range(a.a, a.b), chunksize=2):
            fo.write(json.dumps(st) + '\n')
            allr.append(st)
            if st['fits']:
                fits.append(st)
                print(f"fit: seed {st['seed']}  score {st['score']}  winner {st['winner']}  K.O. {st['t_ko']:.2f}  "
                      f"HP {st['winner_hp']:.0f}  lead changes {st['lead_changes']} {st['lc_t']}  caps {[c[2] for c in st['caps']]}",
                      flush=True)
                if len(fits) >= a.stop:
                    P.terminate()
                    break
    dec = [r for r in allr if r['winner'] is not None and r['t_ko']]
    import timeline as T
    g = sum(r['winner'] == 0 for r in dec)
    print(f'{len(allr)} runs, {len(dec)} decided by K.O.: {T.FIGHTERS[0]} {g} ({100 * g / max(1, len(dec)):.0f}%), '
          f'{T.FIGHTERS[1]} {len(dec) - g}; {len(fits)} fits')
    fits.sort(key=lambda r: -r['score'])
    with open(os.path.join(HERE, 'check', 'seeds.txt'), 'w') as f:
        for r in fits:
            f.write(f"{r['seed']} score {r['score']} winner {r['winner']} ko {r['t_ko']:.2f} hp {r['winner_hp']:.1f} "
                    f"lc {r['lead_changes']} {r['lc_t']} caps {r['caps']} lows {r['lows']}\n")
    print(open(os.path.join(HERE, 'check', 'seeds.txt')).read())

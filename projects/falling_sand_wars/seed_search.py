"""Run many honest seeds of the episode and rank them by how well their story fits the beat sheet.
Nothing is edited: the chosen seed is played exactly as simulated (python sim.py SEED caches it).

    python seed_search.py 0 400 --stop 5     # stop once 5 seeds fit the shape -> ranked list + check/seeds_A_B.txt

Shape : first team out ~33 s, the second soon after (by ~45 s, so "2 LEFT" holds through the turns), a real
        duel (both finalists still in the box at 45 s), the last loser out at ~52 s. A team is out when it drops
        under episode.KO_FRAC of its starting sand (its last grains go dark).
Story : the team leading when the hole opens falls, the winner was not the favourite, lead changes, no team
        lingers for long with a few grains left, the winner keeps enough sand to be seen alone in the box."""
import os
import sys
import numpy as np
from multiprocessing import Pool

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import episode as E
from sands import NAMES

TARGET = (33.0, 39.0, 52.0)


def story(rec):
    cnt = rec['count'].astype(float)
    nf = len(cnt)
    outs = rec['outs']
    ts = [t for t, q in outs]
    fh = int(E.T_HOLE * E.FPS) + 3
    order = list(np.argsort(-cnt[fh]))           # at the hole: everyone equal-ish, so use 25 s as "the favourite"
    f25 = int(25.0 * E.FPS)
    fav = int(np.argmax(cnt[min(f25, nf - 1)]))
    lead, changes, leaders = fav, 0, [fav]
    for f in range(f25, nf, 15):
        if rec['t_win'] is not None and f / E.FPS > rec['t_win']:
            break
        c = cnt[f]
        m = int(np.argmax(c))
        if m != lead and c[m] > c[lead] + 30:
            lead, changes = m, changes + 1
            leaders.append(m)
    win = rec['winner']
    shape = (len(ts) == 3 and 29.0 <= ts[0] <= 39.0 and ts[0] + 0.6 <= ts[1] <= 45.0 and 49.0 <= ts[2] <= 54.5)
    s, duel, linger, keep = 0.0, 0.0, 0.0, 0
    if len(ts) == 3:
        # lingering: seconds a team spends with fewer than 20 grains before it is out
        for t, q in outs:
            fo = int(t * E.FPS)
            ko = E.KO_FRAC * rec['n_each']
            linger = max(linger, ((cnt[fh:fo, q] >= ko) & (cnt[fh:fo, q] < 2.2 * ko)).sum() / E.FPS)
        f45 = min(nf - 1, int(45.0 * E.FPS))
        fin = [q for q in range(4) if q not in [o[1] for o in outs[:2]]]
        duel = min(cnt[f45, q] for q in fin) / rec['n_each']
        keep = int(cnt[min(nf - 1, int(rec['t_win'] * E.FPS)), win])
    if shape:
        s += 10 - sum(abs(a - b) for a, b in zip(ts, TARGET)) * 0.6
        if fav in [q for t, q in outs[:2]]:
            s += 3                                    # the favourite falls early
        if win != fav:
            s += 2
        s += min(changes, 4)
        s += 4 * min(1.0, duel / 0.12)                # a real duel at 45 s
        s -= max(0.0, linger - 2.0) * 1.5
        s += min(3.0, keep / 100.0)                   # the winner must be seen alone in the box...
        if keep > 1300:
            s -= 4                                    # ...and still pour out in the outro
    return dict(score=s, shape=shape, outs=outs, win=win, fav=fav, changes=changes, leaders=leaders, duel=duel,
                linger=linger, keep=keep)


def one(seed):
    r = E.Run(seed)
    r.stop_at_win = True
    rec = r.run()
    st = story(rec)
    st['seed'] = seed
    st['order'] = rec['order']
    return st


if __name__ == '__main__':
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument('a', type=int, nargs='?', default=0)
    ap.add_argument('b', type=int, nargs='?', default=100)
    ap.add_argument('--stop', type=int, default=None, help='stop once this many seeds fit the shape')
    opt = ap.parse_args()
    a, b, stop = opt.a, opt.b, opt.stop
    res = []
    with Pool(os.cpu_count() or 2) as P:
        for st in P.imap_unordered(one, range(a, b), chunksize=1):
            res.append(st)
            tag = '*' if st['shape'] else ' '
            print(f"{tag} seed {st['seed']:4d}  outs " + ' '.join(f"{NAMES[q][:2]}@{t:4.1f}" for t, q in st['outs']) +
                  f"  win {NAMES[st['win']] if st['win'] is not None else '-'}", flush=True)
            if stop is not None and sum(r['shape'] for r in res) >= stop:
                P.terminate()
                break
    nm = [n[:2] for n in NAMES]
    wins = np.bincount([r['win'] for r in res if r['win'] is not None], minlength=4)
    lines = []
    for r in sorted(res, key=lambda r: -r['score'])[:12]:
        lines.append(f"seed {r['seed']:4d} score {r['score']:5.1f}  hoppers {''.join(NAMES[q][0] for q in r['order'])}  outs " +
                     ' '.join(f"{nm[q]}@{t:4.1f}" for t, q in r['outs']) +
                     f"  win {nm[r['win']] if r['win'] is not None else '-'}  fav {nm[r['fav']]}  lead changes {r['changes']} (" +
                     '>'.join(nm[q] for q in r['leaders']) + f")  duel {100 * r['duel']:.0f}%  linger {r['linger']:.1f}s  keep {r['keep']}")
    lines.append(f"{sum(r['shape'] for r in res)}/{len(res)} fit the shape; wins " + ' '.join(f'{n} {w}' for n, w in zip(nm, wins)) +
                 f"; undecided {sum(r['win'] is None for r in res)}")
    print('\n'.join(lines))
    os.makedirs('check', exist_ok=True)
    with open(f'check/seeds_{a}_{b}.txt', 'w') as f:
        f.write('\n'.join(lines) + '\n')

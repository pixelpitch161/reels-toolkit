"""Run many honest seeds of Genre Wars and rank them by how well their story fits the 61 s beat sheet.
Nothing is edited: the chosen seed is played exactly as simulated. Adapted from #20's search.

    python seed_search.py 0 400 --stop 5     # stop once 5 seeds fit the shape -> check/seeds_A_B.txt

Shape  : knockouts ~31 / 42 / 51 s (bell at 20.33 s), spaced >= 6 / 5 s, the win by 53.5 s (>= 7 s of solo
         before the crack at 60.4 s).
Story  : the bell leader falls, an underdog wins, lead changes in the fight AND in the fill (the song follows the
         leader, so every lead change is a genre change you hear), many genres get to lead, everyone visible at the
         bell, a real final duel, no lingering."""
import os
import sys
import numpy as np
from multiprocessing import Pool

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import episode as E

TARGET = (31.0, 42.0, 51.0)


def leaders_over(cnt, ft, f0, f1, hyst=0.01, step=15):
    lead, out = None, []
    for f in range(f0, min(f1, len(ft)), step):
        c = cnt[f]
        m = int(np.argmax(c))
        if lead is None:
            lead = m
            out.append((ft[f], m))
        elif m != lead and c[m] > c[lead] + hyst:
            lead = m
            out.append((ft[f], m))
    return out


def story(rec):
    nt = rec['ntiles']
    ft = rec['frame_times']
    cnt = rec['count'] / nt
    outs = [(t - E.OFFSET, q) for t, q in rec['outs']]
    fb = int(np.searchsorted(ft, E.T_FIGHT)) - 1
    bell = cnt[fb]
    order = list(np.argsort(-bell))
    leader = order[0]
    fw = int(np.searchsorted(ft, rec['t_win'])) if rec['t_win'] is not None else len(ft)
    fight_leads = leaders_over(cnt, ft, fb, fw)
    fill_leads = leaders_over(cnt, ft, int(4 * 30), fb + 1, hyst=0.02)     # after the opening burst settles
    changes = len(fight_leads) - 1
    fill_changes = len(fill_leads) - 1
    distinct = len({q for _, q in fight_leads + fill_leads})
    win = rec['winner']
    ts = [t for t, q in outs]
    b0 = E.T_FIGHT - E.OFFSET
    shape = (len(ts) == 3 and b0 + 7 <= ts[0] <= b0 + 15 and b0 + 17 <= ts[1] <= b0 + 25 and b0 + 27 <= ts[2] <= 53.5
             and ts[1] - ts[0] >= 6 and ts[2] - ts[1] >= 5)
    s, duel, linger = 0.0, 0.0, 0.0
    if shape:
        s += 10 - sum(abs(a - b) for a, b in zip(ts, TARGET)) * 0.5
        out_order = [q for t, q in outs]
        if leader in out_order[:2]:
            s += 3                                   # the early leader falls
        s += order.index(win) * 1.5                  # underdog winner (rank at the bell)
        s += min(changes, 4) * 1.0                   # lead changes in the fight
        s += min(fill_changes, 2) * 1.0              # ... and in the fill (genre changes before the bell)
        s += (distinct - 2) * 1.0                    # many genres get to lead
        s += 2 if bell.min() >= 0.12 else -3         # everyone visible at the bell
        f2 = min(len(ft) - 1, int(np.searchsorted(ft, ts[1] + E.OFFSET + 1.0)))
        alive = [q for q in range(4) if q not in out_order[:2]]
        duel = min(cnt[f2][q] for q in alive)
        s += 4 * min(1.0, duel / 0.3)
        for t, q in outs:
            fo = int(np.searchsorted(ft, t + E.OFFSET))
            linger = max(linger, (cnt[fb:fo, q] < 0.06).sum() / 30.0)
        s -= max(0.0, linger - 2.5) * 1.5
    return dict(score=s, shape=shape, outs=outs, win=win, bell=bell, leader=leader, changes=changes,
                fill_changes=fill_changes, distinct=distinct, leaders=[q for _, q in fill_leads + fight_leads[1:]],
                duel=duel, linger=linger)


def one(seed):
    rec = E.make(seed).run(E.DUR)
    st = story(rec)
    st['seed'] = seed
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
        chunk = 8 * (os.cpu_count() or 2)
        for s0 in range(a, b, chunk):
            res += P.map(one, range(s0, min(b, s0 + chunk)), chunksize=2)
            nfit = sum(r['shape'] for r in res)
            print(f'  {len(res)} runs, {nfit} fit', flush=True)
            if stop is not None and nfit >= stop:
                print(f'stopped after {len(res)} runs: {stop} seeds fit the shape')
                b = s0 + chunk
                break
    names = [n[:2] for n in E.NAMES]
    wins = np.bincount([r['win'] for r in res if r['win'] is not None], minlength=4)
    lines = []
    for r in sorted(res, key=lambda r: -r['score'])[:15]:
        lines.append(f"seed {r['seed']:4d} score {r['score']:5.1f}  bell " + ' '.join(f'{n}{100 * x:3.0f}' for n, x in zip(names, r['bell'])) +
                     '  outs ' + ' '.join(f"{names[q]}@{t:4.1f}" for t, q in r['outs']) +
                     f"  win {names[r['win']] if r['win'] is not None else '-'}  leads fill {r['fill_changes']} fight {r['changes']} (" +
                     '>'.join(names[q] for q in r['leaders']) + f")  duel {100 * r['duel']:.0f}%  linger {r['linger']:.1f}s")
    lines.append(f"{sum(r['shape'] for r in res)}/{len(res)} fit the shape; wins " + ' '.join(f'{n} {w}' for n, w in zip(names, wins)) +
                 f"; undecided {sum(r['win'] is None for r in res)}")
    print('\n'.join(lines))
    os.makedirs(os.path.join(HERE, 'check'), exist_ok=True)
    with open(os.path.join(HERE, f'check/seeds_{a}_{b}.txt'), 'w') as f:
        f.write('\n'.join(lines) + '\n')

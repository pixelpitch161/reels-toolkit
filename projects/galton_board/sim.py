"""Simulate every ball once, cache it (balls.npz), and print the checks.

    python sim.py

Two honest runs with fixed seeds, every bounce a real 50/50 coin flip (numpy RNG):
- the MAIN run: 1,000 balls (hero, second ball, stream, rain, mass, flood);
- the TORRENT: the cold-open flash-forward (~490 balls) that freezes and rewinds away.
Plus one STAGED golden ball whose 12 bounces are all forced left. It is shown as a "what if",
is never counted, and dissolves after landing.

Per ball we store the 12 contact times, the 13 columns visited, the bin, the landing slot and
time, so render.py and audio.py share exactly the same events."""
import os
import sys
import math
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import board as B
import timeline as T

KINDS = ['hero', 'second', 'stream', 'gold', 'rain', 'mass', 'flood', 'torrent']


def spaced_times(rng, a, b, n, min_gap=0.022, shape=None):
    """n sorted random times in [a, b] with a minimum gap (balls leave the outlet one by one)"""
    if shape is None:
        u = rng.uniform(0, 1, n)
    else:                                  # density ramp: sin^p bump
        u = np.sort(rng.beta(shape[0], shape[1], n))
    t = np.sort(a + (b - a) * u)
    for i in range(1, n):
        t[i] = max(t[i], t[i - 1] + min_gap)
    return t


def make_run(rng, t0s, h, jitter, kind, bits=None):
    """balls whose first peg contact is at t0s; returns a list of dicts"""
    out = []
    for i, t0 in enumerate(t0s):
        bb = rng.integers(0, 2, B.ROWS) if bits is None else np.asarray(bits)
        c = np.concatenate([[0], np.cumsum(2 * bb - 1)])                  # columns c_0..c_12
        hk = h * (1 + rng.uniform(-jitter, jitter, B.ROWS)) if jitter else np.full(B.ROWS, h)
        tc = t0 + np.concatenate([[0], np.cumsum(hk[:-1])])               # 12 contact times
        jx = rng.uniform(-0.25, 0.25, B.ROWS) * B.R_BALL if jitter else np.zeros(B.ROWS)
        out.append(dict(kind=kind, tc=tc, h=float(hk[-1]), c=c, bin=int((c[-1] + 12) // 2), jx=jx))
    return out


def land(balls):
    """assign bin slots in landing order and solve the landing times"""
    order = sorted(range(len(balls)), key=lambda i: balls[i]['tc'][-1])
    fill = [0] * B.NB
    for i in order:
        bl = balls[i]
        b = bl['bin']
        s = fill[b]
        if bl['kind'] != 'gold':
            fill[b] += 1
        xs, ys = B.slot_pos(b, s)
        u = B.fall_u(B.Y_C[-1], ys)
        bl.update(slot=s, xs=xs, ys=ys, u_land=u, t_land=bl['tc'][-1] + u * bl['h'])
    return fill


def build():
    rm = np.random.default_rng(T.SEED_MAIN)
    main = []
    main += make_run(rm, [T.HERO_T0], T.H_SLOW, 0, 'hero')
    main += make_run(rm, [T.SECOND_T0], T.H_SLOW, 0, 'second')
    main += make_run(rm, [T.STREAM_T0 + T.STREAM_GAP * i for i in range(T.STREAM_N)], T.H_FAST, 0, 'stream')
    for (a, b, n), kind, shape in ((T.RAIN, 'rain', None), (T.MASS, 'mass', None), (T.FLOOD, 'flood', (1.6, 1.4))):
        main += make_run(rm, spaced_times(rm, a, b, n, 0.03 if kind == 'rain' else 0.004, shape), T.H_FAST, 0.07, kind)
    fill_main = land(main)
    gold = make_run(rm, [T.GOLD_T0], T.H_SLOW, 0, 'gold', bits=np.zeros(B.ROWS, int))
    land(gold)
    rt = np.random.default_rng(T.SEED_TORRENT)
    n_t = int((T.T_TORRENT[1] - T.T_TORRENT[0]) * T.TORRENT_RATE)
    rel = spaced_times(rt, T.T_TORRENT[0], T.T_TORRENT[1], n_t, 0.004)
    torrent = make_run(rt, rel + T.H_FAST, T.H_FAST, 0.07, 'torrent')
    fill_t = land(torrent)
    return main, gold, torrent, fill_main, fill_t


def pack(balls, run):
    n = len(balls)
    return dict(
        run=np.full(n, run, np.int8),
        kind=np.array([KINDS.index(b['kind']) for b in balls], np.int8),
        tc=np.array([b['tc'] for b in balls]),
        h=np.array([b['h'] for b in balls]),
        c=np.array([b['c'] for b in balls], np.int8),
        bin=np.array([b['bin'] for b in balls], np.int8),
        jx=np.array([b['jx'] for b in balls]),
        slot=np.array([b['slot'] for b in balls], np.int32),
        xs=np.array([b['xs'] for b in balls]),
        ys=np.array([b['ys'] for b in balls]),
        u_land=np.array([b['u_land'] for b in balls]),
        t_land=np.array([b['t_land'] for b in balls]),
    )


def main():
    main_, gold, torrent, fill_main, fill_t = build()
    allb = [pack(main_, 0), pack(gold, 2), pack(torrent, 1)]
    data = {k: np.concatenate([d[k] for d in allb]) for k in allb[0]}
    np.savez_compressed(os.path.join(HERE, 'balls.npz'), **data)
    # ---- checks
    from math import comb
    n = len(main_)
    exp = [n * comb(12, b) / 4096 for b in range(13)]
    print(f'main run: {n} balls   torrent: {len(torrent)} balls   staged gold: 1')
    print('bin     ' + ' '.join(f'{b:5d}' for b in range(13)))
    print('ideal   ' + ' '.join(f'{e:5.1f}' for e in exp))
    print('actual  ' + ' '.join(f'{c:5d}' for c in fill_main))
    chi2 = sum((o - e) ** 2 / e for o, e in zip(fill_main, exp) if e > 1)
    print(f'chi-square (bins with expectation > 1): {chi2:.1f} on ~{sum(e > 1 for e in exp) - 1} dof')
    print('torrent bins at the freeze: ', [int(((data["run"] == 1) & (data["bin"] == b) & (data["t_land"] < T.T_FREEZE)).sum()) for b in range(13)])
    top = max(fill_main + fill_t)
    print(f'fullest bin {top} balls -> stack top y = {B.stack_top(top):.3f} (bin top {B.Y_BT:.3f})')
    assert B.stack_top(top) < B.Y_BT - 0.005, 'bins too short'
    print('hero path   ', ''.join('R' if d > 0 else 'L' for d in np.diff(main_[0]['c'])), ' bin', main_[0]['bin'])
    print('second path ', ''.join('R' if d > 0 else 'L' for d in np.diff(main_[1]['c'])), ' bin', main_[1]['bin'])
    print('stream bins ', [b['bin'] for b in main_[2:10]])
    lt = data['t_land'][data['run'] == 0]
    print(f'last main landing {lt.max():.2f} s (drop at {T.T_DROP}); gold lands {gold[0]["t_land"]:.2f} s')
    tt = data['t_land'][data['run'] == 1]
    print(f'torrent landed by 0 s: {(tt < 0).sum()}   by the freeze: {(tt < T.T_FREEZE).sum()}')
    pings = data['tc'][data['run'] != 1].size + (data['tc'][data['run'] == 1] < T.T_FREEZE).sum()
    print('pings total:', pings)
    for a, b in [(26, 32), (32, 40), (40, 46)]:
        x = data['tc'][data['run'] == 0]
        print(f'  pings/s {a}-{b}: {((x >= a) & (x < b)).sum() / (b - a):.0f}')


if __name__ == '__main__':
    main()

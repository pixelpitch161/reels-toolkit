"""Builds every marble path and every hit once and caches them (marbles.npz). One list drives the
picture (marbles, element flashes, basin splashes) and the sound (audio.py plays every hit).

No physics engine: routed marbles roll down their chute, hop off the end in an exact ballistic arc that
lands on the element at the note time, bounce out and fall into the basin ring (the waterfall).
Sprayed marbles (gate open) are deflected by the raised flap, bounce off 1-3 random elements (each
contact rings that element = a random note), then fly out of the machine and drift away weightless.

    python sim.py
"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import timeline as T
import machine as M

ROLL, BALL, DRIFT, SPIRAL = 0, 1, 2, 3
GV = np.array([0, -M.G, 0])
TAU = {'kick': 0.12, 'tom': 0.12, 'snare': 0.1, 'hat': 0.07, 'pipe': 0.14, 'bowl': 0.3, 'bar': 0.14,
       'xylo': 0.09, 'crash': 0.35, 'ride': 0.2, 'bell': 0.26}


class Builder:
    def __init__(self):
        self.segs = []       # (mid, t0, t1, typ, p0, v0, p1, a0, a1)
        self.mch = []        # marble channel (colour)
        self.mtrail = []     # always draw a trail (escapees)
        self.hits = []       # (t, elem, pitch, vel, src, ch)
        self.lands = []      # (t, x, y, z, ch)

    def marble(self, ch, trail=False):
        self.mch.append(ch)
        self.mtrail.append(trail)
        return len(self.mch) - 1

    def seg(self, m, t0, t1, typ, p0, v0=(0, 0, 0), p1=(0, 0, 0), a0=1.0, a1=1.0):
        self.segs.append((m, t0, t1, typ, np.asarray(p0, float), np.asarray(v0, float), np.asarray(p1, float), a0, a1))

    def hit(self, t, el, vel, src, ch):
        e = M.ELEMS[el]
        self.hits.append((t, el, -1 if e.pitch is None else e.pitch, vel, src, ch))


def ball_pos(p0, v0, dt):
    return p0 + v0 * dt + 0.5 * GV * dt * dt


def to_basin(B, m, P, t, rng, ch, vy=None):
    """bounce out of an element at P (time t) and fall into the basin ring"""
    th = math.degrees(math.atan2(P[0], P[2])) + rng.uniform(-12, 12)
    Bp = M.basin_point(th, rng)
    vy = rng.uniform(0.5, 1.1) if vy is None else vy
    dy = P[1] - Bp[1]
    Tf = (vy + math.sqrt(vy * vy + 2 * M.G * max(dy, 0.01))) / M.G
    v0 = M.hop_v0(P, Bp, Tf)
    B.seg(m, t, t + Tf, BALL, P, v0)
    B.lands.append((t + Tf, Bp[0], Bp[1], Bp[2], ch))


def song(B, rng):
    ev = T.song_events()
    # stagger simultaneous notes on one chute so the marbles don't overlap while rolling
    groups = {}
    for e in ev:
        groups.setdefault((e[0], e[1]), []).append(e)
    spiral_bars = {}
    for (t, ch), es in sorted(groups.items()):
        if ch == 5:
            bar = int(round((t - (t % T.BAR)) / T.BAR))
            spiral_bars.setdefault(bar, []).extend(es)
            continue
        es = sorted(es, key=lambda e: e[2])
        for j, (t_, ch_, k, pitch, vel) in enumerate(es):
            el = M.TIER_ELEMS[ch][k]
            G0, C = M.gate_point(ch), M.chute_end(ch)
            Th = M.hop_time(C, el.pos) * (1 + 0.32 * j)
            Tr = np.linalg.norm(C - G0) / M.V_ROLL
            m = B.marble(ch)
            th0 = t - Th
            B.seg(m, th0 - Tr, th0, ROLL, G0, p1=C, a0=0.0 if Tr > 0.12 else 1.0)
            B.seg(m, th0, t, BALL, C, M.hop_v0(C, el.pos, Th))
            B.hit(t, el.id, vel, 0, ch)
            if el.kind == 'pipe':
                B.seg(m, t, t + 0.06, ROLL, el.pos, p1=el.pos - [0, 0.09, 0], a0=1.0, a1=0.0)
            else:
                to_basin(B, m, el.pos, t, rng, ch)
    # spiral: one marble per bar rings the 16 ramp bars on 16ths
    for bar, es in sorted(spiral_bars.items()):
        es.sort()
        t_bar = T.bt(bar)
        t0 = t_bar - 0.5 * T.S16
        t1 = t0 + 16 * T.S16
        G0, C = M.gate_point(5), M.CHUTE6_END
        Tr = np.linalg.norm(C - G0) / M.V_ROLL
        m = B.marble(5, trail=True)
        B.seg(m, t0 - Tr, t0, ROLL, G0, p1=C, a0=0.0)
        for (t, ch, k, pitch, vel) in es:
            el = M.TIER_ELEMS[5][k]
            B.hits.append((t, el.id, pitch, vel, 0, 5))
        last_k = max(e[2] for e in es)
        if last_k == 15:
            B.seg(m, t0, t1, SPIRAL, M.spiral(0.0))
            end = M.spiral(1.0) + [0, 0.012 + M.R_M, 0]
            th = math.degrees(math.atan2(end[0], end[2])) - 24
            Bp = M.basin_point(th, rng)
            Tf = math.sqrt(2 * (end[1] - Bp[1]) / M.G) * 1.02
            B.seg(m, t1, t1 + Tf, BALL, end, M.hop_v0(end, Bp, Tf))
            B.lands.append((t1 + Tf, Bp[0], Bp[1], Bp[2], 5))
        else:                                             # lever: flies off the ramp into the void
            tl = T.T_LEVER
            u = (tl - t0) / (t1 - t0)
            B.seg(m, t0, tl, SPIRAL, M.spiral(0.0), p1=(u, 0, 0))
            p = M.spiral(u) + [0, 0.012 + M.R_M, 0]
            pn = M.spiral(u + 0.01) + [0, 0.012 + M.R_M, 0]
            tang = (pn - p) / np.linalg.norm(pn - p)
            rad = p * [1, 0, 1] / np.linalg.norm(p * [1, 0, 1])
            v = tang * 3.0 + rad * 2.2 + [0, 1.4, 0]
            B.seg(m, tl, tl + 3.2, DRIFT, p, v, a0=1.0, a1=0.0)
    return len(ev)


def spray(B, rng):
    """gate-open streams: deflected by the raised flap, 1-3 random contacts, then out into the void"""
    els = M.ELEMS
    pos = np.array([e.pos for e in els])
    n_spawn = 0
    for g in range(8):
        own = [e.id for e in M.TIER_ELEMS[g] if e.kind != 'kick']
        rmax = T.SPRAY_RATE * 3.0
        t = 0.0
        while True:
            t += rng.exponential(1.0 / rmax)
            if t >= T.DUR:
                break
            r = T.spray_rate(g, t)
            if rng.uniform() * rmax > r:
                continue
            n_spawn += 1
            m = B.marble(g, trail=True)
            p = M.gate_point(g) + rng.normal(0, 0.01, 3)
            tc = t
            prev = -1
            absorbed = False
            nc = rng.choice([1, 2, 3], p=[0.32, 0.42, 0.26])
            for c in range(nc):
                d = pos - p
                dxz = np.hypot(d[:, 0], d[:, 2])
                ok = (d[:, 1] < 0.12) & (dxz < 1.15) & (np.arange(len(els)) != prev)
                ok &= np.array([e.kind != 'kick' for e in els])
                if not ok.any():
                    break
                w = np.exp(-np.linalg.norm(d, axis=1) / 0.55) * ok
                w = w * np.array([0.35 if e.kind in ('crash', 'ride') else 1.0 for e in els])
                if c == 0:
                    w[own] *= 4.0
                w = w / w.sum()
                k = int(rng.choice(len(els), p=w))
                tgt = pos[k] + rng.normal(0, 0.008, 3)
                Th = M.hop_time(p, tgt) * rng.uniform(0.9, 1.3)
                v0 = M.hop_v0(p, tgt, Th)
                if v0[1] > 4.2:
                    Th *= 1.4
                    v0 = M.hop_v0(p, tgt, Th)
                B.seg(m, tc, tc + Th, BALL, p, v0)
                tc += Th
                p = tgt
                prev = k
                B.hit(tc, k, float(rng.uniform(0.35, 0.8)), 1, g)
                if els[k].kind == 'pipe':
                    B.seg(m, tc, tc + 0.06, ROLL, p, p1=p - [0, 0.09, 0], a0=1.0, a1=0.0)
                    absorbed = True
                    break
            if absorbed:
                continue
            # escape: out of the machine, then weightless
            rad = p * [1, 0, 1]
            rn = np.linalg.norm(rad)
            rad = rad / rn if rn > 0.05 else np.array([math.cos(t * 7), 0, math.sin(t * 7)])
            tan = np.cross([0, 1.0, 0], rad)
            ve = rad * rng.uniform(1.6, 3.2) + tan * rng.uniform(-1.4, 1.4) + np.array([0, rng.uniform(0.2, 2.4), 0])
            Tb = 0.2
            B.seg(m, tc, tc + Tb, BALL, p, ve)
            p2 = ball_pos(p, ve, Tb)
            v2 = ve + GV * Tb
            Td = rng.uniform(2.4, 3.6)
            B.seg(m, tc + Tb, tc + Tb + Td, DRIFT, p2, v2, a0=1.0, a1=0.0)
    return n_spawn


def build():
    rng = np.random.default_rng(1800)
    B = Builder()
    n_song = song(B, rng)
    n_spray = spray(B, rng)
    # marble ids -> arrays
    S = B.segs
    mid = np.array([s[0] for s in S], np.int32)
    t0 = np.array([s[1] for s in S]); t1 = np.array([s[2] for s in S])
    typ = np.array([s[3] for s in S], np.int8)
    p0 = np.array([s[4] for s in S]); v0 = np.array([s[5] for s in S]); p1 = np.array([s[6] for s in S])
    a0 = np.array([s[7] for s in S]); a1 = np.array([s[8] for s in S])
    mch = np.array(B.mch, np.int8); mtr = np.array(B.mtrail, bool)
    # loop: marbles that run past 60 s also appear at t - 60 (and before 0 at t + 60)
    nm = len(mch)
    mlast = np.full(nm, -1e9); mfirst = np.full(nm, 1e9)
    np.maximum.at(mlast, mid, t1); np.minimum.at(mfirst, mid, t0)
    extra = []
    for shift, sel in ((-T.DUR, mlast > T.DUR), (T.DUR, mfirst < 0)):
        ms = np.nonzero(sel)[0]
        new_ids = {m: nm + i for i, m in enumerate(ms)}
        nm += len(ms)
        k = np.isin(mid, ms)
        extra.append((np.array([new_ids[m] for m in mid[k]], np.int32), t0[k] + shift, t1[k] + shift, typ[k], p0[k], v0[k], p1[k], a0[k], a1[k], ms))
    for (mi, a, b, ty, q0, w0, q1, x0, x1, ms) in extra:
        mid = np.concatenate([mid, mi]); t0 = np.concatenate([t0, a]); t1 = np.concatenate([t1, b])
        typ = np.concatenate([typ, ty]); p0 = np.concatenate([p0, q0]); v0 = np.concatenate([v0, w0]); p1 = np.concatenate([p1, q1])
        a0 = np.concatenate([a0, x0]); a1 = np.concatenate([a1, x1])
        mch = np.concatenate([mch, mch[ms]]); mtr = np.concatenate([mtr, mtr[ms]])
    H = np.array(B.hits)
    H = H[np.argsort(H[:, 0])]
    Hw = np.concatenate([H, H[H[:, 0] >= T.DUR] - [T.DUR, 0, 0, 0, 0, 0], H[H[:, 0] < 0] + [T.DUR, 0, 0, 0, 0, 0]])
    Hw = Hw[np.argsort(Hw[:, 0])]
    Ld = np.array(B.lands)
    Ldw = np.concatenate([Ld, Ld[Ld[:, 0] >= T.DUR] - [T.DUR, 0, 0, 0, 0], Ld[Ld[:, 0] < 0] + [T.DUR, 0, 0, 0, 0]])
    Ldw = Ldw[np.argsort(Ldw[:, 0])]
    # per-frame flash level of every element (peaks ON the hit frame, lingers ~3 frames)
    ne = len(M.ELEMS)
    fl = np.zeros((T.NF, ne), np.float32)
    for (t, el, p, vel, src, ch) in Hw:
        el = int(el)
        tau = TAU[M.ELEMS[el].kind]
        f0 = int(math.ceil((t - 0.5 / T.FPS) * T.FPS))
        for f in range(max(0, f0), min(T.NF, f0 + int(tau * 5 * T.FPS) + 2)):
            dt = max(0.0, f / T.FPS - t)
            v = (math.exp(-dt / tau) ** 0.55) * (0.55 + 0.45 * vel) * (1.0 if src == 0 else 0.75)
            if v > fl[f, el]:
                fl[f, el] = v
    np.savez_compressed(os.path.join(HERE, 'marbles.npz'), mid=mid, t0=t0, t1=t1, typ=typ, p0=p0, v0=v0, p1=p1, a0=a0, a1=a1,
                        mch=mch, mtrail=mtr, hits=Hw, lands=Ldw, flash=fl)
    print(f'song notes {n_song}, spray marbles {n_spray}, marbles {len(mch)}, segments {len(mid)}, hits {len(Hw)}')
    for a in range(0, 60, 4):
        k = (Hw[:, 0] >= a) & (Hw[:, 0] < a + 4)
        print(f'  {a:2d}-{a + 4:2d}s  hits/s {k.sum() / 4:6.1f}  (spray {((Hw[:, 4] == 1) & k).sum() / 4:5.1f})')
    return mid


# ================================================================== evaluation (shared with render.py)
_D = {}


def load():
    if not _D:
        z = np.load(os.path.join(HERE, 'marbles.npz'))
        _D.update({k: z[k] for k in z.files})
        o = np.argsort(_D['t0'], kind='stable')
        for k in ('mid', 't0', 't1', 'typ', 'p0', 'v0', 'p1', 'a0', 'a1'):
            _D[k] = _D[k][o]
        _D['t1max'] = np.maximum.accumulate(_D['t1'])
    return _D


def positions(t, D=None):
    """active marbles at time t -> (mid, pos (n,3), alpha, vel (n,3))"""
    D = D or load()
    hi = np.searchsorted(D['t0'], t, 'right')
    lo = np.searchsorted(D['t1max'], t, 'right')
    idx = np.arange(lo, hi)
    idx = idx[D['t1'][idx] > t]
    if not len(idx):
        return np.zeros(0, int), np.zeros((0, 3)), np.zeros(0), np.zeros((0, 3))
    typ = D['typ'][idx]
    t0, t1 = D['t0'][idx], D['t1'][idx]
    dt = (t - t0)[:, None]
    u = ((t - t0) / np.maximum(t1 - t0, 1e-6))
    p0, v0, p1 = D['p0'][idx], D['v0'][idx], D['p1'][idx]
    pos = np.zeros((len(idx), 3)); vel = np.zeros((len(idx), 3))
    k = typ == ROLL
    pos[k] = p0[k] + (p1[k] - p0[k]) * u[k, None]
    vel[k] = (p1[k] - p0[k]) / np.maximum(t1 - t0, 1e-6)[k, None]
    k = typ == BALL
    pos[k] = p0[k] + v0[k] * dt[k] + 0.5 * GV * dt[k] ** 2
    vel[k] = v0[k] + GV * dt[k]
    k = typ == DRIFT
    pos[k] = p0[k] + v0[k] * dt[k]
    vel[k] = v0[k]
    k = typ == SPIRAL
    if k.any():
        uu = u[k]
        scale = np.where(p1[k, 0] > 0, p1[k, 0], 1.0)       # truncated (lever) spiral segments store u_end
        uu = uu * scale
        pos[k] = M.spiral(uu) + np.array([0, 0.012 + M.R_M, 0])
        vel[k] = (M.spiral(np.minimum(uu + 0.01, 1)) - M.spiral(np.maximum(uu - 0.01, 0))) / 0.02 / 2.0
    a = D['a0'][idx] + (D['a1'][idx] - D['a0'][idx]) * u
    return D['mid'][idx], pos, a, vel


if __name__ == '__main__':
    build()

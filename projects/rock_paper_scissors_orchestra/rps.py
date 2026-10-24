"""Rock Paper Scissors Orchestra: the chase engine (honest, seeded, cached).

120 discs (40 per team) on a round stage. Each piece steers toward the nearest piece it beats and away from the
nearest piece that beats it (sliding along the stage rim when it is cornered), with a little wander, damping and a
speed cap. Discs collide elastically with each other and the rim. When a piece touches a piece it beats, the caught
piece joins the hunter's team (a short immunity after a conversion keeps each catch readable).

    ROCK (0) beats SCISSORS (2), PAPER (1) beats ROCK (0), SCISSORS (2) beats PAPER (1):
    team q beats (q + 2) % 3 and is beaten by (q + 1) % 3.

Video time = sim time:
    t = 0          frame 0: the pieces sit in a tight clump at centre stage in three colour wedges and burst out
    t < T_CATCH0   no catches yet (the hook: the three teams fan out)
    ...            the honest chase: catches, swings, a wipe-out, the mop-up, the last straggler
    T_WIN .. T_G0  the winner alone: a slow victory swirl round the stage
    T_G0 .. DUR    scripted gather back into the clump (same slots as frame 0), every piece arriving at exactly
                   DUR and turning to face outward, so the last frame flows into the burst at frame 0

Nothing about who wins is edited: seed + parameters decide; seed_search.py picks a seed whose story fits."""
import math
from dataclasses import dataclass, asdict
import numpy as np
from scipy.optimize import linear_sum_assignment

NAMES = ['ROCK', 'PAPER', 'SCISSORS']
FPS = 30
DUR = 61.0
NF = int(FPS * DUR)


def prey_of(q):
    return (q + 2) % 3


def pred_of(q):
    return (q + 1) % 3


@dataclass
class Params:
    n_each: int = 40
    R: float = 3.4            # stage radius (piece centres stay inside R - r)
    r: float = 0.13           # piece radius
    vmax: float = 1.5         # speed cap (units / s)
    acc: float = 6.0          # steering acceleration scale
    seek: float = 1.0
    flee: float = 1.25
    see_prey: float = 2.8     # how far a piece notices prey
    see_pred: float = 1.2     # how far a piece notices a predator
    wander: float = 0.55
    damp: float = 0.9         # 1/s
    cap: float = 1.06         # catch distance in units of 2r
    immune: float = 0.35      # s after a conversion: can't catch or be caught
    t_catch0: float = 2.18    # catches start (bar 1)
    burst_v: float = 3.2      # outward speed at frame 0
    clump: float = 1.7        # slot spacing in the clump, in units of r (tighter than touching: the tokens overlap)
    lap_r: float = 2.0        # radius of the victory swirl
    lap_v: float = 0.9
    t_g0: float = 57.2        # gather starts
    sprint: float = 1.0       # speed cap multiplier while a predator is within see_pred (panic sprint)
    sub: int = 4


def clump_slots(n, spacing, seed=0):
    """n points in a hex-packed disc, sorted by radius (centre first)"""
    pts = []
    k = int(math.ceil(math.sqrt(n))) + 4
    for i in range(-k, k + 1):
        for j in range(-k, k + 1):
            x = (i + 0.5 * (j % 2)) * spacing
            y = j * spacing * math.sqrt(3) / 2
            pts.append((x, y))
    pts = np.array(pts)
    d = np.hypot(pts[:, 0], pts[:, 1])
    pts = pts[np.argsort(d + 1e-6 * np.arctan2(pts[:, 1], pts[:, 0]))][:n]
    return pts - pts.mean(0)


def start_state(p, rot=0.0):
    """frame 0: the clump in three colour wedges (40 each), bursting outward"""
    n = 3 * p.n_each
    S = clump_slots(n, p.clump * p.r)
    ang = (np.arctan2(S[:, 1], S[:, 0]) - rot) % (2 * math.pi)
    order = np.argsort(ang)
    team = np.zeros(n, np.int8)
    for q in range(3):
        team[order[q * p.n_each:(q + 1) * p.n_each]] = q
    rad = np.hypot(S[:, 0], S[:, 1])
    u = S / np.maximum(rad, 1e-6)[:, None]
    u[rad < 1e-6] = [1.0, 0.0]
    v = u * (p.burst_v * (0.55 + 0.45 * rad / rad.max()))[:, None]
    return S.copy(), v, team


def run(seed, p=None, record=True):
    """simulate the honest run; returns a dict (positions, headings, teams per frame, catch events, outs)"""
    p = p or Params()
    rng = np.random.default_rng(seed)
    n = 3 * p.n_each
    rot = rng.uniform(0, 2 * math.pi)
    pos, vel, team = start_state(p, rot)
    slots = pos.copy()
    dt = 1.0 / (FPS * p.sub)
    wand = rng.normal(0, 1, (n, 2))
    imm = np.zeros(n)
    rmax = p.R - p.r
    P = np.zeros((NF, n, 2), np.float32)
    V = np.zeros((NF, n, 2), np.float32)
    TM = np.zeros((NF, n), np.int8)
    catches = []            # (t, hunter, caught, new_team, old_team)
    outs = []               # (t, team)
    alive = [True, True, True]
    t_win = None
    iu = np.triu_indices(n, 1)
    two_r = 2 * p.r
    d = pos[None, :, :] - pos[:, None, :]
    dist = np.sqrt((d ** 2).sum(-1)) + np.eye(n) * 1e9
    for f in range(NF):
        t_frame = f / FPS
        P[f], V[f], TM[f] = pos, vel, team
        if t_frame >= p.t_g0:
            break
        for s in range(p.sub):
            t = t_frame + s * dt                           # d[i, j] = pos[j] - pos[i] (from the end of the last substep)
            acc = np.zeros((n, 2))
            panic = np.zeros(n)
            if t_win is None:
                # nearest prey / predator per piece
                for q in range(3):
                    me = team == q
                    if not me.any():
                        continue
                    for other, sgn, see, w in ((prey_of(q), 1.0, p.see_prey, p.seek), (pred_of(q), -1.0, p.see_pred, p.flee)):
                        cand = team == other
                        if not cand.any():
                            continue
                        dd = np.where(cand[None, :], dist[me], 1e9)
                        j = dd.argmin(1)
                        dj = dd[np.arange(len(j)), j]
                        vec = d[me][np.arange(len(j)), j] / np.maximum(dj, 1e-6)[:, None]
                        if sgn > 0:
                            k = np.clip(1.25 - dj / see, 0.25, 1.0)        # a faint pull from far prey, strong when close
                        else:
                            k = np.clip(1.0 - dj / see, 0.0, 1.0) ** 0.7
                            panic[me] = (dj < see).astype(float)
                        a = sgn * w * k[:, None] * vec
                        if sgn < 0:
                            # cornered: slide along the rim instead of into it
                            pm = pos[me]
                            rr = np.hypot(pm[:, 0], pm[:, 1])
                            nr = pm / np.maximum(rr, 1e-6)[:, None]
                            near = np.clip((rr - (rmax - 0.7)) / 0.7, 0, 1)
                            outward = np.clip((a * nr).sum(1), 0, None)
                            a = a - (near * outward)[:, None] * nr
                            tang = np.stack([-nr[:, 1], nr[:, 0]], 1)
                            side = np.sign((a * tang).sum(1) + 1e-9)
                            a = a + (near * outward * 0.8 * side)[:, None] * tang
                        acc[me] += a
            else:
                # the winner alone: a slow victory swirl round the stage
                rr = np.hypot(pos[:, 0], pos[:, 1])
                nr = pos / np.maximum(rr, 1e-6)[:, None]
                tang = np.stack([-nr[:, 1], nr[:, 0]], 1)
                acc += (tang * p.lap_v - vel) * 1.2 + nr * np.clip(p.lap_r - rr, -1, 1)[:, None] * 0.8
            # wander (smooth random walk)
            wand += (-wand * 1.5 * dt) + rng.normal(0, 1, (n, 2)) * math.sqrt(3.0 * dt)
            acc += wand * p.wander
            # soft rim
            rr = np.hypot(pos[:, 0], pos[:, 1])
            nr = pos / np.maximum(rr, 1e-6)[:, None]
            acc -= nr * np.clip(rr - (rmax - 0.35), 0, None)[:, None] * 8.0
            vel = vel + acc * p.acc * dt
            vel *= math.exp(-p.damp * dt)
            vcap = p.vmax * (1 + (p.sprint - 1) * panic) + p.burst_v * math.exp(-t / 0.45)
            sp = np.hypot(vel[:, 0], vel[:, 1])
            vel *= np.minimum(1.0, vcap / np.maximum(sp, 1e-9))[:, None]
            pos = pos + vel * dt
            # rim collision
            rr = np.hypot(pos[:, 0], pos[:, 1])
            over = rr > rmax
            if over.any():
                nr = pos[over] / rr[over][:, None]
                pos[over] = nr * rmax
                vn = (vel[over] * nr).sum(1)
                vel[over] -= (1.6 * np.clip(vn, 0, None))[:, None] * nr
            # disc collisions
            d = pos[None, :, :] - pos[:, None, :]
            dist = np.sqrt((d ** 2).sum(-1))
            ov = dist[iu] < two_r
            if ov.any():
                ii, jj = iu[0][ov], iu[1][ov]
                dij = dist[ii, jj]
                nrm = d[ii, jj] / np.maximum(dij, 1e-6)[:, None]
                push = (two_r - dij) * 0.5
                disp = np.zeros((n, 2))
                np.add.at(disp, ii, -nrm * push[:, None])
                np.add.at(disp, jj, nrm * push[:, None])
                pos += disp
                vn = ((vel[jj] - vel[ii]) * nrm).sum(1)
                J = np.where(vn < 0, -(1 + 0.75) * vn / 2, 0.0)
                dv = np.zeros((n, 2))
                np.add.at(dv, ii, -nrm * J[:, None])
                np.add.at(dv, jj, nrm * J[:, None])
                vel += dv
                d = pos[None, :, :] - pos[:, None, :]
                dist = np.sqrt((d ** 2).sum(-1))
            dist = dist + np.eye(n) * 1e9
            # catches
            imm = np.maximum(imm - dt, 0)
            if t >= p.t_catch0 and t_win is None:
                close = dist[iu] < two_r * p.cap
                if close.any():
                    ii, jj = iu[0][close], iu[1][close]
                    ta, tb = team[ii], team[jj]
                    fight = (tb == (ta + 2) % 3) | (ta == (tb + 2) % 3)
                    ii, jj = ii[fight], jj[fight]
                    newteam = team.copy()
                    done = set()
                    for a, b in zip(ii, jj):
                        if imm[a] > 0 or imm[b] > 0 or a in done or b in done:
                            continue
                        ta, tb = team[a], team[b]
                        if tb == prey_of(ta):
                            h, c = a, b
                        elif ta == prey_of(tb):
                            h, c = b, a
                        else:
                            continue
                        newteam[c] = team[h]
                        imm[c] = p.immune
                        imm[h] = max(imm[h], p.immune * 0.5)
                        done.update((a, b))
                        catches.append((t + dt, int(h), int(c), int(team[h]), int(team[c])))
                    team = newteam
                    cnt = np.bincount(team, minlength=3)
                    for q in range(3):
                        if alive[q] and cnt[q] == 0:
                            alive[q] = False
                            outs.append((t + dt, q))
                    if sum(alive) == 1:
                        t_win = t + dt
    f_end = f
    winner = int(np.argmax(np.bincount(team, minlength=3))) if t_win is not None else None
    rec = dict(P=P, V=V, team=TM, catches=catches, outs=outs, t_win=t_win, winner=winner, slots=slots,
               f_g0=f_end, params=asdict(p), seed=seed, rot=rot)
    if t_win is not None and record:
        _gather(rec, p)
    return rec


def _gather(rec, p):
    """T_G0 .. DUR: every piece spirals in to its clump slot (the frame-0 slots), arriving at exactly DUR"""
    P, V, TM = rec['P'], rec['V'], rec['team']
    f0 = rec['f_g0']
    slots = rec['slots']
    p0 = P[f0].astype(float)
    v0 = V[f0].astype(float)
    cost = ((p0[:, None, :] - slots[None]) ** 2).sum(-1)
    _, col = linear_sum_assignment(cost)
    tgt = slots[col]
    T0 = f0 / FPS
    span = DUR - T0
    a0 = np.arctan2(p0[:, 1], p0[:, 0])
    r0 = np.hypot(p0[:, 0], p0[:, 1])
    at = np.arctan2(tgt[:, 1], tgt[:, 0])
    rt = np.hypot(tgt[:, 0], tgt[:, 1])
    da = (at - a0 + math.pi) % (2 * math.pi) - math.pi
    da = np.where(da < 0, da + 2 * math.pi, da)                 # keep swirling the same way (CCW)
    for f in range(f0, NF):
        u = (f / FPS - T0) / span
        e = u * u * (3 - 2 * u)
        e = 1 - (1 - e) ** 1.6
        ang = a0 + da * e
        rad = r0 + (rt - r0) * e
        P[f] = np.stack([rad * np.cos(ang), rad * np.sin(ang)], 1)
        TM[f] = rec['winner']
    # velocities by finite difference (headings), starting from the sim's own velocity
    for f in range(f0 + 1, NF):
        V[f] = (P[f] - P[f - 1]) * FPS
    V[f0] = v0


def headings(rec):
    """facing angle per frame (radians): smoothed direction of travel; the last 0.45 s turn to face outward
    (radially), matching the burst at frame 0"""
    V = rec['V'].astype(float)
    P = rec['P'].astype(float)
    n = V.shape[1]
    H = np.zeros((NF, n))
    cur = np.arctan2(V[0, :, 1], V[0, :, 0])
    for f in range(NF):
        sp = np.hypot(V[f, :, 0], V[f, :, 1])
        want = np.arctan2(V[f, :, 1], V[f, :, 0])
        k = np.clip(sp / 0.6, 0, 1) * 0.35
        dd = (want - cur + math.pi) % (2 * math.pi) - math.pi
        cur = cur + dd * k
        H[f] = cur
    out = np.arctan2(P[0, :, 1], P[0, :, 0])                 # frame 0 faces outward (the burst)
    nt = int(0.45 * FPS)
    for k in range(nt):
        f = NF - nt + k
        u = (k + 1) / (nt + 1)
        u = u * u * (3 - 2 * u)
        dd = (out - H[f] + math.pi) % (2 * math.pi) - math.pi
        H[f] = H[f] + dd * u
    H[0] = out
    return H


def counts(rec):
    TM = rec['team']
    return np.stack([(TM == q).sum(1) for q in range(3)], 1)


def save(rec, path):
    c = rec['catches']
    np.savez_compressed(path, P=rec['P'].astype(np.float32), team=rec['team'], H=headings(rec).astype(np.float32),
                        catches=np.array(c, float).reshape(-1, 5), outs=np.array(rec['outs'], float).reshape(-1, 2),
                        t_win=rec['t_win'] if rec['t_win'] is not None else -1.0, winner=rec['winner'],
                        slots=rec['slots'], f_g0=rec['f_g0'], seed=rec['seed'])

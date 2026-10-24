"""Episode 1 of Falling Sand Wars: the schedule (pour, lid, hole, turns, roll), the balance and the run.

Schedule (video time = sim time, 61.0 s, 94.43 BPM: bar k starts at k * 2.5417 s):
    0.0          the four inlet valves spring open (box empty, upright)  - frame 0 IS the new run
    0 - 19.0     four hoppers pour the same number of grains each, with seeded flow noise; valves shut
    20.33        bar 8: the plug pops, the hole opens (a notch in a seeded bottom corner)
    22.9 ...     45 deg turns on downbeats: every 2 bars, then every bar, then every half bar in the duel;
                 each turn's direction is seeded; turning stops at the win
    win          the third team's last grain leaves; the plug snaps back (the box stops)
    57.19        the plug pops again; the box rolls over until the notch corner points straight down, holds
                 while the winner pours out in one long stream (an hourglass), then turns the last 45 deg and
                 comes to rest upright at exactly 61.0 = frame 0 (the plug is back in once the box is empty)"""
import math
import numpy as np
from sands import SandBox, Params, gravity_index, NAMES

FPS = 30
DUR = 61.0
NF = int(DUR * FPS)
BPM = 4 * 24 * 60.0 / DUR          # 94.43: 24 bars in 61 s (reelkit.config.loop_bpm(96, 61))
BEAT = 60.0 / BPM
BAR = 4 * BEAT

T_VALVE_SHUT = 19.0
T_HOLE = 8 * BAR                    # 20.33
TURN_DUR = 0.4
TURN_BEATS = ([9 * 4, 11 * 4, 13 * 4] + [b * 4 for b in range(14, 18)] +
              [b * 2 for b in range(36, 48)])          # bars 9, 11, 13, every bar 14-17, every half bar from 18
TURN_TIMES = [b * BEAT for b in TURN_BEATS]
T_ROLL0 = 22 * BAR + 2 * BEAT       # 57.19: a beat; the plug pops again
T_ROLL1 = T_ROLL0 + 1.0             # the notch corner points straight down
T_HOLD1 = 60.05                     # start the last 45 deg back to upright
T_REPLUG = 60.4                     # the plug is back in (the box is empty)
T_GATE = 1.5                        # the gate slides fully open over this long once the notch points down

KO_FRAC = 0.12                      # a team under 12 % of its starting sand is out (its last grains go dark)
TURN_GREEDY = 0.7                  # a turn tips the hole lower with this probability, else the other way
INLETS = [0.5, 0.5, 0.5, 0.5]       # the four spouts meet in ONE inlet: a braided stream, so the colours mix (then sort)
INLET_HW = 8                        # inlet half width (cells)
POUR_FRAC = 0.42                    # fill the box a little under halfway

PARAMS = Params()
SEED = None                         # set from seed_search.py


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * (3 - 2 * x)


def hermite(t, k0, k1):
    """cubic Hermite between keys (t, value, slope)"""
    (ta, a, va), (tb, b, vb) = k0, k1
    h = tb - ta
    u = min(1.0, max(0.0, (t - ta) / h))
    h00, h10, h01, h11 = 2 * u ** 3 - 3 * u ** 2 + 1, u ** 3 - 2 * u ** 2 + u, -2 * u ** 3 + 3 * u ** 2, u ** 3 - u ** 2
    return h00 * a + h10 * h * va + h01 * b + h11 * h * vb


def roll_keys(theta_win, side):
    """the closing roll: over and round (at least 135 deg) until the notch corner points straight down, a slow
    creep while the winner pours out of it, then the last 45 deg back to upright, landing on a multiple of 360 at
    61.0 with zero speed. A right-hand notch is approached turning CCW (it points down at -45), a left one CW."""
    s = 1.0 if side == 'R' else -1.0
    cd = -45.0 if side == 'R' else 45.0
    d1 = ((cd - theta_win) * s) % 360.0
    while d1 < 135.0:
        d1 += 360.0
    creep = 6.0
    return [(T_ROLL0, theta_win, 0.0),
            (T_ROLL1, theta_win + s * d1, s * 4.0),
            (T_HOLD1, theta_win + s * (d1 + creep), s * 4.0),
            (DUR, theta_win + s * (d1 + 45.0), 0.0)]


def pour_profile(rng, nh, n_total):
    """per-hopper grains per frame (nh, NF): thin streams that thicken by ~5 s, seeded flow noise, then shut.
    Every hopper pours exactly n_total grains (a fair start)."""
    t = np.arange(NF) / FPS
    base = np.clip(t / 5.0, 0, 1) ** 1.6 * 0.85 + 0.15
    base *= (t < T_VALVE_SHUT) * (1 - np.clip((t - (T_VALVE_SHUT - 1.2)) / 1.2, 0, 1) * 0.7)
    out = []
    for h in range(nh):
        noise = 1 + sum(a * np.sin(2 * math.pi * t / p + rng.uniform(0, 6.3))
                        for a, p in zip(rng.uniform(0.12, 0.3, 3), rng.uniform(1.6, 6.5, 3)))
        r = base * np.clip(noise, 0.25, None)
        out.append(r * n_total / r.sum())
    return np.array(out)


class Run:
    def __init__(self, seed, P=None, record=False):
        self.P = P or PARAMS
        self.seed = seed
        self.rng = np.random.default_rng(seed + 7919)
        self.box = SandBox(self.P, seed)
        N = self.P.N
        self.order = list(self.rng.permutation(4))                 # team at each hopper (left to right)
        # where the plug pops: a seeded bottom corner
        self.hole_side = 'L' if self.rng.random() < 0.5 else 'R'
        self.box.set_hole(self.hole_side)
        self.n_each = int(N * N * POUR_FRAC / 4)
        self.rates = pour_profile(self.rng, 4, self.n_each)
        self.turn_coin = self.rng.random(len(TURN_TIMES))        # one seeded coin per turn
        self.record = record
        self.stop_at_win = False

    def turn_direction(self, theta, i):
        """the rule for every turn: with probability TURN_GREEDY (a seeded coin) tip the box the way that puts the
        hole lower, else the other way (a tie is a coin flip). Like someone shaking the last sand out of a box,
        never quite sure which way it will go."""
        xh, yh = self.box.hole_local()
        h = {}
        for d in (1, -1):
            a = math.radians(theta + 45 * d)
            h[d] = math.sin(a) * xh + math.cos(a) * yh              # world height of the hole after the turn
        if abs(h[1] - h[-1]) < 1e-6:
            return 1 if self.turn_coin[i] < 0.5 else -1
        greedy = 1 if h[1] < h[-1] else -1
        return greedy if self.turn_coin[i] < TURN_GREEDY else -greedy

    def run(self):
        P, box = self.P, self.box
        N, K = P.N, P.K
        acc = np.zeros(4)
        theta = 0.0
        turns_done = []                                 # (t0, theta0, dir)
        cur = None
        t_win, outs, roll = None, [], None
        rec_theta = np.zeros(NF, np.float32)
        rec_cnt = np.zeros((NF, 4), np.int32)
        rec_land = np.zeros((NF, 4), np.int32)
        rec_move = np.zeros((NF, 4), np.int32)
        rec_drain = np.zeros((NF, 4), np.int32)
        rec_lid = np.zeros(NF, np.float32)
        rec_hole = np.zeros(NF, bool)
        grids = np.zeros((NF, N, N), np.uint8) if self.record else None
        ex_f, ex_s, ex_m, ex_r, ex_c, ex_t = [], [], [], [], [], []
        poured = np.zeros(4, np.int64)
        ti = 0
        for f in range(NF):
            t = f / FPS
            if self.stop_at_win and t_win is not None and t > t_win + 0.5:
                break
            tm = t + 0.5 / FPS
            # ---- box angle
            if roll is None and t >= T_ROLL0:
                roll = roll_keys(theta if cur is None else cur[1] + cur[2] * 45.0, self.hole_side)
                cur = None
            if roll is not None:
                k = 0 if tm < roll[1][0] else (1 if tm < roll[2][0] else 2)
                theta = hermite(min(tm, DUR), roll[k], roll[k + 1])
            else:
                while ti < len(TURN_TIMES) and tm >= TURN_TIMES[ti]:
                    if t_win is None and cur is None:
                        cur = (TURN_TIMES[ti], theta, self.turn_direction(theta, ti))
                        turns_done.append(cur)
                    ti += 1
                if cur is not None:
                    u = (tm - cur[0]) / TURN_DUR
                    theta = cur[1] + cur[2] * 45.0 * ease(u)
                    if u >= 1:
                        theta = cur[1] + cur[2] * 45.0
                        cur = None
            gk = gravity_index(theta)
            # ---- openings
            valve = 1.0 if t < T_VALVE_SHUT else 0.0
            hole = ((t >= T_HOLE) and (t_win is None or t < t_win + 0.1) and t < T_ROLL0) or (T_ROLL0 + 0.1 <= t < T_REPLUG)
            # the top wall counts as open while the inlets are (to 19.85 s): nothing leaves upward in an upright box,
            # but clay at the top row doesn't stick to a wall that isn't there (this is the rule the seeds were run on)
            box.set_open(t < 19.85, hole)
            box.rattle = t >= T_ROLL0 + 0.4                 # the closing roll rattles every grain loose
            if T_ROLL1 <= t < T_REPLUG:                     # the closing pour: the gate slides all the way open
                up0 = max(4, int(round(P.hole_w * N)))
                up = up0 + int((N - up0) * ease((t - T_ROLL1) / T_GATE))
                if up != box.hole_up:
                    box.set_hole(self.hole_side, up=up)
            elif box.hole_up > max(4, int(round(P.hole_w * N))):
                box.set_hole(self.hole_side)
            box.hold = P.clay_hold if not box.rattle else 0.0
            pouring = t < T_VALVE_SHUT + 0.05
            if pouring:
                acc += self.rates[:, f]
            KK = K * 3 if box.rattle else K               # the rattle shakes the sand down faster
            for s in range(KK):
                if pouring and s < K:                   # pour (box upright and still); a fair, shuffled hopper order
                    for h in self.rng.permutation(4):
                        n = int(acc[h] / (K - s) + 0.999)
                        h = int(h)
                        if n > 0:
                            q = self.order[h]
                            c = int(round(INLETS[h] * (N - 1)))
                            got = box.inject(max(0, c - INLET_HW), min(N - 1, c + INLET_HW), n, q + 1)
                            acc[h] -= got
                            poured[q] += got
                box.substep(gk)
                lnd, mv, exits = box.pop_stats()
                rec_land[f] += lnd[1:5]
                rec_move[f] += mv[1:5]
                for (m, r, c, tn) in exits:
                    rec_drain[f] += np.bincount(m, minlength=5)[1:5]
                    ex_f.append(np.full(len(m), f, np.int16)); ex_s.append(np.full(len(m), s, np.int8))
                    ex_m.append(m); ex_r.append(r); ex_c.append(c); ex_t.append(tn)
            cnt = box.counts()
            rec_cnt[f] = cnt
            rec_theta[f] = theta
            rec_lid[f] = valve
            rec_hole[f] = hole
            if grids is not None:
                grids[f] = box.snapshot()
            if T_HOLE <= t < T_ROLL0 and t_win is None:
                for q in range(4):
                    if cnt[q] < KO_FRAC * self.n_each and q not in [o[1] for o in outs]:
                        outs.append((t, q))
                if len(outs) >= 3:
                    t_win = t
        winner = None
        left = [q for q in range(4) if q not in [o[1] for o in outs]]
        if len(outs) >= 3 and len(left) == 1:
            winner = left[0]                            # (if the last two empty in the same frame: no winner, a tie)
        cat = lambda L, dt: np.concatenate(L) if L else np.zeros(0, dt)
        return dict(seed=self.seed, order=self.order, hole_side=self.hole_side, hole_cells=self.box.hole_cells(), n_each=self.n_each, poured=poured, theta=rec_theta,
                    count=rec_cnt, landed=rec_land, moves=rec_move, drained=rec_drain, lid=rec_lid, hole=rec_hole,
                    grids=grids, outs=outs, t_win=t_win, winner=winner,
                    turns=[(a, b, c) for a, b, c in turns_done], roll=roll,
                    ex_f=cat(ex_f, np.int16), ex_s=cat(ex_s, np.int8), ex_m=cat(ex_m, np.int8),
                    ex_r=cat(ex_r, np.int16), ex_c=cat(ex_c, np.int16), ex_t=cat(ex_t, np.uint8),
                    left=rec_cnt[-1].sum())


def summary(rec):
    outs = ' '.join(f"{NAMES[q][:2]}@{t:4.1f}" for t, q in rec['outs'])
    w = NAMES[rec['winner']] if rec['winner'] is not None else '-'
    return f"order {''.join(NAMES[q][0] for q in rec['order'])} hole {rec['hole_side']}  outs {outs}  win {w}  left at end {rec['left']}"

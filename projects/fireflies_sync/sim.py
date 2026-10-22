"""Flash schedule for every firefly: ONE list drives the glows, the light on the ferns, the HUD and the bells.

Populations (males):
    0           the hero (lone male of the opening)
    1..19       his neighbours: hero + 19 = the ~20 that lock in (burst A)
    20..59      the rest of the opening's random flashers (join at burst 1)
    60..99      newcomers that rise from the ferns in the first gap (burst 1 = 100 "dozens")
    100..399    newcomers of the long first gap (burst 2 = 400 "hundreds")
    400..499    newcomers of the second gap (near swarm for the climax)
    500..2999   the valley: dark until it erupts on burst 3
Females: 0..15 near the hero (answer in the gaps), 16.. across the valley (answer after the climax).

Lock-in: a Kuramoto mean-field model in the frame of the 109 BPM grid (phase 0 = on the beat):
    dphi_i/dt = d_i + K(t) R sin(psi - phi_i)
Each fly flashes once per beat at grid(b) - phi_i / omega, skipping beats with probability 1 - p(t) while
the neighbours are still finding each other; in burst A all 20 flash every beat and the spread collapses
from a smear (~+-100 ms) through a flam to unison. Bursts 2 and 3 use the burst-onset wave (0.5 m/s).

    python sim.py      -> sim.npz (per-frame intensities + event lists), prints the lock-in spread"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import timeline as T
import scene as S

rng = np.random.default_rng(1515)
OMEGA = 2 * math.pi / T.BEAT
N_M = 3000
G_LOCK = list(range(0, 20))
R_OPEN = list(range(0, 60))
B1_NEW = list(range(60, 100))
B2_NEW = list(range(100, 400))
B3_NEW = list(range(400, 500))
VALLEY = list(range(500, 3000))
N_FN = 16
N_F = N_FN + 300

# ------------------------------------------------------------------ positions
P = np.zeros((N_M, 3))          # hover centre
AMP = np.zeros((N_M, 3))        # drift amplitude
WN = np.zeros((N_M, 3))         # drift harmonics (integer cycles per 60 s -> seamless loop)
PH = np.zeros((N_M, 3))
ARR = np.full(N_M, -1e9)        # arrival time (rise from a fern)
GROUND = np.zeros((N_M, 3))     # where a newcomer sits before rising


def place(n, xr, zr, yr, near=None, min_d=0.0, trunk_m=0.35):
    out = []
    while len(out) < n:
        x, z, y = rng.uniform(*xr), rng.uniform(*zr), rng.uniform(*yr)
        if near is not None and math.hypot(x - near[0], z - near[2]) > near[3]:
            continue
        if not S.clear_of_trunks(x, z, trunk_m):
            continue
        if min_d and out and min(math.hypot(x - a, z - c) + abs(y - b) * 0.5 for a, b, c in out) < min_d:
            continue
        out.append((x, y, z))
    return np.array(out)


P[0] = S.HERO0
P[1:20] = place(19, (-1.5, 1.5), (-1.4, 1.1), (0.55, 2.1), near=(0, 0, 0, 1.55), min_d=0.32)
def place_view(n, cam_xz, d_rng, lat_k, y_rng, bias=1.0, trunk_m=0.4):
    """inside a shot's view wedge (camera looking toward -z): depth d from the camera, |lateral| < lat_k * d"""
    out = []
    while len(out) < n:
        d = d_rng[0] + (d_rng[1] - d_rng[0]) * rng.uniform() ** bias
        x = cam_xz[0] + rng.uniform(-1, 1) * lat_k * d
        z = cam_xz[1] - d
        if not S.clear_of_trunks(x, z, trunk_m):
            continue
        out.append((x, rng.uniform(*y_rng), z))
    return np.array(out)


P[20:46] = place_view(26, (-0.4, 6.0), (4.5, 13.0), 0.25, (0.45, 2.6), bias=1.3)    # the opening's sparkle
P[46:60] = place_view(14, (-0.4, 6.0), (13.0, 26.0), 0.27, (0.6, 3.4))
P[60:100] = place(40, (-3.6, 3.6), (-5.4, 2.4), (0.35, 2.4), min_d=0.25)
P[100:400] = place(300, (-1.8, 1.8), (-7.5, 2.3), (0.3, 2.9))
P[400:500] = place(100, (-3.6, 3.6), (-7.5, 2.3), (0.3, 2.9))
P[500:3000] = place_view(2500, (-0.8, 9.0), (16.5, 66.0), 0.33, (0.3, 4.2), bias=1.6, trunk_m=0.6)   # the climax frame
AMP[:] = rng.uniform(0.08, 0.28, (N_M, 3)) * np.array([1.0, 0.45, 1.0])
AMP[0] = [0.07, 0.035, 0.05]
AMP[500:] *= 0.6
WN[:] = rng.integers(2, 8, (N_M, 3))
WN[0] = [2, 3, 2]
PH[:] = rng.uniform(0, 2 * math.pi, (N_M, 3))

# newcomers rise from a nearby fern
ferns = np.array([(x, z) for x, z, s in S.FERNS])
for ids, a, b in [(B1_NEW, 16.3, 19.6), (B2_NEW, 30.6, 33.0), (B3_NEW, 41.8, 45.4)]:
    for i in ids:
        ARR[i] = rng.uniform(a, b)
        k = np.argmin(np.hypot(ferns[:, 0] - P[i, 0], ferns[:, 1] - P[i, 2]))
        d = math.hypot(ferns[k, 0] - P[i, 0], ferns[k, 1] - P[i, 2])
        GROUND[i] = [ferns[k, 0], 0.28, ferns[k, 1]] if d < 1.2 else [P[i, 0], 0.12, P[i, 2]]

# pitch: left = low, right = high, with jitter (each fly keeps its own note)
PITCH = np.zeros(N_M, int)
PITCH[:500] = np.clip(np.round((P[:500, 0] + 3.6) / 7.2 * 10 + rng.normal(0, 1.1, 500)), 0, 10)
PITCH[500:] = np.clip(np.round((P[500:, 0] + 26) / 52 * 10 + rng.normal(0, 1.5, 2500)), 0, 10)
PITCH[0] = 5                                               # the hero sings D6

# females: dim points in the leaf litter / on fern tips
PF = np.zeros((N_F, 3))
# near females sit in the leaf litter inside the low shot's view wedge (camera ~(0.5, 2.6) looking up-cove)
_c, _d = np.array([0.5, 2.62]), np.array([-0.146, -0.989])
_lat = np.array([0.989, -0.146])
_fn = []
while len(_fn) < N_FN:
    dd, uu = rng.uniform(1.35, 3.7), rng.uniform(-1, 1)
    xz = _c + _d * dd + _lat * uu * 0.26 * dd
    if not S.clear_of_trunks(xz[0], xz[1], 0.3) or any(math.hypot(xz[0] - a, xz[1] - c) < 0.22 for a, b, c in _fn):
        continue
    _fn.append((xz[0], 0.03, xz[1]))
PF[:N_FN] = np.array(_fn)
for j in range(N_FN):                                      # some sit on fern fronds
    k = np.argmin(np.hypot(ferns[:, 0] - PF[j, 0], ferns[:, 1] - PF[j, 2]))
    if math.hypot(ferns[k, 0] - PF[j, 0], ferns[k, 1] - PF[j, 2]) < 0.5:
        PF[j, 1] = 0.18
PF[N_FN:] = place_view(300, (-0.8, 9.0), (15.0, 50.0), 0.3, (0.02, 0.2), bias=1.3, trunk_m=0.5)
PITCH_F = rng.integers(0, len(T.FEMALE_NOTES), N_F)


def pos(i, t):
    """male positions at time t (array of ids)"""
    i = np.atleast_1d(i)
    w = 2 * math.pi * WN[i] / T.DUR
    p = P[i] + AMP[i] * np.sin(w * t + PH[i])
    u = np.clip((t - ARR[i]) / 2.6, 0, 1)
    u = u * u * (3 - 2 * u)
    rising = (t < ARR[i] + 2.6) & (ARR[i] > 0)
    if rising.any():
        g = GROUND[i][rising]
        wob = 0.06 * np.sin(t * 9 + i[rising])[:, None] * np.array([1, 0, 1])
        p[rising] = g + (p[rising] - g) * u[rising, None] + wob * (1 - u[rising, None])
    return p


def hero(t):
    return pos(0, t)[0]


# ------------------------------------------------------------------ flash events
EV_T, EV_I = [], []             # male flashes (time, id)


def add(t, i):
    EV_T.append(t)
    EV_I.append(i)


def random_train(i, t0, t1, rate=1.0, seed=None):
    """irregular flashing: no clear rhythm (lognormal gaps, occasional short trains)"""
    r = np.random.default_rng(seed)
    t = t0 + r.uniform(0, 1.2 / max(rate, 0.05))
    out = []
    while t < t1:
        out.append(t)
        if r.uniform() < 0.25:
            for _ in range(int(r.integers(1, 3))):
                t += r.uniform(0.42, 0.7)
                if t < t1:
                    out.append(t)
        t += float(np.clip(r.lognormal(math.log(1.15 / rate), 0.55), 0.3, 4.0 / rate))
    return out


# 1) opening / ending random flashers: one process on [55.6, 72), wrapped -> seamless loop
for i in R_OPEN:
    for t in random_train(i, T.T_RANDOM_BACK, 72.0, rate=1.15, seed=900 + i):
        tw = t - 60.0 if t >= 60.0 else t
        if i == 0 and 3.9 <= tw < T.T_RET:
            continue                     # the hero: hand-placed macro flashes, then the lock-in model
        if 1 <= i < 20 and T.T_NEIGH <= tw < T.T_RET:
            continue
        if 20 <= i < 60 and T.T_RANDOM_END <= tw < T.T_RET:
            continue
        add(tw, i)
# the hero alone (macro, 4-8 s): stray flashes, plainly no rhythm
for t in [4.42, 5.21, 5.47, 6.74, 7.06, 7.88]:
    add(t, 0)

# 2) neighbours lock in: Kuramoto in the grid frame, 8.0 s -> end of burst A
DT = 1 / 600.0
ts = np.arange(T.T_NEIGH, T.T_A_END + 0.2, DT)
krng = np.random.default_rng(1515)                # tuned seed: smear -> flam -> unison over burst A
phi = krng.uniform(-math.pi, math.pi, 20)
phi[0] = 0.5
det = krng.normal(0, 0.03, 20)                     # tiny natural tempo spread (rad/s)
det -= det.mean()
PHI = np.zeros((len(ts), 20))


def K_of(t):
    if t < T.T_A:
        return 0.3 + 1.0 * (t - T.T_NEIGH) / (T.T_A - T.T_NEIGH)
    return 0.85 + 0.3 * (t - T.T_A)


for s, t in enumerate(ts):
    z = np.exp(1j * phi).mean()
    R, psi = abs(z), np.angle(z)
    phi = phi + (det + K_of(t) * R * np.sin(psi - phi)) * DT
    phi = np.angle(np.exp(1j * (phi - np.angle(np.exp(1j * phi).mean()))))      # keep the mean on the beat
    PHI[s] = phi


def phi_at(t):
    return PHI[min(len(ts) - 1, max(0, int(round((t - T.T_NEIGH) / DT))))]


spread = []
b0 = math.ceil((T.T_NEIGH + 0.2 - T.TB1) / T.BEAT)
for b in range(b0, -8):
    tg = T.grid(b)
    ph = phi_at(tg)
    in_a = b >= -16
    p_flash = 1.0 if in_a else 0.3 + 0.55 * (tg - T.T_NEIGH) / (T.T_A - T.T_NEIGH)
    for i in G_LOCK:
        if in_a or rng.uniform() < p_flash:
            add(tg - ph[i] / OMEGA, i)
    if in_a:
        spread.append((b + 16 + 1, 1000 * (ph.max() - ph.min()) / OMEGA, 1000 * ph.std() / OMEGA))


def run_burst(members, b0, rule):
    """members flash on grid beats b0..b0+7 when rule(i, k) is True (+-small natural jitter)"""
    for i in members:
        for k in range(T.FLASHES):
            if rule(i, k):
                add(T.grid(b0 + k) + rng.normal(0, 0.006), i)


def run_len(i, k, seed_base):
    r = np.random.default_rng(seed_base + i)
    start = int(r.integers(0, 3))
    n = int(r.integers(6, 9))
    return start <= k < start + n


# 3) gap A: newcomers rise and flash at random, denser toward the drop
for i in list(range(20, 60)) + B1_NEW:
    t0 = max(T.T_A_END + 0.25, ARR[i] + 0.4)
    for t in random_train(i, t0, T.T_B1 - 0.16, rate=0.75, seed=2000 + i):
        add(t, i)
# 4) burst 1: dozens in unison (each fly 6-8 of the 8 flashes)
B1 = list(range(0, 100))
run_burst(B1, 0, lambda i, k: i == 0 or run_len(i, k, 3000))
# 5) gap 1: true dark first (females answer), then the next wave of newcomers rises
for i in B2_NEW:
    t0 = ARR[i] + 0.5
    r = np.random.default_rng(4000 + i)
    for t in random_train(i, t0, T.T_B2 - 0.16, rate=0.32, seed=4000 + i):
        if r.uniform() < 0.25 + 0.75 * (t - 31.0) / 3.0:
            add(t, i)
# 6) burst 2: the burst rolls left -> right at 0.5 m/s (each fly flashes while the front is near it)
B2 = list(range(0, 400))
FRONT2 = lambda k: -1.1 + T.V_WAVE * T.BEAT * k
run_burst(B2, 24, lambda i, k: abs(P[i, 0] - FRONT2(k)) <= T.WAVE_HW)
# 7) gap 2: near newcomers rise (the valley stays dark)
for i in B3_NEW:
    for t in random_train(i, ARR[i] + 0.5, T.T_B3 - 0.16, rate=0.42, seed=5000 + i):
        add(t, i)
# 8) burst 3: two fronts roll in from both sides and meet over the hero; the valley erupts
X0 = 1.1
NEAR3 = list(range(0, 500))


def rule3(i, k):
    ax = abs(P[i, 0])
    if ax < X0:
        return k >= math.ceil((X0 - ax) / (T.V_WAVE * T.BEAT) - 1e-9)
    return run_len(i, k, 6000) or k >= 4


run_burst(NEAR3, 48, rule3)
run_burst(VALLEY, 48, lambda i, k: run_len(i, k, 7000))

EV_T = np.array(EV_T)
EV_I = np.array(EV_I, int)
o = np.argsort(EV_T)
EV_T, EV_I = EV_T[o], EV_I[o]

# ------------------------------------------------------------------ female answers (quantised to 8ths)
FE_T, FE_I = [], []
q8 = lambda t: T.TB1 + round((t - T.TB1) / (T.BEAT / 2)) * T.BEAT / 2
for (a, b, ids, n) in [(26.2, 31.8, range(0, N_FN), 22), (31.8, 33.6, range(0, N_FN), 5),
                       (39.3, 43.6, range(0, N_FN), 12), (52.0, 55.5, range(N_FN, N_F), 150)]:
    ids = list(ids)
    times = np.sort(rng.uniform(a, b, n))
    for t in times:
        FE_T.append(q8(t))
        FE_I.append(ids[int(rng.integers(0, len(ids)))])
# a guaranteed answer right under the hero for the low shot
FE_T += [q8(26.6), q8(28.25)]
FE_I += [0, 0]
FE_T = np.array(FE_T)
FE_I = np.array(FE_I, int)
o = np.argsort(FE_T)
FE_T, FE_I = FE_T[o], FE_I[o]


# ------------------------------------------------------------------ per-frame intensities
def env_m(tau):
    """male flash (100-150 ms): peaks on the flash frame, fast decay"""
    return np.where((tau >= -1 / 60) & (tau < 0.28), np.exp(-np.maximum(tau, 0) / 0.075), 0.0)


def env_f(tau):
    """female answer: dimmer, a little longer"""
    return np.where((tau >= -1 / 60) & (tau < 0.6), np.exp(-np.maximum(tau, 0) / 0.17), 0.0)


def intensities():
    IM = np.zeros((T.NF, N_M), np.float16)
    IF = np.zeros((T.NF, N_F), np.float16)
    ft = np.arange(T.NF) / T.FPS
    for t, i in zip(EV_T, EV_I):
        f0, f1 = max(0, int(math.floor((t - 0.02) * T.FPS))), min(T.NF, int(math.ceil((t + 0.29) * T.FPS)) + 1)
        if f1 <= f0:
            continue
        v = env_m(ft[f0:f1] - t)
        IM[f0:f1, i] = np.maximum(IM[f0:f1, i], v)
    for t, i in zip(FE_T, FE_I):
        f0, f1 = max(0, int(math.floor((t - 0.02) * T.FPS))), min(T.NF, int(math.ceil((t + 0.62) * T.FPS)) + 1)
        v = env_f(ft[f0:f1] - t)
        IF[f0:f1, i] = np.maximum(IF[f0:f1, i], v)
    return IM, IF


def sync_count(t):
    """flies in sync, for the HUD (measured from the model / burst membership)"""
    if t < T.T_NEIGH or t >= T.T_RET:
        return 0
    if t < T.T_A_END:
        ph = phi_at(t)
        return int((np.abs(ph) < 0.35).sum())
    if t < T.T_B1:
        return 20
    if t < T.T_B2:
        return 100
    if t < T.T_B3:
        return 400
    return 3000


if __name__ == '__main__':
    print('burst A spread (flash #, range ms, std ms):')
    for k, rg, sd in spread:
        print(f'  {k}: {rg:6.1f}  {sd:6.1f}')
    for t in [8.5, 9.5, 10.5, 11.5, 12.19, 13.0, 14.0, 15.0, 16.0]:
        print(f'  t={t:5.2f}  in sync {sync_count(t):2d}')
    print('male flashes', len(EV_T), ' female answers', len(FE_T))
    for a, b in [(0, 4), (4, 8), (8, 12), (12, 16.6), (16.6, 21), (21, 25.4), (25.4, 30), (30, 34.2), (34.2, 38.6),
                 (38.6, 42), (42, 47.4), (47.4, 51.8), (51.8, 56), (56, 60)]:
        m = (EV_T >= a) & (EV_T < b)
        print(f'  {a:5.1f}-{b:5.1f}s  male flashes {m.sum():6d} ({m.sum() / (b - a):6.1f}/s)  females {((FE_T >= a) & (FE_T < b)).sum()}')
    IM, IF = intensities()
    np.savez_compressed(os.path.join(HERE, 'sim.npz'), IM=IM, IF=IF, EV_T=EV_T, EV_I=EV_I, FE_T=FE_T, FE_I=FE_I)
    print('wrote sim.npz', IM.shape, IF.shape)

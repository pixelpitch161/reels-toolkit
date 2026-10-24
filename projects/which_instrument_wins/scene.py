"""Static geometry for #26: a low-poly colosseum at sunset with a floating ring over a pit.

World: y up, arena floor y = 0. The ring hangs in the plane z = 0 with its centre at RING_C; the physics (sim.py)
runs in that plane (sim x, y -> world x, RING_C[1] + y). The pit (for the eliminated) is under the ring. The
colosseum is centred at ARENA_C, so the camera (on +z) stands inside the arena looking at the ring, with the far
stands behind it: tall on the left, broken low on the right where the low sun shows through.
"""
import math
import numpy as np
from reelkit.config import dirv
from reelkit.geometry import face

rng = np.random.default_rng(26)

RING_C = np.array([0.0, 3.5, 0.0])
SUBJECT = RING_C.copy()
PIT_R = 2.7
PIT_Y = -0.5
ARENA_C = np.array([0.0, 0.0, -4.0])
SUN = dirv(-11.3, 3.7)                     # sun on the horizon, behind-left, seen through the collapsed side of the stands

PODIUM_R, PODIUM_H = 17.0, 2.6
STEP_N, STEP_W, STEP_H = 6, 1.05, 1.0
WALL_R = PODIUM_R + STEP_N * STEP_W + 0.3
WALL_T = 1.0
SEG = 72                                   # bays around the colosseum


def arena_pt(alpha, r, y):
    """alpha: degrees around the arena centre, 0 = far side (behind the ring), + toward the right"""
    a = math.radians(alpha)
    return np.array([ARENA_C[0] + r * math.sin(a), y, ARENA_C[2] - r * math.cos(a)])


BROKEN = (-40.0, 0.0)                      # alpha range where the stands have collapsed (the sun shines in here)


def broken_k(alpha):
    """0 = intact stands, 1 = fully collapsed (only the podium and rubble left)"""
    a = (alpha + 180) % 360 - 180
    lo, hi = BROKEN
    if lo + 6 <= a <= hi - 6:
        return 1.0
    if lo - 6 < a < lo + 6:
        return (a - (lo - 6)) / 12
    if hi - 6 < a < hi + 6:
        return 1 - (a - (hi - 6)) / 12
    return 0.0


def wall_height(alpha):
    """outer wall height by azimuth: tall, stepping down into the collapse on the sunset side"""
    return 15.0 - broken_k(alpha) * 15.0


STONE = np.array([214, 172, 124])
STONE2 = np.array([196, 150, 106])
SEAT_A = np.array([206, 160, 116])
SEAT_B = np.array([226, 186, 136])
ARCH = np.array([70, 44, 48])

# backdrop items: (centre (3,), [(Q, n, albedo, emit)]) drawn as a unit in painter order
BACK = []
_jag = rng.uniform(-1.1, 0.5, SEG + 1)


def _add(items):
    C = np.mean([q.mean(0) for q, *_ in items], axis=0)
    BACK.append((C, items))


def _rubble(c, r, h, rot):
    from reelkit.geometry import box
    return box(c[0], h / 2, c[2], r, h / 2, r * 0.8, rot)


def build_backdrop():
    da = 360.0 / SEG
    for k in range(SEG):
        a0, a1 = -180 + k * da, -180 + (k + 1) * da
        am = (a0 + a1) / 2
        # podium wall (faces the arena centre); broken down to ~1 m in the collapse so the low sun reaches the sand
        ph0 = PODIUM_H - broken_k(a0) * (PODIUM_H - 0.9 - 0.25 * _jag[k])
        ph1 = PODIUM_H - broken_k(a1) * (PODIUM_H - 0.9 - 0.25 * _jag[k + 1])
        p = [arena_pt(a0, PODIUM_R, 0), arena_pt(a1, PODIUM_R, 0), arena_pt(a1, PODIUM_R, ph1), arena_pt(a0, PODIUM_R, ph0)]
        Q, n = face(p, ARENA_C + [0, 1.3, 0])
        n = -n if np.dot(n, ARENA_C - Q.mean(0)) < 0 else n
        tone = 1 + rng.uniform(-0.04, 0.04)
        items = [(Q, n, STONE2 * tone, 0.0)]
        bkm = broken_k(am)
        # dark doorway every 6th bay, plaques otherwise
        if k % 6 == 3 and bkm < 0.2:
            d0, d1 = am - da * 0.28, am + da * 0.28
            dq = np.array([arena_pt(d0, PODIUM_R - 0.03, 0), arena_pt(d1, PODIUM_R - 0.03, 0),
                           arena_pt(d1, PODIUM_R - 0.03, 1.7), arena_pt(am, PODIUM_R - 0.03, 2.05), arena_pt(d0, PODIUM_R - 0.03, 1.7)])
            items.append((dq, n, ARCH * 0.7, 0.0))
        # parapet band
        bq = np.array([arena_pt(a0, PODIUM_R - 0.02, PODIUM_H - 0.35), arena_pt(a1, PODIUM_R - 0.02, PODIUM_H - 0.35),
                       arena_pt(a1, PODIUM_R - 0.02, PODIUM_H - 0.2), arena_pt(a0, PODIUM_R - 0.02, PODIUM_H - 0.2)])
        if bkm < 0.2:
            items.append((bq, n, np.array([168, 120, 88]), 0.0))
        _add(items)
        bk = broken_k(am)
        n_steps = STEP_N if bk <= 0 else int(round(STEP_N * (1 - bk)))
        if bk > 0.6:
            n_steps = 0                                       # collapsed: only rubble
        # stepped seating (risers + treads)
        for s in range(n_steps):
            r0 = PODIUM_R + s * STEP_W
            y0 = PODIUM_H + s * STEP_H
            tread = np.array([arena_pt(a0, r0, y0), arena_pt(a1, r0, y0), arena_pt(a1, r0 + STEP_W, y0), arena_pt(a0, r0 + STEP_W, y0)])
            riser = np.array([arena_pt(a0, r0 + STEP_W, y0), arena_pt(a1, r0 + STEP_W, y0),
                              arena_pt(a1, r0 + STEP_W, y0 + STEP_H), arena_pt(a0, r0 + STEP_W, y0 + STEP_H)])
            nt = np.array([0, 1.0, 0])
            Qr, nr = face(riser, ARENA_C + [0, y0 + 0.5, 0])
            nr = -nr if np.dot(nr, ARENA_C - Qr.mean(0)) < 0 else nr
            col = (SEAT_A if s % 2 else SEAT_B) * (1 + rng.uniform(-0.035, 0.035))
            _add([(tread, nt, col, 0.0), (Qr, nr, col * 0.86, 0.0)])
        # rubble blocks on the collapsed side
        if bk > 0.3:
            for j in range(2):
                ra = PODIUM_R + 0.8 + rng.uniform(0, 4.0)
                aa = am + rng.uniform(-da * 0.4, da * 0.4)
                c = arena_pt(aa, ra, 0)
                hh = rng.uniform(0.5, 1.5)
                for Q, n in _rubble(c, rng.uniform(0.5, 0.9), hh, rng.uniform(0, 3)):
                    _add([(Q, n, STONE2 * rng.uniform(0.85, 1.05), 0.0)])
        if bk >= 0.98:
            continue
        # outer wall inner face with two rows of arches, jagged where broken
        y_base = PODIUM_H + STEP_N * STEP_H
        h0 = max(y_base + 0.3, wall_height(a0) + (_jag[k] * 2.5 if broken_k(a0) > 0 else 0))
        h1 = max(y_base + 0.3, wall_height(a1) + (_jag[k + 1] * 2.5 if broken_k(a1) > 0 else 0))
        wq = [arena_pt(a0, WALL_R, y_base), arena_pt(a1, WALL_R, y_base), arena_pt(a1, WALL_R, h1), arena_pt(a0, WALL_R, h0)]
        Qw, nw = face(wq, ARENA_C + [0, 8, 0])
        nw = -nw if np.dot(nw, ARENA_C - Qw.mean(0)) < 0 else nw
        items = [(Qw, nw, STONE * (1 + rng.uniform(-0.04, 0.04)), 0.0)]
        hmin = min(h0, h1)
        for row, (ya, yb) in enumerate(((y_base + 0.7, y_base + 2.9), (y_base + 3.9, y_base + 5.6))):
            if yb + 0.4 > hmin:
                continue
            w = da * 0.30
            arc = [arena_pt(am - w, WALL_R - 0.03, ya), arena_pt(am + w, WALL_R - 0.03, ya)]
            for j in range(5):
                th = math.pi * j / 4
                arc.append(arena_pt(am + w * math.cos(th), WALL_R - 0.03, yb + 0.55 * math.sin(th)))
            items.append((np.array(arc), nw, ARCH * (0.95 if row else 0.8), 0.0))
        # cornice on intact bays
        if hmin > 13.5:
            cq = np.array([arena_pt(a0, WALL_R - 0.05, hmin - 0.9), arena_pt(a1, WALL_R - 0.05, hmin - 0.9),
                           arena_pt(a1, WALL_R - 0.05, hmin - 0.7), arena_pt(a0, WALL_R - 0.05, hmin - 0.7)])
            items.append((cq, nw, np.array([238, 200, 150]), 0.0))
        _add(items)
        # wall top (seen only where the camera sees over it: never here, but it catches the sun on the rim)
        tq = np.array([arena_pt(a0, WALL_R, h0), arena_pt(a1, WALL_R, h1), arena_pt(a1, WALL_R + WALL_T, h1), arena_pt(a0, WALL_R + WALL_T, h0)])
        _add([(tq, np.array([0, 1.0, 0]), STONE * 1.05, 0.0)])


build_backdrop()

# torches on the podium wall (alpha) and two braziers on the sand flanking the ring
TORCH_A = [-44, -6, 6, 18, 30, 42]
TORCHES = [arena_pt(a, PODIUM_R - 0.35, PODIUM_H - 0.45) for a in TORCH_A]
BRAZIERS = [np.array([-4.6, 1.25, -8.0]), np.array([4.6, 1.25, -8.0])]
# torches on the top step that ignite in a wave when the winner is crowned
WIN_TORCH_A = [a for a in np.arange(-44, 60, 4.0) if broken_k(a) < 0.5]
WIN_TORCHES = [arena_pt(a, PODIUM_R + STEP_N * STEP_W - 0.2, PODIUM_H + STEP_N * STEP_H + 0.5) for a in WIN_TORCH_A]

# braziers: bowls on legs (static faces)
BRAZIER_FACES = []
for b in BRAZIERS:
    x, y, z = b
    for k in range(8):
        a0, a1 = 2 * math.pi * k / 8, 2 * math.pi * (k + 1) / 8
        p = [[x + 0.42 * math.cos(a0), y - 0.3, z + 0.42 * math.sin(a0)], [x + 0.42 * math.cos(a1), y - 0.3, z + 0.42 * math.sin(a1)],
             [x + 0.6 * math.cos(a1), y, z + 0.6 * math.sin(a1)], [x + 0.6 * math.cos(a0), y, z + 0.6 * math.sin(a0)]]
        BRAZIER_FACES.append(face(p, b - [0, 0.15, 0]) + (np.array([70, 50, 40]),))
    for k in range(3):
        a = 2 * math.pi * k / 3 + 0.3
        q = [[x + 0.15 * math.cos(a), 0, z + 0.15 * math.sin(a)], [x + 0.25 * math.cos(a), 0, z + 0.25 * math.sin(a)],
             [x + 0.42 * math.cos(a), y - 0.3, z + 0.42 * math.sin(a)], [x + 0.32 * math.cos(a), y - 0.3, z + 0.32 * math.sin(a)]]
        BRAZIER_FACES.append(face(q, [x, 0.5, z]) + (np.array([60, 44, 36]),))

# pennants on poles along the tall top of the stands
PENNANTS = [(arena_pt(a, WALL_R + 0.3, wall_height(a)), (a * 7.31) % 1.0) for a in (12, 20, 28, 36, 44)]
PENNANT_COLS = [np.array(c) for c in ((196, 40, 52), (240, 196, 80), (196, 40, 52), (240, 196, 80), (196, 40, 52))]

# crowd: dots along the seat steps (alpha range in view), bobbing on the beat
CROWD = []
for s in range(STEP_N):
    for a in np.arange(-34, 34, 0.42):
        if broken_k(a) > 0 and s >= int(round(STEP_N * (1 - broken_k(a)))):
            continue
        if rng.random() < 0.62:
            CROWD.append((a + rng.uniform(-0.12, 0.12), s, int(rng.integers(0, 5)), rng.random()))
CROWD_COLS = np.array([(150, 60, 70), (70, 90, 150), (220, 190, 140), (90, 130, 90), (230, 220, 210)], float)

# sand: raked bands + a few darker stones (procedural, by world XZ)
def sand_albedo(X, Z):
    r = np.hypot(X - ARENA_C[0], Z - ARENA_C[2])
    band = (np.floor(r * 2.2) % 2) * 0.05
    base = np.array([222, 184, 134], float)
    return base[None] * (0.95 + band)[:, None]


# far landscape seen through the collapse: two hazy ridges and a ruined aqueduct
from reelkit.geometry import ridge
FAR1 = ridge(-200.0, 8.0, -1.0, 30, 200.0, seed=7)
FAR2 = ridge(-120.0, 4.5, -2.5, 26, 130.0, seed=11)
AQUEDUCT = []
for k in range(16):
    x0 = -64 + k * 3.0
    z = -170.0
    AQUEDUCT.append(np.array([[x0, 0, z], [x0 + 0.8, 0, z], [x0 + 0.8, 7.0, z], [x0, 7.0, z]]))
AQUEDUCT.append(np.array([[-64, 7.0, z], [-64 + 16 * 3.0, 7.0, z], [-64 + 16 * 3.0, 8.4, z], [-64, 8.4, z]]))
CYPRESS = [(np.array([x, 0, zz]), h) for x, zz, h in ((-21, -62, 5.0), (-24, -66, 6.5), (-31, -74, 5.5), (-16, -58, 4.0))]

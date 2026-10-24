"""Static scene for BALLFIGHT: a low-poly boxing ring floating in a dark arena, seen from a 3/4 top-down camera.

World: y up, canvas at y = 0. The fight runs in the ring's own frame (sx, sz) in [-A, A]^2 (fight.py); the ring is
turned 45 deg so the red corner sits on the screen's left and the blue corner on its right:
    world X = (sx - sz) / sqrt 2,  world Z = (sx + sz) / sqrt 2      (camera on +Z, looking toward -Z)
Red corner (sx, sz) = (-A, +A) -> screen left; blue (+A, -A) -> screen right; neutral corners front / back.
Light: one hard spotlight over the ring (the key: its can hangs from the truss at the top of frame, with a visible
beam), red and blue corner lights on the corner posts, warm spill on the crowd; the crowd's phones twinkle."""
import math
import numpy as np
from reelkit.geometry import box, prism, face

rng = np.random.default_rng(27)
S2 = math.sqrt(2.0)
A = 3.0                          # ropes (= the walls in fight.py)
AP = A + 0.62                    # apron edge (canvas)
POST = A + 0.24                  # corner posts
POST_H = 1.42
ROPE_Y = (0.42, 0.80, 1.18)
FLOOR_Y = -1.15                  # arena floor
R_BALL = 0.6
RING_C = np.array([0.0, 0.0, 0.0])
SUBJECT = np.array([0.0, 0.4, 0.0])

SPOT = np.array([0.0, 9.5, -1.4])          # the key light: a hard spot over the ring (can on the truss)
SPOT_AIM = np.array([0.0, 0.0, 0.0])
SPOT_COS = (math.cos(math.radians(27)), math.cos(math.radians(19)))   # cone edge (soft from 19 to 27 deg)


def to_world(sx, sz, y=0.0):
    sx, sz = np.asarray(sx, float), np.asarray(sz, float)
    return np.stack([(sx - sz) / S2, np.broadcast_to(y, np.shape(sx)).astype(float), (sx + sz) / S2], -1)


def to_ring(X, Z):
    X, Z = np.asarray(X, float), np.asarray(Z, float)
    return (X + Z) / S2, (Z - X) / S2


CORNERS = {                                 # ring frame
    'red': np.array([-POST, POST]), 'blue': np.array([POST, -POST]),
    'front': np.array([POST, POST]), 'back': np.array([-POST, -POST])}
CORNER_COL = {'red': np.array([228, 52, 52]), 'blue': np.array([52, 112, 236]),
              'front': np.array([236, 232, 222]), 'back': np.array([236, 232, 222])}
RED_LIGHT = to_world(*CORNERS['red'], POST_H + 0.35)
BLUE_LIGHT = to_world(*CORNERS['blue'], POST_H + 0.35)

# ---------------------------------------------------------------- canvas + platform
CANVAS = np.array([104, 98, 142])          # slate-blue canvas; the spot pool lifts it to a bright lilac-grey
CANVAS_RIM = np.array([70, 58, 104])        # apron band outside the ropes
SKIRT = np.array([34, 30, 58])
SKIRT_BAND = np.array([196, 54, 70])
ARENA_FLOOR = np.array([46, 40, 62])


def canvas_albedo(sx, sz):
    """per-pixel canvas colour in the ring frame: canvas, a faint centre ring + star, the apron band"""
    r = np.hypot(sx, sz)
    col = np.tile(CANVAS.astype(float), (len(sx), 1))
    m = (np.abs(r - 1.55) < 0.06) | (np.abs(r - 1.32) < 0.025)
    col[m] = col[m] * 0.82 + np.array([220, 200, 255]) * 0.18
    # centre star (5 points), very faint
    ang = np.arctan2(sz, sx)
    star = r < 0.42 + 0.30 * np.abs(np.cos(2.5 * (ang + 0.3)))
    col[star & (r < 0.9)] = col[star & (r < 0.9)] * 0.9 + np.array([230, 210, 255]) * 0.1
    out = (np.abs(sx) > A) | (np.abs(sz) > A)
    col[out] = CANVAS_RIM
    edge = (np.maximum(np.abs(sx), np.abs(sz)) > AP - 0.08)
    col[edge] = np.array([224, 214, 236])
    return col


def platform_faces():
    """the ring platform's skirt (4 sides, seen from above only the front two) with a red band"""
    out = []
    c = [(AP, AP), (AP, -AP), (-AP, -AP), (-AP, AP)]
    for k in range(4):
        a, b = c[k], c[(k + 1) % 4]
        for y0, y1, col in ((FLOOR_Y, -0.32, SKIRT), (-0.32, -0.18, SKIRT_BAND), (-0.18, 0.0, SKIRT)):
            Q = np.array([to_world(a[0], a[1], y0), to_world(b[0], b[1], y0), to_world(b[0], b[1], y1), to_world(a[0], a[1], y1)])
            mid = to_world((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
            n = mid / (np.linalg.norm(mid) + 1e-9)
            n[1] = 0
            out.append((Q, n / np.linalg.norm(n), col))
    return out


SKIRT_FACES = platform_faces()


def post_faces(name):
    """a padded corner post: steel pole + three turnbuckle cushions in the corner's colour"""
    sx, sz = CORNERS[name]
    w = to_world(sx, sz)
    out = []
    for Q, n in prism(w[0], w[2], 0.085, 0.0, POST_H, n=6):
        out.append((Q, n, np.array([190, 190, 204])))
    for y in ROPE_Y:
        for Q, n in prism(w[0], w[2], 0.16, y - 0.11, y + 0.11, n=6):
            out.append((Q, n, CORNER_COL[name]))
    for Q, n in prism(w[0], w[2], 0.11, POST_H, POST_H + 0.06, n=6):
        out.append((Q, n, np.array([220, 220, 230])))
    return out


POSTS = {k: post_faces(k) for k in CORNERS}
ROPE_SIDES = [('red', 'front'), ('front', 'blue'), ('blue', 'back'), ('back', 'red')]   # 4 sides of the ring
ROPE_COL = [np.array([236, 230, 222]), np.array([228, 72, 78]), np.array([236, 230, 222])]

# ---------------------------------------------------------------- crowd (stands rising away from the ring)
BARRIER_R = 5.55                 # LED boards around the ring floor
BARRIER_H = 0.72
STAND_R0, ROW_W, ROW_H, N_ROWS = 6.2, 0.82, 0.40, 10
CROWD = []          # (feet position on its tier, shirt colour index, phase, row, height)
CROWD_COLS = np.array([[86, 70, 118], [118, 74, 104], [70, 92, 132], [124, 98, 84], [96, 70, 82], [74, 80, 108],
                       [132, 112, 70], [64, 104, 98]], float)
for row in range(N_ROWS):
    rr = STAND_R0 + row * ROW_W + 0.42
    yy = FLOOR_Y + (row + 1) * ROW_H
    n = int(2 * math.pi * rr / 0.40)
    for k in range(n):
        a = 2 * math.pi * (k + 0.5 * (row % 2)) / n + rng.uniform(-0.035, 0.035)
        if rng.random() < 0.10:
            continue
        p = np.array([rr * math.sin(a), yy, -rr * math.cos(a)])
        CROWD.append((p, int(rng.integers(0, len(CROWD_COLS))), float(rng.random()), row, float(rng.uniform(0.92, 1.1))))
STANDS = []         # stepped tiers under the crowd (dark)
for row in range(N_ROWS):
    r0, r1 = STAND_R0 + row * ROW_W, STAND_R0 + (row + 1) * ROW_W
    y = FLOOR_Y + row * ROW_H
    for k in range(56):
        a0, a1 = 2 * math.pi * k / 56, 2 * math.pi * (k + 1) / 56
        P = lambda r, a, yv: np.array([r * math.sin(a), yv, -r * math.cos(a)])
        top = np.array([P(r0, a0, y + ROW_H), P(r0, a1, y + ROW_H), P(r1, a1, y + ROW_H), P(r1, a0, y + ROW_H)])
        riser = np.array([P(r0, a0, y), P(r0, a1, y), P(r0, a1, y + ROW_H), P(r0, a0, y + ROW_H)])
        am = (a0 + a1) / 2
        STANDS.append((top, np.array([0, 1.0, 0]), np.array([54, 46, 74]), am))
        STANDS.append((riser, -np.array([math.sin(am), 0, -math.cos(am)]), np.array([40, 34, 58]), am))
BARRIER = []        # (quad, normal, angle) both faces of the LED boards
for k in range(64):
    a0, a1 = 2 * math.pi * k / 64, 2 * math.pi * (k + 1) / 64
    P = lambda a, yv: np.array([BARRIER_R * math.sin(a), yv, -BARRIER_R * math.cos(a)])
    Q = np.array([P(a0, FLOOR_Y), P(a1, FLOOR_Y), P(a1, FLOOR_Y + BARRIER_H), P(a0, FLOOR_Y + BARRIER_H)])
    am = (a0 + a1) / 2
    n_out = np.array([math.sin(am), 0, -math.cos(am)])
    BARRIER.append((Q, n_out, am))
    BARRIER.append((Q[::-1].copy(), -n_out, am))

# ---------------------------------------------------------------- truss + lamps (top of frame)
TRUSS_Y = 8.2
TRUSS = []
for (x0, z0), (x1, z1) in [((-5.5, -4.5), (5.5, -4.5)), ((-5.5, -4.5), (-5.5, 3.0)), ((5.5, -4.5), (5.5, 3.0))]:
    TRUSS.append((np.array([x0, TRUSS_Y, z0]), np.array([x1, TRUSS_Y, z1])))
CANS = [np.array([-3.6, TRUSS_Y - 0.25, -4.5]), np.array([3.6, TRUSS_Y - 0.25, -4.5]),
        np.array([-5.5, TRUSS_Y - 0.25, -1.2]), np.array([5.5, TRUSS_Y - 0.25, -1.2])]

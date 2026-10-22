"""The domino layout + schedule: one table of boxes (dominoes and chime bars), each with a position,
fall direction, size and strike time t0. No physics sim: the fall-start time of every domino comes from
the 120 BPM grid (timeline.py), and each one rotates about its front-bottom edge until it rests on the
next. render.py draws the table, audio.py turns every t0 into a click (and every bar into a chime).

    python layout.py            -> layout.npz + test/map.png (top-down map) + counts and checks

World: metres, y up, x right, z toward the default camera (screen up = -z). Dominoes 5 x 2.5 x 0.8 cm."""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import timeline as T

DH, DW, DT = 0.05, 0.025, 0.008          # domino height, width, thickness
TARGET = 10000
rng = np.random.default_rng(1700)


# ---------------------------------------------------------------- table
class Table:
    KEYS = ['cx', 'cz', 'y0', 'fx', 'fz', 'dw', 'dt', 'dh', 't0', 'nxt', 'lane', 'sec', 'kind', 'pitch',
            'cr', 'cg', 'cb', 'br', 'bg', 'bb', 'pips']

    def __init__(self):
        self.c = {k: [] for k in self.KEYS}
        self.n = 0
        self.lane_n = 0

    def new_lane(self):
        self.lane_n += 1
        return self.lane_n

    def add(self, x, z, f, t0, col, back=None, s=1.0, y0=0.0, lane=0, sec=0, kind=0, pitch=-1, dims=None, pips=0):
        f = np.asarray(f, float)
        f = f / (np.linalg.norm(f) + 1e-12)
        dw, dt, dh = dims if dims is not None else (DW * s, DT * s, DH * s)
        back = col if back is None else back
        for k, v in zip(self.KEYS, [x, z, y0, f[0], f[1], dw, dt, dh, t0, -1, lane, sec, kind, pitch,
                                     col[0], col[1], col[2], back[0], back[1], back[2], pips]):
            self.c[k].append(v)
        self.n += 1
        return self.n - 1

    def link(self, i, j):
        self.c['nxt'][i] = j

    def arrays(self):
        return {k: np.array(v) for k, v in self.c.items()}


TB = Table()
SEC = dict(trunk=0, fan=1, funnel=2, bundle=3, bell=4, chord=5, feeder=6, field=7, spiral=8, stair=9, giant=10,
           frame=11, bar=12, final=13)


# ---------------------------------------------------------------- path helpers
def hermite(p0, t0, p1, t1, n=60, k=1.0):
    p0, p1 = np.asarray(p0, float), np.asarray(p1, float)
    L = np.linalg.norm(p1 - p0) * k
    t0 = np.asarray(t0, float) / (np.linalg.norm(t0) + 1e-12) * L
    t1 = np.asarray(t1, float) / (np.linalg.norm(t1) + 1e-12) * L
    u = np.linspace(0, 1, n)[:, None]
    h00, h10, h01, h11 = 2 * u ** 3 - 3 * u ** 2 + 1, u ** 3 - 2 * u ** 2 + u, -2 * u ** 3 + 3 * u ** 2, u ** 3 - u ** 2
    return h00 * p0 + h10 * t0 + h01 * p1 + h11 * t1


def chain(*parts):
    out = [parts[0]]
    for p in parts[1:]:
        out.append(p[1:] if np.allclose(p[0], out[-1][-1]) else p)
    return np.concatenate(out)


def arclen(P):
    return np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(P, axis=0), axis=1))])


def sample(P, s):
    """points + unit tangents at arc lengths s along polyline P"""
    S = arclen(P)
    s = np.clip(np.asarray(s, float), 0, S[-1])
    x = np.interp(s, S, P[:, 0])
    z = np.interp(s, S, P[:, 1])
    e = 1e-3
    xa, za = np.interp(np.clip(s - e, 0, S[-1]), S, P[:, 0]), np.interp(np.clip(s - e, 0, S[-1]), S, P[:, 1])
    xb, zb = np.interp(np.clip(s + e, 0, S[-1]), S, P[:, 0]), np.interp(np.clip(s + e, 0, S[-1]), S, P[:, 1])
    tg = np.stack([xb - xa, zb - za], 1)
    tg /= np.linalg.norm(tg, axis=1, keepdims=True) + 1e-12
    return np.stack([x, z], 1), tg


def left_of(h):
    """left perpendicular of a heading on the top-down map (x right, z down)"""
    h = np.asarray(h, float)
    return np.array([h[..., 1], -h[..., 0]]).T if h.ndim > 1 else np.array([h[1], -h[0]])


def offset_path(P, d):
    """offset polyline P by d to its LEFT (negative = right)"""
    S = arclen(P)
    _, tg = sample(P, S)
    return P + left_of(tg) * d


def place(P, times, col, sec, lane=None, back=None, s_pos=None, colfn=None, backfn=None, **kw):
    """N dominoes along path P at strike times `times` (equal spacing unless s_pos given); linked in order"""
    times = np.asarray(times, float)
    N = len(times)
    L = arclen(P)[-1]
    if s_pos is None:
        s_pos = (np.arange(N) + 0.5) * L / N
    pts, tg = sample(P, s_pos)
    lane = TB.new_lane() if lane is None else lane
    idx = []
    for k in range(N):
        c = colfn(k, N) if colfn else col
        b = backfn(k, N) if backfn else (back if back is not None else c)
        i = TB.add(pts[k, 0], pts[k, 1], tg[k], times[k], c, b, lane=lane, sec=sec, **kw)
        if idx:
            TB.link(idx[-1], i)
        idx.append(i)
    return idx


def hsv(h, s=0.8, v=1.0):
    h = h % 1.0
    i = int(h * 6)
    f = h * 6 - i
    p, q, t = v * (1 - s), v * (1 - f * s), v * (1 - (1 - f) * s)
    r, g, b = [(v, t, p), (q, v, p), (p, v, t), (p, q, v), (t, p, v), (v, p, q)][i % 6]
    return (int(r * 255), int(g * 255), int(b * 255))


def bar_len(m):
    """glockenspiel bar length (m) by pitch: low = long"""
    return float(np.clip(0.10 - (m - 66) * 0.0021, 0.042, 0.11))


PITCH_COL = [(255, 92, 92), (255, 150, 70), (255, 206, 70), (180, 230, 80), (90, 220, 120), (70, 210, 200),
             (80, 170, 255), (120, 120, 255), (180, 110, 255), (240, 100, 220), (255, 110, 160), (255, 120, 120)]


def add_bar(i_dom, m, side=-1.0, gap=0.031):
    """a chime bar beside domino i (on its right when side = -1), struck when the domino falls"""
    c = TB.c
    x, z, fx, fz = c['cx'][i_dom], c['cz'][i_dom], c['fx'][i_dom], c['fz'][i_dom]
    lx, lz = fz, -fx
    L = bar_len(m)
    bx, bz = x + lx * gap * side + fx * 0.01, z + lz * gap * side + fz * 0.01
    j = TB.add(bx, bz, (fx, fz), c['t0'][i_dom], PITCH_COL[int(m) % 12], kind=1, sec=SEC['bar'], pitch=int(m),
               dims=(0.017, L, 0.006), y0=0.004, lane=0)
    c['pitch'][i_dom] = int(m)
    return j


def grid_times(t_start, n, step):
    return t_start + np.arange(n) * step


# ================================================================= 1. trunk + fan (0-4 s)
ZR = 5.35                                   # domino #1 (the paw taps it)
ROOT = np.array([0.0, ZR])
UP = np.array([0.0, -1.0])
trunk = place(np.array([[0, ZR + 0.02], [0, ZR - 0.30]]), grid_times(0.0, 8, T.S16), (255, 255, 255), SEC['trunk'],
              colfn=lambda k, N: (250, 246, 236))
O = np.array([0.0, ZR - 0.32])
N_LEAF = 64
SPREAD = 63.0
LEAF_PHI = np.radians(-SPREAD + (np.arange(N_LEAF) + 0.5) * 2 * SPREAD / N_LEAF)
R_LEV = [0.0, 0.12, 0.28, 0.48, 0.72, 1.0, 1.32]
d_phi = lambda p: np.array([math.sin(p), -math.cos(p)])
fan_col = lambda phi: hsv(0.98 - 0.80 * (phi + math.radians(SPREAD)) / math.radians(2 * SPREAD), 0.78, 1.0)


class Node:
    def __init__(self, level, j0, j1, pos, t, head):
        self.level, self.j0, self.j1, self.pos, self.t, self.head = level, j0, j1, np.asarray(pos, float), t, np.asarray(head, float)
        self.last = None          # index of the last domino of the edge arriving at this node
        self.first = []           # first dominoes of edges leaving this node


fan_leaves = [None] * N_LEAF


def build_fan(node, last_idx):
    if node.level == 6:
        fan_leaves[node.j0] = (node, last_idx)
        return
    half = (node.j0 + node.j1) // 2
    for side, (a, b) in ((-1, (node.j0, half)), (1, (half, node.j1))):
        phi = LEAF_PHI[a:b].mean()
        lev = node.level + 1
        cp = O + R_LEV[lev] * d_phi(phi)
        eps = float(np.clip(rng.normal(0, 0.28), -0.42, 0.6))
        tc = node.t + 0.5 * (1 + eps)
        start = node.pos + left_of(node.head) * (-side) * 0.0145
        P = hermite(start, node.head * 0.7 + d_phi(phi) * 0.3, cp, d_phi(phi), n=40, k=0.9)
        L = arclen(P)[-1]
        N = max(2, int(round(L / 0.041)))
        times = node.t + (tc - node.t) * np.arange(N) / N
        idx = place(P, times, None, SEC['fan'], colfn=lambda k, NN, ph=phi: fan_col(ph))
        if last_idx is not None:
            TB.link(last_idx, idx[0])
        child = Node(lev, a, b, cp, tc, d_phi(phi))
        build_fan(child, idx[-1])


root_node = Node(0, 0, N_LEAF, O, T.T_FAN, UP)
build_fan(root_node, trunk[-1])

# ================================================================= 2. funnel (4-8 s): pairs merge, clicks line up
R_NECK = 2.90                          # the neck (4 bundle lanes) sits this far above O
ZB = O[1] - R_NECK                     # bundle start (4 lanes) reached at 7.0 s
XB = -0.02
BUND_GAP = 0.055
BUND_X = XB + (np.arange(4) - 1.5) * BUND_GAP
F_T = [4.75, 5.5, 6.25, 7.0]           # merge times (nominal) per funnel level
F_JIT = [0.07, 0.035, 0.012, 0.0]
R_M = [1.68, 2.08, 2.48, R_NECK]       # merge radii (from O), one row per level
# r * h(r) = lateral "spread" of the meridians: bulges a little past the leaf ring, then halves per merge row
from scipy.interpolate import PchipInterpolator
_RH = PchipInterpolator([R_LEV[6], 1.48, R_M[0], R_M[1], R_M[2], R_M[3]], [R_LEV[6], 1.43, 1.40, 0.70, 0.35, 0.19])
h_of = lambda r: _RH(np.clip(r, R_LEV[6], R_NECK)) / np.clip(r, R_LEV[6], R_NECK)
DELTA = 0.14                           # length of the converge-and-merge transition


def funnel_lane_path(lev, j0, r0, r1):
    """lane for leaves j0 .. j0+2^(lev-1) between radii r0 and r1 (lev 1 = leaf edges), ending side by side
    with its merge partner just before the next merge row"""
    size = 2 ** (lev - 1)
    phi_c = LEAF_PHI[j0:j0 + size].mean()
    psize = 2 * size
    pj0 = (j0 // psize) * psize
    phi_p = LEAF_PHI[pj0:pj0 + psize].mean()
    side = -1.0 if j0 == pj0 else 1.0
    r = np.linspace(r0, r1, 120)
    w = np.clip((r - (r1 - DELTA)) / DELTA, 0, 1)
    w = w * w * (3 - 2 * w)
    phi = (phi_c + (phi_p - phi_c) * w) * h_of(r)
    x = O[0] + r * np.sin(phi)
    z = O[1] - r * np.cos(phi)
    # last stretch into the neck: slide onto the bundle lane positions
    if lev == 4:
        g = np.clip((r - R_M[2]) / (R_NECK - R_M[2]), 0, 1)
        g = g * g * (3 - 2 * g)
        xt = BUND_X[j0 // 16] + side * 0.0145
        x = x * (1 - g) + xt * g
        z = z * (1 - g) + (O[1] - r) * g
        return np.stack([x, z], 1)
    # end side by side with the partner: offset across the PARENT lane's own direction (outer lanes curve
    # sideways, so "across the radius" would put the pair almost on top of each other)
    phi_par = phi_p * h_of(r)
    pp = np.stack([O[0] + r * np.sin(phi_par), O[1] - r * np.cos(phi_par)], 1)
    tg = np.gradient(pp, axis=0)
    tg /= np.linalg.norm(tg, axis=1, keepdims=True) + 1e-12
    lp = left_of(tg)
    P = np.stack([x, z], 1) - side * 0.0145 * w[:, None] * lp
    return P


funnel_edges = []        # (lev, j0, path, t_start, t_arrive, last_idx_in) - lengths decided later for the exact count


def plan_funnel():
    arrivals = {}
    starts = {}
    for lev in range(1, 5):
        size = 2 ** (lev - 1)
        r0 = R_LEV[6] if lev == 1 else R_M[lev - 2]
        r1 = R_M[lev - 1]
        for j0 in range(0, N_LEAF, size):
            if lev == 1:
                node, last = fan_leaves[j0]
                ts = node.t
            else:
                last = None
                ts = min(arrivals[(lev - 1, j0)])
            P = funnel_lane_path(lev, j0, r0, r1)
            ta = F_T[lev - 1] + float(np.clip(rng.normal(0, F_JIT[lev - 1]), -0.15, 0.15))
            if lev == 4:
                ta = F_T[3]
            funnel_edges.append([lev, j0, P, ts, ta, last])
            arrivals.setdefault((lev, (j0 // (2 * size)) * (2 * size)), []).append(ta)
    return arrivals


funnel_arrivals = plan_funnel()

# ================================================================= 3. bundle + bell lane + chord lanes (7-20 s)
Z_FB = ZB - 2.75                       # field bottom edge
COLS, ROWS = 64, 62
PX, PZ = 0.030, 0.035                  # field column / row pitch
FIELD_X0 = -(COLS - 1) / 2 * PX
Z_FT = Z_FB - ROWS * PZ

# 7-8 s: four lanes straight up on the 16th grid
bund_lanes = []
for m in range(4):
    P = np.array([[BUND_X[m], ZB], [BUND_X[m], ZB - 0.32]])
    bund_lanes.append(place(P, grid_times(T.T_LOCK, 8, T.S16), None, SEC['bundle'],
                            colfn=lambda k, N, mm=m: fan_col(LEAF_PHI[mm * 16:(mm + 1) * 16].mean())))
ZP = ZB - 0.32                          # peel point

# bundle (lanes 0-2): up and left in an S to the field's bottom-left corner, 96 dominoes each (8-20 s)
FEED_Z = Z_FB + 0.045
LF0 = np.array([FIELD_X0 - 0.10, FEED_Z + 0.02])
bc = chain(hermite([BUND_X[1], ZP], UP, [-0.55, ZP - 1.05], [-0.6, -1], n=80, k=1.1),
           hermite([-0.55, ZP - 1.05], [-0.6, -1], [-0.62, Z_FB + 0.75], [0.25, -1], n=80, k=1.2),
           hermite([-0.62, Z_FB + 0.75], [0.25, -1], LF0, [-0.15, -1], n=60, k=1.0))
bundle = []
for m, d in zip(range(3), (0.045, 0.0, -0.045)):
    P = offset_path(bc, d)
    idx = place(P, grid_times(T.T_BELL, 96, T.S16), None, SEC['bundle'],
                colfn=lambda k, N, mm=m: fan_col(LEAF_PHI[mm * 16:(mm + 1) * 16].mean()))
    TB.link(bund_lanes[m][-1], idx[0])
    bundle.append(idx)

# bell lane (lane 3): peels right at 8 s, 48 dominoes, every 4th rings the melody
GOLD = (255, 204, 70)
SPLIT = np.array([0.52, ZP - 1.55])
bellP = chain(hermite([BUND_X[3], ZP], UP, [0.42, ZP - 0.6], [0.5, -1], n=60, k=1.1),
              hermite([0.42, ZP - 0.6], [0.5, -1], SPLIT, [-0.05, -1], n=60, k=1.1))
bell = place(bellP, grid_times(T.T_BELL, 48, T.S16), GOLD, SEC['bell'])
TB.link(bund_lanes[3][-1], bell[0])
for k, i in enumerate(bell):
    if k % 4 == 0:
        add_bar(i, T.melody_at(TB.c['t0'][i]), side=-1)

# chord lanes (14-20 s): 4 lanes 6 cm apart; lane 0 keeps the melody, lanes 1-3 arpeggiate on the off 16ths
CHORD_COL = [GOLD, (255, 120, 190), (110, 230, 190), (120, 190, 255)]
RF0 = np.array([-FIELD_X0 + 0.10, FEED_Z + 0.02])
cc = chain(hermite(SPLIT, [-0.05, -1], [0.78, Z_FB + 0.9], [0.35, -1], n=80, k=1.1),
           hermite([0.78, Z_FB + 0.9], [0.35, -1], RF0, [0.1, -1], n=80, k=1.1))
chords = []
for m in range(4):
    d = (1.5 - m) * 0.06
    P = offset_path(cc, d)
    idx = place(P, grid_times(T.T_CHORD, 48, T.S16), CHORD_COL[m], SEC['chord'])
    TB.link(bell[-1], idx[0])
    for k, i in enumerate(idx):
        if k % 4 == m:
            t = TB.c['t0'][i]
            add_bar(i, T.melody_at(t - m * T.S16) if m == 0 else T.arp_at(t, m), side=-1)
    chords.append(idx)

# ================================================================= 4. the field (20-26 s): 4,096 dominoes, chevron sweep
CREAM = (244, 236, 218)


def note_art():
    """64 x 64 pixel music note (two beamed eighth notes) for the painted back faces. row 0 = bottom."""
    W_, H_ = COLS, ROWS
    img = np.zeros((H_, W_, 3))
    yy, xx = np.mgrid[0:H_, 0:W_].astype(float)
    # physical aspect: a cell is 3.0 wide x 3.5 tall -> work in metres
    X, Y = (xx + 0.5) * PX, (yy + 0.5) * PZ
    cx, cy = W_ * PX / 2, H_ * PZ / 2
    r = np.hypot((X - cx) / (W_ * PX), (Y - cy) / (H_ * PZ))
    bg = np.array([30, 54, 128]) * (1 - 0.55 * r[..., None]) + np.array([60, 30, 110]) * (0.55 * r[..., None])
    img[:] = bg
    # rays of light behind the note
    ang = np.arctan2(Y - cy, X - cx)
    rays = (np.sin(ang * 8) > 0.55) & (r > 0.12)
    img[rays] = img[rays] * 0.78 + np.array([90, 130, 220]) * 0.22

    def ell(px, py, a, b, rot):
        c, s = math.cos(rot), math.sin(rot)
        u = ((X - px) * c + (Y - py) * s) / a
        v = (-(X - px) * s + (Y - py) * c) / b
        return u * u + v * v <= 1.0
    h1 = ell(0.62, 0.62, 0.215, 0.15, 0.42)
    h2 = ell(1.30, 0.80, 0.215, 0.15, 0.42)
    st1 = (X > 0.79) & (X < 0.86) & (Y > 0.66) & (Y < 1.86)
    st2 = (X > 1.47) & (X < 1.54) & (Y > 0.84) & (Y < 2.04)
    beam_lo = lambda x: 1.62 + (x - 0.79) * (0.18 / 0.75)
    beam1 = (X > 0.79) & (X < 1.54) & (Y > beam_lo(X)) & (Y < beam_lo(X) + 0.24)
    beam2 = (X > 0.79) & (X < 1.54) & (Y > beam_lo(X) - 0.36) & (Y < beam_lo(X) - 0.22)
    note = h1 | h2 | st1 | st2 | beam1 | beam2
    from scipy.ndimage import binary_dilation
    outline = binary_dilation(note, iterations=2) & ~note
    glow = binary_dilation(note, iterations=6) & ~binary_dilation(note, iterations=2)
    img[glow] = img[glow] * 0.6 + np.array([255, 170, 90]) * 0.4
    img[outline] = (24, 18, 48)
    shade = 0.5 + 0.5 * np.clip((Y - X * 0.4) / 2.2, 0, 1)
    gold = np.array([255, 196, 52])[None, None] * (0.75 + 0.25 * shade[..., None])
    img[note] = gold[note]
    hl = (ell(0.56, 0.68, 0.10, 0.045, 0.42) | ell(1.24, 0.86, 0.10, 0.045, 0.42) |
          ((X > 0.80) & (X < 0.82) & (Y > 0.75) & (Y < 1.55)) | ((X > 1.48) & (X < 1.50) & (Y > 0.93) & (Y < 1.75)))
    img[hl] = (255, 248, 210)
    # sparkle pixels
    rs = np.random.default_rng(9)
    for _ in range(26):
        i, j = rs.integers(2, H_ - 2), rs.integers(2, W_ - 2)
        if not (note[i, j] or outline[i, j] or glow[i, j]):
            img[i, j] = (255, 240, 170)
            for di, dj in ((0, 1), (1, 0), (0, -1), (-1, 0)):
                if rs.random() < 0.6:
                    img[i + di, j + dj] = img[i + di, j + dj] * 0.4 + np.array([255, 230, 150]) * 0.6
    # a pink border frame
    img[:2, :] = img[-2:, :] = (255, 110, 170)
    img[:, :2] = img[:, -2:] = (255, 110, 170)
    return np.clip(img, 0, 255)


ART = note_art()

# feeders along the bottom edge, from both corners inward (16th... 32nd grid), then the columns at 16/s
F_START = np.zeros(COLS)
for c in range(COLS):
    k = c if c < COLS // 2 else COLS - 1 - c
    F_START[c] = T.T_FIELD + k * T.S32
feed_left, feed_right = [], []
for c in range(COLS // 2):
    feed_left.append(TB.add(FIELD_X0 + c * PX, FEED_Z, (1, 0), F_START[c], CREAM, lane=900, sec=SEC['feeder']))
for c in range(COLS - 1, COLS // 2 - 1, -1):
    feed_right.append(TB.add(FIELD_X0 + c * PX, FEED_Z, (-1, 0), F_START[c], CREAM, lane=901, sec=SEC['feeder']))
for a, b in zip(feed_left, feed_left[1:]):
    TB.link(a, b)
for a, b in zip(feed_right, feed_right[1:]):
    TB.link(a, b)
for idx in bundle:
    TB.link(idx[-1], feed_left[0])
for idx in chords:
    TB.link(idx[-1], feed_right[0])
field_cols = []
for c in range(COLS):
    lane = TB.new_lane()
    col_idx = []
    for r in range(ROWS):
        x = FIELD_X0 + c * PX
        z = Z_FB - PZ / 2 - r * PZ
        art = tuple(int(v) for v in ART[r, c])
        i = TB.add(x, z, UP, F_START[c] + (r + 1) * T.S32, CREAM, art, lane=lane, sec=SEC['field'])
        if col_idx:
            TB.link(col_idx[-1], i)
        col_idx.append(i)
    field_cols.append(col_idx)
    fd = feed_left[c] if c < COLS // 2 else feed_right[COLS - 1 - c]
    TB.c['nxt'][fd] = TB.c['nxt'][fd] if TB.c['nxt'][fd] >= 0 else col_idx[0]

# ================================================================= 5. spiral (26-35.5 s): 2 lanes wind inward
R_S = 0.44
PITCH = 0.11
C_S = np.array([0.0, Z_FT - 0.27 - R_S])
R_END = 0.20                            # inner lane A ends here (centreline); lane B gets one more slow arc
POST_R = 0.065                          # round post under the plank = the "ledge"
BAND = 0.025                            # lanes sit +-2.5 cm either side of the spiral centreline
A0 = math.radians(135)                  # spiral starts up-left of the field's top centre; angle increases ->
TURNS = (R_S - R_END) / PITCH
A1 = A0 + TURNS * 2 * math.pi
r_main = lambda a: R_S - (a - A0) / (2 * math.pi) * PITCH


def spiral_pts(off, a0, a1, r_of, n=900):
    a = np.linspace(a0, a1, n)
    r = r_of(a) + off
    return np.stack([C_S[0] + r * np.cos(a), C_S[1] + r * np.sin(a)], 1), a


def hook(p_end, p_prev):
    d = (p_end - C_S) / np.linalg.norm(p_end - C_S)
    return hermite(p_end, p_end - p_prev, C_S + d * (POST_R + 0.013), -d, n=24, k=0.8)


# entry: one centreline from between columns 31/32 curving left into the spiral; the lanes are offset curves
# (offset 1.5 cm at the field -> 2.5 cm in the spiral) so they never cross
ent_c = hermite([0.0, Z_FT - 0.03], UP, C_S + r_main(A0) * np.array([math.cos(A0), math.sin(A0)]),
                [-math.sin(A0), math.cos(A0)], n=60, k=1.0)
_, tg_e = sample(ent_c, arclen(ent_c))
grow = np.linspace(0.015, BAND, len(ent_c))
entB = ent_c + left_of(tg_e) * grow[:, None]          # left of travel = outer after the left turn (column 31)
entA = ent_c - left_of(tg_e) * grow[:, None]          # column 32 -> inner lane A
spA, aA = spiral_pts(-BAND, A0, A1, r_main)
PA = chain(entA, spA, hook(spA[-1], spA[-2]))
tA = np.concatenate([grid_times(26.0, 32, 1 / 16), grid_times(28.0, 48, 1 / 24), grid_times(30.0, 64, 1 / 32)])
spcA = np.concatenate([np.full(32, 4.3), np.full(48, 2.9), np.full(64, 2.2)])
sA = np.cumsum(spcA) - spcA[0] / 2
sA = sA / sA[-1] * (arclen(PA)[-1] - 0.012)
SPIRAL_COL = lambda k, N: hsv(0.83 + 0.22 * k / max(1, N - 1), 0.72, 1.0)
spiralA = place(PA, tA, None, SEC['spiral'], s_pos=sA, colfn=SPIRAL_COL)
# outer lane B: matched to A's angles while A runs (fronts side by side), then a 300-degree slow arc
EXTRA = math.radians(300)
r_B = lambda a: np.where(a <= A1, r_main(a), R_END - (a - A1) / EXTRA * 0.095)
spB, aB = spiral_pts(BAND, A0, A1 + EXTRA, r_B, n=1600)
PB = chain(entB, spB, hook(spB[-1], spB[-2]))
SB_ent = arclen(entB)[-1]
SA_ent = arclen(entA)[-1]
SB_cum = arclen(spB)
ptsA = np.array([[TB.c['cx'][i], TB.c['cz'][i]] for i in spiralA])
angA = np.unwrap(np.arctan2(ptsA[:, 1] - C_S[1], ptsA[:, 0] - C_S[0]))
angA = angA + (A0 - angA[np.argmax(sA > SA_ent)]) if (sA > SA_ent).any() else angA
sB = []
for k in range(len(spiralA)):
    if sA[k] <= SA_ent:
        sB.append(sA[k] / SA_ent * SB_ent)
    else:
        j = min(np.searchsorted(aB, min(angA[k], A1)), len(aB) - 1)
        sB.append(SB_ent + SB_cum[j])
sB = np.maximum.accumulate(np.array(sB))
tB_tail = np.concatenate([grid_times(32.0, 16, 1 / 8), grid_times(34.0, 6, 1 / 4)])
nt = len(tB_tail)
s_tail = sB[-1] + np.arange(1, nt + 1) * (arclen(PB)[-1] - 0.012 - sB[-1]) / nt
spiralB = place(PB, np.concatenate([tA, tB_tail]), None, SEC['spiral'], s_pos=np.concatenate([sB, s_tail]),
                colfn=lambda k, N: hsv(0.55 + 0.25 * k / max(1, N - 1), 0.7, 1.0))
TB.link(field_cols[31][-1], spiralB[0])
TB.link(field_cols[32][-1], spiralA[0])
T_HOLD_A = TB.c['t0'][spiralA[-1]]
T_HOLD_B = TB.c['t0'][spiralB[-1]]

# ================================================================= 6. holdout + plank + growing staircase (32-40 s)
PLANK_Y = 0.095
STEP_H = [DH * 1.5 ** k for k in range(1, 9)]             # 7.5 cm ... 1.28 m
pos = [0.0]
for k in range(7):
    pos.append(pos[-1] + 0.6 * STEP_H[k])
STAIR = []
pip_counts = [0, 2, 3, 4, 5, 6, 6, 6]
for k in range(8):
    s = 1.5 ** (k + 1)
    z = C_S[1] - pos[k]
    on_plank = pos[k] + DT * s < 0.47
    t0 = T.T_TIP if k == 0 else T.STAIR_CONTACTS[k - 1] if k <= 6 else 39.5
    col = (238, 70, 58) if k == 0 else (246, 240, 226)
    i = TB.add(C_S[0], z, UP, t0, col, col, s=s, y0=PLANK_Y if on_plank else 0.0, lane=950, sec=SEC['giant'] if k == 7 else SEC['stair'],
               pips=pip_counts[k] if k else 1)
    if STAIR:
        TB.link(STAIR[-1], i)
    STAIR.append(i)
HOLDOUT = STAIR[0]
GIANT = STAIR[-1]
PLANK = dict(z0=C_S[1] + 0.07, z1=C_S[1] - 0.47, w=0.10, y=PLANK_Y)
G_S = 1.5 ** 8
Z_LAND = C_S[1] - pos[7] - DT * G_S / 2 - DH * G_S          # where the giant's top edge lands

# ================================================================= 7. the frame (40-52 s): 8 lanes in sync round everything
FR_GAP = 0.06
FRAME_TOP = Z_LAND + 0.12                 # inner lane of the top edge sits under the giant's top end


def frame_path(k, sgn, xf, zt, zb, rc=0.45):
    """left (sgn=-1) or right (+1) half of the rounded-rectangle frame, lane k (0 = inner)"""
    o = k * FR_GAP
    r = rc + o
    xin = 0.03
    pts = [np.array([sgn * xin, zt - o])]
    xc = sgn * (xf - rc)
    pts.append(np.array([xc, zt - o]))
    a = np.linspace(0, math.pi / 2, 24)
    for aa in a[1:]:
        pts.append(np.array([xc + sgn * r * math.sin(aa), (zt + rc) - r * math.cos(aa)]))
    pts.append(np.array([sgn * (xf + o), zb - rc]))
    for aa in a[1:]:
        pts.append(np.array([xc + sgn * r * math.cos(aa), (zb - rc) + r * math.sin(aa)]))
    pts.append(np.array([sgn * 0.026, zb + o]))
    P = np.array(pts)
    # densify straight runs
    out = [P[0]]
    for a_, b_ in zip(P, P[1:]):
        n = max(2, int(np.linalg.norm(b_ - a_) / 0.02))
        out.extend(list(a_ + (b_ - a_) * u for u in np.linspace(0, 1, n)[1:]))
    return np.array(out)


def layout_xmax():
    c = TB.arrays()
    sel = (c['kind'] == 0) | (c['kind'] == 1)
    return np.abs(c['cx'][sel]).max()


N_FR = int(round((T.T_END - T.T_DROP) * 32))
FRAME_SEED = None
FRAME_COL = [(255, 92, 150), (255, 250, 240), (84, 136, 255)]
N_FRAME_LANES = 3


def build_frame():
    xf = layout_xmax() + 0.42
    zb = ZR + 0.62
    zt = FRAME_TOP
    lanes = []
    for sgn in (-1, 1):
        for k in range(N_FRAME_LANES):
            P = frame_path(k, sgn, xf, zt, zb)
            idx = place(P, grid_times(T.T_DROP, N_FR, 1 / 32), FRAME_COL[k], SEC['frame'])
            TB.link(GIANT, idx[0]) if k == 0 and sgn < 0 else None
            for n, i in enumerate(idx):
                q = n % 16
                t = TB.c['t0'][i]
                m = None
                if k == 0 and q == 0:
                    m = T.melody_at(t)
                elif k == 1 and q in (4, 12):
                    m = T.arp_at(t, 1 if q == 4 else 3)
                elif k == 2 and q == 8:
                    m = T.arp_at(t, 2)
                if m is not None:
                    add_bar(i, m, side=-1 if sgn < 0 else 1, gap=0.03)
            lanes.append(idx)
    return lanes, xf, zb, zt


# ================================================================= exact count: size the funnel so the total is 10,000
def n_dominoes():
    k = np.array(TB.c['kind'])
    return int((k == 0).sum())


def place_funnel(budget):
    lens = np.array([arclen(e[2])[-1] for e in funnel_edges])
    k = budget / (lens / 0.04).sum()
    N = np.maximum(3, np.round(lens / 0.04 * k)).astype(int)
    diff = budget - N.sum()
    order = np.argsort(-lens)
    i = 0
    while diff != 0:
        N[order[i % len(order)]] += int(np.sign(diff))
        diff -= int(np.sign(diff))
        i += 1
    first_of = {}
    last_of = {}
    for e, n in zip(funnel_edges, N):
        lev, j0, P, ts, ta, last_in = e
        times = ts + (ta - ts) * np.arange(n) / n
        phi = LEAF_PHI[j0:j0 + 2 ** (lev - 1)].mean()
        mcol = fan_col(phi)
        idx = place(P, times, None, SEC['funnel'], colfn=lambda kk, NN, c=mcol: c)
        if last_in is not None:
            TB.link(last_in, idx[0])
        first_of[(lev, j0)] = idx[0]
        last_of[(lev, j0)] = idx[-1]
    # link merges: the children's last dominoes lean on the parent edge's first domino
    for e in funnel_edges:
        lev, j0 = e[0], e[1]
        size = 2 ** (lev - 1)
        par_j0 = (j0 // (2 * size)) * (2 * size)
        if lev < 4:
            TB.link(last_of[(lev, j0)], first_of[(lev + 1, par_j0)])
        else:
            m = j0 // 16
            TB.link(last_of[(lev, j0)], bund_lanes[m][0])
    sp = lens / N
    if __name__ == '__main__':
        print(f'funnel spacing: min {sp.min()*100:.1f} cm  median {np.median(sp)*100:.1f}  max {sp.max()*100:.1f}  '
              f'(per level max: {[round(float(sp[[e[0] == l for e in funnel_edges]].max()*100), 1) for l in (1, 2, 3, 4)]})')
    return N


frame_lanes, XF, ZBOT, ZTOP = build_frame()
FINAL_BAR = TB.add(0.0, ZBOT + 0.04, (1, 0), T.T_END, (255, 214, 92), kind=1, sec=SEC['final'], pitch=T.FINAL_NOTE,
                   dims=(0.06, 0.16, 0.012), y0=0.004)
other = n_dominoes()
FUNNEL_N = place_funnel(TARGET - other)


def relax(sec_ids=(SEC['funnel'], SEC['fan']), iters=60, need=0.003):
    """nudge standing dominoes of different lanes apart (mm moves) until no two footprints intersect"""
    from scipy.spatial import cKDTree
    c = TB.c
    idx = np.array([i for i in range(TB.n) if c['sec'][i] in sec_ids])
    for it in range(iters):
        P = np.stack([np.array(c['cx'])[idx], np.array(c['cz'])[idx]], 1)
        moved = 0
        for a, b in cKDTree(P).query_pairs(0.03):
            i, j = idx[a], idx[b]
            if c['lane'][i] == c['lane'][j] or c['nxt'][i] == j or c['nxt'][j] == i:
                continue
            d = np.array([c['cx'][j] - c['cx'][i], c['cz'][j] - c['cz'][i]])
            n = np.linalg.norm(d) + 1e-9
            # half extents of each footprint along the separating direction
            u = d / n
            ext = 0.0
            for k in (i, j):
                f = np.array([c['fx'][k], c['fz'][k]])
                l = np.array([f[1], -f[0]])
                ext += abs(u @ f) * c['dt'][k] / 2 + abs(u @ l) * c['dw'][k] / 2
            gap = n - ext
            if gap < need:
                push = (need - gap) / 2 + 0.0004
                c['cx'][i] -= u[0] * push
                c['cz'][i] -= u[1] * push
                c['cx'][j] += u[0] * push
                c['cz'][j] += u[1] * push
                moved += 1
        if moved == 0:
            break
    return it, moved


RELAX = relax()


# ================================================================= timing per domino (fall model inputs)
def finish():
    c = TB.arrays()
    n = len(c['cx'])
    nxt = c['nxt']
    t0 = c['t0']
    s = c['dh'] / DH
    tc = np.full(n, 0.0)
    thc = np.full(n, math.pi / 2)
    thr = np.full(n, math.pi / 2)
    has = (nxt >= 0) & (c['kind'] == 0)
    j = np.where(has, nxt, 0)
    d = np.hypot(c['cx'][j] - c['cx'], c['cz'][j] - c['cz'])
    d = np.where(has, d, 1.0)
    h, th = c['dh'], c['dt']
    thc = np.where(has, np.arcsin(np.clip((d - th) / h, 0.05, 1.0)), math.pi / 2)
    thr = np.where(has, np.arccos(np.clip(th / np.maximum(d, th + 1e-4), 0, 1)), math.pi / 2)
    thr = np.maximum(thr, thc)
    tc = np.where(has, t0[j] - t0, 0.0)
    fallback = 0.17 * np.sqrt(s)
    tc = np.where((tc <= 0.004) | ~has, fallback, tc)
    # cap: a domino never takes longer than ~0.35 s (x size) to reach its neighbour
    tc = np.minimum(tc, 0.35 * np.sqrt(s))
    # staircase: contact timings are the beat sheet's; the giant falls flat by 40.0
    tc[GIANT] = T.T_DROP - t0[GIANT]
    thr[GIANT] = thc[GIANT] = math.pi / 2
    c.update(tc=tc, thc=thc, thr=thr)
    return c


ARR = finish()

if __name__ == '__main__':
    c = ARR
    dom = c['kind'] == 0
    print(f'dominoes: {dom.sum()}   chime bars: {(c["kind"] == 1).sum()}   lanes: {TB.lane_n}')
    names = {v: k for k, v in SEC.items()}
    for s_ in sorted(set(c['sec'][dom])):
        m = dom & (c['sec'] == s_)
        print(f'  {names[s_]:8s} {m.sum():5d}   t0 {c["t0"][m].min():6.2f} .. {c["t0"][m].max():6.2f}')
    nx = c['nxt']
    ok = (nx >= 0) & dom
    dd = np.hypot(c['cx'][nx[ok]] - c['cx'][ok], c['cz'][nx[ok]] - c['cz'][ok])
    for s_ in sorted(set(c['sec'][ok])):
        m = c['sec'][ok] == s_
        print(f'  spacing {names[s_]:8s} p5 {np.percentile(dd[m], 5)*100:5.1f}  med {np.median(dd[m])*100:5.1f}  p95 {np.percentile(dd[m], 95)*100:5.1f} cm')
    print(f'last domino t0 {c["t0"][dom].max():.3f}; struck by 52.0: {(c["t0"][dom] <= 52.0).sum()}')
    print(f'frame half-width {XF:.2f}  top {ZTOP:.2f}  bottom {ZBOT:.2f}  height {ZBOT - ZTOP:.2f}')
    # click density
    t = c['t0'][dom]
    h, e = np.histogram(t, bins=np.arange(0, 53, 1.0))
    print('clicks/s:', ' '.join(f'{int(v)}' for v in h))
    # overlap check: dominoes of different lanes closer than 2.4 cm (centre to centre)
    from scipy.spatial import cKDTree
    P = np.stack([c['cx'], c['cz']], 1)
    tree = cKDTree(P)
    bad = []
    for a, b in tree.query_pairs(0.022):
        if c['lane'][a] == c['lane'][b] and c['lane'][a] not in (900, 901):
            continue
        if c['nxt'][a] == b or c['nxt'][b] == a:
            continue
        if c['kind'][a] == 1 and c['kind'][b] == 1:
            bad.append((a, b))
            continue
        bad.append((a, b))
    print(f'close pairs (<2.2 cm, not chain neighbours): {len(bad)}')
    names_bad = {}
    for a, b in bad:
        key = (names[c['sec'][a]], names[c['sec'][b]])
        names_bad[key] = names_bad.get(key, 0) + 1
    print('  by section:', names_bad)
    np.savez_compressed(os.path.join(HERE, 'layout.npz'), **c)
    # ---------------------------------------------------------------- map
    from PIL import Image, ImageDraw
    os.makedirs(os.path.join(HERE, 'test'), exist_ok=True)
    sc = 70.0
    x0, x1, z0, z1 = -XF - 0.6, XF + 0.6, ZTOP - 0.5, ZBOT + 0.5
    Wm, Hm = int((x1 - x0) * sc), int((z1 - z0) * sc)
    im = Image.new('RGB', (Wm, Hm), (196, 150, 100))
    d = ImageDraw.Draw(im)
    tp = lambda x, z: ((x - x0) * sc, (z - z0) * sc)
    for i in range(len(c['cx'])):
        x, z, fx, fz = c['cx'][i], c['cz'][i], c['fx'][i], c['fz'][i]
        w, th = c['dw'][i] / 2, c['dt'][i] / 2
        lx, lz = fz, -fx
        corners = [(x + fx * th + lx * w, z + fz * th + lz * w), (x + fx * th - lx * w, z + fz * th - lz * w),
                   (x - fx * th - lx * w, z - fz * th - lz * w), (x - fx * th + lx * w, z - fz * th + lz * w)]
        col = (int(c['br'][i]), int(c['bg'][i]), int(c['bb'][i]))
        d.polygon([tp(*p) for p in corners], fill=col)
    for a, b in bad[:400]:
        d.ellipse([tp(c['cx'][a], c['cz'][a])[0] - 4, tp(c['cx'][a], c['cz'][a])[1] - 4,
                   tp(c['cx'][a], c['cz'][a])[0] + 4, tp(c['cx'][a], c['cz'][a])[1] + 4], outline=(255, 0, 0))
    im.save(os.path.join(HERE, 'test', 'map.png'))
    print('map ->', os.path.join(HERE, 'test', 'map.png'), im.size)

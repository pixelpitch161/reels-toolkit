"""The marble machine: geometry (low-poly faces) + the playable elements + marble paths.

World: y up, the lift axis is x = z = 0, the camera's "front" is +z. Angles th (degrees) are measured
from +z toward +x: dir(th) = (sin th, 0, cos th). 1 unit ~ 1 m; the machine is ~4.9 m tall and floats
on a stone plinth (top at y = 0, rocky underside to y = -1.9).

Tiers (gate height, chute direction):
    1 kick     y 1.00  th   0   big drum (+ 2 toms that only the spray hits) on the plinth
    2 snare    y 1.45  th  62   snare + 3 glass discs on stands
    3 bass     y 1.90  th -58   10 tuned pipes standing on the plinth (marbles drop into the mouths)
    4 bowls    y 2.35  th  40   7 glass bowls on an arc shelf
    5 bars     y 2.80  th -38   7 tuned bars on an arc frame
    6 spiral   y 3.25  th  16   a 1.25-turn spiral ramp of 16 bars round the whole machine
    7 cymbal   y 3.65  th   0   crash (left) + ride (right) discs
    8 melody   y 4.10  th   0   10 golden bell bars on an arc + resonator tubes
"""
import math
import numpy as np
from reelkit.geometry import face, box, prism, icosa, lathe
import timeline as T

R_M = 0.032                 # marble radius
G = 9.8
V_ROLL = 1.6                # rolling speed on chutes (m/s)
R_BASIN = (1.26, 1.60)      # basin trough on the plinth top
PLINTH_R = 1.78

BRASS = (222, 176, 92)
BRASS_D = (172, 124, 64)
COPPER = (214, 128, 86)
IRON = (96, 92, 112)
STONE = (150, 128, 168)
STONE_D = (104, 88, 132)
SKIN = (246, 236, 214)
LACQUER = (204, 60, 76)
GLASS = (168, 236, 255)
GOLD = (255, 206, 96)
ROSE = (190, 92, 80)


def P(r, th, y):
    a = math.radians(th)
    return np.array([r * math.sin(a), y, r * math.cos(a)])


def dirv_xz(th):
    a = math.radians(th)
    return np.array([math.sin(a), 0.0, math.cos(a)])


def beam(a, b, w, h, up=(0, 1, 0)):
    """oriented box between centre points a and b (width w across, height h along `up`)"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    L = np.linalg.norm(d)
    fx = d / (L + 1e-12)
    up = np.asarray(up, float)
    side = np.cross(fx, up)
    if np.linalg.norm(side) < 1e-6:
        side = np.cross(fx, [1.0, 0, 0])
    side /= np.linalg.norm(side)
    upv = np.cross(side, fx)
    c = (a + b) / 2
    V = [c + sx * fx * L / 2 + sy * upv * h / 2 + sz * side * w / 2
         for sy in (-1, 1) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7), (0, 3, 2, 1)]
    return [face([V[i] for i in q], c) for q in quads]


def disc_faces(c, r, n, nrm, thick=0.0):
    """flat n-gon disc (optionally a thin slab) facing nrm"""
    c = np.asarray(c, float)
    nrm = np.asarray(nrm, float) / np.linalg.norm(nrm)
    u = np.cross(nrm, [0, 0, 1.0]) if abs(nrm[2]) < 0.9 else np.cross(nrm, [1.0, 0, 0])
    u /= np.linalg.norm(u)
    v = np.cross(nrm, u)
    ring = [c + r * (math.cos(2 * math.pi * k / n) * u + math.sin(2 * math.pi * k / n) * v) for k in range(n)]
    out = [(np.array(ring), nrm)]
    if thick:
        ring2 = [p - nrm * thick for p in ring]
        out.append((np.array(ring2[::-1]), -nrm))
        for k in range(n):
            out.append(face([ring[k], ring[(k + 1) % n], ring2[(k + 1) % n], ring2[k]], c - nrm * thick / 2))
    return out


def band(cx, cz, r, y0, y1, n=12):
    """open cylinder band (no caps), drawn double-sided"""
    out = []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        q = [[cx + r * math.sin(a0), y0, cz + r * math.cos(a0)], [cx + r * math.sin(a1), y0, cz + r * math.cos(a1)],
             [cx + r * math.sin(a1), y1, cz + r * math.cos(a1)], [cx + r * math.sin(a0), y1, cz + r * math.cos(a0)]]
        out.append(face(q, (cx, (y0 + y1) / 2, cz)))
    return out


def arc_beam(r, th0, th1, y, w, h, n):
    out = []
    ths = np.linspace(th0, th1, n + 1)
    for a, b in zip(ths, ths[1:]):
        out += beam(P(r, a, y), P(r, b, y), w, h)
    return out


# ================================================================== parts
class Part:
    """faces + colour; elem = index of the playable element it belongs to (flash/wobble), or -1"""
    def __init__(self, name, faces, color, elem=-1, emit=0.0, double=False, grp=0, glass=False, cast=True):
        self.name, self.faces, self.color, self.elem = name, faces, np.array(color, float), elem
        self.emit, self.double, self.grp, self.glass, self.cast = emit, double, grp, glass, cast


class Elem:
    def __init__(self, tier, idx, kind, pitch, pos, col, size):
        self.tier, self.idx, self.kind, self.pitch = tier, idx, kind, pitch
        self.pos = np.asarray(pos, float)          # marble centre at impact
        self.col = np.array(col, float)            # flash colour
        self.size = size                           # glow radius (m)
        self.id = -1


PARTS = []
ELEMS = []
TIER_ELEMS = [[] for _ in range(8)]
_grp = [10]


def grp():
    _grp[0] += 1
    return _grp[0]


def add_elem(e):
    e.id = len(ELEMS)
    ELEMS.append(e)
    TIER_ELEMS[e.tier].append(e)
    return e.id


def hue(i, n, s=0.62, v=1.0, off=0.0):
    import colorsys
    r, g, b = colorsys.hsv_to_rgb((off + i / n) % 1.0, s, v)
    return (int(r * 255), int(g * 255), int(b * 255))


# ------------------------------------------------------------------ tiers: gates + chutes
GATE_Y = [1.00, 1.45, 1.90, 2.35, 2.80, 3.25, 3.65, 4.10]
GATE_TH = [0, 62, -58, 40, -38, 16, 0, 0]
CHUTE_R = [0.62, 0.60, 0.56, 0.50, 0.50, 1.02, 0.52, 0.46]
CHUTE_DROP = [0.07, 0.07, 0.06, 0.06, 0.06, 0.07, 0.07, 0.07]
R_LIFT = 0.21
GATE_R0, GATE_R1 = 0.235, 0.40            # flap from hinge (inner) to tip


def gate_point(g):
    """where a marble leaves the lift at gate g (marble centre)"""
    return P(GATE_R0 + 0.03, GATE_TH[g], GATE_Y[g] + 0.015 + R_M)


def chute_end(g):
    return P(CHUTE_R[g], GATE_TH[g], GATE_Y[g] - CHUTE_DROP[g] + 0.015 + R_M)


for g in range(8):
    a = P(R_LIFT, GATE_TH[g], GATE_Y[g])
    b = P(CHUTE_R[g], GATE_TH[g], GATE_Y[g] - CHUTE_DROP[g])
    gg = grp()
    PARTS.append(Part(f'chute{g}', beam(a, b, 0.085, 0.03), BRASS, grp=gg))
    # side lips
    for s in (-1, 1):
        off = np.cross(dirv_xz(GATE_TH[g]), [0, 1, 0]) * 0.045 * s
        PARTS.append(Part(f'lip{g}', beam(a + off + [0, 0.025, 0], b + off + [0, 0.025, 0], 0.012, 0.03), BRASS_D, grp=gg))

# ------------------------------------------------------------------ tier 1: kick drum + toms (on the plinth)
KICK_C = np.array([0.0, 0.0, 0.80])
KICK_R, KICK_H = 0.34, 0.42
g1 = grp()
PARTS.append(Part('kick_shell', prism(KICK_C[0], KICK_C[2], KICK_R, 0.02, KICK_H - 0.02, 14)[:-1], LACQUER, grp=g1))
PARTS.append(Part('kick_hoop0', band(KICK_C[0], KICK_C[2], KICK_R + 0.012, 0.0, 0.06, 14), BRASS, grp=g1, double=False))
PARTS.append(Part('kick_hoop1', band(KICK_C[0], KICK_C[2], KICK_R + 0.012, KICK_H - 0.06, KICK_H, 14), BRASS, grp=g1))
e = add_elem(Elem(0, 0, 'kick', None, KICK_C + [0, KICK_H + R_M, 0], T.COL[0], 0.45))
PARTS.append(Part('kick_skin', [(np.array([[KICK_C[0] + (KICK_R - 0.01) * math.sin(2 * math.pi * k / 14), KICK_H,
                                             KICK_C[2] + (KICK_R - 0.01) * math.cos(2 * math.pi * k / 14)] for k in range(14)][::-1]),
                                  np.array([0, 1.0, 0]))], SKIN, elem=e, grp=g1))
TOMS = [(-0.50, 1.02, 0.13, 0.30), (0.50, 1.02, 0.12, 0.26)]
for i, (x, z, r, h) in enumerate(TOMS):
    gg = grp()
    PARTS.append(Part('tom_shell', prism(x, z, r, 0.0, h, 10)[:-1], LACQUER, grp=gg))
    PARTS.append(Part('tom_hoop', band(x, z, r + 0.01, h - 0.04, h, 10), BRASS, grp=gg))
    e = add_elem(Elem(0, 1 + i, 'tom', 45 - 5 * i, [x, h + R_M, z], T.COL[0], 0.25))
    PARTS.append(Part('tom_skin', [(np.array([[x + r * math.sin(2 * math.pi * k / 10), h, z + r * math.cos(2 * math.pi * k / 10)]
                                              for k in range(10)][::-1]), np.array([0, 1.0, 0]))], SKIN, elem=e, grp=gg))

# ------------------------------------------------------------------ tier 2: snare + 3 glass discs on stands
SNARE_C = P(0.80, 75, 0.0)
gg = grp()
PARTS.append(Part('snare_stand', prism(SNARE_C[0], SNARE_C[2], 0.014, 0.0, 0.93, 5)[:-1], IRON, grp=gg))
PARTS.append(Part('snare_foot', prism(SNARE_C[0], SNARE_C[2], 0.12, 0.0, 0.025, 3, r1=0.04), IRON, grp=gg))
PARTS.append(Part('snare_shell', prism(SNARE_C[0], SNARE_C[2], 0.15, 0.93, 1.03, 12)[:-1], (90, 156, 230), grp=gg))
e = add_elem(Elem(1, 0, 'snare', None, SNARE_C + [0, 1.03 + R_M, 0], T.COL[1], 0.28))
PARTS.append(Part('snare_skin', [(np.array([[SNARE_C[0] + 0.15 * math.sin(2 * math.pi * k / 12), 1.03,
                                              SNARE_C[2] + 0.15 * math.cos(2 * math.pi * k / 12)] for k in range(12)][::-1]),
                                   np.array([0, 1.0, 0]))], SKIN, elem=e, grp=gg))
DISCS = [(0.86, 44, 1.16), (0.94, 57, 1.10), (0.96, 92, 1.20)]
for i, (r, th, y) in enumerate(DISCS):
    c = P(r, th, y)
    gg = grp()
    PARTS.append(Part('disc_stand', prism(c[0], c[2], 0.011, 0.0, y - 0.012, 5)[:-1], IRON, grp=gg))
    PARTS.append(Part('disc_foot', prism(c[0], c[2], 0.09, 0.0, 0.02, 3, r1=0.03), IRON, grp=gg))
    e = add_elem(Elem(1, 1 + i, 'hat', None, c + [0, R_M, 0], T.COL[1], 0.2))
    PARTS.append(Part('disc', disc_faces(c, 0.10, 10, [0, 1, 0], 0.012), GLASS, elem=e, emit=0.25, glass=True, grp=gg))

# ------------------------------------------------------------------ tier 3: 10 tuned pipes
PIPE_TH = np.linspace(-120, -38, 10)
PIPE_R = 0.93
PIPE_H = [0.55 * 2 ** ((50 - p) / 12) for p in T.PIPES]
for i, p in enumerate(T.PIPES):
    c = P(PIPE_R, PIPE_TH[i], 0.0)
    rr = 0.062 - 0.0016 * i
    h = PIPE_H[i]
    gg = grp()
    col = np.array(COPPER) * (0.92 + 0.012 * i)
    PARTS.append(Part('pipe', prism(c[0], c[2], rr, 0.0, h, 8, rot=0.2)[:-1], col, grp=gg))
    PARTS.append(Part('pipe_lip', band(c[0], c[2], rr + 0.008, h - 0.03, h, 8), BRASS, grp=gg))
    # mouth (the organ-pipe slot) facing outward
    m = P(PIPE_R + rr * 0.98, PIPE_TH[i], 0.0)
    PARTS.append(Part('pipe_mouth', [face([m + dirv_xz(PIPE_TH[i] + 90) * rr * 0.55 + [0, 0.12, 0],
                                         m - dirv_xz(PIPE_TH[i] + 90) * rr * 0.55 + [0, 0.12, 0],
                                         m - dirv_xz(PIPE_TH[i] + 90) * rr * 0.55 + [0, 0.2, 0],
                                         m + dirv_xz(PIPE_TH[i] + 90) * rr * 0.55 + [0, 0.2, 0]], c)], (60, 34, 40), grp=gg))
    e = add_elem(Elem(2, i, 'pipe', p, c + [0, h + 0.012, 0], T.COL[2], 0.22))
    PARTS.append(Part('pipe_top', [(np.array([[c[0] + rr * 0.8 * math.sin(2 * math.pi * k / 8 + 0.2), h + 0.002,
                                               c[2] + rr * 0.8 * math.cos(2 * math.pi * k / 8 + 0.2)] for k in range(8)][::-1]),
                                    np.array([0, 1.0, 0]))], (40, 22, 30), elem=e, grp=gg))

# ------------------------------------------------------------------ tier 4: 7 glass bowls on an arc shelf
BOWL_TH = np.linspace(4, 100, 7)
BOWL_R, BOWL_Y = 0.84, 2.04
gg = grp()
PARTS.append(Part('bowl_shelf', arc_beam(BOWL_R, -2, 106, 1.955, 0.2, 0.03, 9), BRASS_D, grp=gg))
PARTS.append(Part('bowl_arm', beam(P(R_LIFT, 52, 1.94), P(BOWL_R, 52, 1.94), 0.05, 0.04), BRASS, grp=gg))
for i, p in enumerate(T.BOWL_P):
    c = P(BOWL_R, BOWL_TH[i], 1.97)
    prof = [(0.0, 0.03), (0.02, 0.062), (0.045, 0.082), (BOWL_Y - 1.97, 0.088)]
    fs = lathe(c[0], c[2], prof, n=9, cy=1.97)[:-1]
    col = hue(i, 7, 0.38, 1.0, 0.45)
    e = add_elem(Elem(3, i, 'bowl', p, c + [0, 0.04 + R_M, 0], T.COL[3], 0.22))
    PARTS.append(Part('bowl', fs, col, elem=e, emit=0.3, double=True, glass=True, grp=gg))
    PARTS.append(Part('bowl_in', [(np.array([[c[0] + 0.084 * math.sin(2 * math.pi * k / 9), BOWL_Y - 0.006,
                                              c[2] + 0.084 * math.cos(2 * math.pi * k / 9)] for k in range(9)][::-1]),
                                   np.array([0, 1.0, 0]))], np.array(col) * 0.72, elem=e, emit=0.35, glass=True, grp=gg))

# ------------------------------------------------------------------ tier 5: 7 tuned bars on an arc frame
BAR_TH = np.linspace(-104, -8, 7)
BAR_R, BAR_Y = 0.84, 2.50
gg = grp()
for rr in (0.77, 0.91):
    PARTS.append(Part('bar_rail', arc_beam(rr, -110, -2, BAR_Y - 0.03, 0.025, 0.025, 8), BRASS_D, grp=gg))
PARTS.append(Part('bar_arm', beam(P(R_LIFT, -56, BAR_Y - 0.06), P(0.92, -56, BAR_Y - 0.06), 0.05, 0.04), BRASS, grp=gg))
for i, p in enumerate(T.BAR_P):
    L = 0.22 - 0.012 * i
    a, b = P(BAR_R - L / 2, BAR_TH[i], BAR_Y - 0.0125), P(BAR_R + L / 2, BAR_TH[i], BAR_Y - 0.0125)
    col = hue(i, 7, 0.55, 1.0, 0.52)
    e = add_elem(Elem(4, i, 'bar', p, P(BAR_R, BAR_TH[i], BAR_Y + R_M), T.COL[4], 0.2))
    PARTS.append(Part('bar', beam(a, b, 0.062, 0.025), col, elem=e, emit=0.12, grp=gg))

# ------------------------------------------------------------------ tier 6: the spiral ramp (16 bars)
SP_R = 1.15
SP_TH0, SP_TURN = 16.0, -450.0
SP_Y0, SP_Y1 = 3.14, 1.00


def spiral(u):
    """centre line of the spiral ramp, u in [0, 1]"""
    u = np.asarray(u, float)
    th = np.radians(SP_TH0 + SP_TURN * u)
    return np.stack([SP_R * np.sin(th), SP_Y0 + (SP_Y1 - SP_Y0) * u, SP_R * np.cos(th)], -1)


SP_LEN = float(np.linalg.norm(np.diff(spiral(np.linspace(0, 1, 400)), axis=0), axis=1).sum())
gg = grp()
us = np.linspace(0, 1, 37)
for dr in (-0.07, 0.07):
    pts = spiral(us)
    rad = pts * np.array([1, 0, 1]) / SP_R
    pts = pts + rad * dr + np.array([0, -0.012, 0])
    for a, b in zip(pts, pts[1:]):
        PARTS.append(Part('sp_rail', beam(a, b, 0.022, 0.02), BRASS, grp=gg))
for uu in (0.18, 0.44, 0.70, 0.95):
    p = spiral(uu)
    th = math.degrees(math.atan2(p[0], p[2]))
    PARTS.append(Part('sp_strut', beam(P(R_LIFT, th, p[1] - 0.03), P(SP_R - 0.06, th, p[1] - 0.03), 0.03, 0.03), BRASS_D, grp=gg))
for k in range(16):
    u = (k + 0.5) / 16
    c = spiral(u)
    rad = c * np.array([1, 0, 1]) / SP_R
    col = hue(k, 16, 0.6, 1.0, 0.0)
    e = add_elem(Elem(5, k, 'xylo', None, c + [0, 0.012 + R_M, 0], T.COL[5], 0.18))
    PARTS.append(Part('sp_bar', beam(c - rad * 0.10 + [0, -0.004, 0], c + rad * 0.10 + [0, -0.004, 0], 0.07, 0.022), col,
                      elem=e, emit=0.15, grp=gg))
SP_END = spiral(1.0)
CHUTE6_END = spiral(0.0) + [0, 0.012 + R_M, 0]

# ------------------------------------------------------------------ tier 7: crash + ride
CYMBALS = [np.array([-0.60, 3.30, 0.62]), np.array([0.60, 3.26, 0.62])]
for i, c in enumerate(CYMBALS):
    gg = grp()
    n = np.array([-0.26 * np.sign(c[0]), 1.0, 0.28])
    n /= np.linalg.norm(n)
    PARTS.append(Part('cym_arm', beam(P(R_LIFT, math.degrees(math.atan2(c[0], c[2])), c[1] - 0.06), c - [0, 0.06, 0], 0.03, 0.03),
                      IRON, grp=gg))
    e = add_elem(Elem(6, i, 'crash' if i == 0 else 'ride', None, c + n * (0.02 + R_M) + [0.04 * -np.sign(c[0]), 0, -0.03],
                      T.COL[6], 0.32))
    fs = disc_faces(c, 0.22, 14, n, 0.0) + disc_faces(c + n * 0.025, 0.06, 8, n, 0.0)
    PARTS.append(Part('cymbal', fs, GOLD, elem=e, double=True, grp=gg))

# ------------------------------------------------------------------ tier 8: 10 bell bars (melody)
BELL_TH = np.linspace(-67.5, 67.5, 10)
BELL_R, BELL_Y = 0.80, 3.80
gg = grp()
PARTS.append(Part('bell_rail', arc_beam(BELL_R, -75, 75, BELL_Y - 0.03, 0.05, 0.022, 10), BRASS_D, grp=gg))
for th in (-45, 45):
    PARTS.append(Part('bell_arm', beam(P(R_LIFT, th, BELL_Y - 0.05), P(BELL_R, th, BELL_Y - 0.05), 0.04, 0.035), BRASS, grp=gg))
for i, p in enumerate(T.BELL_P):
    L = 0.17 - 0.005 * i
    a, b = P(BELL_R, BELL_TH[i] - math.degrees(L / 2 / BELL_R), BELL_Y - 0.014), P(BELL_R, BELL_TH[i] + math.degrees(L / 2 / BELL_R), BELL_Y - 0.014)
    e = add_elem(Elem(7, i, 'bell', p, P(BELL_R, BELL_TH[i], BELL_Y + R_M), T.COL[7], 0.2))
    PARTS.append(Part('bell', beam(a, b, 0.058, 0.028), GOLD, elem=e, emit=0.15, grp=gg))
    c = P(BELL_R, BELL_TH[i], 0)
    PARTS.append(Part('bell_tube', prism(c[0], c[2], 0.022, BELL_Y - 0.28 + 0.012 * i, BELL_Y - 0.04, 6)[:-1], BRASS_D, grp=gg))

# ------------------------------------------------------------------ the lift (static cage), crown, housing
gL = grp()
PARTS.append(Part('housing', prism(0, 0, 0.30, 0.0, 0.36, 12), BRASS_D, grp=gL))
PARTS.append(Part('housing_band', band(0, 0, 0.312, 0.24, 0.30, 12), GOLD, grp=gL))
for k in range(6):
    a = 2 * math.pi * k / 6 + 0.26
    x, z = R_LIFT * math.sin(a), R_LIFT * math.cos(a)
    PARTS.append(Part('cage_rod', prism(x, z, 0.013, 0.34, 4.36, 4)[:-1], BRASS, grp=gL))
for y in np.arange(0.62, 4.3, 0.62):
    PARTS.append(Part('cage_ring', band(0, 0, R_LIFT + 0.006, y, y + 0.035, 12), BRASS, double=True, grp=gL))
PARTS.append(Part('shaft', prism(0, 0, 0.032, 0.34, 4.38, 6)[:-1], BRASS_D, grp=gL))
PARTS.append(Part('collar', prism(0, 0, 0.27, 4.33, 4.42, 12), GOLD, grp=gL))
PARTS.append(Part('dome', lathe(0, 0, [(0.0, 0.25), (0.1, 0.22), (0.2, 0.15), (0.27, 0.06), (0.30, 0.0)], n=12, cy=4.42), BRASS, grp=gL))
PARTS.append(Part('spike', prism(0, 0, 0.018, 4.70, 4.76, 5)[:-1], BRASS_D, grp=gL))
ORB_C = np.array([0.0, 4.85, 0.0])
PARTS.append(Part('orb', icosa(ORB_C, 0.095, jit=0.0), (236, 250, 255), emit=1.2, glass=True, grp=gL, cast=False))

# lantern nodes on short arms (warm local lights)
LANTERNS = [P(0.32, -90, 1.22), P(0.32, 90, 2.12), P(0.32, -90, 3.02), P(0.32, 90, 3.92)]
for c in LANTERNS:
    gg = grp()
    side = np.sign(c[0])
    PARTS.append(Part('lant_arm', beam([side * R_LIFT, c[1] + 0.09, 0], c + [0, 0.09, 0], 0.02, 0.02), IRON, grp=gg))
    PARTS.append(Part('lant_cap', prism(c[0], c[2], 0.05, c[1] + 0.05, c[1] + 0.09, 6, r1=0.015), BRASS_D, grp=gg))
    PARTS.append(Part('lantern', prism(c[0], c[2], 0.04, c[1] - 0.05, c[1] + 0.05, 6), (255, 196, 110), emit=1.0, grp=gg, cast=False))

# ------------------------------------------------------------------ plinth (floating stone) + basin
gP = grp()
NPL = 18
_rng = np.random.default_rng(18)
rings = [(0.0, PLINTH_R, 0.0), (-0.24, PLINTH_R, 0.0), (-0.62, 1.50, 0.08), (-1.05, 1.02, 0.1), (-1.52, 0.48, 0.08), (-1.92, 0.05, 0.0)]
rv = []
for (y, r, j) in rings:
    ring = []
    for k in range(NPL):
        a = 2 * math.pi * k / NPL
        rr = r * (1 + _rng.uniform(-j, j))
        yy = y + (_rng.uniform(-0.08, 0.08) if 0 < j else 0)
        ring.append(np.array([rr * math.sin(a), yy, rr * math.cos(a)]))
    rv.append(ring)
PLINTH_FACES = []
for i in range(len(rv) - 1):
    for k in range(NPL):
        q = [rv[i][k], rv[i][(k + 1) % NPL], rv[i + 1][(k + 1) % NPL], rv[i + 1][k]]
        f1 = face([q[0], q[1], q[2]], (0, rings[i][0] - 0.5, 0))
        f2 = face([q[0], q[2], q[3]], (0, rings[i][0] - 0.5, 0))
        PLINTH_FACES += [f1, f2]
PLINTH_TOP = np.array(rv[0])
for i, (Q, n) in enumerate(PLINTH_FACES):
    y = Q[:, 1].mean()
    col = STONE if y > -0.25 else (np.array(STONE_D) * (0.95 + 0.25 * math.sin(y * 9.0)))
    PARTS.append(Part('plinth', [(Q, n)], col, grp=gP))
gB = grp()
PARTS.append(Part('basin_in', band(0, 0, R_BASIN[0], 0.0, 0.10, 20), BRASS, double=True, grp=gB))
PARTS.append(Part('basin_out', band(0, 0, R_BASIN[1], 0.0, 0.14, 24), BRASS, double=True, grp=gB))
PARTS.append(Part('basin_lip', [face([P(R_BASIN[1], 360 * k / 24, 0.14), P(R_BASIN[1], 360 * (k + 1) / 24, 0.14),
                                      P(R_BASIN[1] + 0.06, 360 * (k + 1) / 24, 0.14), P(R_BASIN[1] + 0.06, 360 * k / 24, 0.14)], up=True)
                                 for k in range(24)], GOLD, grp=gB))

# floating rocks (orbit slowly; positions animated in render)
ROCKS = [(2.6, 0.0, -0.7, 0.22, 7), (2.3, 120.0, -1.3, 0.16, 8), (3.0, 230.0, 0.4, 0.19, 9)]   # (orbit r, phase, y, size, seed)

# the lever (front-right, on the plinth)
LEVER_PIVOT = np.array([0.80, 0.16, 0.86])
LEVER_LEN = 0.46

# ------------------------------------------------------------------ subject points (focus audit)
SUBJECT_PTS = np.array([[0, -0.2, 0], [0, 4.9, 0], [0, 2.3, 0]] +
                       [list(P(1.25, a, y)) for a in range(0, 360, 45) for y in (0.0, 1.5, 3.2)] +
                       [list(P(0.9, a, 3.85)) for a in (-70, 0, 70)] +
                       [list(P(PLINTH_R, a, 0.0)) for a in range(0, 360, 30)])


# ================================================================== marble paths
def hop_v0(a, b, T_):
    """initial velocity for a ballistic hop from a to b in time T_"""
    return (np.asarray(b) - np.asarray(a)) / T_ + np.array([0, 0.5 * G * T_, 0])


def hop_time(a, b):
    dy = a[1] - b[1]
    dxz = math.hypot(b[0] - a[0], b[2] - a[2])
    if dy > 0:
        T_ = math.sqrt(2 * dy / G) * 1.04 + 0.04
    else:
        T_ = 0.22
    return float(np.clip(max(T_, 0.12 + 0.18 * dxz), 0.16, 0.62))


def basin_point(th_deg, rng):
    r = rng.uniform(R_BASIN[0] + 0.07, R_BASIN[1] - 0.07)
    return P(r, th_deg, 0.03 + R_M)


if __name__ == '__main__':
    print('parts', len(PARTS), 'faces', sum(len(p.faces) for p in PARTS), 'elements', len(ELEMS))
    for t in range(8):
        print(t + 1, T.CH[t], len(TIER_ELEMS[t]), [e.kind for e in TIER_ELEMS[t]][:3])
    print('spiral length', round(SP_LEN, 2), 'm  (marble speed', round(SP_LEN / 2.0, 2), 'm/s)')
    print('pipe heights', [round(h, 2) for h in PIPE_H])

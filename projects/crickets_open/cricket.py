"""Low-poly snowy tree cricket (units: cm). Local frame: +x = head, +y = up.
Returns polygons/lines in world space for a given time and chirp state."""
import math
import numpy as np
from scene import HERO

XS = [-0.98, -0.82, -0.55, -0.25, 0.04, 0.2, 0.42, 0.55, 0.68, 0.76]
RS = [0.04, 0.12, 0.185, 0.19, 0.165, 0.17, 0.15, 0.14, 0.11, 0.04]
YS = [0.03, 0.01, 0.0, 0.0, 0.02, 0.04, 0.045, 0.05, 0.04, 0.02]
SEGCOL = [(200, 238, 128), (200, 238, 128), (196, 236, 132), (192, 236, 136), (184, 234, 146),
          (176, 232, 152), (170, 228, 156), (166, 226, 156), (160, 222, 156)]
NS = 8


def _body():
    rings = []
    for x, r, y in zip(XS, RS, YS):
        ring = [np.array([x, y + r * math.cos(a), 0.86 * r * math.sin(a)]) for a in np.linspace(0, 2 * math.pi, NS + 1)[:-1]]
        rings.append(ring)
    faces = []
    for i in range(len(rings) - 1):
        for k in range(NS):
            k2 = (k + 1) % NS
            Q = np.array([rings[i][k], rings[i][k2], rings[i + 1][k2], rings[i + 1][k]])
            c = Q.mean(0)
            axis_c = np.array([c[0], YS[i], 0])
            n = np.cross(Q[1] - Q[0], Q[3] - Q[0])
            n /= np.linalg.norm(n) + 1e-9
            if np.dot(n, c - axis_c) < 0:
                n = -n
            col = np.array(SEGCOL[i], float)
            if k in (0, 1, 7):            # darker dorsal stripe
                col = col * np.array([0.92, 0.96, 0.9])
            faces.append((Q, n, col))
    return faces


BODY = _body()

LEGS = [  # hip, knee, foot (z positive side; mirrored), femur width, tibia width
    ((0.5, -0.06, 0.09), (0.72, -0.2, 0.3), (0.95, -0.42, 0.36), 0.045, 0.028),
    ((0.25, -0.08, 0.11), (0.2, -0.24, 0.4), (0.05, -0.42, 0.53), 0.045, 0.028),
    ((0.02, -0.06, 0.13), (-0.44, 0.22, 0.25), (-1.06, -0.42, 0.4), 0.09, 0.032),
]

WING = [(0.23, 0.0), (0.29, 0.42), (0.22, 0.84), (0.05, 1.03), (-0.16, 1.0), (-0.3, 0.78), (-0.27, 0.36), (-0.2, 0.0)]
WING_BASE = np.array([0.07, 0.17, 0.0])


def chirp_env(t, chirp_times):
    """0..1 envelope of the current chirp (wings buzzing)."""
    e = 0.0
    for tc in chirp_times:
        if tc <= t < tc + 0.2:
            e = max(e, math.exp(-(t - tc) / 0.07))
    return e


def build(t, env, heat):
    """-> dict with body faces (world), legs (segments), antennae (polylines),
    wings (polygons), eyes; all in world coordinates."""
    off = HERO.copy()
    out = {'faces': [], 'legs': [], 'ant': [], 'wings': [], 'eyes': []}
    # tiny breathing bob
    bob = np.array([0, 0.004 * math.sin(t * 3.1), 0])
    for Q, n, c in BODY:
        out['faces'].append((Q + off + bob, n, c))
    for (h, k, f, w1, w2) in LEGS:
        for sgn in (1, -1):
            H = np.array([h[0], h[1], h[2] * sgn]) + off + bob
            Kp = np.array([k[0], k[1], k[2] * sgn]) + off
            F = np.array([f[0], f[1], f[2] * sgn]) + off
            out['legs'].append((H, Kp, w1, sgn))
            out['legs'].append((Kp, F, w2, sgn))
    for sgn in (1, -1):
        pts = []
        for j in range(14):
            u = j / 13
            wig = 0.12 * math.sin(t * 1.7 + u * 3 + sgn) * u * u
            x = 0.74 + 3.1 * u
            y = 0.07 + 1.2 * u - 1.0 * u * u + wig
            z = sgn * (0.05 + 1.3 * u * u) + 0.1 * math.sin(t * 1.3 + sgn * 2) * u * u
            pts.append(np.array([x, y, z]) + off + bob)
        out['ant'].append((pts, sgn))
    # wings: held straight up; they buzz open/closed during a chirp
    for sgn in (1, -1):
        open_ang = math.radians(4 + 9 * env * (0.5 + 0.5 * math.sin(t * 2 * math.pi * 46 + (0 if sgn > 0 else math.pi))))
        tilt = math.radians(-14)
        pts = []
        for (u, v) in WING:
            x = u * math.cos(tilt) - v * math.sin(tilt)
            y = u * math.sin(tilt) + v * math.cos(tilt)
            # rotate about the vertical axis through the base to open
            z = sgn * (0.015 + (x + 0.3) * math.sin(open_ang) * 0.5)
            pts.append(WING_BASE + np.array([x, y, z]) + off + bob)
        out['wings'].append((np.array(pts), sgn))
    for sgn in (1, -1):
        c = np.array([0.62, 0.1, 0.105 * sgn]) + off + bob
        out['eyes'].append(np.array([c + [0.04, 0, 0], c + [0, 0.04, 0], c + [-0.04, 0, 0], c + [0, -0.04, 0]]))
    return out


def leaf_polys():
    """Faceted V-shaped leaf under the cricket, world coordinates."""
    out = []
    xs = np.linspace(-7.0, 14.0, 10)
    def w(x):
        return 1.9 * max(0.0, math.sin(math.pi * (x + 7.0) / 21.0)) ** 0.75
    def yb(x):
        return -0.46 - 0.006 * (x - 1.5) ** 2
    for i in range(len(xs) - 1):
        xa, xb = xs[i], xs[i + 1]
        for sgn in (1, -1):
            A = np.array([xa, yb(xa), 0]); B = np.array([xb, yb(xb), 0])
            C = np.array([xb, yb(xb) + 0.22 * w(xb) / 1.9, sgn * w(xb)])
            D = np.array([xa, yb(xa) + 0.22 * w(xa) / 1.9, sgn * w(xa)])
            Q = np.array([A, B, C, D]) + HERO
            n = np.cross(Q[1] - Q[0], Q[3] - Q[0]); n /= np.linalg.norm(n) + 1e-9
            if n[1] < 0:
                n = -n
            out.append((Q, n, i, sgn))
    return out


LEAF = leaf_polys()
STEM = [np.array([-8.5, 0.0, 0.6]), np.array([-7.8, 20.0, 0.3]), HERO + np.array([-7.0, -0.48, 0.0]),
        np.array([-6.0, 56.0, -0.6]), np.array([-5.5, 68.0, -1.0])]
UMBEL = np.array([-5.5, 69.5, -1.0])

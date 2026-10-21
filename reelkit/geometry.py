"""Low-poly geometry builders. Every builder returns a list of faces: (Q (k,3) array, unit normal)."""
import math
import numpy as np


def face(pts, center=None, up=False):
    Q = np.array(pts, float)
    n = np.cross(Q[1] - Q[0], Q[2] - Q[0])
    n /= np.linalg.norm(n) + 1e-12
    if up and n[1] < 0:
        n = -n
    if center is not None and np.dot(n, Q.mean(0) - np.asarray(center, float)) < 0:
        n = -n
    return Q, n


def box(cx, cy, cz, hx, hy, hz, rot=0.0):
    """axis box with half sizes, rotated by rot (radians) about y"""
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    V = [P(sx * hx, sy * hy, sz * hz) for sy in (-1, 1) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7), (0, 3, 2, 1)]
    return [face([V[i] for i in q], (cx, cy, cz)) for q in quads]


def prism(cx, cz, r, y0, y1, n=8, r1=None, rot=0.0, tilt=(0.0, 0.0)):
    """n-sided prism / frustum (r at y0, r1 at y1); tilt slants the top cap (broken columns)"""
    r1 = r if r1 is None else r1
    out = []
    ang = [rot + 2 * math.pi * k / n for k in range(n)]
    top_y = lambda x, z: y1 + tilt[0] * (x - cx) + tilt[1] * (z - cz)
    for k in range(n):
        a0, a1 = ang[k], ang[(k + 1) % n]
        b0 = [cx + r * math.cos(a0), y0, cz + r * math.sin(a0)]
        b1 = [cx + r * math.cos(a1), y0, cz + r * math.sin(a1)]
        t1 = [cx + r1 * math.cos(a1), 0, cz + r1 * math.sin(a1)]
        t0 = [cx + r1 * math.cos(a0), 0, cz + r1 * math.sin(a0)]
        t1[1], t0[1] = top_y(t1[0], t1[2]), top_y(t0[0], t0[2])
        out.append(face([b0, b1, t1, t0], (cx, (y0 + y1) / 2, cz)))
    cap = []
    for a in ang[::-1]:
        x, z = cx + r1 * math.cos(a), cz + r1 * math.sin(a)
        cap.append([x, top_y(x, z), z])
    out.append(face(cap, up=True))
    return out


def lathe(cx, cz, prof, n=8, rot=0.0, cy=0.0):
    """surface of revolution from (y, r) pairs (jars, lanterns, pots), with a top cap"""
    faces = []
    ang = [rot + 2 * math.pi * k / n for k in range(n)]
    for (y0, r0), (y1, r1) in zip(prof, prof[1:]):
        if r0 < 1e-4 and r1 < 1e-4:
            continue
        for k in range(n):
            a0, a1 = ang[k], ang[(k + 1) % n]
            Q = np.array([[cx + r0 * math.cos(a0), cy + y0, cz + r0 * math.sin(a0)], [cx + r0 * math.cos(a1), cy + y0, cz + r0 * math.sin(a1)],
                          [cx + r1 * math.cos(a1), cy + y1, cz + r1 * math.sin(a1)], [cx + r1 * math.cos(a0), cy + y1, cz + r1 * math.sin(a0)]])
            nn = np.cross(Q[1] - Q[0], Q[3] - Q[0]) if r0 > 1e-4 else np.cross(Q[2] - Q[3], Q[0] - Q[3])
            nn /= np.linalg.norm(nn) + 1e-12
            am = (a0 + a1) / 2
            if np.dot(nn, [math.cos(am), 0, math.sin(am)]) < 0:
                nn = -nn
            faces.append((Q, nn))
    y, r = prof[-1]
    if r > 1e-3:
        faces.append((np.array([[cx + r * math.cos(a), cy + y, cz + r * math.sin(a)] for a in ang[::-1]]), np.array([0, 1.0, 0])))
    return faces


def icosa(c, r, sq=(1, 1, 1), jit=0.12, rng=None):
    """jittered icosahedron blob (tree crowns, bushes, rocks)"""
    rng = rng or np.random.default_rng(0)
    t = (1 + 5 ** 0.5) / 2
    V = np.array([[-1, t, 0], [1, t, 0], [-1, -t, 0], [1, -t, 0], [0, -1, t], [0, 1, t], [0, -1, -t], [0, 1, -t],
                  [t, 0, -1], [t, 0, 1], [-t, 0, -1], [-t, 0, 1]], float)
    V /= np.linalg.norm(V[0])
    V = V * r * rng.uniform(1 - jit, 1 + jit, (12, 1)) * np.array(sq) + np.asarray(c, float)
    F = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
         (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9), (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    return [face(V[list(f)], c) for f in F]


def gable(cx, cy, cz, hx, hz, hr, rot=0.0, over=0.1):
    """pitched roof"""
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    a, b = hx + over, hz + over
    r0, r1 = P(-a, hr, 0), P(a, hr, 0)
    e = [P(-a, 0, b), P(a, 0, b), P(a, 0, -b), P(-a, 0, -b)]
    ctr = (cx, cy - 1e-3, cz)
    return [face([e[0], e[1], r1, r0], ctr), face([e[2], e[3], r0, r1], ctr), face([e[1], e[2], r1], ctr), face([e[3], e[0], r0], ctr)]


def mound(cx, cz, rx, rz, h, n=6, rot=0.0):
    """faceted dome (sand drifts, hills)"""
    c, s_ = math.cos(rot), math.sin(rot)
    def P(u, v):
        rr = math.hypot(u, v)
        y = h * max(0.0, 1 - rr * rr) ** 0.8 + (0.02 if rr < 1 else 0)
        x, z = u * rx, v * rz
        return np.array([cx + x * c - z * s_, y, cz + x * s_ + z * c])
    us = np.linspace(-1.05, 1.05, n + 1)
    faces = []
    for i in range(n):
        for j in range(n):
            q = [P(us[i], us[j]), P(us[i + 1], us[j]), P(us[i + 1], us[j + 1]), P(us[i], us[j + 1])]
            if max(p[1] for p in q) < 0.025:
                continue
            faces.append(face([q[0], q[1], q[2]], up=True))
            faces.append(face([q[0], q[2], q[3]], up=True))
    return faces


def ridge(z, amp, base, n, span, seed, depth=400.0):
    """far mountain silhouette strip at depth z (quads facing the camera)"""
    r = np.random.default_rng(seed)
    xs = np.linspace(-span, span, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * (2.6 / span) + r.uniform(0, 6))) + r.uniform(-amp * 0.25, amp * 0.25, n)
    out = []
    for i in range(n - 1):
        Q = np.array([[xs[i], ys[i], z], [xs[i + 1], ys[i + 1], z], [xs[i + 1], -60, z + depth], [xs[i], -60, z + depth]])
        out.append((Q, np.array([0, 1.0, 0.35]) / np.linalg.norm([0, 1.0, 0.35])))
    return out


def catenary(a, b, sag, n):
    """points along a hanging string between a and b"""
    u = np.linspace(0, 1, n)
    P = np.asarray(a, float)[None] + (np.asarray(b, float) - np.asarray(a, float))[None] * u[:, None]
    P[:, 1] -= sag * 4 * u * (1 - u)
    return P


def thin_quad(a, b, w):
    """screen-facing-ish ribbon between two points (stems, legs) - also handy as a shadow caster"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    p = np.cross(d, [0, 0, 1.0])
    p = p / (np.linalg.norm(p) + 1e-9) * w / 2
    return np.array([a + p, b + p, b - p, a - p])


class Obj:
    """a named group of faces with a colour; grp groups objects that should not shadow each other"""
    _next = [0]

    def __init__(self, faces, color, name='', grp=None, cast=True, double=False, jitter=0.04, rng=None):
        rng = rng or np.random.default_rng(len(faces) * 7 + int(sum(color)))
        self.faces = faces
        self.color = np.array(color, float)
        self.name = name
        if grp is None:
            Obj._next[0] += 1
            grp = Obj._next[0]
        self.grp = grp
        self.cast = cast
        self.double = double
        self.tone = 1 + rng.uniform(-jitter, jitter, len(faces))
        self.ctr = np.mean([f[0].mean(0) for f in faces], axis=0)
        V = np.concatenate([f[0] for f in faces])
        self.lo, self.hi = V.min(0), V.max(0)


def new_group():
    Obj._next[0] += 1
    return Obj._next[0]

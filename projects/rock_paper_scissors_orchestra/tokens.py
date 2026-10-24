"""The three low-poly tokens (our own models, never emoji artwork). Token-local coordinates: the piece faces +x,
stands on y = 0, and is sized to the sim radius (about 0.13). Each model is a list of (faces, colour) parts."""
import math
import numpy as np
from reelkit.geometry import face, icosa, prism

R = 0.13
ROCK_COL = (118, 126, 140)
PAPER_COL = (240, 232, 210)
STEEL = (206, 212, 224)
HANDLE = (214, 44, 54)


def _rock():
    rng = np.random.default_rng(3)
    blob = icosa([0, R * 0.62, 0], R * 1.02, (1.05, 0.68, 0.95), jit=0.16, rng=rng)
    # flatten the bottom so it sits on the boards
    out = []
    for Q, n in blob:
        Q = Q.copy()
        Q[:, 1] = np.maximum(Q[:, 1], 0.0)
        out.append((Q, n))
    return [(out, ROCK_COL)]


def _paper():
    """a folded sheet: a low tent with a crisp ridge along the direction of travel, one corner dog-eared"""
    L, Wd, Hh = R * 1.15, R * 0.95, R * 0.75
    a = np.array([-L, 0.02, -Wd]); b = np.array([L, 0.02, -Wd])
    c = np.array([L, Hh, 0.0]); d = np.array([-L, Hh, 0.0])
    e = np.array([L, 0.02, Wd]); f = np.array([-L, 0.02, Wd])
    left = face([a, b, c, d], [0, Hh * 0.3, Wd * 2])
    right = face([d, c, e, f], [0, Hh * 0.3, -Wd * 2])
    # flip normals to point away from the tent's inside
    left = (left[0], left[1] if left[1][2] < 0 else -left[1])
    right = (right[0], right[1] if right[1][2] > 0 else -right[1])
    ear = face([c + [0, 0, 0], c + [-R * 0.42, -Hh * 0.05, R * 0.34], e + [-R * 0.05, Hh * 0.55, -R * 0.2]], [0, -1, 0])
    ear = (ear[0], ear[1] if ear[1][1] > 0 else -ear[1])
    return [([left, right], PAPER_COL), ([ear], (252, 248, 236))]


def _scissors():
    """two crossed blades (steel) over two red handle loops; blades point forward"""
    parts = []
    blades = []
    piv = np.array([R * 0.05, R * 0.38, 0.0])
    for s in (-1, 1):
        tip = np.array([R * 1.35, R * 0.3, s * R * 0.42])
        base = piv + [-R * 0.15, 0.0, -s * R * 0.08]
        wv = np.array([0, R * 0.22, 0])
        side = np.cross(tip - base, [0, 1, 0])
        side = side / np.linalg.norm(side) * R * 0.17
        top = face([base + wv * 0.4, tip + wv * 0.1, tip - side * 0.1, base - side], [0, -1, 0])
        top = (top[0], top[1] if top[1][1] > 0 else -top[1])
        edge = face([base - side, tip - side * 0.1, tip - side * 0.1 - wv * 0.6, base - side - wv * 0.7], piv + side * 3)
        blades += [top, edge]
    parts.append((blades, STEEL))
    loops = []
    for s in (-1, 1):
        c = np.array([-R * 0.72, R * 0.22, s * R * 0.45])
        loops += prism(c[0], c[2], R * 0.34, 0.0, R * 0.34, 6, r1=R * 0.3, rot=0.3)
    parts.append((loops, HANDLE))
    return parts


MODELS = [_rock(), _paper(), _scissors()]


def flatten(model):
    """-> (Q list, normals (k,3), colours (k,3)) for fast per-frame transforms"""
    Qs, Ns, Cs = [], [], []
    for faces, col in model:
        for Q, n in faces:
            Qs.append(np.asarray(Q, float))
            Ns.append(n)
            Cs.append(col)
    return Qs, np.array(Ns), np.array(Cs, float)


FLAT = [flatten(m) for m in MODELS]


def place(q, x, z, heading, scale=1.0, lift=0.0):
    """world faces of a team-q token at (x, z) facing `heading` (radians, in the xz plane, 0 = +x)"""
    Qs, Ns, Cs = FLAT[q]
    c, s = math.cos(heading), math.sin(heading)
    Rm = np.array([[c, 0, -s], [0, 1, 0], [s, 0, c]])       # local +x -> (cos h, 0, sin h)
    out = []
    for Q in Qs:
        out.append((Q * scale) @ Rm.T + [x, lift, z])
    return out, Ns @ Rm.T, Cs

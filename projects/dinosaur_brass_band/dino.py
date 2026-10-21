"""Low-poly Parasaurolophus built from rigid lofted segments (body+tail, 4 legs, neck, head+crest).

Only pivots move: neck and head (raise / honk / lower), legs swing for a simple walk, the whole
animal slides along a path. Local frame: the animal faces +x, y is up, z is its left/right.
Units are metres for the adult (scale 1.0); smaller animals are scaled copies with relatively
shorter crests (crest air length sets the note, see timeline.py)."""
import math
import numpy as np


# ------------------------------------------------------------------ small matrix helpers
def T(v):
    M = np.eye(4)
    M[:3, 3] = v
    return M


def S(s):
    M = np.eye(4)
    M[0, 0] = M[1, 1] = M[2, 2] = s
    return M


def Rz(a):
    c, s = math.cos(a), math.sin(a)
    M = np.eye(4)
    M[0, 0], M[0, 1], M[1, 0], M[1, 1] = c, -s, s, c
    return M


def Rhead(psi):
    """rotation about y that maps +x to (cos psi, 0, sin psi): psi > 0 turns toward +z (the camera)"""
    c, s = math.cos(psi), math.sin(psi)
    M = np.eye(4)
    M[0, 0], M[0, 2], M[2, 0], M[2, 2] = c, -s, s, c
    return M


# ------------------------------------------------------------------ geometry
class Part:
    """vertices + faces (index arrays) + per-face normal, colour and tag, in part-local coords"""
    def __init__(self):
        self.V = []
        self.F = []
        self.N = []
        self.C = []
        self.tag = []

    def add_face(self, pts, col, tag, center):
        i0 = len(self.V)
        self.V.extend([np.asarray(p, float) for p in pts])
        idx = np.arange(i0, i0 + len(pts))
        Q = np.array(pts, float)
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0])
        if np.linalg.norm(n) < 1e-9 and len(Q) > 3:
            n = np.cross(Q[2] - Q[0], Q[3] - Q[0])
        n /= np.linalg.norm(n) + 1e-12
        if np.dot(n, Q.mean(0) - np.asarray(center, float)) < 0:
            n = -n
        self.F.append(idx)
        self.N.append(n)
        self.C.append(np.asarray(col, float))
        self.tag.append(tag)

    def done(self):
        self.V = np.array(self.V)
        self.N = np.array(self.N)
        self.C = np.array(self.C)
        return self


def loft(part, secs, cols, tag, n=8, cap0=True, cap1=True, rot0=None, z0=0.0, stripe=None):
    """secs: [(x, y, ry, rz)] along a path in the xy plane. cols: (side, top, belly) colours.
    stripe: optional darker colour applied to every other segment's upper faces."""
    rot0 = math.pi / n if rot0 is None else rot0
    P = np.array([[s[0], s[1]] for s in secs], float)
    rings, ctrs = [], []
    for i, (x, y, ry, rz) in enumerate(secs):
        if i == 0:
            t = P[1] - P[0]
        elif i == len(secs) - 1:
            t = P[-1] - P[-2]
        else:
            t = P[i + 1] - P[i - 1]
        t = t / (np.linalg.norm(t) + 1e-12)
        nn = np.array([-t[1], t[0]])                  # in-plane normal (points "up" for a +x path)
        ring = []
        for k in range(n):
            a = rot0 + 2 * math.pi * k / n
            off = nn * ry * math.cos(a)
            ring.append([x + off[0], y + off[1], z0 + rz * math.sin(a)])
        rings.append(np.array(ring))
        ctrs.append(np.array([x, y, z0]))
    side, top, belly = cols
    for i in range(len(rings) - 1):
        a, b = rings[i], rings[i + 1]
        c = (ctrs[i] + ctrs[i + 1]) / 2
        for k in range(n):
            k1 = (k + 1) % n
            q = [a[k], a[k1], b[k1], b[k]]
            if np.linalg.norm(q[0] - q[1]) < 1e-6 and np.linalg.norm(q[2] - q[3]) < 1e-6:
                continue
            ang = rot0 + 2 * math.pi * (k + 0.5) / n
            up = math.cos(ang)
            col = top if up > 0.55 else (belly if up < -0.45 else side)
            if stripe is not None and i % 2 == 1 and up > -0.1:
                col = stripe
            part.add_face(q, col, tag, c)
    if cap0 and secs[0][2] > 1e-3:
        part.add_face(list(rings[0][::-1]), side, tag, ctrs[1])
    if cap1 and secs[-1][2] > 1e-3:
        part.add_face(list(rings[-1]), side, tag, ctrs[-2])
    return rings


def tint(c, k):
    return np.clip(np.asarray(c, float) * k, 0, 255)


# crest centre-line in head-local coords (x forward along the skull, y up); scaled by crest factor
CREST_PATH = [(0.14, 0.17), (-0.12, 0.27), (-0.45, 0.37), (-0.80, 0.45), (-1.10, 0.50), (-1.30, 0.50)]
CREST_R = [(0.13, 0.085), (0.12, 0.08), (0.115, 0.075), (0.11, 0.072), (0.10, 0.068), (0.06, 0.045)]
SNOUT = [(-0.08, 0.02, 0.20, 0.16), (0.22, 0.02, 0.22, 0.17), (0.52, -0.05, 0.16, 0.14), (0.82, -0.12, 0.105, 0.125), (1.04, -0.16, 0.06, 0.13)]


def crest_secs(cf):
    """crest loft sections; cf scales the length (and a little the thickness) of the crest"""
    out = []
    x0, y0 = CREST_PATH[0]
    for (x, y), (ry, rz) in zip(CREST_PATH, CREST_R):
        th = 0.75 + 0.25 * cf
        out.append((x0 + (x - x0) * cf, y0 + (y - y0) * cf, ry * th, rz * th))
    return out


def air_path(cf, n=60):
    """air passage centre-line (head-local): nostril -> up into the crest -> out to the tip -> back -> throat"""
    c = crest_secs(cf)
    out_leg = [(0.86, -0.08), (0.55, -0.0), (0.26, 0.09)] + [(x, y + 0.03) for x, y, *_ in c[:-1]]
    tip = c[-1]
    back_leg = [(x, y - 0.035) for x, y, *_ in c[-2::-1]] + [(-0.02, 0.10), (-0.06, -0.06)]
    P = np.array(out_leg + [(tip[0] + 0.02, tip[1])] + back_leg, float)
    seg = np.r_[0, np.cumsum(np.linalg.norm(np.diff(P, axis=0), axis=1))]
    u = np.linspace(0, seg[-1], n)
    return np.stack([np.interp(u, seg, P[:, 0]), np.interp(u, seg, P[:, 1])], 1), seg[-1]


class Dino:
    def __init__(self, scale, crest_f, pal, head_f=1.0, seed=0):
        self.s = scale
        self.cf = crest_f
        self.hf = head_f
        self.pal = pal
        body, stripe, belly, crest, beak = pal['body'], pal['stripe'], pal['belly'], pal['crest'], pal['beak']
        dark = tint(body, 0.82)
        # body + tail
        p = Part()
        secs = [(-5.8, 2.42, 0.05, 0.04), (-4.7, 2.58, 0.19, 0.13), (-3.6, 2.74, 0.37, 0.25), (-2.5, 2.84, 0.6, 0.42),
                (-1.4, 2.84, 0.83, 0.6), (-0.3, 2.64, 0.9, 0.66), (0.8, 2.3, 0.78, 0.6), (1.5, 2.08, 0.6, 0.5), (1.9, 2.1, 0.45, 0.4)]
        loft(p, secs, (body, tint(body, 0.9), belly), 'body', n=8, stripe=stripe)
        self.body = p.done()
        # legs: (pivot, path, radii)
        self.legs = []
        hind = [(0, 0.1, 0.62, 0.4), (0.35, -1.05, 0.4, 0.28), (-0.12, -1.92, 0.21, 0.17), (0.22, -2.42, 0.14, 0.17), (0.5, -2.5, 0.07, 0.16)]
        fore = [(0, 0, 0.3, 0.23), (0.1, -0.95, 0.2, 0.16), (0.16, -1.82, 0.13, 0.115), (0.32, -1.92, 0.07, 0.11)]
        for side in (-1, 1):
            for name, piv, secs_, col in (('hind', (-1.35, 2.55, 0.5 * side), hind, dark), ('fore', (1.1, 1.95, 0.4 * side), fore, tint(body, 0.78))):
                q = Part()
                loft(q, secs_, (col, tint(col, 1.05), tint(col, 0.85)), name, n=6)
                self.legs.append((name, side, np.array(piv), q.done()))
        # neck (pivots at its base, built along +x)
        self.neck_piv = np.array([1.45, 2.25, 0.0])
        self.neck_len = 1.55
        q = Part()
        loft(q, [(-0.25, -0.02, 0.6, 0.48), (0.45, 0.05, 0.45, 0.37), (1.05, 0.05, 0.34, 0.27), (self.neck_len + 0.06, 0, 0.27, 0.21)],
             (body, tint(body, 0.9), belly), 'neck', n=8, stripe=stripe)
        self.neck = q.done()
        # head: skull + crest + eyes, origin at the back of the skull (the head pivot)
        q = Part()
        loft(q, SNOUT, (body, tint(body, 0.92), belly), 'skull', n=8)
        # beak tip in a lighter colour: recolour the last ring of faces
        nb = 8
        for i in range(len(q.C) - nb - 1, len(q.C)):
            q.C[i] = np.asarray(beak, float)
        loft(q, crest_secs(crest_f), (crest, tint(crest, 1.08), tint(crest, 0.85)), 'crest', n=6, rot0=0.0)
        for side in (-1, 1):
            z = 0.168 * side
            e = [(0.24, 0.11, z), (0.33, 0.11, z), (0.33, 0.035, z), (0.24, 0.035, z)]
            q.add_face(e, (24, 20, 26), 'eye', (0.28, 0.07, 0))
            h = [(0.25, 0.105, z * 1.02), (0.285, 0.105, z * 1.02), (0.285, 0.075, z * 1.02), (0.25, 0.075, z * 1.02)]
            q.add_face(h, (250, 246, 230), 'eyehi', (0.27, 0.09, 0))
        self.head = q.done()
        self.air, self.air_len = air_path(crest_f)
        self.ct = self._ct_cards()

    def _ct_cards(self, n=22):
        """CT 'slices' across the skull+crest: (x, yc, ry, rz, hole) cards perpendicular to the head axis;
        hole = relative radius of the air passage seen in the cross-section"""
        lofts = [SNOUT, crest_secs(self.cf)]
        xs_all = np.concatenate([[s[0] for s in L] for L in lofts])
        xs = np.linspace(xs_all.min() + 0.03, xs_all.max() - 0.02, n)
        cards = []
        for x in xs:
            for li, L in enumerate(lofts):
                A = np.array(L)
                order = np.argsort(A[:, 0])
                A = A[order]
                if A[0, 0] <= x <= A[-1, 0]:
                    cards.append((x, np.interp(x, A[:, 0], A[:, 1]), np.interp(x, A[:, 0], A[:, 2]), np.interp(x, A[:, 0], A[:, 3]),
                                  0.55 if li == 1 else 0.38))
        return cards

    # ------------------------------------------------------------------ posing
    def mats(self, pos, heading, neck_a, head_a, swing=0.0, bob=0.0):
        """world matrices for every part. neck_a / head_a are angles above horizontal (radians)."""
        B = T(np.asarray(pos, float) + [0, bob, 0]) @ Rhead(heading) @ S(self.s)
        out = {'body': B}
        legs = []
        for k, (name, side, piv, part) in enumerate(self.legs):
            sw = swing * (1 if (name == 'hind') == (side > 0) else -1) * (0.7 if name == 'fore' else 1.0)
            legs.append(B @ T(piv) @ Rz(sw))
        out['legs'] = legs
        N = B @ T(self.neck_piv) @ Rz(neck_a)
        out['neck'] = N
        out['head'] = N @ T([self.neck_len, 0, 0]) @ Rz(head_a - neck_a) @ S(self.hf)
        return out

    def parts(self, M):
        """[(part, 4x4)] in draw-agnostic order"""
        out = [(self.body, M['body']), (self.neck, M['neck']), (self.head, M['head'])]
        out += [(lp[3], m) for lp, m in zip(self.legs, M['legs'])]
        return out

    def feet(self, M):
        """world xz of each foot (for contact AO) - the last loft point of each leg"""
        out = []
        for (name, side, piv, part), m in zip(self.legs, M['legs']):
            tip = part.V[-1]
            w = (m @ np.r_[tip, 1])[:3]
            out.append((w[0], w[2]))
        return out


def xf(M, V):
    return V @ M[:3, :3].T + M[:3, 3]


def xn(M, N):
    R = M[:3, :3]
    n = N @ R.T
    return n / (np.linalg.norm(n, axis=1, keepdims=True) + 1e-12)

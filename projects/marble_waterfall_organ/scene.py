"""The abstract void: no room, no ground. A deep indigo -> magenta -> teal gradient with dithered
nebula clouds whose colours shift with each chord, star glitter in parallax layers (real 3D points),
floating low-poly crystals and rings drifting past (they pulse on the kick), comets, and the small
star orb behind the machine (the key light). Everything here is periodic over 60 s, so it loops."""
import math
import numpy as np
from PIL import Image
from reelkit.config import dirv
from reelkit.geometry import face, icosa
import timeline as T

# ------------------------------------------------------------------ the key light (star orb behind the machine)
ORB_DIR = dirv(2, 7)            # where the orb is drawn (visible in the wide shots)
KEY = dirv(-4, 28)                # the light direction used for shading/shadows (a little higher: shadows read)
KEY_COL = np.array([0.95, 0.86, 0.72])

# ------------------------------------------------------------------ palettes
GRAD_TOP = np.array([40, 28, 104.0])
GRAD_MID = np.array([150, 56, 158.0])
GRAD_LOW = np.array([26, 118, 140.0])
# nebula cloud colours per harmony state: -1 (chaos, no chord), Bm, G, D, A
NEB_A = {-1: (255, 112, 186), 0: (255, 92, 210), 1: (40, 204, 172), 2: (255, 146, 84), 3: (92, 168, 255)}   # similar luminance
NEB_B = {-1: (74, 196, 255), 0: (136, 120, 255), 1: (56, 178, 236), 2: (255, 100, 168), 3: (172, 116, 255)}

_rng = np.random.default_rng(31)


def _noise(h, w, octaves, rng):
    out = np.zeros((h, w))
    for k, amp in octaves:
        g = rng.normal(0, 1, (k, 2 * k))
        g = np.concatenate([g, g[:, :1]], 1)                        # wrap in azimuth
        im = Image.fromarray(((g - g.min()) / (np.ptp(g) + 1e-9) * 255).astype(np.uint8)).resize((w + w // (2 * k), h), Image.BICUBIC)
        out += (np.asarray(im, float)[:, :w] / 255 - 0.5) * amp
    return out


NEB_H, NEB_W = 180, 360
NEB1 = np.clip(_noise(NEB_H, NEB_W, [(4, 1.0), (9, 0.6), (20, 0.32)], _rng) * 1.7 + 0.12, 0, 1)
NEB2 = np.clip(_noise(NEB_H, NEB_W, [(5, 1.0), (11, 0.5), (24, 0.3)], _rng) * 1.7 - 0.02, 0, 1)
DUST = np.clip(_noise(NEB_H, NEB_W, [(14, 1.0), (32, 0.6)], _rng) * 2.2, 0, 1)


def harmony_mix(t):
    """(colour A, colour B, brightness kick) for the nebula at time t, crossfading 0.35 s after each bar line"""
    t = t % T.DUR
    bar = int(t // T.BAR)
    c_now = T.chord_idx(bar)
    c_prev = T.chord_idx(bar - 1) if bar > 0 else -1
    if t >= T.T_LEVER:
        c_now = -1
        c_prev = T.chord_idx(28)
        u = min(1.0, (t - T.T_LEVER) / 1.5)
    else:
        u = min(1.0, (t - bar * T.BAR) / 0.7)
    A = np.array(NEB_A[c_prev], float) * (1 - u) + np.array(NEB_A[c_now], float) * u
    Bc = np.array(NEB_B[c_prev], float) * (1 - u) + np.array(NEB_B[c_now], float) * u
    return A, Bc


# ------------------------------------------------------------------ stars: three parallax layers of real points
def _stars(n, r0, r1, seed):
    r = np.random.default_rng(seed)
    v = r.normal(0, 1, (n, 3))
    v /= np.linalg.norm(v, axis=1, keepdims=True)
    d = r.uniform(r0, r1, n)
    return v * d[:, None], r.uniform(0.25, 1.0, n) ** 2.2, r.uniform(0, 2 * math.pi, n), r.integers(1, 5, n)


STARS = [_stars(500, 60, 90, 1), _stars(260, 26, 40, 2), _stars(90, 13, 19, 3)]
STAR_COLS = np.array([[255, 246, 230], [196, 226, 255], [255, 210, 240], [255, 236, 190]], float)

# ------------------------------------------------------------------ floating crystals + rings (drift on circles, whole cycles in 60 s)
CRYSTALS = []   # (kind, orbit r, phase, y, y-bob, size, colour, spin turns/60 s, orbit turns/60 s, seed)
_specs = [
    ('oct', 7.5, -150, 3.8, 0.6, 0.55, (255, 150, 220)), ('ico', 9.0, -120, -1.2, 0.5, 0.75, (120, 220, 255)),
    ('ring', 8.5, -30, 5.6, 0.4, 0.9, (255, 214, 120)), ('oct', 10.5, 20, 1.0, 0.7, 0.6, (180, 140, 255)),
    ('ico', 6.5, 118, 6.0, 0.4, 0.45, (130, 255, 200)), ('ring', 11.0, -95, -2.6, 0.5, 1.2, (120, 200, 255)),
    ('oct', 12.0, 120, -3.4, 0.6, 0.8, (255, 170, 120)), ('ico', 14.0, -60, 7.4, 0.5, 0.9, (255, 120, 190)),
    ('oct', 6.8, -165, 0.8, 0.3, 0.36, (255, 236, 160)), ('ring', 16.0, 60, 2.6, 0.8, 1.5, (220, 140, 255)),
    ('ico', 18.0, -10, -4.5, 0.7, 1.1, (110, 240, 230)), ('oct', 20.0, 175, 4.4, 0.6, 1.0, (255, 140, 160)),
    ('oct', 7.2, 95, 3.4, 0.25, 0.3, (200, 240, 255)), ('ico', 7.6, -85, 1.6, 0.25, 0.28, (255, 190, 230)),
]
for i, (k, r, ph, y, yb, s, c) in enumerate(_specs):
    CRYSTALS.append((k, r, ph, y, yb, s, c, (1 + i % 3) * (1 if i % 2 else -1), 1 if i % 4 == 0 else (-1 if i % 4 == 2 else 0), 50 + i))


def _rot(ax, a):
    ax = np.asarray(ax, float) / np.linalg.norm(ax)
    c, s = math.cos(a), math.sin(a)
    K = np.array([[0, -ax[2], ax[1]], [ax[2], 0, -ax[0]], [-ax[1], ax[0], 0]])
    return np.eye(3) * c + s * K + (1 - c) * np.outer(ax, ax)


_UNIT_OCT = None


def crystal_faces(spec, t):
    """faces + centre of one floating shape at time t"""
    k, r, ph, y, yb, s, c, spin, orb, seed = spec
    u = (t % T.DUR) / T.DUR
    a = math.radians(ph + 360.0 * orb * u + 6.0 * math.sin(2 * math.pi * 2 * u + seed))
    ctr = np.array([r * math.sin(a), y + yb * math.sin(2 * math.pi * 3 * u + seed), r * math.cos(a)])
    R = _rot([math.sin(seed), 1.0, math.cos(seed * 1.7)], 2 * math.pi * spin * u)
    if k == 'oct':
        V = np.array([[0, 1.5, 0], [0, -1.5, 0], [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]], float) * s
        F = [(0, 2, 4), (0, 4, 3), (0, 3, 5), (0, 5, 2), (1, 4, 2), (1, 3, 4), (1, 5, 3), (1, 2, 5)]
        V = V @ R.T + ctr
        return [face(V[list(f)], ctr) for f in F], ctr
    if k == 'ico':
        fs = icosa([0, 0, 0], s, jit=0.18, rng=np.random.default_rng(seed))
        return [(Q @ R.T + ctr, n @ R.T) for Q, n in fs], ctr
    # ring: 12 segments, square cross-section
    out = []
    n_ = 12
    rr, w = s, s * 0.16
    for j in range(n_):
        a0, a1 = 2 * math.pi * j / n_, 2 * math.pi * (j + 1) / n_
        for (r0, r1, h0, h1) in ((rr - w, rr + w, w, w), (rr + w, rr + w, w, -w), (rr - w, rr + w, -w, -w), (rr - w, rr - w, -w, w)):
            q = np.array([[r0 * math.cos(a0), h0, r0 * math.sin(a0)], [r1 * math.cos(a0), h1, r1 * math.sin(a0)],
                          [r1 * math.cos(a1), h1, r1 * math.sin(a1)], [r0 * math.cos(a1), h0, r0 * math.sin(a1)]])
            m = np.array([(rr) * math.cos((a0 + a1) / 2), 0, (rr) * math.sin((a0 + a1) / 2)])
            f_ = face(q, m)
            out.append((f_[0] @ R.T + ctr, f_[1] @ R.T))
    return out, ctr


# ------------------------------------------------------------------ comets (one every 7.5 s, periodic)
COMETS = []    # (t0, duration, az, el, sweep az, sweep el, colour); az 0 = behind the machine
_cr = np.random.default_rng(77)
for i in range(8):
    t0 = i * 7.5 + _cr.uniform(0.5, 3.0)
    COMETS.append((t0, 1.4, _cr.uniform(-45, 45), _cr.uniform(4, 34), _cr.uniform(14, 30) * (1 if i % 2 else -1),
                   _cr.uniform(-10, -3), tuple(STAR_COLS[i % 4])))

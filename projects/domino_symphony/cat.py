"""The cat: taps domino #1, wanders off, comes back for a near-miss over the spiral, watches the holdout,
bolts when the giant falls (crossing the frame lane just ahead of its wave), and rewinds with everything.
Generic ginger low-poly cat (adapted from count_the_thunder). It never knocks anything over."""
import math
import numpy as np
from reelkit.geometry import icosa, box, face
import layout as LY

S = 1.3                                   # size factor (body ~0.34 m long, head at ~0.26 m)
GINGER = (214, 128, 62)
STRIPE = (176, 94, 44)
CHEST = (244, 214, 170)
ROOT = LY.ROOT
TAP_SEAT = np.array([0.135, ROOT[1] + 0.20])
C_S = LY.C_S

# (tau, x, z, pose, facing yaw or None) - yaw 0 faces -z; poses: sit / walk / tap / startle
WAY = [
    (-9.0, *TAP_SEAT, 'tap', 0.0), (0.55, *TAP_SEAT, 'tap', 0.0),
    (0.6, *TAP_SEAT, 'sit', 0.0), (3.2, *TAP_SEAT, 'sit', 0.0),
    (3.3, *TAP_SEAT, 'walk', None), (5.0, 0.95, ROOT[1] + 0.62, 'walk', None), (7.2, 2.3, ROOT[1] + 1.05, 'walk', None),
    (10.5, 2.85, 4.4, 'walk', None), (24.0, 2.85, -2.7, 'walk', None), (26.6, 1.9, -3.3, 'walk', None),
    (29.0, 0.62, -3.15, 'walk', None), (29.6, 0.25, -3.10, 'walk', None), (30.3, -0.36, -3.17, 'walk', None),
    (30.9, -0.64, -3.62, 'walk', None), (31.4, -0.56, -4.15, 'walk', None), (31.6, -0.56, -4.15, 'sit', -2.44),
    (39.7, -0.56, -4.15, 'sit', -2.44), (39.75, -0.56, -4.15, 'startle', -2.44), (40.25, -0.56, -4.15, 'startle', -2.44),
    (40.3, -0.58, -4.12, 'walk', None), (41.6, -2.45, -3.35, 'walk', None),
    (42.0, -2.45, -3.35, 'sit', 1.9), (99.0, -2.45, -3.35, 'sit', 1.9),
]


def _seg(tau):
    for a, b in zip(WAY, WAY[1:]):
        if tau <= b[0]:
            return a, b
    return WAY[-2], WAY[-1]


def state(tau):
    """(pos xz, yaw, pose, walk phase, look yaw offset)"""
    a, b = _seg(tau)
    u = 0.0 if b[0] == a[0] else min(1.0, max(0.0, (tau - a[0]) / (b[0] - a[0])))
    pa, pb = np.array(a[1:3]), np.array(b[1:3])
    pose = a[3]
    if pose == 'walk' and b[3] == 'walk':
        p = pa + (pb - pa) * u
        d = pb - pa
        yaw = math.atan2(-d[0], -d[1]) if np.linalg.norm(d) > 1e-6 else 0.0
        # distance walked so far for the leg phase
        ph = (np.linalg.norm(d) * u + _walked(a)) / 0.11
        return p, yaw, 'walk', ph, 0.0
    if pose == 'walk':
        pose = b[3]
    yaw = a[4] if a[4] is not None else (b[4] if b[4] is not None else 0.0)
    look = 0.0
    if pose == 'sit' and tau < 4:
        look = 0.45 * math.sin(tau * 1.3)              # watching the fan burst
    if pose == 'sit' and 31 < tau < 40:
        look = 0.12 * math.sin(tau * 2.1)
    return pa.copy(), yaw, pose, 0.0, look


_WALK_CUM = {}


def _walked(seg_start):
    key = seg_start[0]
    if key not in _WALK_CUM:
        tot = 0.0
        for a, b in zip(WAY, WAY[1:]):
            _WALK_CUM[a[0]] = tot
            if a[3] == 'walk' and b[3] == 'walk':
                tot += float(np.hypot(b[1] - a[1], b[2] - a[2]))
    return _WALK_CUM.get(key, 0.0)


def paw_target(tau):
    """world position of the tapping paw tip (left fore leg) around tau = 0"""
    hover = np.array([0.012, 0.085, ROOT[1] + 0.07])
    touch = np.array([0.004, 0.044, ROOT[1] + 0.008])
    push = np.array([0.0, 0.040, ROOT[1] - 0.012])
    rest = np.array([TAP_SEAT[0] - 0.05, 0.012, TAP_SEAT[1] - 0.10])
    if tau <= -0.14:
        k = 0.004 * math.sin(tau * 9.0)
        return hover + np.array([0, k, 0])
    if tau <= 0.0:
        u = (tau + 0.14) / 0.14
        u = u * u
        return hover + (touch - hover) * u
    if tau <= 0.07:
        return touch + (push - touch) * (tau / 0.07)
    u = min(1.0, (tau - 0.07) / 0.45)
    u = u * u * (3 - 2 * u)
    return push + (rest - push) * u + np.array([0, 0.04 * math.sin(math.pi * u), 0])


def faces(tau, flinch=0.0):
    """-> (list of (Q, n), list of colours, centre)"""
    pos, yaw, pose, ph, look = state(tau)
    r2 = np.random.default_rng(11)
    f, cols = [], []

    def add(fs, col=GINGER):
        f.extend(fs)
        cols.extend([col] * len(fs))
    s = S
    if pose in ('sit', 'tap', 'startle'):
        fl = 1.0 if pose == 'startle' else flinch
        crouch = 1 - 0.14 * fl
        add(icosa([0, 0.11 * s * crouch, 0.02 * s], 0.1 * s, (0.85, 1.12 * crouch, 0.9), jit=0.07, rng=r2))
        add(icosa([0, 0.16 * s * crouch, -0.05 * s], 0.07 * s, (0.9, 1.0, 0.8), jit=0.05, rng=r2), CHEST)
        hc = np.array([0.0, 0.27 * s * crouch + 0.02 * fl, -0.06 * s])
        head = icosa(hc, 0.062 * s, (1.05, 0.92, 1.0), jit=0.05, rng=r2)
        hcols = [GINGER] * len(head)
        for sgn in (-1, 1):
            tilt = 0.035 * s * fl
            head.append(face([hc + [0.022 * s * sgn, 0.035 * s, 0.0], hc + [0.05 * s * sgn, 0.03 * s - tilt, 0.01 * s],
                              hc + [0.04 * s * sgn, 0.1 * s - tilt * 1.5, 0.0]], hc + [0, 0, 1]))
            hcols.append(GINGER)
        fb, fbc = _face_bits(hc, s)
        head += fb
        hcols += fbc
        lk = look
        R = np.array([[math.cos(lk), 0, math.sin(lk)], [0, 1, 0], [-math.sin(lk), 0, math.cos(lk)]])
        f.extend([((Q - hc) @ R.T + hc, n @ R.T) for Q, n in head])
        cols.extend(hcols)
        for sgn in (-1, 1):
            if pose == 'tap' and sgn < 0:
                continue
            sh = np.array([0.035 * s * sgn, 0.12 * s, -0.07 * s])
            foot = np.array([0.035 * s * sgn, 0.0, -0.09 * s])
            add(_limb(sh, foot, 0.016 * s))
        p = np.array([0.06, 0.02, 0.06]) * s
        for k in range(7):
            sw = 0.6 * math.sin(tau * 2.1 - k * 0.5) * (1 + 2 * fl)
            q = p + np.array([0.035 + 0.004 * k, 0.003 + 0.02 * fl, 0.02 * sw - 0.012]) * s
            w = (0.022 - 0.002 * k) * s
            add([face([p + [0, w, 0], p + [0, -0.004, 0], q + [0, -0.004, 0], q + [0, w * 0.9, 0]], p + [0, 0, 1]),
                 face([p + [0, w, 0], p + [0, w, w], q + [0, w * 0.9, w * 0.9], q + [0, w * 0.9, 0]], p + [0, 1, 0])],
                STRIPE if k % 2 else GINGER)
            p = q
    else:  # walk
        step = math.sin(ph * math.pi)
        add(icosa([0, 0.13 * s, 0], 0.085 * s, (0.8, 0.75, 1.55), jit=0.06, rng=r2))
        hc = np.array([0, 0.2 * s, -0.15 * s])
        add(icosa(hc, 0.06 * s, (1.0, 0.92, 1.05), jit=0.05, rng=r2))
        fb, fbc = _face_bits(hc + [0, 0, -0.004 * s], s)
        f.extend(fb)
        cols.extend(fbc)
        for sgn in (-1, 1):
            add([face([hc + [0.022 * s * sgn, 0.035 * s, 0.0], hc + [0.05 * s * sgn, 0.03 * s, 0.01 * s],
                       hc + [0.04 * s * sgn, 0.1 * s, 0.0]], hc + [0, 0, -1])])
        for (lx, lz, sg) in ((-0.04, -0.09, 1), (0.04, -0.09, -1), (-0.04, 0.09, -1), (0.04, 0.09, 1)):
            sw = 0.035 * step * sg
            lift = max(0.0, 0.03 * math.sin(ph * math.pi + (0 if sg > 0 else math.pi)))      # high careful steps
            add(box(lx * s, (0.05 + lift / 2) * s, (lz + sw) * s, 0.014 * s, 0.05 * s, 0.014 * s))
        p = np.array([0, 0.15, 0.12]) * s
        for k in range(6):
            q = p + np.array([0.0, 0.035, 0.03 - 0.006 * k]) * s
            w = (0.02 - 0.002 * k) * s
            add([face([p + [-w, 0, 0], p + [w, 0, 0], q + [w, 0, 0], q + [-w, 0, 0]], p + [0, 0, 1])], STRIPE if k % 2 else GINGER)
            p = q
    c, sn = math.cos(yaw), math.sin(yaw)
    Rm = np.array([[c, 0, sn], [0, 1, 0], [-sn, 0, c]])
    P3 = np.array([pos[0], 0.0, pos[1]])
    out = [(Q @ Rm.T + P3, n @ Rm.T) for Q, n in f]
    if pose == 'tap':
        sh = np.array([-0.035 * s, 0.12 * s, -0.07 * s]) @ Rm.T + P3
        lf = _limb(sh, paw_target(tau), 0.016 * s)
        out += lf
        cols += [GINGER] * len(lf)
    return out, cols, np.array([pos[0], 0.12 * S, pos[1]])


FACE_COL = {'eye': (34, 40, 30), 'nose': (236, 120, 130), 'muzzle': (246, 226, 196)}


def _face_bits(hc, s):
    """eyes, nose and a pale muzzle on the front (-z) of the head; tagged by a tiny normal tweak"""
    out = []
    z = hc[2] - 0.061 * s
    for sgn in (-1, 1):
        e = hc + np.array([0.024 * s * sgn, 0.012 * s, 0.0])
        e[2] = z + 0.004 * s
        out.append(('eye', face([e + [-0.009 * s, 0.006 * s, 0], e + [0.009 * s, 0.006 * s, 0], e + [0.006 * s, -0.006 * s, 0],
                                 e + [-0.006 * s, -0.006 * s, 0]], e + [0, 0, 1])))
    m = hc + np.array([0.0, -0.016 * s, 0.0])
    m[2] = z - 0.004 * s
    out.append(('muzzle', face([m + [-0.022 * s, 0.01 * s, 0], m + [0.022 * s, 0.01 * s, 0], m + [0.014 * s, -0.016 * s, 0],
                                m + [-0.014 * s, -0.016 * s, 0]], m + [0, 0, 1])))
    n = m + np.array([0, 0.004 * s, -0.002 * s])
    out.append(('nose', face([n + [-0.007 * s, 0.004 * s, 0], n + [0.007 * s, 0.004 * s, 0], n + [0, -0.006 * s, 0]], n + [0, 0, 1])))
    return [Fc for _, Fc in out], [FACE_COL[k] for k, _ in out]


def _limb(a, b, r):
    """a low-poly leg from shoulder a to paw b, plus a paw blob"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    up = np.array([0, 0, 1.0]) if abs(d[1]) > 0.7 * np.linalg.norm(d) else np.array([0, 1.0, 0])
    n1 = np.cross(d, up)
    n1 = n1 / (np.linalg.norm(n1) + 1e-9) * r
    n2 = np.cross(d, n1)
    n2 = n2 / (np.linalg.norm(n2) + 1e-9) * r
    ctr = (a + b) / 2
    out = []
    for e1, e2 in ((n1, n2), (n2, -n1), (-n1, -n2), (-n2, n1)):
        out.append(face([a + e1, b + e1, b + e2, a + e2], ctr))
    out += icosa(b, r * 1.25, (1.2, 0.8, 1.3), jit=0.05, rng=np.random.default_rng(3))
    return out

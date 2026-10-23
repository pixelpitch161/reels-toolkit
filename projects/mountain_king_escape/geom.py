"""Shared geometry of the rings and the ball (used by path.py, render.py, the checks).

The 12 rings are concentric, in the vertical plane z = 0, centred on C. Ring k (1 = innermost) has centre-line
radius R[k], radial thickness TH, depth 2*DZ and one gap of angular half-width GAP[k]. Ring k turns at
OMEGA[k] rad per beat (neighbours turn opposite ways), so the rings speed up with the music. Every ring's gap
points straight up (pi/2) at the moment the ball drops through it on the way in (the loop tail), so the
restart is consistent with frame 0; ring 12's speed is set so its gap is back on top at the final chord."""
import math
import numpy as np
import timeline as T

C = np.array([0.0, 3.9, 0.0])
NR = T.NRINGS
R = {k: 0.62 + 0.24 * (k - 1) for k in range(1, NR + 2)}
TH = 0.075
DZ = 0.075
RB = {k: 0.075 + 0.004 * (k - 1) for k in range(1, NR + 2)}     # ball radius while trapped in ring k
GAPW = {k: 4.0 * RB[k + 1] for k in range(1, NR + 1)}          # world width of ring k's gap
GAP = {k: GAPW[k] / 2 / R[k] for k in range(1, NR + 1)}        # angular half-width
RHO = {k: R[k] - TH / 2 - RB[k] for k in range(1, NR + 1)}     # ball-centre radius at a bounce
SIGN = {k: (1 if k % 2 else -1) for k in range(1, NR + 1)}
_OM = [0.30, 0.26, 0.28, 0.24, 0.27, 0.23, 0.26, 0.22, 0.25, 0.21, 0.24, 0.25]
DIR = {k: (-1 if k % 2 else 1) for k in range(1, NR + 1)}      # the ball circulates against its ring

# ------------------------------------------------------------------ the drop-in (loop tail)
G_DROP = 12.0
Y_TOP = C[1] + R[NR] + 0.35
M_TOP = T.M0 - 0.62                       # ball directly above ring 12, falling, at this m
Y_BOT = C[1] - RHO[1]                     # first bounce: bottom of ring 1 at m = M0
_TD = T.M0 - M_TOP
V0_DROP = (Y_TOP - Y_BOT) / _TD - G_DROP * _TD / 2      # downward speed at M_TOP


def drop_y(m):
    u = m - M_TOP
    return Y_TOP - V0_DROP * u - 0.5 * G_DROP * u * u


def _align_m(k):
    """m at which the falling ball passes ring k's (top) gap"""
    y = C[1] + R[k]
    a, b, c = 0.5 * G_DROP, V0_DROP, y - Y_TOP     # Y_TOP - V0 u - g/2 u^2 = y
    u = (-b + math.sqrt(b * b - 4 * a * c)) / (2 * a)
    return M_TOP + u


M_ALIGN = {k: _align_m(k) for k in range(1, NR + 1)}
B_ALIGN = {k: float(T.beat_of_m(M_ALIGN[k])) for k in range(1, NR + 1)}
OMEGA = {k: _OM[k - 1] for k in range(1, NR + 1)}
_db = float(T.beat_of_m(T.M_FINAL)) - B_ALIGN[NR]
OMEGA[NR] = 2 * math.pi * round(OMEGA[NR] * _db / (2 * math.pi)) / _db     # gap on top at the final chord


def phase(k, m):
    """gap-centre angle of ring k at music time m (radians, 0 = +x, pi/2 = up)"""
    return math.pi / 2 + SIGN[k] * OMEGA[k] * (float(T.beat_of_m(m)) - B_ALIGN[k])


def _first_m(k):
    """m of the ball's first hit on ring k (the note after ring k-1's escape)"""
    i = [j for j, n in enumerate(T.NOTES) if n['ring'] == k][0]
    return T.NOTES[i]['m']


# Hand-off rule: when ring k breaks, ring k+1's gap must be well away from ring k's gap, or the escaping
# ball would fly straight through both. Pick each ring's speed (outermost first; ring 12 is fixed) as close
# to its planned value as allows >= 0.9 rad between the two gaps at the hand-off.
for _k in range(NR - 1, 0, -1):
    _mb, _mf = T.RING_BREAK[_k], _first_m(_k + 1)
    for _d in sorted(np.arange(-0.04, 0.0401, 0.0025), key=abs):
        OMEGA[_k] = _OM[_k - 1] + _d
        if abs((phase(_k, _mb) - phase(_k + 1, _mf) + math.pi) % (2 * math.pi) - math.pi) >= 0.9:
            break


def ring_m(t):
    """music time that drives the ring phases at video time t: the re-formed rings (from T_REFORM0)
    already belong to the restart"""
    return t - T.DUR if t >= T.T_REFORM0 else t


def angdist(a, b):
    return abs((a - b + math.pi) % (2 * math.pi) - math.pi)


def pol(rad, ang):
    return np.array([C[0] + rad * math.cos(ang), C[1] + rad * math.sin(ang), 0.0])


# ------------------------------------------------------------------ ring meshes
NSEG = {k: 20 + 4 * k for k in range(1, NR + 1)}


def ring_segments(k, ph):
    """list of segment blocks (a0, a1) in radians for ring k with its gap centred on ph"""
    n = NSEG[k]
    g = GAP[k]
    span = 2 * math.pi - 2 * g
    a = ph + g
    return [(a + span * i / n, a + span * (i + 1) / n) for i in range(n)]


def seg_faces(k, a0, a1, explode=0.0):
    """faces of one ring block: front, back, outer, inner (+ the gap end caps by the caller)"""
    r0, r1 = R[k] - TH / 2, R[k] + TH / 2
    c0, s0, c1, s1 = math.cos(a0), math.sin(a0), math.cos(a1), math.sin(a1)
    cx, cy = C[0], C[1]
    P = lambda r, c, s, z: (cx + r * c, cy + r * s, z)
    fi0, fo0, fo1, fi1 = P(r0, c0, s0, DZ), P(r1, c0, s0, DZ), P(r1, c1, s1, DZ), P(r0, c1, s1, DZ)
    bi0, bo0, bo1, bi1 = P(r0, c0, s0, -DZ), P(r1, c0, s0, -DZ), P(r1, c1, s1, -DZ), P(r0, c1, s1, -DZ)
    am = (a0 + a1) / 2
    nr = np.array([math.cos(am), math.sin(am), 0.0])
    return [
        (np.array([fi0, fo0, fo1, fi1]), np.array([0, 0, 1.0]), 'front'),
        (np.array([bi1, bo1, bo0, bi0]), np.array([0, 0, -1.0]), 'back'),
        (np.array([fo0, bo0, bo1, fo1]), nr, 'outer'),
        (np.array([fi1, bi1, bi0, fi0]), -nr, 'inner'),
    ]


def cap_faces(k, a, side):
    """end cap of the block next to the gap at angle a; side +1 faces increasing angle"""
    r0, r1 = R[k] - TH / 2, R[k] + TH / 2
    c, s = math.cos(a), math.sin(a)
    P = lambda r, z: (C[0] + r * c, C[1] + r * s, z)
    n = np.array([-s, c, 0.0]) * side
    return (np.array([P(r0, DZ), P(r1, DZ), P(r1, -DZ), P(r0, -DZ)]), n, 'cap')

"""Pixel Pitch: the modern euphonium - low-poly model (and, below, its voice). No history video yet: built for BALLFIGHT
#12 (Bass Clarinet vs Euphonium) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the
other instruments' modern showcase models are. (tuba60_lowpoly.py's 1843 "euphonium" history scene is a small silver
tuba from that video and stays as it is.) A later Pixel Pitch euphonium video can grow from this file.

    python3 euphonium_lowpoly.py            # face count + extent

Model from reference photos (Wikimedia Commons, found in Chrome, 2026-10-26):
  - "Yamaha Euphonium YEP-621 transparent.png" (its description: a YEP-321, B-flat, yellow brass, four pistons), a
    straight-on front product shot, 1062 x 1798 px. Scaled with Yamaha's published YEP-321 numbers (bell 280 mm, height
    664 mm): the rim spans 712 px and the instrument 1755 px tall, so 0.385 mm per px (the rim reads 274 mm, the height
    676 mm). Measured: the upright bell stack on the right (62 mm across at the bottom bow, 80 mm half-way up, flaring
    from ~250 mm below the rim), the big bottom bow (~225 mm wide, 42-58 mm tubing), the left branch rising to the top
    arch over the valves (22 mm tubing, the arch 146 mm wide), three top-action pistons in a row in front (27 mm
    casings, 31 mm apart, finger buttons on 22 mm stems), the 4th valve mounted diagonally low on the left with its
    button pointing up-left, the 1st/2nd and 3rd valve slides hanging under the block, the 4th valve's long U slide
    inside the bottom bow, the leadpipe crossing in front of the bell stack to the silver mouthpiece 110 mm right of the
    rim's edge, ring bands on the bow legs.
  - "Euphonium.jpg" (B&S 3171A, a four-valve inline euphonium, side view): the same bell / bow / arch layout from the
    side: the valves sit in front of the main loop, the bell stack and the loop share one plane."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
from pp_framework import lerp, dim, mix

OUT, TAG = "euphonium_lowpoly.mp4", "eu"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE EUPHONIUM"

_NOTES = [(0, 58, 2, .85), (2, 62, 1, .85), (3, 65, 1, .85), (4, 63, 2, .85), (6, 58, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="EUPHONIUM", credit="4-VALVE EUPHONIUM", notes=_NOTES),
    # the same instrument for small sprites (BALLFIGHT balls): tubing / casings / keys x `chunk`, the bell's flare as is
    dict(key="ball", theme="intro", name="EUPHONIUM", credit="4-VALVE EUPHONIUM", notes=_NOTES, chunk=1.35),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- the instrument (real proportions, metres)
# local frame = the photo's: +y up (y = 0 at the bottom of the bow), +x toward the bell / mouthpiece side, +z toward the
# camera (the valves' side)
BRASS = (230, 182, 88)          # yellow-brass lacquer
BRASS_D = (196, 146, 60)
NICKEL = (214, 216, 226)        # mouthpiece
PEARL = (244, 236, 218)         # finger-button inlays
HOLE = (30, 18, 10)
PX = 0.000385                   # metres per photo px (YEP-321: 280 mm bell, 664 mm tall)
OX, OY = 380.0, 1790.0          # photo px of the origin (the bow's middle, its bottom)
ZV = .036                       # the valve block's plane, in front of the main loop
CH = [1.0]


def P(px, py, z=0.0):
    return np.array([(px - OX) * PX, (OY - py) * PX, z])


def unit(v):
    v = np.asarray(v, float)
    return v / np.linalg.norm(v)


def path(g, pts, r0, col, r1=None, n=8, spec=.9, two=False):
    """tube swept along a polyline with a rotation-minimising frame (as tuba60's), radius r0 -> r1"""
    P_ = np.array([np.asarray(p, float) for p in pts])
    N = len(P_)
    r1 = r0 if r1 is None else r1
    tg = np.gradient(P_, axis=0)
    tg /= np.linalg.norm(tg, axis=1)[:, None]
    a = np.array([0, 0, 1.]) if abs(tg[0][2]) < .9 else np.array([1., 0, 0])
    u = np.cross(tg[0], a)
    u /= np.linalg.norm(u)
    rings = []
    for i in range(N):
        u = u - np.dot(u, tg[i]) * tg[i]
        u /= np.linalg.norm(u)
        v = np.cross(tg[i], u)
        r = lerp(r0, r1, i / (N - 1))
        rings.append([P_[i] + r * (math.cos(2 * math.pi * k / n) * u + math.sin(2 * math.pi * k / n) * v) for k in range(n)])
    for i in range(N - 1):
        ref = (P_[i] + P_[i + 1]) / 2
        A, B = rings[i], rings[i + 1]
        for k in range(n):
            g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col, ref=ref, spec=spec, two=two)


def arc(cx, cy, r, a0, a1, segs=12, z=0.0, rx=None):
    rx = r if rx is None else rx
    return [P(cx + rx * math.cos(math.radians(a)), cy - r * math.sin(math.radians(a)), z) for a in np.linspace(a0, a1, segs + 1)]


def bell(g):
    """the upright bell stack (axis at px x 645) from the bottom bow to the 274 mm rim; returns the mouth + its axis"""
    c = CH[0]
    prof = [(1450, 80), (1200, 82), (1000, 87), (800, 94), (600, 104), (450, 118), (330, 140), (240, 168), (170, 205),
            (120, 248), (85, 295), (60, 330), (44, 356)]
    for (y0, r0), (y1, r1) in zip(prof, prof[1:]):
        q0 = r0 * PX * (c if r0 < 150 else 1.0)
        q1 = r1 * PX * (c if r1 < 150 else 1.0)
        T.tube(g, P(645, y0), P(645, y1), q0, q1, col=BRASS, n=16 if y1 < 300 else 12, spec=.95, two=y1 < 300)
    rim = P(645, 40)
    up = np.array([0, 1., 0])
    T.tube(g, rim - up * .006, rim + up * .003, 360 * PX, col=BRASS, n=18, spec=.95, two=True)        # the rolled rim
    g.face(S.ring(rim - up * .05, up, 150 * PX, 14), HOLE, ref=rim - up, emit=True, two=True)        # the throat
    for py in (1400, 1425):                                                                       # ring bands
        T.tube(g, P(645, py), P(645, py - 9), 86 * PX * c, col=BRASS_D, n=12, spec=.9)
    return rim, up


def bows(g):
    """the big bottom bow from the bell stack round to the left branch, up to the top arch and into the valves"""
    c = CH[0]
    bow = [P(645, 1450)] + arc(350, 1450, 295, 0, -180, 16)[1:] + [P(55, 1420)]
    path(g, bow, 78 * PX * c, BRASS, r1=46 * PX * c, n=12, spec=.95)
    for py in (1420, 1445):
        T.tube(g, P(55, py), P(55, py - 8), 52 * PX * c, col=BRASS_D, n=10, spec=.9)
    left = [P(55, 1420), P(55, 1200), P(62, 900), P(72, 600), P(80, 430)]
    path(g, left, 46 * PX * c, BRASS, r1=30 * PX * c, n=10, spec=.95)
    top = arc(270, 430, 190, 180, 0, 14)
    path(g, top, 30 * PX * c, BRASS, r1=28 * PX * c, n=10, spec=.95)
    down = [P(460, 430), P(460, 560, .01), P(440, 640, ZV * .6), P(400, 700, ZV)]
    path(g, down, 28 * PX * c, BRASS, n=10, spec=.95)
    # the braces between the left branch, the valves and the stack
    for a, b in ((P(80, 700), P(190, 700, ZV)), (P(400, 820, ZV), P(570, 820)), (P(470, 1300), P(570, 1300))):
        T.tube(g, a, b, 7 * PX * c, col=BRASS_D, n=5, spec=.9)


def valves(g, down=()):
    """three top-action pistons in a row in front of the loop + the 4th on the left, diagonal; `down` = pressed (0-3)"""
    c = CH[0]
    for i, px in enumerate((228, 308, 387)):
        dn = .010 if i in down else 0.0
        T.tube(g, P(px, 595, ZV), P(px, 875, ZV), 35 * PX * c, col=BRASS, n=12, spec=.95)          # casing
        for y0, y1 in ((575, 598), (872, 895)):                                                    # top / bottom caps
            T.tube(g, P(px, y0, ZV), P(px, y1, ZV), 41 * PX * c, col=BRASS_D, n=12, spec=.95)
        T.tube(g, P(px, 575, ZV), P(px, 520 + dn / PX, ZV), 6 * PX * c, col=BRASS_D, n=6, spec=.9)   # stem
        bt = P(px, 520 + dn / PX, ZV)
        T.tube(g, bt, bt + [0, .007, 0], 30 * PX * c, col=BRASS, n=12, spec=.9)                     # finger button
        g.face(S.ring(bt + [0, .0072, 0], (0, 1, 0), 22 * PX * c, 10), PEARL, ref=bt, spec=.5)
    # the 4th valve, mounted diagonally low on the left, its button pointing up-left
    a, b = P(320, 1120, ZV * .6), P(50, 965, ZV * .6)
    T.tube(g, a, b, 33 * PX * c, col=BRASS, n=12, spec=.95)
    d = unit(b - a)
    for p in (a, b):
        T.tube(g, p - d * .006, p + d * .006, 39 * PX * c, col=BRASS_D, n=12, spec=.95)
    T.tube(g, b, b + d * .022, 6 * PX * c, col=BRASS_D, n=6)
    T.tube(g, b + d * .022, b + d * .029, 28 * PX * c, col=BRASS, n=12, spec=.9)
    g.face(S.ring(b + d * .0295, d, 20 * PX * c, 10), PEARL, ref=b, spec=.5)


def slides(g):
    """valve slides: the 1st/2nd U under the block, the 3rd on the right, the 4th's long U inside the bottom bow"""
    c = CH[0]
    r = 17 * PX * c
    for x0, x1, ya, yb, z in ((130, 205, 860, 1150, ZV * .7), (372, 497, 860, 1080, ZV * .7)):
        cx = (x0 + x1) / 2
        rr = (x1 - x0) / 2
        u = [P(x0, ya, z), P(x0, yb, z)] + arc(cx, yb, rr, 180, 360, 8, z)[1:] + [P(x1, ya, z)]
        path(g, u, r, BRASS, n=8, spec=.95)
    u = [P(110, 1150, .012), P(110, 1420, .012)] + arc(305, 1420, 195, 180, 360, 12, .012)[1:] + [P(500, 1150, .012)]
    path(g, u, 25 * PX * c, BRASS, n=10, spec=.95)
    for py in (1290, 1340):                                                                       # slide braces
        T.tube(g, P(110, py, .012), P(240, py, .012), 6 * PX * c, col=BRASS_D, n=5)
    T.tube(g, P(130, 1150, .012), P(130, 1100, ZV * .6), 18 * PX * c, col=BRASS, n=8)
    T.tube(g, P(500, 1150, .012), P(470, 1100, ZV * .5), 18 * PX * c, col=BRASS, n=8)
    # water key on the bottom bow
    wk = P(420, 1795, .02)
    T.tube(g, wk, wk + [.03, .004, .006], 5 * PX * c, col=BRASS_D, n=5)


def leadpipe(g):
    """from the valve block across in front of the bell stack to the silver mouthpiece; returns the mouthpiece rim"""
    c = CH[0] * (1.3 if CH[0] > 1 else 1.0)
    pts = [P(400, 760, ZV), P(500, 748, ZV + .012), P(640, 712, ZV + .03), P(800, 680, ZV + .03), P(925, 662, ZV + .022)]
    path(g, pts, 16 * PX * c, BRASS, r1=11 * PX * c, n=8, spec=.95)
    a, b = P(925, 662, ZV + .022), P(1048, 655, ZV + .018)
    d = unit(b - a)
    T.tube(g, a - d * .006, a + d * .008, 15 * PX * c, col=BRASS_D, n=8)                         # receiver
    T.tube(g, a, b - d * .012, 9 * PX * c, 12 * PX * c, col=NICKEL, n=8, spec=.95)               # shank
    T.tube(g, b - d * .012, b, 12 * PX * c, 30 * PX * c, col=NICKEL, n=10, spec=.95)             # cup
    T.tube(g, b, b + d * .004, 31 * PX * c, col=NICKEL, n=10, spec=.95)
    return b


def rot_x(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[1, 0, 0], [0, c, -s_], [0, s_, c]])


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


POSE = rot_x(math.radians(14))      # upright, the bell tipped a little toward the camera so its mouth reads


def xform(g, f0, l0, R=None, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]
        if R is not None:
            P_ = P_ @ R.T
            n = R @ n
        g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]
        if R is not None:
            a, b = R @ a, R @ b
        g.L[i] = (a + t, b + t, col, gid)


def euphonium(g, down=()):
    """the whole instrument in its local frame; returns (bell mouth, bell axis, mouthpiece)"""
    mouth, up = bell(g)
    bows(g)
    valves(g, down)
    slides(g)
    mp = leadpipe(g)
    return mouth, up, mp


def build(g, t, ev, fx, s):
    f0, l0 = len(g.F), len(g.L)
    CH[0] = float(s.get("chunk", 1.0))
    mouth, up, mp = euphonium(g)
    CH[0] = 1.0
    cen = P(500, 900)
    o = np.array([0, .40, 0]) - POSE @ cen
    xform(g, f0, l0, R=POSE, t=o)
    mouth = POSE @ mouth + o
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(mouth + [.12, .06, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(mouth + [0, .02, 0]), age, 0))


# ---------------------------------------------------------------- sound: the euphonium (warm, round, singing)
# A conical tenor-tuba bore: every harmonic, a strong fundamental, the upper partials rolling off early (warmer and
# darker than a trombone or a baritone horn), with a soft attack, a gentle lip scoop and a slow, singing vibrato. Louder =
# brighter. Pitch may be fractional. Range Bb1-Bb4 and up (midi 34-70+); the singing tenor register sits around 53-70.
SR = 44100


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def euph(f, dur, v, bright=.45, vib=.0045, vrate=5.0, scoop=.018, att=.04):
    n = int((dur + .16) * SR)
    t = np.arange(n) / SR
    env = np.minimum(1, t / att) ** 1.4 * np.where(t < dur, 1, np.exp(-(t - dur) / .06)) * (.9 + .1 * np.exp(-t * 3))
    fr = f * (1 - scoop * np.exp(-t / .03)) * (1 + vib * np.sin(2 * np.pi * vrate * t) * np.clip((t - .22) / .3, 0, 1))
    ph = 2 * np.pi * np.cumsum(fr) / SR
    dark = np.zeros(n)
    brt = np.zeros(n)
    sd = sb = 0.0
    for k in range(1, 36):
        fk = f * k
        if fk > 7000:
            break
        a1 = math.exp(-k / 1.9) / k ** .9 * (1.25 if k == 1 else 1.0)
        a2 = math.exp(-k / (3.2 + 5.5 * bright)) / k ** .8 * (1.1 if k == 1 else 1.0)
        a2 *= 1 + .5 * math.exp(-((fk - 1100) / 500) ** 2)                 # a touch of brass "ring" when pushed
        s_ = np.sin(k * ph + .2 * k)
        dark += a1 * s_
        brt += a2 * s_
        sd += a1
        sb += a2
    mb = np.clip(env * v, 0, 1) ** 1.35
    x = dark / sd * (1 - mb) + brt / sb * mb
    return x * env * v


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return euph(_midi_hz(m), dur, v)


if __name__ == "__main__":
    import piano_lowpoly as Pn
    for sc in SCENES:
        g = Pn.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces, extent (m):", np.round(V.max(0) - V.min(0), 3))

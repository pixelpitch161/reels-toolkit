"""Pixel Pitch: the modern bass clarinet - low-poly model (and, below, its voice). No history video yet: built for
BALLFIGHT #12 (Bass Clarinet vs Euphonium) and used read-only through projects/which_instrument_wins/pp_bridge.py, the
same way the other instruments' modern showcase models are. A later Pixel Pitch bass clarinet video can grow from this
file (add SCENES rows, backdrop, music) without touching the model.

    python3 bassclarinet_lowpoly.py         # face count + extent (sprites: projects/which_instrument_wins/sprites.py)

Model from reference photos (Wikimedia Commons, found in Chrome, 2026-10-26):
  - "Yamaha Bass Clarinet YCL-622II.jpg" (a low-C professional model): a straight-on product shot, 983 x 4021 px, the
    keys facing the camera, the neck turned sideways in its socket (the usual product-shot pose). Scaled to a 40 mm
    grenadilla body (137 px across -> 0.29 mm per px): the wood runs 1.0 m from the neck socket ring to the bottom
    ferrule; the tenon ring at 37.6 % up from the bottom of the wood (px 1500); a silver bottom ferrule 44 mm long; the
    silver U-bow turns up on the left side into a tilted bell (rim 104 mm across, the mouth facing up, left and toward
    the camera); the floor peg's black rubber tip under the bow beside the low-C cup. The silver neck rises 45 mm out of
    the socket, loops over (top 58 mm above the wood) and comes down the right side, then a U turns it up-right into
    the black mouthpiece (silver ligature, the reed underneath), whose tip sits 140 mm right of the bore axis.
    Keywork: three covered left-hand plates (the first perforated), the left-pinky spatula cluster at px 1240-1300,
    three right-hand plates, the right-pinky spatulas at px 2050-2110, two big open pad cups at px 2270 / 2620, black
    thumb-rest / strap brackets at px 2165 / 2745, long rods down both sides, the low-C cup above the peg.
  - "BC basscl Prestige 1193.jpg" (a Buffet Prestige low C, 3/4 view): the same layout from the other side - the neck
    curling back over the top, the bell turning up and forward, the peg's black tip under the bow."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
from pp_framework import lerp, dim, mix

OUT, TAG = "bassclarinet_lowpoly.mp4", "bc"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE BASS CLARINET"

# one scene for now: the modern low-C bass clarinet (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 46, 2, .85), (2, 50, 1, .85), (3, 53, 1, .85), (4, 51, 2, .85), (6, 46, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="BASS CLARINET", credit="LOW-C BASS CLARINET", notes=_NOTES),
    # the same instrument for small sprites (BALLFIGHT balls, ~40-90 px long): true lengths, every tube / key radius x
    # `chunk` (the neck and mouthpiece x 1.2 more, the bell rim x chunk ** 0.75), so the black body, the silver neck and the keys still read as more than a
    # one-pixel line. A Pixel Pitch video would use "modern".
    dict(key="ball", theme="intro", name="BASS CLARINET", credit="LOW-C BASS CLARINET", notes=_NOTES, chunk=2.0),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- the instrument (real proportions, metres)
# local frame = the photo's: +y up the body (y = 0 at the bottom of the wood), +x toward the mouthpiece side, +z toward
# the camera (the keys' side)
BLACK = (30, 26, 30)            # grenadilla
NICKEL = (214, 216, 226)        # silver-plated keys, neck, ferrules and bell
RUBBER = (24, 22, 24)           # the peg's tip, the thumb-rest brackets, the mouthpiece
REED_C = (226, 204, 148)
HOLE = (14, 10, 12)
PX = 0.00029                    # metres per photo px (a 40 mm body = 137 px)
BX, BY = 470.0, 3650.0          # photo px of the bore axis / the bottom of the wood
RB = .0200                      # body radius (40 mm)
Y_TOP = (BY - 205) * PX         # .999: the neck socket ring
Y_TEN = (BY - 1500) * PX        # .624: the tenon ring
CH = [1.0]                      # sprite chunk factor (set by build(); radii only)


def P(px, py, z=0.0):
    """photo px -> model metres"""
    return np.array([(px - BX) * PX, (BY - py) * PX, z])


def ypx(py):
    return (BY - py) * PX


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
        r = lerp(r0, r1, i / (N - 1)) if callable(lerp) else r0
        rings.append([P_[i] + r * (math.cos(2 * math.pi * k / n) * u + math.sin(2 * math.pi * k / n) * v) for k in range(n)])
    for i in range(N - 1):
        ref = (P_[i] + P_[i + 1]) / 2
        A, B = rings[i], rings[i + 1]
        for k in range(n):
            g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col, ref=ref, spec=spec, two=two)


def smooth(pts, k=3):
    """Catmull-Rom-ish resample of a few control points into a smooth polyline (k points per span)"""
    P_ = [np.asarray(p, float) for p in pts]
    out = []
    for i in range(len(P_) - 1):
        p0, p1, p2, p3 = P_[max(0, i - 1)], P_[i], P_[i + 1], P_[min(len(P_) - 1, i + 2)]
        for j in range(k):
            t = j / k
            t2, t3 = t * t, t * t * t
            out.append(.5 * (2 * p1 + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 + (-p0 + 3 * p1 - 3 * p2 + p3) * t3))
    out.append(P_[-1])
    return out


def ring_band(g, y, r, w=.006, col=NICKEL):
    T.tube(g, (0, y, 0), (0, y + w, 0), r, col=col, n=12, spec=.85)


def body(g):
    c = CH[0]
    T.tube(g, (0, 0, 0), (0, Y_TEN, 0), RB * c, col=BLACK, n=12, spec=.45)                 # lower joint (+ low extension)
    T.tube(g, (0, Y_TEN + .006, 0), (0, Y_TOP, 0), RB * .985 * c, col=BLACK, n=12, spec=.45)   # upper joint
    ring_band(g, Y_TEN - .002, RB * 1.04 * c, .009)                                        # the tenon ring
    T.tube(g, (0, Y_TOP, 0), (0, Y_TOP + .013, 0), RB * .88 * c, RB * .80 * c, col=NICKEL, n=12, spec=.9)   # socket ring
    for dy in (.003, .008):
        T.tube(g, (0, Y_TOP + dy, 0), (0, Y_TOP + dy + .0015, 0), RB * .92 * c, col=dim(NICKEL, 1.05), n=12, spec=.9)
    T.tube(g, (0, .0, 0), (0, -.044, 0), RB * 1.05 * c, col=NICKEL, n=12, spec=.9)            # bottom ferrule (the boot)
    for y in (-.004, -.040):
        T.tube(g, (0, y, 0), (0, y + .004, 0), RB * 1.12 * c, col=NICKEL, n=12, spec=.9)


def neck(g):
    """the silver neck: up out of the socket, over the top and down the right side, a U, then up-right into the
    mouthpiece (photo px centreline, a little forward in z toward the mouthpiece so it reads in 3D)"""
    c = CH[0] * (1.2 if CH[0] > 1 else 1.0)
    ctrl = [(492, 205, 0), (492, 150, 0), (496, 92, .002), (512, 42, .004), (545, 16, .006), (585, 14, .008),
            (617, 38, .010), (638, 85, .012), (652, 150, .014), (666, 218, .016), (684, 275, .018), (712, 314, .02),
            (748, 322, .022), (774, 300, .024), (792, 272, .026)]
    pts = smooth([P(x, y, z) for x, y, z in ctrl], 3)
    path(g, pts, .0090 * c, NICKEL, r1=.0078 * c, n=8, spec=.95)
    # the neck's octave key: a little pad cup and its lever along the neck's base
    a, b = P(500, 120, .006 * c), P(560, 228, .008)
    T.tube(g, a, b, .0012 * c, col=NICKEL, n=4, spec=.9)
    S.prism(g, P(568, 232, .012 * c), unit([.6, -.5, .6]), .0045 * c, .002, 8, NICKEL, dim(NICKEL, .8))
    return pts[-1], unit(pts[-1] - pts[-2])


def mouthpiece(g, base, d):
    """black hard-rubber mouthpiece (cork at the joint, silver ligature), the reed on its underside, the beak tip"""
    c = CH[0] * (1.2 if CH[0] > 1 else 1.0)
    tip = P(965, 140, .03)
    d = unit(tip - base)
    L = float(np.linalg.norm(tip - base))
    T.tube(g, base - d * .004, base + d * .010, .0098 * c, .0098 * c, col=(176, 140, 96), n=8, spec=.2)    # cork
    T.tube(g, base + d * .010, base + d * .52 * L, .0105 * c, .0102 * c, col=RUBBER, n=10, spec=.55)
    T.tube(g, base + d * .52 * L, tip - d * .004, .0102 * c, .0045 * c, col=RUBBER, n=10, spec=.55)       # the beak
    for f in (.30, .46):                                                                            # ligature bands
        T.tube(g, base + d * f * L, base + d * (f + .06) * L, .0115 * c, col=NICKEL, n=10, spec=.95)
    side = unit(np.cross(d, [0, 0, 1.]))                                                            # the reed underneath
    if side[1] > 0:
        side = -side
    r0, r1 = base + d * .25 * L + side * .0098 * c, tip - d * .006 + side * .0048 * c
    w = .0062 * c
    zf = np.array([0, 0, 1.])
    g.face([r0 - zf * w, r0 + zf * w, r1 + zf * w * .8, r1 - zf * w * .8], REED_C, ref=r0 - side, two=True, spec=.15)
    return tip


def bell(g):
    """the silver bow: out of the bottom ferrule, round a U to the left, up the left leg and flaring into the tilted
    bell (rim 104 mm, the mouth facing up, left and toward the camera); returns the mouth's centre"""
    c = CH[0]
    cx, cy, rx, ry = 355.0, 3800.0, 115.0, 165.0
    bow = [P(470, 3650 + 150 * k / 3) for k in range(1, 3)]
    bow += [P(cx + rx * math.cos(a), cy + ry * math.sin(a)) for a in np.linspace(0, math.pi, 12)]
    bow += [P(236, 3700), P(232, 3600)]
    path(g, bow, RB * 1.0 * c, NICKEL, r1=RB * 1.02 * c, n=12, spec=.95)
    base = P(232, 3600)
    nrm = unit([-.36, .78, .52])                                        # the bell's axis -> the rim's normal
    L = .095
    Rr = .052 * c ** .75
    r0 = RB * 1.02 * c
    prof = [(0.0, r0), (.020, r0 * 1.06), (.040, r0 * 1.22), (.060, r0 * 1.55), (.075, (r0 + Rr) * .5),
            (.086, Rr * .82), (.092, Rr * .95), (L, Rr)]
    for (s0, q0), (s1, q1) in zip(prof, prof[1:]):
        T.tube(g, base + nrm * s0, base + nrm * s1, q0, q1, col=NICKEL, n=16, spec=.95, two=True)
    rim = base + nrm * L
    T.tube(g, rim - nrm * .004, rim + nrm * .002, Rr * 1.05, col=NICKEL, n=16, spec=.95, two=True)    # the rolled rim
    g.face(S.ring(rim - nrm * .03, nrm, Rr * .55, 12), (40, 34, 30), ref=rim - nrm, emit=True, two=True)
    for y in (3810, 3828):                                                                      # ring bands on the leg
        T.tube(g, P(236, y), P(236, y - 8), RB * 1.12 * c, col=dim(NICKEL, 1.06), n=12, spec=.95)
    return rim, nrm


def peg(g, ext=0.0):
    """the floor peg: a silver rod out of its socket under the bow and the black rubber tip (ext = extra length out)"""
    c = CH[0]
    top = P(392, 3862, -.004)
    tip = P(392, 3955 + ext / PX, -.004)
    T.tube(g, top, tip, .0024 * c, col=NICKEL, n=6, spec=.9)
    T.tube(g, top + [0, .006, 0], top - [0, .004, 0], .0060 * c, col=NICKEL, n=8, spec=.9)        # the socket
    T.tube(g, tip, tip - [0, .022, 0], .0062 * c, .0050 * c, col=RUBBER, n=8, spec=.2)
    return tip - np.array([0, .022, 0])


def plate(g, y, side=0.0, r=.0072, perf=False, open_=False, col=NICKEL, h=.0026):
    """a key plate / pad cup on the body surface (side = angle from the front, +z, toward +x)"""
    c = CH[0]
    d = np.array([math.sin(side), 0, math.cos(side)])
    cen = np.array([0, y, 0]) + d * (RB * c + .0012)
    top = S.prism(g, cen, d, r * c, h, 10, col, dim(col, .8))
    if perf:
        g.face(S.ring(top + d * .0002, d, r * c * .32, 6), HOLE, ref=cen, emit=True)
    elif not open_:
        g.face(S.ring(top + d * .0002, d, r * c * .55, 8), mix(col, (255, 255, 255), .25), ref=cen)
    return top


def knob(g, y, side, r=.0032, lift=.0045, col=NICKEL):
    c = CH[0]
    d = np.array([math.sin(side), 0, math.cos(side)])
    p = np.array([0, y, 0]) + d * (RB * c + lift)
    S.prism(g, p, d, r * c, .0024, 6, col, dim(col, .8))
    T.tube(g, np.array([0, y, 0]) + d * RB * c * .9, p, .0012 * c, col=dim(col, .85), n=4, spec=.6)


def rod(g, y0, y1, side, lift=.0042, r=.0014, col=NICKEL):
    c = CH[0]
    d = np.array([math.sin(side), 0, math.cos(side)])
    a = np.array([0, y0, 0]) + d * (RB * c + lift)
    b = np.array([0, y1, 0]) + d * (RB * c + lift)
    T.tube(g, a, b, r * c, col=col, n=5, spec=.85)
    for y in (y0, y1, (y0 + y1) / 2):                                                    # posts
        p = np.array([0, y, 0]) + d * RB * c * .95
        T.tube(g, p, p + d * (lift + .001), .0015 * c, col=dim(col, .9), n=4, spec=.7)


def spatula(g, y, side, out=.020, ang=-25.0, L=.032, w=.008, col=NICKEL):
    """a pinky spatula: an arm off the side of the body, then a flat teardrop touch leaning along the body"""
    c = CH[0]
    d = np.array([math.sin(side), 0, math.cos(side)])
    base = np.array([0, y, 0]) + d * RB * c
    tip = base + d * out * c
    T.tube(g, base, tip, .0015 * c, col=col, n=4, spec=.85)
    a = math.radians(ang)
    ax = unit(np.array([0, math.cos(a), 0]) * -1 + d * math.sin(abs(a)))
    n_ = unit(np.array([0, 0, 1.]) * .8 + d * .2)
    n_ = unit(n_ - ax * (n_ @ ax))
    m = 8
    pts = []
    for k in range(m):
        u = 2 * math.pi * k / m
        rr = w * c * (.55 + .45 * math.cos(u))
        pts.append(tip + ax * (L * .5 * (1 + math.cos(u)) * .9) + np.cross(n_, ax) * rr * math.sin(u))
    g.face(pts, col, ref=tip - n_, spec=.9, two=True)
    g.face([p + n_ * .0018 for p in pts], mix(col, (255, 255, 255), .2), ref=tip, spec=.9)


def bracket(g, y, side=1.25):
    """the black thumb-rest / strap brackets on the right side (px 2165, 2745)"""
    c = CH[0]
    d = np.array([math.sin(side), 0, math.cos(side)])
    p = np.array([0, y, 0]) + d * (RB * c + .006 * c)
    q = d * .011 * c
    e = np.array([0, .0045 * c, 0])
    lo, hi = np.minimum(p - q - e, p + q + e), np.maximum(p - q - e, p + q + e)
    g.box(lo[0], hi[0], lo[1], hi[1], lo[2] - .005 * c, hi[2] + .005 * c, RUBBER, spec=.2)


def keys(g):
    yu0, yu1 = Y_TEN + .006, Y_TOP
    # upper joint: throat keys near the top, the three left-hand plates (the first perforated), trills, the pinky cluster
    plate(g, ypx(560), -.5, r=.0055)                                     # throat A / register vent (front-left)
    knob(g, ypx(600), .9, r=.0030)
    for py, perf in ((905, True), (1040, False), (1170, False)):
        plate(g, ypx(py), 0.0, r=.0080, perf=perf)
    for py in (935, 1090):
        knob(g, ypx(py), .85, r=.0028)
    for py, side, out, L in ((1240, 1.25, .022, .034), (1268, 1.55, .028, .036), (1300, 1.05, .018, .030)):
        spatula(g, ypx(py), side, out=out, L=L)
    rod(g, ypx(640), ypx(1480), 1.25)
    rod(g, ypx(700), ypx(1470), -1.20)
    for py in (1150, 1380):                                              # the bent levers on the left side
        knob(g, ypx(py), -1.45, r=.0032, lift=.008)
    # lower joint: three right-hand plates, trill rings, the right-pinky spatulas
    for py in (1660, 1700):
        knob(g, ypx(py), .9, r=.0028)
    for py in (1720, 1850, 1950):
        plate(g, ypx(py), 0.0, r=.0082)
    spatula(g, ypx(2060), -1.25, out=.020, L=.030)
    spatula(g, ypx(2095), -1.55, out=.026, L=.034)
    spatula(g, ypx(2080), 1.35, out=.022, L=.032)
    # the low extension: two big open pad cups, guards, brackets, rods to the bottom
    for py in (2270, 2620):
        plate(g, ypx(py), .45, r=.0112, open_=True, h=.0034)
    for py in (2415, 2575):
        knob(g, ypx(py), -1.30, r=.0036, lift=.010)
    for py in (2165, 2745):
        bracket(g, ypx(py))
    rod(g, ypx(1520), ypx(3600), 1.15)
    rod(g, ypx(1530), ypx(3600), 1.38, lift=.0062)
    rod(g, ypx(1530), ypx(3560), -1.15)
    for py in (3110, 3155):                                              # the low keys' cups toward the bell
        plate(g, ypx(py), -.95, r=.0090, h=.0030)
    knob(g, ypx(3580), .6, r=.0030)
    # the low-C cup on the bow, above the peg
    c = CH[0]
    S.prism(g, P(392, 3855, .024 * c), np.array([0, 0, 1.]), .0095 * c, .003, 10, NICKEL, dim(NICKEL, .8))


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])


POSE = rot_z(math.radians(-24))     # leaning: the neck up-right, the bell low on the left (a Prestige-like diagonal)


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


def bass_clarinet(g, peg_ext=0.0):
    """the whole instrument in its local frame; returns (mouthpiece tip, bell mouth, bell axis, peg tip)"""
    body(g)
    end, d = neck(g)
    tip = mouthpiece(g, end, d)
    mouth, nrm = bell(g)
    keys(g)
    ptip = peg(g, peg_ext)
    return tip, mouth, nrm, ptip


def build(g, t, ev, fx, s):
    f0, l0 = len(g.F), len(g.L)
    CH[0] = float(s.get("chunk", 1.0))
    tip, mouth, nrm, ptip = bass_clarinet(g)
    CH[0] = 1.0
    cen = np.array([0.0, .55, 0])
    o = np.array([0, .62, 0]) - POSE @ cen
    xform(g, f0, l0, R=POSE, t=o)
    mouth = POSE @ mouth + o
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(mouth + [-.06, .14, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(mouth + [-.02, .03, 0]), age, 0))


# ---------------------------------------------------------------- sound: the bass clarinet (chalumeau growl, slap tongue)
# A cylindrical bore closed at the reed: mostly odd harmonics, strongest in the low (chalumeau) register; the bass
# clarinet's colour is dark and woody with a formant around 500-700 Hz and a breathy edge. `growl` adds a rasp (the
# reed beating against a hummed note: a little subharmonic + noise at the fundamental); slap() is the slap-tongue pop:
# the tongue sticks to the reed and snaps off, a hollow thock pitched at the note with almost no sustain. Pitch may be
# fractional. The written range of the low-C instrument sounds Bb1-F5 (midi 34-77); the chalumeau sits around 38-58.
SR = 44100
FORMANTS = ((560., 260., 1.4), (1450., 500., .35))   # (centre Hz, width Hz, gain)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def bass_reed(f, dur, v, bright=.45, vib=.0025, vrate=5.0, odd=.92, att=.035, growl=0.0, breath=.010):
    n = int((dur + .16) * SR)
    t = np.arange(n) / SR
    env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / .05)) * (.9 + .1 * np.exp(-t * 4))
    fr = f * (1 - .015 * np.exp(-t / .04)) * (1 + vib * np.sin(2 * np.pi * vrate * t) * np.clip((t - .18) / .25, 0, 1))
    ph = 2 * np.pi * np.cumsum(fr) / SR
    dark = np.zeros(n)
    brt = np.zeros(n)
    sd = sb = 0.0
    for k in range(1, 48):
        fk = f * k
        if fk > 7500:
            break
        base = (1 / k ** 1.05) * ((1 - odd) if k % 2 == 0 else 1) * (.45 + sum(g * math.exp(-((fk - c) / w) ** 2)
                                                                             for c, w, g in FORMANTS))
        a1, a2 = base * math.exp(-k / 3.0), base * math.exp(-k / (4 + 9 * bright))
        s_ = np.sin(k * ph + .3 * k)
        dark += a1 * s_
        brt += a2 * s_
        sd += a1
        sb += a2
    mb = np.clip(env * v, 0, 1) ** 1.2
    x = dark / sd * (1 - mb) + brt / sb * mb
    rng = np.random.default_rng(int(f * 10) % 100003)
    from scipy.signal import butter, sosfilt
    nz = sosfilt(butter(2, [700, 3200], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    x = x + nz * breath * 3.0
    if growl > 0:                                                    # the rasp: a subharmonic + noise riding the pitch
        x = x * (1 + growl * .55 * np.sin(ph * .5)) + growl * .25 * nz * np.abs(np.sin(ph))
    return x * env * v


def slap(f, v, dur=.09):
    """the slap-tongue pop: a hollow, pitched thock (the bore's ring at the note) on a click, ~90 ms"""
    n = int((dur + .08) * SR)
    t = np.arange(n) / SR
    rng = np.random.default_rng(7)
    click = rng.standard_normal(n) * np.exp(-t / .0025)
    body = (np.sin(2 * np.pi * f * t) + .5 * np.sin(2 * np.pi * 3 * f * t) + .22 * np.sin(2 * np.pi * 5 * f * t))
    body *= np.exp(-t / .045) * np.minimum(1, t / .0015)
    knock = np.sin(2 * np.pi * 180 * t) * np.exp(-t / .02)
    return (.9 * body + .35 * click + .5 * knock) * v * .8


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return bass_reed(_midi_hz(m), dur, v, growl=.25 if m < 45 else 0.0)


if __name__ == "__main__":
    import piano_lowpoly as Pn
    for sc in SCENES:
        g = Pn.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces, extent (m):", np.round(V.max(0) - V.min(0), 3))

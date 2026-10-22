"""Pixel Pitch: history of the bagpipes, 60-second cut.  python3 bagpipes60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys, bisect
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import pp_light
import trumpet_lowpoly as TR
import piano_lowpoly as P
import drums60_lowpoly as D
from piano_lowpoly import Geo
from pp_framework import mix, dim, lerp, clamp, midi_hz, ease, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "bagpipes60_lowpoly.mp4", "bp"
BPM = 100
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.75, 44, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE BAGPIPES", "1,960 YEARS", "TODAY", 0
INTRO_LINES = (("9 NOTES, 3 DRONES,", "AND ZERO RESTS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("1,960 YEARS FROM", "NERO'S BAGPIPE")
OUTRO_CREDIT = "BAGPIPE BIG BEAT"
LIGHTING = True

# ---------------------------------------------------------------- music (bagpipe big beat, 100 BPM, everything over an A drone)
G4, A4, B4, C5, CS5, D5, E5, FS5, G5, GS5, A5 = 67, 69, 71, 72, 73, 74, 76, 78, 79, 80, 81
CH = {"A": (45, (57, 61, 64)), "G": (43, (55, 59, 62)), "D": (38, (57, 62, 66)), "Am": (45, (57, 60, 64)),
      "E": (40, (56, 59, 64))}
BARS = ["A", "G", "D", "A",                                     # showcase
        "Am", ("G", "G", "Am", "Am"),                           # Roman
        "Am", ("G", "G", "Am", "Am"),                           # Cantigas
        "A", ("A", "A", "G", "A"),                              # Flemish pijpzak
        "A", ("D", "D", "E", "A"),                              # musette de cour
        ("A", "A", "G", "G"), ("A", "A", "G", "A"),             # uilleann
        "A", ("D", "D", "A", "A"),                              # Northumbrian
        "A", ("A", "A", "G", "G"),                              # 1914
        ("A", "A", "D", "D"), "A",                              # 1977
        ("A", "A", "G", "G"), "A",                              # 1990s electronic
        "A", "G",                                               # modern again
        ("D", "D", "A", "A"),                                   # outro
        "A"]                                                    # final chord
def r(items, v=.85): return [(b, m, d, v) for b, m, d in items]
T3 = 1/3
SHOW = r([(0, A4, .5), (.5, E5, .5), (1, A5, .75), (1.75, G5, .25), (2, E5, .5), (2.5, FS5, .5), (3, E5, .5), (3.5, CS5, .5),
          (4, D5, .75), (4.75, B4, .25), (5, G4, .5), (5.5, B4, .5), (6, D5, .5), (6.5, G5, .5), (7, FS5, .5), (7.5, D5, .5),
          (8, FS5, .75), (8.75, A5, .25), (9, FS5, .5), (9.5, E5, .5), (10, D5, .5), (10.5, E5, .5), (11, FS5, .5), (11.5, A5, .5),
          (12, A5, .75), (12.75, G5, .25), (13, E5, .5), (13.5, CS5, .5), (14, B4, .5), (14.5, CS5, .25), (14.75, B4, .25), (15, A4, 1)])
OUTRO_N = r([(8, FS5, .75), (8.75, A5, .25), (9, G5, .5), (9.5, E5, .5), (10, CS5, .5), (10.5, D5, .25), (10.75, CS5, .25),
             (11, B4, .5), (11.5, CS5, .5)]) + [(12, [A4, E5, A5], 4.3, .95)]

SCENES = [
 dict(key="ghb", theme="intro", name="GREAT HIGHLAND BAGPIPE", credit="BAGPIPE BIG BEAT", size=.9, notes=SHOW),
 dict(key="roman", theme="roman", year=60, ys="60 AD", name="ROMAN BAGPIPE", credit="ANCIENT ROMAN MODE", size=1.0,
      fact=("EMPEROR NERO", "PLAYED ONE"),
      notes=r([(0, A4, 1), (1, C5, .5), (1.5, D5, .5), (2, E5, 1.5), (3.5, G5, .5),
               (4, E5, .5), (4.5, D5, .5), (5, C5, .5), (5.5, B4, .5), (6, C5, .5), (6.5, B4, .5), (7, A4, 1)], .8)),
 dict(key="cantigas", theme="cantigas", year=1255, ys="1250s", name="CANTIGAS BAGPIPE", credit="MEDIEVAL CANTIGA", size=1.0,
      fact=("IN ALFONSO X'S", "ROYAL SONGBOOK"),
      notes=r([(0, E5, 1), (1, D5, .5), (1.5, C5, .5), (2, D5, .75), (2.75, E5, .25), (3, C5, .5), (3.5, A4, .5),
               (4, B4, .5), (4.5, C5, .5), (5, D5, 1), (6, C5, .5), (6.5, B4, .5), (7, A4, 1)], .85)),
 dict(key="pijpzak", theme="pijpzak", year=1567, ys="1567", name="FLEMISH PIJPZAK", credit="FLEMISH PEASANT DANCE", size=.8,
      fact=("TWO PIPERS PLAY", "BRUEGEL'S WEDDING"),
      notes=r([(0, A4, .75), (.75, B4, .25), (1, CS5, .5), (1.5, A4, .5), (2, E5, .75), (2.75, D5, .25), (3, CS5, .5), (3.5, B4, .5),
               (4, A4, .75), (4.75, B4, .25), (5, CS5, .5), (5.5, E5, .5), (6, D5, .5), (6.5, B4, .5), (7, A4, 1)], .9)),
 dict(key="musette", theme="musette", year=1672, ys="1672", name="MUSETTE DE COUR", credit="FRENCH BAROQUE GAVOTTE", size=1.1,
      fact=("BELLOWS-BLOWN FOR", "FRENCH ARISTOCRATS"),
      notes=r([(0, CS5, .5), (.5, D5, .5), (1, [CS5, E5], 1), (2, A5, .5), (2.5, GS5, .25), (2.75, A5, .25), (3, [CS5, E5], 1),
               (4, FS5, .5), (4.5, E5, .5), (5, D5, .5), (5.5, CS5, .5), (6, B4, .75), (6.75, CS5, .25), (7, [A4, CS5], 1)], .8)),
 dict(key="uilleann", theme="uilleann", year=1775, ys="1770s", name="UILLEANN PIPES", credit="IRISH JIG", size=1.05,
      fact=("'UILLEANN' MEANS", "'OF THE ELBOW'"),
      notes=r([(0, E5, 2*T3), (2*T3, FS5, T3), (1, E5, T3), (1 + T3, CS5, T3), (1 + 2*T3, A4, T3), (2, B4, 2*T3), (2 + 2*T3, CS5, T3),
               (3, D5, T3), (3 + T3, B4, T3), (3 + 2*T3, G4, T3), (4, A4, 2*T3), (4 + 2*T3, B4, T3), (5, CS5, T3), (5 + T3, E5, T3),
               (5 + 2*T3, A5, T3), (6, G5, 2*T3), (6 + 2*T3, E5, T3), (7, A4, 1)], .85)
            + r([(0, [57, 61], .4), (2, [55, 59], .4), (4, [57, 61], .4), (6, [55, 59], .4), (7, [57, 61, 64], .6)], .6)),
 dict(key="northumbrian", theme="northumbrian", year=1805, ys="1800s", name="NORTHUMBRIAN PIPES", credit="NORTHUMBRIAN RANT", size=1.1,
      fact=("A CLOSED CHANTER:", "NOTES CAN STOP"),
      notes=r([(0, A4, .5), (.5, CS5, .5), (1, E5, .5), (1.5, CS5, .5), (2, A4, .5), (2.5, CS5, .5), (3, E5, .25), (3.25, FS5, .25),
               (3.5, E5, .5), (4, D5, .5), (4.5, FS5, .5), (5, A5, .5), (5.5, FS5, .5), (6, E5, .5), (6.5, CS5, .5), (7, A4, .5),
               (7.5, E5, .5)], .9)),
 dict(key="ww1", theme="ww1", year=1914, ys="1914", name="WARTIME HIGHLAND PIPES", credit="PIPE BAND MARCH", size=.9,
      fact=("PIPED TROOPS INTO", "WWI BATTLES"),
      notes=r([(0, A4, .75), (.75, B4, .25), (1, CS5, .5), (1.5, A4, .5), (2, CS5, .5), (2.5, E5, .5), (3, A5, 1),
               (4, G5, .75), (4.75, E5, .25), (5, CS5, .5), (5.5, E5, .5), (6, B4, .75), (6.75, A4, .25), (7, B4, 1)], .9)),
 dict(key="kintyre", theme="kintyre", year=1977, ys="1977", name="CAMPBELTOWN PIPE BAND", credit="70s FOLK-ROCK BALLAD", size=.9,
      fact=("1ST UK SINGLE TO", "SELL 2 MILLION"),
      notes=r([(0, CS5, 1), (1, E5, .5), (1.5, FS5, .5), (2, A5, 1.5), (3.5, FS5, .5),
               (4, E5, 1), (5, CS5, .5), (5.5, B4, .5), (6, A4, 2)], .85)),
 dict(key="hevia", theme="hevia", year=1993, ys="1990s", name="ELECTRONIC BAGPIPE", credit="CELTIC DANCE POP", size=1.0,
      fact=("HEVIA'S PATENTED", "ELECTRONIC PIPES"),
      notes=r([(0, A4, .5), (.5, E5, .25), (.75, A5, .5), (1.25, G5, .25), (1.5, E5, .5), (2, D5, .25), (2.25, E5, .25), (2.5, G5, .5),
               (3, E5, .5), (3.5, D5, .5), (4, CS5, .5), (4.5, D5, .25), (4.75, E5, .5), (5.25, A4, .25), (5.5, CS5, .5), (6, E5, .5),
               (6.5, G5, .25), (6.75, A5, .25), (7, E5, 1)], .85)),
 dict(key="ghb", theme="intro", year=2020, ys="2020s", name="GREAT HIGHLAND BAGPIPE", credit="BAGPIPE BIG BEAT", size=.9,
      fact=("PLAYED ON THE ISS", "IN NOVEMBER 2015"),
      notes=[e for e in SHOW if e[0] < 8] + OUTRO_N),
]
TRIM = {"musette": -2.5, "uilleann": -2.0}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers (models are built in a screen-aligned frame:
# X = screen right, Y = up, Z = toward the camera at the resting yaw; build() rotates them into model space)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
_ya = math.radians(YAW)
SCR2M = np.column_stack([(math.cos(_ya), 0, math.sin(_ya)), (0, 1., 0), (-math.sin(_ya), 0, math.cos(_ya))])
ZF = np.array([0, 0, 1.])

def disc(g, c, nrm, rad, col, n=6, **kw):
    c = np.asarray(c, float); nrm = unit(nrm)
    g.face(S.ring(c, nrm, rad, n, .3), col, ref=c - nrm*.01, **kw)

def pipe(g, p0, d, segs, n=8):
    """turned sections stacked along d: segs = [(length, r0, r1, colour, spec), ...]; capped at every joint; returns the joint points"""
    p = np.asarray(p0, float); d = unit(d); pts = [p.copy()]
    for L, r0, r1, col, sp in segs:
        q = p + d*L
        if L > 1e-4:
            TR.tube(g, p, q, r0, r1, col=col, n=n, spec=sp)
            g.face(S.ring(p, d, r0, n), col, ref=p + d*.01, spec=sp)
            g.face(S.ring(q, d, r1, n), col, ref=q - d*.01, spec=sp)
        p = q; pts.append(p.copy())
    return pts

def path(g, pts, r0, col, r1=None, n=7, spec=.2):
    """tube swept along a polyline with a rotation-minimising frame"""
    P_ = np.array([np.asarray(p, float) for p in pts]); N = len(P_); r1 = r0 if r1 is None else r1
    tg = np.gradient(P_, axis=0); tg /= np.linalg.norm(tg, axis=1)[:, None]
    a = np.array([0, 1., 0]) if abs(tg[0][1]) < .9 else np.array([1., 0, 0])
    u = np.cross(tg[0], a); u /= np.linalg.norm(u); rings = []
    for i in range(N):
        u = u - np.dot(u, tg[i])*tg[i]; u /= np.linalg.norm(u); v = np.cross(tg[i], u)
        rr = lerp(r0, r1, i/(N - 1))
        rings.append([P_[i] + rr*(math.cos(2*math.pi*k/n)*u + math.sin(2*math.pi*k/n)*v) for k in range(n)])
    for i in range(N - 1):
        ref = (P_[i] + P_[i + 1])/2; A, B = rings[i], rings[i + 1]
        col_i = col(i) if callable(col) else col
        for k in range(n): g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col_i, ref=ref, spec=spec)

def bezier(a, b, c, d, n=9):
    a, b, c, d = (np.asarray(x, float) for x in (a, b, c, d))
    return [(1 - q)**3*a + 3*(1 - q)**2*q*b + 3*(1 - q)*q*q*c + q**3*d for q in np.linspace(0, 1, n)]

_OB = ((0, 1, 3, 2), (4, 5, 7, 6), (0, 2, 6, 4), (1, 3, 7, 5), (0, 1, 5, 4), (2, 3, 7, 6))
def obox(g, cs, col, **kw):
    """box from 8 corners ordered (y0/y1) x (w0/w1) x (u0/u1)"""
    c = sum(cs)/8
    for idx in _OB: g.face([cs[i] for i in idx], col, ref=c, **kw)

class Bag:
    """pillow / teardrop bag: long axis rotated by `ang` degrees in the screen plane; s in [-1, 1] along it (s = -1 is the neck end),
    phi around it (0 = top, pi/2 = toward the camera, pi = bottom)"""
    def __init__(s_, c, ang, hx, hy, hz, taper=.2, ex=.8):
        s_.c = np.asarray(c, float); a = math.radians(ang)
        s_.R = np.array([[math.cos(a), -math.sin(a), 0], [math.sin(a), math.cos(a), 0], [0, 0, 1.]])
        s_.hx, s_.hy, s_.hz, s_.taper, s_.ex = hx, hy, hz, taper, ex
        s_.axis = s_.R[:, 0]
    def rr(s_, sv): return max(0., 1 - sv*sv)**(s_.ex/2)*(1 + s_.taper*sv)
    def loc(s_, sv, ph):
        r = s_.rr(sv); return np.array([sv*s_.hx, r*s_.hy*math.cos(ph), r*s_.hz*math.sin(ph)])
    def nrm(s_, sv, ph):
        return s_.R @ unit(np.array([sv*.5/s_.hx, math.cos(ph)/s_.hy, math.sin(ph)/s_.hz]))
    def pt(s_, sv, ph, off=0.):
        return s_.c + s_.R @ s_.loc(sv, ph) + s_.nrm(sv, ph)*off
    def draw(s_, g, colfn, ns=12, nph=12, spec=0.):
        rings = [[s_.c + s_.R @ s_.loc(-1 + 2*i/ns, 2*math.pi*j/nph) for j in range(nph)] for i in range(ns + 1)]
        for i in range(ns):
            for j in range(nph):
                j2 = (j + 1) % nph
                g.face([rings[i][j], rings[i + 1][j], rings[i + 1][j2], rings[i][j2]], colfn(i, j), ref=s_.c, spec=spec)

def tartan(sett, k=1):
    L = len(sett)
    def f(i, j):
        a, b = sett[(i*k) % L], sett[(j*k + 3) % L]
        return a if a == b else mix(a, b, .5)
    return f

def brocade(base, pats, seed=4):
    M = np.random.default_rng(seed).integers(0, len(pats) + 3, (16, 16))
    return lambda i, j: pats[M[i % 16, j % 16]] if M[i % 16, j % 16] < len(pats) else base

def squeeze(t): return 1 - .035*(.5 + .5*math.sin(2*math.pi*t/BAR_S))   # the arm squeezes the bag once a bar

def stand(g, top, col=(58, 54, 62)):
    """slim display stand from the pedestal to the bag"""
    x, y, z = top
    TR.tube(g, (x, 0., z), (x, .014, z), .085, .075, col=col, n=8, spec=.4)
    disc(g, (x, .0142, z), (0, 1, 0), .075, col, n=8, spec=.4)
    TR.tube(g, (x, .014, z), (x, y - .012, z), .011, col=col, n=6, spec=.5)
    TR.tube(g, (x - .05, y - .014, z), (x + .05, y - .014, z), .013, col=col, n=6, spec=.5)

def cord(g, a, b, sag, c1, c2, n=8, off=(.0, .004, .0)):
    a, b = np.asarray(a, float), np.asarray(b, float); o = np.asarray(off)
    pts = [a + (b - a)*q + np.array([0, -4*sag*q*(1 - q), 0]) for q in np.linspace(0, 1, n + 1)]
    for k in range(n):
        g.line(pts[k], pts[k + 1], c1 if k % 2 == 0 else c2); g.line(pts[k] + o, pts[k + 1] + o, c2 if k % 2 == 0 else c1)
    return pts

def tassel(g, p, col, L=.07):
    p = np.asarray(p, float); TR.tube(g, p, p + np.array([0, -L, 0]), .005, .019, col=col, n=6, spec=0.)
    disc(g, p + np.array([0, -L, 0]), (0, -1, 0), .019, dim(col, .8), n=6)

def live(ev, stacc=1.): return {e[1] for e in ev if e[0] < e[3]*stacc + .03}

HSCALE = [G4, A4, B4, CS5, D5, E5, FS5, G5, A5]            # the Highland chanter's nine notes
def scene_scale(s, lo=64): return sorted({e[1] for e in s["ev"] if e[1] >= lo})

def chanter_fx(fx, ev, sole, hole_pts, lit):
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(sole + np.array([-.06, .02, 0])), age))
        if tp and age < .06 and lit is not None and 0 <= lit < len(hole_pts): fx.append(("spark", tuple(hole_pts[lit]), age))

def holes(g, top, d, L, r0, r1, n, lit, dark=(22, 14, 10), fr=(.16, .86), toward=None):
    """finger holes on the camera side of a chanter body; index 0 = nearest the sole; lit = the open (sounding) hole"""
    d = unit(d); tw = unit(ZF if toward is None else toward); nr = unit(tw - d*np.dot(tw, d)); pts = []
    for k in range(n):
        f = lerp(fr[1], fr[0], k/max(1, n - 1)); rad = lerp(r0, r1, f)
        c = np.asarray(top, float) + d*L*f + nr*(rad + .0015); on = k == lit
        disc(g, c, nr, .0085 if on else .0062, HOT if on else dark, n=5, emit=on)
        pts.append(c + nr*.01)
    return pts

def chanter_len(p0, d, ends, y_end=.012):
    """body length so that a chanter starting at p0 along d lands its sole on the pedestal top"""
    return max(.1, (p0[1] - y_end)/(-unit(d)[1]) - ends)

def drone_waves(fx, t, top, d, k=0):
    age = (t + k*.17) % BEAT_S
    if age < .6: fx.append(("wave", tuple(np.asarray(top) + unit(d)*.05), age, 0))

def bellows(g, H, L, W, a, board, leather, face=None, th=.013, toward=1.):
    """hand bellows: two boards hinged at H (nozzle end), opening by angle a at the far (+X) end; returns the nozzle tip"""
    H = np.asarray(H, float)
    def bp(u, w, rot, y): return H + np.array([u*math.cos(rot) - y*math.sin(rot), u*math.sin(rot) + y*math.cos(rot), w])
    for rot, y0 in ((0., 0.), (a, th)):
        obox(g, [bp(u, w, rot, y) for y in (y0, y0 + th) for w in (-W/2, W/2) for u in (.02, L)], board, spec=.25)
    if face:
        f0 = [bp(u, w, a, 2*th + .001) for u, w in ((.06, -W/2 + .02), (L - .02, -W/2 + .02), (L - .02, W/2 - .02), (.06, W/2 - .02))]
        g.face(f0, face, ref=bp(L/2, 0, a, 0), spec=.3)
    ring_lo = [(.03, -W/2 + .01), (L - .01, -W/2 + .01), (L - .01, W/2 - .01), (.03, W/2 - .01)]
    lo = [bp(u, w, 0., th) for u, w in ring_lo]
    mid = [bp(lerp(.03, u, .9), w*.82, a/2, th) for u, w in ring_lo]
    hi = [bp(u, w, a, th) for u, w in ring_lo]
    cen = bp(L/2, 0, a/2, th)
    for A_, B_, col in ((lo, mid, leather), (mid, hi, dim(leather, .8))):
        for k in range(3):
            g.face([A_[k], A_[k + 1], B_[k + 1], B_[k]], col, ref=cen, two=True)
    nz0 = bp(.02, 0, a/2, th)
    pts = pipe(g, nz0, (-1., 0, 0), [(.05, .016, .01, dim(board, .9), .3)], n=6)
    return pts[-1]

# ---------------------------------------------------------------- the Great Highland bagpipe (modern, 1914, 1977)
HIGH = {
 "ghb":     dict(wood=(30, 26, 28), mount=(206, 208, 218), proj=(206, 208, 218), mouth=(24, 22, 24),
                 sett=[(22, 34, 70), (22, 34, 70), (24, 70, 44), (16, 18, 22), (24, 70, 44), (22, 34, 70)],
                 cords=((28, 104, 58), (16, 22, 18)), tassel=(28, 110, 60), ribbons=False),
 "ww1":     dict(wood=(110, 60, 40), mount=(170, 168, 160), proj=(222, 210, 180), mouth=(196, 184, 156),
                 sett=[(90, 92, 54), (66, 70, 44), (150, 120, 66), (90, 92, 54), (126, 44, 36), (66, 70, 44)],
                 cords=((120, 118, 78), (92, 90, 60)), tassel=(110, 108, 70), ribbons=False),
 "kintyre": dict(wood=(24, 22, 26), mount=(236, 232, 216), proj=(236, 232, 216), mouth=(236, 232, 216),
                 sett=[(196, 30, 38), (196, 30, 38), (28, 36, 84), (196, 30, 38), (196, 30, 38), (236, 226, 206)],
                 cords=((214, 36, 44), (240, 240, 236)), tassel=(220, 40, 48), ribbons=True),
}
def highland(g, t, ev, fx, s, c):
    W, M1, M2 = c["wood"], c["mount"], c["proj"]
    bag = Bag((0., .36, 0.), 8, .28, .16*squeeze(t), .13, taper=.18)
    bag.draw(g, tartan(c["sett"]), spec=.04)
    for sv in np.linspace(-.85, .85, 15):                          # cover fringe along the bottom seam
        p = bag.pt(sv, math.pi*.9); g.line(p, p + np.array([.004, -.028, .01]), mix(c["sett"][0], (255, 255, 255), .2))
    stand(g, bag.pt(.05, math.pi))
    tops = []
    for sv, ph, ang, dz, bass in ((-.05, .3, 67, .04, False), (.15, 0., 56, 0., False), (.36, -.3, 44, -.05, True)):
        base = bag.pt(sv, ph); d = unit((math.cos(math.radians(ang)), math.sin(math.radians(ang)), dz))
        segs = [(.07, .027, .027, W, .2), (.012, .032, .032, M1, .9), (.12, .018, .018, W, .25), (.024, .031, .031, M2, .7),
                (.012, .022, .022, M1, .9)]
        if bass: segs += [(.15, .019, .019, W, .25), (.024, .031, .031, M2, .7), (.012, .022, .022, M1, .9)]
        segs += [(.13, .017, .021, W, .25), (.045, .021, .03, W, .25), (.014, .033, .033, M1, .9)]
        pts = pipe(g, base - d*.035, d, segs)
        disc(g, pts[-1] + d*.001, d, .021, (14, 10, 10))
        tops.append((pts, d))
    (p1, d1), (p2, d2), (p3, d3) = tops
    a = p1[-4]; b = p2[-4]; cc = p3[7]
    cord(g, a, b, .02, *c["cords"]); cord(g, b, cc, .03, *c["cords"])
    loop = cord(g, cc, p1[3], .12, *c["cords"], n=10)
    tassel(g, loop[4], c["tassel"]); tassel(g, loop[7], c["tassel"], .06)
    if c["ribbons"]:                                                # long pipe-band ribbons streaming from the bass drone
        for k, ph in enumerate((0., 1.7)):
            p0 = p3[7] + np.array([0, -.01, .02 - .04*k]); pts = []
            for i in range(9):
                q = i/8
                pts.append(p0 + np.array([.03*q + .035*q*math.sin(t*3.1 + q*4 + ph), -.36*q, .05*q*math.sin(t*2.2 + q*3 + ph)]))
            for i in range(8):
                A_, B_ = pts[i], pts[i + 1]; sd = np.array([.017, 0, 0])
                g.face([A_ - sd, A_ + sd, B_ + sd, B_ - sd], (214, 36, 44) if (i + k) % 2 == 0 else (240, 240, 236), two=True)
    drone_waves(fx, t, p3[-1], d3)
    # chanter: hangs down in front, conical, with its sole on the pedestal
    d = unit((-.68, -.73, .06)); p0 = bag.pt(-.8, 2.5) - d*.03
    Lb = chanter_len(p0, d, .065 + .012 + .03)
    pts = pipe(g, p0, d, [(.065, .027, .027, W, .2), (.012, .031, .031, M1, .9), (Lb, .016, .023, W, .25), (.03, .032, .03, M2, .7)])
    act = [m for m in live(ev) if m >= 64]; k = HSCALE.index(max(act)) if act and max(act) in HSCALE else None
    lit = None if k is None else k - 1
    hp = holes(g, pts[2], d, Lb, .016, .023, 8, lit)
    if k == 0: disc(g, pts[-1] + d*.002, d, .026, HOT, n=8, emit=True)
    chanter_fx(fx, ev, pts[-1], hp, lit)
    d = unit((-.45, 1, .08)); p0 = bag.pt(-.5, .45) - d*.03
    pipe(g, p0, d, [(.05, .022, .022, W, .2), (.01, .026, .026, M1, .9), (.15, .014, .012, W, .2), (.035, .011, .008, c["mouth"], .4)])

# ---------------------------------------------------------------- historical models (each from its reference photos / depiction)
def roman(g, t, ev, fx, s):
    """Gloucester altar / Alexandrian terracotta type: a hide bag in the crook of the arm, a long drone pipe, and panpipes"""
    hide, hide2, cane, cane2, strap = (150, 102, 60), (126, 84, 48), (206, 178, 104), (168, 138, 74), (88, 52, 30)
    bag = Bag((.04, .34, 0.), 6, .23, .15*squeeze(t), .13, taper=.12)
    bag.draw(g, lambda i, j: hide if (i*3 + j*5) % 7 else hide2, spec=.05)
    stand(g, bag.pt(0, math.pi))
    dn = -bag.axis; nk = bag.pt(-1, 0)
    pipe(g, nk - dn*.045, dn, [(.06, .034, .022, hide, .05), (.016, .025, .025, strap, .1), (.02, .022, .016, hide, .05)])
    d = unit((1, -.3, -.05)); p0 = bag.pt(.55, 2.3) - d*.03
    pts = pipe(g, p0, d, [(.05, .021, .021, strap, .1), (.16, .014, .014, cane, .15), (.012, .017, .017, cane2, .15),
                          (.16, .014, .014, cane, .15), (.012, .017, .017, cane2, .15), (.06, .015, .031, (178, 142, 92), .2)])
    disc(g, pts[-1] + d*.001, d, .025, (26, 14, 8))
    drone_waves(fx, t, pts[-1], d)
    notes = scene_scale(s, 0); act = live(ev); n = 7; x0 = -.12; sp = .034; top_y = bag.pt(-.45, 0)[1] - .025 + .3
    for k in range(n):
        L = .3 - k*.026; x = x0 + k*sp; on = k < len(notes) and notes[k] in act
        col = mix(cane, (255, 236, 170), .4) if on else (cane if k % 2 == 0 else dim(cane, .9))
        TR.tube(g, (x, top_y - L, 0.), (x, top_y, 0.), .016, col=col, n=6, spec=.15)
        disc(g, (x, top_y + .001, 0.), (0, 1, 0), .011, HOT if on else (30, 18, 10), emit=on)
        disc(g, (x, top_y - L - .001, 0.), (0, -1, 0), .016, dim(cane, .8))
        if on:
            for age, m, v, dur, tp, cs in ev:
                if m == notes[k] and tp and age < .9: fx.append(("note", (x - .02, top_y + .06, 0.), age))
                if m == notes[k] and tp and age < .06: fx.append(("spark", (x, top_y + .02, 0.), age))
    for yb in (top_y - .05, top_y - .13):
        g.box(x0 - .021, x0 + (n - 1)*sp + .021, yb - .012, yb + .012, -.021, .021, strap)
    g.line((x0 - .02, top_y - .13, 0), nk, strap)

def cantigas(g, t, ev, fx, s):
    """Cantigas de Santa Maria miniature: red bag, a golden chanter held forward, two long belled drones pointing back"""
    red, red2, wood, gold, dark = (176, 34, 38), (148, 26, 30), (206, 156, 70), (240, 200, 92), (40, 24, 14)
    bag = Bag((.02, .40, 0.), -6, .24, .16*squeeze(t), .13, taper=.22)
    bag.draw(g, lambda i, j: red if (i + j) % 3 else red2, spec=.05)
    stand(g, bag.pt(.05, math.pi))
    for sv, ph, dv, L in ((.12, .1, (1, .62, -.06), .56), (.6, 1.75, (1, .02, -.12), .46)):
        dd = unit(dv); p0 = bag.pt(sv, ph) - dd*.03
        pts = pipe(g, p0, dd, [(.05, .025, .025, wood, .2), (.012, .029, .029, gold, .8), (L*.48, .017, .017, wood, .2),
                               (.012, .023, .023, gold, .8), (L*.38, .016, .019, wood, .2), (.07, .019, .04, wood, .2), (.01, .042, .042, gold, .8)])
        disc(g, pts[-1] + dd*.001, dd, .031, dark)
        drone_waves(fx, t, pts[-1], dd, 1 if sv > .5 else 0)
    d = unit((-1, -.32, .05)); p0 = bag.pt(-.85, 2.0) - d*.03; Lb = .26
    pts = pipe(g, p0, d, [(.05, .026, .026, wood, .2), (.012, .03, .03, gold, .8), (Lb, .016, .02, wood, .2), (.07, .02, .045, wood, .2),
                          (.01, .047, .047, gold, .8)])
    disc(g, pts[-1] + d*.001, d, .034, dark)
    notes = scene_scale(s); act = [m for m in live(ev) if m >= 64]
    k = notes.index(max(act)) if act else None; lit = None if k is None else k - 1
    hp = holes(g, pts[2], d, Lb, .016, .02, 7, lit, toward=(0, .7, 1))
    if k == 0: disc(g, pts[-1] + d*.003, d, .03, HOT, n=8, emit=True)
    chanter_fx(fx, ev, pts[-1], hp, lit)
    d = unit((-.3, 1, .05)); p0 = bag.pt(-.45, .3) - d*.03
    pipe(g, p0, d, [(.04, .02, .02, wood, .2), (.01, .024, .024, gold, .8), (.14, .013, .011, wood, .2)])

def pijpzak(g, t, ev, fx, s):
    """Bruegel's Peasant Wedding: a big leather bag, one long drone over the shoulder, blowpipe, dark chanter"""
    hide, hide2, pale, pale2, dark = (150, 118, 64), (126, 98, 52), (214, 186, 128), (182, 150, 96), (116, 74, 40)
    bag = Bag((.04, .42, 0.), 14, .27, .18*squeeze(t), .15, taper=.25)
    bag.draw(g, lambda i, j: hide if (i*2 + j) % 5 else hide2, spec=.08)
    stand(g, bag.pt(.05, math.pi))
    dd = unit((.5, 1, -.05)); p0 = bag.pt(.3, -.15) - dd*.03
    pts = pipe(g, p0, dd, [(.06, .031, .031, pale2, .15), (.012, .035, .035, dark, .2), (.25, .021, .021, pale, .15),
                           (.03, .028, .028, pale2, .15), (.25, .019, .022, pale, .15), (.06, .022, .037, pale, .15)])
    disc(g, pts[-1] + dd*.001, dd, .028, (30, 18, 10))
    drone_waves(fx, t, pts[-1], dd)
    d = unit((-.3, -1, .05)); p0 = bag.pt(-.85, 2.6) - d*.03
    Lb = chanter_len(p0, d, .05 + .06)
    pts = pipe(g, p0, d, [(.05, .028, .028, pale2, .15), (Lb, .016, .024, dark, .2), (.06, .024, .04, dark, .2)])
    disc(g, pts[-1] + d*.001, d, .03, (24, 14, 8))
    notes = scene_scale(s); act = [m for m in live(ev) if m >= 64]
    k = notes.index(max(act)) if act else None; lit = None if k is None else k - 1
    hp = holes(g, pts[1], d, Lb, .016, .024, 7, lit)
    if k == 0: disc(g, pts[-1] + d*.003, d, .03, HOT, n=8, emit=True)
    chanter_fx(fx, ev, pts[-1], hp, lit)
    d = unit((-.45, 1, .05)); p0 = bag.pt(-.5, .35) - d*.03
    pipe(g, p0, d, [(.05, .022, .022, pale2, .15), (.16, .014, .012, pale, .15)])

def musette(g, t, ev, fx, s):
    """musette de cour: flat brocade bag with a ruffled ribbon edge, ivory/ebony double chanter, shuttle drone, bellows"""
    base = (204, 180, 194); pats = [mix(base, c_, .55) for c_ in ((150, 186, 168), (234, 150, 174), (160, 178, 222), (240, 222, 206))]
    ebony, ivory, brass, ruff = (34, 26, 26), (238, 228, 204), (216, 176, 84), (160, 160, 172)
    bag = Bag((.06, .43, 0.), 22, .24, .15*squeeze(t), .06, taper=.35)
    bag.draw(g, brocade(base, pats), spec=.15)
    loop = [bag.pt(sv, 0) for sv in np.linspace(-.95, .95, 13)] + [bag.pt(sv, math.pi) for sv in np.linspace(.95, -.95, 13)]
    outs = []
    for sv, ph in [(sv, 0) for sv in np.linspace(-.95, .95, 13)] + [(sv, math.pi) for sv in np.linspace(.95, -.95, 13)]:
        n_ = bag.nrm(sv, ph); n_[2] = 0; outs.append(unit(n_))
    for k in range(len(loop) - 1):
        a, b = loop[k], loop[k + 1]; zj = .013 if k % 2 else -.013
        g.face([a, b, b + outs[k + 1]*.03 - ZF*zj, a + outs[k]*.03 + ZF*zj], ruff if k % 2 else dim(ruff, .85), two=True, spec=.3)
    stand(g, bag.pt(.25, math.pi))
    # shuttle drone on the bag's face: an ebony cylinder with ivory caps and sliding layettes
    sc = bag.pt(.1, math.pi/2, .045); ax = unit(bag.axis + np.array([0, .25, 0]))
    pipe(g, bag.pt(.1, math.pi/2) - ZF*.01, ZF, [(.03, .012, .012, ivory, .3)], n=6)
    sp_ = pipe(g, sc - ax*.075, ax, [(.012, .033, .033, ivory, .4), (.126, .029, .029, ebony, .45), (.012, .033, .033, ivory, .4)])
    beat = int(t/BEAT_S)
    for k in range(5):
        p = sp_[1] + ax*(.016 + k*.024) + ZF*.029; on = k == beat % 5
        g.box(p[0] - .005, p[0] + .005, p[1] - .004, p[1] + .004, p[2], p[2] + .006, mix(ivory, (255, 236, 200), .3) if on else dim(ivory, .85), spec=.4)
    drone_waves(fx, t, sp_[-1], ax)
    # double chanter from one ivory stock at the neck
    nk = bag.pt(-1, 0); dn = -bag.axis; se = pipe(g, nk - dn*.02, dn, [(.05, .031, .028, ivory, .35)])[-1]
    d = unit((-.28, -1, .02)); pg = se + ZF*.017
    Lg = chanter_len(pg, d, .02 + .015 + .02)
    pts = pipe(g, pg, d, [(.02, .022, .022, ivory, .35), (Lg, .014, .016, ebony, .4), (.015, .021, .021, ivory, .35), (.02, .017, .023, ivory, .35)])
    pp = pipe(g, se + np.array([.04, -.01, .012]), d, [(.015, .018, .018, ivory, .35), (.15, .011, .012, ebony, .4), (.012, .015, .015, ivory, .35)], n=6)
    for f in (.62, .8):
        q = pts[1] + d*Lg*f + np.array([-.016, 0, .006])
        g.box(q[0] - .006, q[0] + .002, q[1] - .02, q[1] + .02, q[2] - .006, q[2] + .006, brass, spec=.8)
    notes = scene_scale(s); act = sorted(m for m in live(ev) if m >= 64)
    k = notes.index(act[-1]) if act else None; lit = None if k is None else k - 1
    hp = holes(g, pts[1], d, Lg, .014, .016, 7, lit)
    lo = notes.index(act[0]) if len(act) > 1 else None
    holes(g, pp[1], d, .15, .011, .012, 4, None if lo is None else min(3, lo//2))
    chanter_fx(fx, ev, pts[-1], hp, lit)
    # bellows on the pedestal, a brocade tube up to the bag
    a = lerp(.07, .3, .5 - .5*math.cos(2*math.pi*t/BEAT_S))
    nz = bellows(g, (.2, 0., -.16), .26, .14, a, (112, 66, 38), (40, 34, 36), face=(28, 28, 32))
    path(g, bezier(nz, nz + np.array([-.08, .08, .04]), bag.pt(.6, math.pi) + np.array([.0, -.12, 0]), bag.pt(.6, math.pi)), .014,
         lambda i: pats[i % 4], spec=.1)

def uilleann(g, t, ev, fx, s):
    """early union / uilleann pipes: boxwood, brass and ivory; 3 drones + 3 keyed regulators in one stock; bellows"""
    bx, brass, ivory, green, green2, leather = (206, 152, 78), (222, 178, 86), (236, 226, 200), (36, 104, 64), (28, 84, 50), (122, 76, 42)
    bag = Bag((-.2, .46, .02), 12, .19, .13*squeeze(t), .1, taper=.2)
    bag.draw(g, lambda i, j: green if (i + 2*j) % 4 else green2, spec=.05)
    for sv in np.linspace(-.85, .85, 13):
        p = bag.pt(sv, math.pi*.9); g.line(p, p + np.array([.003, -.032, .01]), (90, 160, 110))
    stand(g, bag.pt(0, math.pi))
    ax = unit((1, -.18, .06)); sp_ = pipe(g, bag.pt(.75, 1.9) - ax*.03, ax, [(.03, .04, .04, brass, .8), (.12, .042, .042, bx, .25),
                                                                            (.015, .046, .046, brass, .8)])
    se = sp_[-1]; u1 = unit(np.cross(ax, ZF)); u2 = unit(np.cross(u1, ax))
    regs = sorted({e[1] for e in s["ev"] if e[1] < 66}); act = live(ev); keys_on = set()
    for m in act:
        if m in regs: i = regs.index(m); keys_on.add((i % 3, (i//3 + i) % 4))
    for kind, L, rad, o1, o2, ri in (("d", .26, .011, -.024, -.02, 0), ("d", .38, .013, 0., -.026, 1), ("d", .5, .016, .024, -.02, 2),
                                     ("r", .22, .015, -.026, .022, 0), ("r", .31, .018, 0., .028, 1), ("r", .41, .021, .027, .022, 2)):
        p0 = se + u1*o1 + u2*o2
        if kind == "d":
            pts = pipe(g, p0, ax, [(L*.55, rad, rad, bx, .25), (.012, rad*1.35, rad*1.35, brass, .8), (L*.45, rad*.9, rad*.9, bx, .25),
                                   (.02, rad*1.3, rad*1.3, ivory if ri < 2 else brass, .5)], n=6)
            if ri == 2: drone_waves(fx, t, pts[-1], ax)
        else:
            pts = pipe(g, p0, ax, [(L, rad, rad*.9, bx, .25), (.025, rad*1.25, rad*1.1, ivory, .4)], n=6)
            for kk in range(4):
                q = p0 + ax*L*(.3 + .17*kk) + u2*(rad + .006); on = (ri, kk) in keys_on
                g.box(q[0] - .016, q[0] + .016, q[1] - .004, q[1] + .004, q[2] - .006, q[2] + .006, HOT if on else brass, emit=on, spec=.8)
    d = unit((-.12, -1, .08)); p0 = bag.pt(-.75, 2.7) - d*.03
    Lb = chanter_len(p0, d, .04 + .012 + .03)
    pts = pipe(g, p0, d, [(.04, .026, .026, ivory, .35), (.012, .029, .029, brass, .8), (Lb, .015, .019, bx, .25), (.03, .025, .022, ivory, .35)])
    notes = scene_scale(s, 66); cact = [m for m in act if m >= 66]
    k = notes.index(max(cact)) if cact else None; lit = None if k is None else k - 1
    hp = holes(g, pts[2], d, Lb, .015, .019, 8, lit)
    if k == 0: disc(g, pts[-1] + d*.002, d, .022, HOT, n=8, emit=True)
    chanter_fx(fx, [e for e in ev if e[1] >= 66], pts[-1], hp, lit)
    a = lerp(.07, .3, .5 - .5*math.cos(2*math.pi*t/BEAT_S))
    nz = bellows(g, (.06, 0., -.25), .27, .15, a, (124, 80, 44), leather)
    path(g, bezier(nz, nz + np.array([-.1, .1, .04]), bag.pt(.5, -2.2) + np.array([.05, -.1, -.05]), bag.pt(.5, -2.2)), .014, (96, 60, 34))

def northumbrian(g, t, ev, fx, s):
    """Northumbrian smallpipes: velvet bag, four slim ivory drones in one stock, a short closed keyed chanter, bellows"""
    ivory, ivory2, horn, silver, velvet, velvet2, stk = (236, 226, 202), (216, 204, 178), (30, 26, 28), (208, 210, 218), (56, 42, 88), (44, 32, 72), (66, 42, 30)
    bag = Bag((-.06, .44, 0.), 4, .2, .13*squeeze(t), .1, taper=.15)
    bag.draw(g, lambda i, j: velvet if (i + j) % 2 else velvet2, spec=.12)
    stand(g, bag.pt(0, math.pi))
    ax = unit((1, .5, -.05)); sp_ = pipe(g, bag.pt(.45, .35) - ax*.03, ax, [(.025, .034, .034, silver, .9), (.07, .036, .036, stk, .25),
                                                                            (.012, .038, .038, silver, .9)])
    se = sp_[-1]; u1 = unit(np.cross(ax, ZF)); u2 = unit(np.cross(u1, ax))
    for k, (L, rad, o1, o2) in enumerate(((.44, .013, .016, .016), (.38, .012, -.016, .016), (.32, .011, .016, -.016), (.26, .011, -.016, -.016))):
        pts = pipe(g, se + u1*o1 + u2*o2, ax, [(L*.45, rad, rad, ivory, .35), (.01, rad*1.3, rad*1.3, horn, .2), (L*.4, rad*.95, rad*.95, ivory2, .35),
                                              (.01, rad*1.3, rad*1.3, horn, .2), (L*.15, rad, rad*1.15, ivory, .35)], n=6)
        disc(g, pts[-1] + ax*.004, ax, rad*.8, ivory, n=6)
        if k == 0: drone_waves(fx, t, pts[-1], ax)
    d = unit((-.35, -1, .05)); p0 = bag.pt(-.8, 2.6) - d*.03; Lb = .24
    pts = pipe(g, p0, d, [(.035, .026, .026, silver, .9), (Lb, .016, .018, ivory, .35), (.018, .019, .019, silver, .9), (.012, .019, .01, ivory, .35)])
    notes = scene_scale(s); act = [m for m in live(ev, .32) if m >= 64]
    k = notes.index(max(act)) if act else None; lit = None if k is None else k - 1
    hp = holes(g, pts[1], d, Lb, .016, .018, 7, lit)
    side = unit(np.cross(d, ZF))
    for kk in range(7):                                             # seven silver keys
        q = pts[1] + d*Lb*(.12 + .12*kk) + side*(.017 if kk % 2 else -.017) + ZF*.004
        g.box(q[0] - .005, q[0] + .005, q[1] - .012, q[1] + .012, q[2] - .004, q[2] + .004, silver, spec=.9)
    if k == 0: disc(g, pts[-1] + d*.002, d, .016, HOT, n=6, emit=True)
    chanter_fx(fx, ev, pts[-1], hp, lit)
    a = lerp(.07, .3, .5 - .5*math.cos(2*math.pi*t/BEAT_S))
    nz = bellows(g, (.1, 0., -.22), .24, .13, a, (40, 32, 30), (70, 52, 44), face=(150, 120, 84))
    path(g, bezier(nz, nz + np.array([-.08, .08, .03]), bag.pt(.55, -2.3) + np.array([.04, -.1, -.04]), bag.pt(.55, -2.3)), .013, (56, 46, 42))

def hevia(g, t, ev, fx, s):
    """Hevia's electronic gaita (live + product photos): padded red bag, wide end low-left, narrow neck high-right; a black figure band;
    a long black blowpipe up to the left; the sensor chanter with its LCD and LEDs hanging from the neck. The drones are synthesised."""
    red, red2, black, blk2, white, lcd = (208, 30, 42), (180, 22, 34), (26, 24, 28), (46, 44, 50), (232, 232, 236), (110, 240, 210)
    bag = Bag((.02, .4, 0.), 200, .29, .13*squeeze(t), .095, taper=.5)
    def band(i, j):
        ph = 2*math.pi*(j + .5)/12
        if i >= 4 and (ph < .68*math.pi or ph > 1.8*math.pi): return black if (i + j) % 3 else blk2
        return red if (i + j) % 4 else red2
    bag.draw(g, band, spec=.12)
    for k, sv in enumerate(np.linspace(-.1, .75, 6)):              # small white cave-art figures printed on the band
        c = bag.pt(sv, .95, .004); n_ = bag.nrm(sv, .95); t1 = -bag.axis; t2 = unit(np.cross(n_, t1)); hs = .011 + .003*(k % 2)
        if t2[1] < 0: t2 = -t2
        for a0, a1, b0, b1 in ((-.15, .15, -.9, .9), (-.8, .8, .2, .4), (-.6, -.2, -1.4, -.9), (.2, .6, -1.4, -.9), (-.2, .2, 1.1, 1.5)):
            g.face([c + t1*a0*hs + t2*b0*hs, c + t1*a1*hs + t2*b0*hs, c + t1*a1*hs + t2*b1*hs, c + t1*a0*hs + t2*b1*hs], white, ref=c - n_*.01)
    stand(g, bag.pt(.35, 0))
    dd = unit((-.6, 1, -.04)); p0 = bag.pt(-.1, math.pi - .1) - dd*.03
    pts = pipe(g, p0, dd, [(.05, .03, .03, blk2, .45), (.015, .033, .033, black, .5), (.4, .022, .009, black, .5)])
    tip = bag.pt(.3, math.pi*.8)
    drone_waves(fx, t, tip + np.array([0, .04, 0]), (0, 1, 0))
    d = np.array([0, -1., .03]); top = bag.pt(-.92, 0) + np.array([0, .02, 0])
    g.box(top[0] - .026, top[0] + .026, top[1] - .11, top[1], top[2] - .018, top[2] + .018, black, spec=.5)
    g.face([(top[0] - .019, top[1] - .05, top[2] + .0185), (top[0] + .019, top[1] - .05, top[2] + .0185),
            (top[0] + .019, top[1] - .015, top[2] + .0185), (top[0] - .019, top[1] - .015, top[2] + .0185)],
           mix(lcd, (255, 255, 255), .2*(int(t/BEAT_S) % 2)), ref=(top[0], top[1] - .03, 0), emit=True)
    for k in range(3): disc(g, (top[0] - .012 + k*.012, top[1] - .075, top[2] + .019), ZF, .005, (90, 90, 96), n=5)
    p0 = top + np.array([0, -.11, 0]); Lb = chanter_len(p0, d, .02)
    pts = pipe(g, p0, d, [(Lb, .016, .017, black, .5), (.02, .022, .02, blk2, .5)])
    notes = scene_scale(s); act = [m for m in live(ev) if m >= 64]
    k = notes.index(max(act)) if act else None; lit = None if k is None else k - 1
    hp = holes(g, pts[0], d, Lb, .016, .017, 7, lit, dark=(90, 20, 26))
    chanter_fx(fx, ev, pts[-1], hp, lit)

BUILD = {"roman": roman, "cantigas": cantigas, "pijpzak": pijpzak, "musette": musette, "uilleann": uilleann,
         "northumbrian": northumbrian, "hevia": hevia}
def build(g, t, ev, fx, s):
    gs = Geo(); fxs = []
    k = s["key"]
    if k in HIGH: highland(gs, t, ev, fxs, s, HIGH[k])
    else: BUILD[k](gs, t, ev, fxs, s)
    M = SCR2M
    for P_, n, col, two, emit, gid, spec in gs.F: g.F.append((P_ @ M.T, M @ n, col, two, emit, gid, spec))
    for a, b, col, gid in gs.L: g.L.append((M @ a, M @ b, col, gid))
    for f in fxs: fx.append((f[0], tuple(M @ np.asarray(f[1], float))) + tuple(f[2:]))

# ---------------------------------------------------------------- sound: chanters and drones
_t = lambda n: np.arange(n)/SR
_PH = np.random.default_rng(5).random(80)*2*np.pi
_WT = {}
def wt(f, tilt, form, fbw, fgain, odd):
    key = (int(f*4), tilt, form, fbw, fgain, odd)
    if key not in _WT:
        K = int(max(1, min(64, 11000/f))); k = np.arange(1, K + 1); fk = k*f
        amp = k**(-tilt)*(1 + fgain*np.exp(-((fk - form)/fbw)**2))
        if odd: amp = amp*np.where(k % 2 == 1, 1., 1 - odd)
        x = np.arange(2048)/2048
        tb = (amp[:, None]*np.sin(2*np.pi*k[:, None]*x[None, :] + _PH[:K, None])).sum(0); _WT[key] = tb/np.abs(tb).max()
    return _WT[key]

HIGHLAND = dict(tilt=.75, form=2400, fbw=1100, fgain=2.2, odd=0., attack=.006, rel=.014, grace="hg")
TONES = {
 "ghb":          HIGHLAND,
 "ww1":          dict(HIGHLAND, form=2100, lpf=5200),
 "kintyre":      dict(HIGHLAND, form=2250),
 "roman":        dict(tilt=1.6, form=1100, fbw=600, fgain=.8, odd=.45, attack=.04, rel=.08, breath=.4, vib=.004, lpf=3600),
 "cantigas":     dict(tilt=1.0, form=1400, fbw=700, fgain=1.6, odd=.2, attack=.012, rel=.03, breath=.06, lpf=4600),
 "pijpzak":      dict(tilt=.85, form=1100, fbw=800, fgain=2.0, odd=.1, attack=.008, rel=.02, breath=.08, grace="cut", lpf=5000),
 "musette":      dict(tilt=1.5, form=1500, fbw=900, fgain=.7, odd=.5, attack=.02, rel=.05, breath=.04, vib=.003, lpf=3000),
 "uilleann":     dict(tilt=1.2, form=1800, fbw=900, fgain=1.1, odd=.25, attack=.01, rel=.03, breath=.03, grace="cut", lpf=4200),
 "northumbrian": dict(tilt=1.35, form=1700, fbw=800, fgain=.9, odd=.35, attack=.004, rel=.012, stacc=.32, lpf=4000),
 "hevia":        dict(tilt=1.0, form=1000, fbw=1, fgain=0., odd=.3, attack=.004, rel=.03, lpf=3400, grace="hg", detune=9),
}
DRONES = {   # (midi, amp, cents) voices, timbre, level
 "ghb":          dict(p=((57, 1., -1.5), (57, 1., 1.5), (45, 1.1, 0)), tilt=.9, form=900, fbw=600, fgain=1.2, odd=.55, v=.2, lpf=3600),
 "ww1":          dict(p=((57, 1., -2.), (57, 1., 2.), (45, 1.1, 0)), tilt=.95, form=850, fbw=600, fgain=1.1, odd=.55, v=.2, lpf=3000),
 "kintyre":      dict(p=((57, 1., -1.), (57, 1., 1.), (45, 1.1, 0)), tilt=.9, form=900, fbw=600, fgain=1.2, odd=.55, v=.2, lpf=3600),
 "roman":        dict(p=((45, 1., 0),), tilt=1.3, form=700, fbw=500, fgain=1., odd=.6, v=.34, lpf=2200),
 "cantigas":     dict(p=((45, 1., 0), (52, .8, 0)), tilt=1.1, form=800, fbw=500, fgain=1., odd=.5, v=.28, lpf=2600),
 "pijpzak":      dict(p=((45, 1., 0),), tilt=1.0, form=700, fbw=500, fgain=1.4, odd=.45, v=.36, lpf=2800),
 "musette":      dict(p=((45, .8, 0), (52, .5, 0), (57, .5, 0)), tilt=1.6, form=600, fbw=400, fgain=.5, odd=.7, v=.22, lpf=1800),
 "uilleann":     dict(p=((45, 1., 0), (57, .7, 0), (69, .3, 0)), tilt=1.3, form=800, fbw=500, fgain=.8, odd=.5, v=.22, lpf=2500),
 "northumbrian": dict(p=((57, 1., 0), (64, .55, 0), (69, .4, 0)), tilt=1.4, form=900, fbw=500, fgain=.6, odd=.6, v=.17, lpf=2600),
 "hevia":        dict(p=((45, 1., -6), (45, .8, 6), (57, .6, 0)), tilt=1.0, form=600, fbw=1, fgain=0., odd=0., v=.22, lpf=1100),
}
def chanter(f, dur, v, tn, grace=None, seed=0):
    d = max(.03, dur*tn.get("stacc", 1.)); rel = tn["rel"]; n = int((d + rel*6 + .02)*SR); t = _t(n); rg = np.random.default_rng(seed)
    fr = np.full(n, f)
    if grace: fr[:int(.028*SR)] = grace
    if tn.get("vib"): fr = fr*(1 + tn["vib"]*np.sin(2*np.pi*5.3*t))
    tb = wt(f, tn["tilt"], tn["form"], tn["fbw"], tn["fgain"], tn["odd"])
    ph = np.cumsum(fr)/SR + rg.random()
    x = tb[(ph*2048).astype(int) & 2047]
    if tn.get("detune"):
        ph2 = np.cumsum(fr*2**(tn["detune"]/1200))/SR + rg.random(); x = .6*x + .6*tb[(ph2*2048).astype(int) & 2047]
    env = np.minimum(1, t/tn["attack"])*np.where(t < d, 1., np.exp(-(t - d)/rel))
    if tn.get("breath"): x = x + tn["breath"]*D.bp(D.nz(n), 1500, 7000)
    if tn.get("lpf"): x = D.lp(x, tn["lpf"])
    return x*env*v*.3

def drone_voice(key, dur):
    dc = DRONES[key]; n = int((dur + .14)*SR); t = _t(n); x = np.zeros(n); rg = np.random.default_rng(len(key))
    for m, a, cents in dc["p"]:
        f = midi_hz(m)*2**(cents/1200); tb = wt(f, dc["tilt"], dc["form"], dc["fbw"], dc["fgain"], dc["odd"])
        x += a*tb[(((f*t + rg.random()) % 1.)*2048).astype(int) & 2047]
    env = np.minimum(1, t/.05)*np.clip((dur + .12 - t)/.08, 0, 1)
    if dc.get("lpf"): x = D.lp(x, dc["lpf"])
    return x*env*dc["v"]*.3

def grace_for(key, s, idx, m):
    tn = TONES[key]; gk = tn.get("grace")
    if gk == "hg": return midi_hz(G5) if m < G5 else (midi_hz(A5) if m == G5 else None)
    if gk == "cut":
        beat = s["ev"][idx][0]/BEAT_S
        return midi_hz(m + 3) if abs(beat - round(beat)) < .02 and m < 80 else None
    return None

def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]; tn = TONES[key]
    if key == "uilleann" and m < 66:                              # regulator chords: sweet, short
        x = chanter(midi_hz(m), dur, v*.7, dict(tn, stacc=.6, lpf=2400), seed=idx*5 + m)
    else:
        x = chanter(midi_hz(m), dur, v*(1 if tp else .78), tn, grace_for(key, s, idx, m), seed=idx*7 + m)
    if key == "hevia":                                             # a dotted-eighth echo on the electronic pipes
        dl = int(.75*BEAT_S*SR); y = np.zeros(len(x) + dl); y[:len(x)] += x; y[dl:] += .22*x; x = y
    if idx == 0:                                                   # the drones strike in with the first note and hold the whole scene
        last = s is SCENES[-1]
        dr = drone_voice(key, s["len"] - nt + (OUTRO_BARS*BAR_S + 2.6 if last else .03))
        y = np.zeros(max(len(x), len(dr))); y[:len(x)] += x; y[:len(dr)] += dr; x = y
    return x

# ---------------------------------------------------------------- backing: big beat (chopped break, pipe-band snare rolls, distorted bass)
def kick(v): return D.kick_s(48, 160, 9, .35, .5, 2.6, .5)*v
def bsnare(v): a = D.snare_s(190, 14, 24, 900, 9000, .85, .5); return np.tanh(2.4*a)/np.tanh(2.4)*v
def ghost(v): return D.snare_s(230, 32, 40, 1500, 8000, .6, .15)*v*.45
def hat(v): n = int(.07*SR); t = _t(n); return D.hp(D.nz(n), 7500)*np.exp(-t*65)*v*.5
def ohat(v): n = int(.32*SR); t = _t(n); return D.hp(D.nz(n), 6500)*np.exp(-t*9)*v*.42
def pbsn(v): a = D.snare_s(440, 46, 70, 2800, 14000, .95, .14); return np.tanh(1.6*a)*v*.85
def thunder(v):
    n = int(2.6*SR); t = _t(n)
    x = D.hp(D.nz(n), 1800)*np.exp(-t*14)*1.1 + D.lp(D.nz(n), 140)*np.exp(-t*1.3)*(1 - np.exp(-t*25))*6
    return np.tanh(x)*v
def crack(v): n = int(.45*SR); t = _t(n); return D.hp(D.nz(n), 2200)*np.exp(-t*20)*v*.7
def riser(v):
    n = int(BAR_S*SR); t = _t(n); q = t/BAR_S; nz_ = D.nz(n); out = np.zeros(n); seg = n//16
    for k in range(16):
        lo = 400*2**(k*.3); a = k*seg; b = n if k == 15 else (k + 1)*seg
        out[a:b] = D.bp(nz_[a:b], lo, min(lo*2.5, 16000))
    tone = np.sin(2*np.pi*np.cumsum(220*2**(2.5*q))/SR)*.3
    return (out + tone)*q*q*v
def impact(v): n = int(1.2*SR); t = _t(n); f = 40 + 60*np.exp(-t*8); return np.tanh(2*np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*3))*v
KIT = {"kick": kick, "bsnare": bsnare, "ghost": ghost, "hat": hat, "ohat": ohat, "pb": pbsn, "thunder": thunder, "crack": crack,
       "riser": riser, "impact": impact, "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("bsnare", "pb")
HATS = [(st, "hat", .5 if st % 4 == 0 else .34) for st in range(0, 16, 2)] + [(3, "hat", .2), (11, "hat", .2)]
BREAK_A = [(0, "kick", 1.), (7, "kick", .8), (10, "kick", .95), (4, "bsnare", 1.), (12, "bsnare", 1.),
           (6, "ghost", .35), (9, "ghost", .3), (15, "ghost", .4)] + [h for h in HATS if h[0] != 14] + [(14, "ohat", .4)]
BREAK_B = [(0, "kick", 1.), (2, "kick", .7), (10, "kick", .9), (11, "kick", .6), (4, "bsnare", 1.), (12, "bsnare", 1.), (14, "bsnare", .55),
           (7, "ghost", .35), (9, "ghost", .3)] + [h for h in HATS if h[0] != 6] + [(6, "ohat", .4)]
def drum_events(c):
    ev = []; Q = c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*c.BAR
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "bsnare", .9), (T0, "thunder", .7)]; continue
        roll = b == 3 or (4 <= b <= nb - 3 and (b - 4) % 2 == 1)     # a pipe-band roll leads into every new scene
        if b == 0: ev += [(T0, "thunder", 1.), (T0, "crash", 1.)]
        elif b >= 4 and (b - 4) % 2 == 0: ev.append((T0, "crash", .75))
        if b == 2: ev.append((T0, "riser", 1.))
        if b == 3: ev += [(T0, "crash", .95), (T0, "impact", 1.)]
        for st, kind, vel in (BREAK_A if b % 2 == 0 else BREAK_B):
            if roll and st >= 8 and kind != "kick": continue
            ev.append((T0 + st*Q/4, kind, vel))
        if roll:
            for k in range(24): ev.append((T0 + 2*Q + k*Q/12, "pb", .25 + .65*(k/23)**1.5))
            ev.append((T0 + 2*Q, "crack", .6))
    return ev
def _chord(b, beat):
    ch = BARS[b]; return ch[beat] if isinstance(ch, tuple) else ch
def _root(name):
    rt = CH[name][0]
    while rt > 40: rt -= 12
    while rt < 28: rt += 12
    return rt
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord(b, 0)), 2.4, 1.)); continue
        for bt, oc, d, v in ((0, 0, .45, 1.), (.75, 0, .2, .6), (1.5, 12, .35, .7), (2, 0, .45, .95), (2.75, 0, .2, .6), (3.5, 7, .35, .7)):
            out.append((T0 + bt*c.BEAT, _root(_chord(b, int(bt))) + oc, d*c.BEAT, v))
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .12)*SR); t = _t(n); ph = (f*t) % 1.
    x = .7*(2*ph - 1) + .5*np.sin(2*np.pi*f*t) + .25*np.sign(np.sin(2*np.pi*f*t))
    env = np.minimum(1, t/.003)*np.where(t < dur, np.exp(-t*1.5), math.exp(-dur*1.5)*np.exp(-(t - dur)/.03))
    return D.lp(np.tanh(2.6*x*env), 1100)*v
MIX = dict(drums=-19, bass=-23, verb=-24, verb_len=1.8)

# ---------------------------------------------------------------- background: a ring of standing stones on a moor in a storm
THEMES = {   # sky top, sky bottom, cloud, cloud light, ground far, ground near, stone, stone dark, carving glow, rain, lightning
 "intro":        ((24, 30, 42), (72, 84, 102), (40, 46, 60), (92, 102, 120), (44, 52, 56), (24, 30, 32), (118, 122, 128), (66, 70, 78), (220, 236, 255), (170, 186, 210), (240, 246, 255)),
 "roman":        ((70, 30, 22), (178, 94, 58), (96, 44, 32), (198, 122, 78), (96, 70, 42), (58, 42, 28), (170, 122, 90), (106, 72, 52), (228, 204, 92), (232, 184, 144), (255, 236, 182)),
 "cantigas":     ((30, 50, 122), (212, 198, 160), (52, 70, 140), (226, 214, 182), (150, 130, 90), (96, 84, 60), (206, 190, 152), (130, 116, 88), (255, 212, 80), (222, 212, 182), (255, 232, 144)),
 "pijpzak":      ((96, 66, 28), (208, 164, 82), (122, 86, 40), (224, 188, 112), (150, 110, 46), (98, 66, 30), (172, 142, 98), (110, 84, 54), (224, 98, 46), (234, 208, 152), (255, 236, 190)),
 "musette":      ((92, 120, 172), (198, 216, 238), (118, 144, 190), (238, 222, 232), (150, 162, 152), (100, 112, 104), (216, 208, 216), (150, 140, 158), (242, 172, 142), (238, 238, 252), (255, 240, 246)),
 "uilleann":     ((10, 50, 34), (62, 142, 98), (20, 70, 48), (112, 182, 132), (72, 62, 40), (44, 34, 24), (122, 128, 114), (66, 70, 60), (124, 255, 172), (172, 222, 192), (222, 255, 232)),
 "northumbrian": ((42, 28, 62), (128, 98, 152), (60, 42, 84), (166, 138, 188), (92, 62, 102), (58, 38, 64), (152, 152, 158), (90, 90, 100), (232, 182, 255), (202, 192, 232), (246, 236, 255)),
 "ww1":          ((60, 56, 40), (152, 142, 102), (80, 74, 52), (178, 168, 126), (102, 92, 62), (64, 56, 38), (142, 136, 114), (86, 82, 66), (232, 52, 42), (198, 192, 162), (255, 246, 222)),
 "kintyre":      ((38, 50, 112), (252, 142, 62), (90, 60, 100), (255, 188, 112), (60, 70, 102), (34, 40, 64), (152, 122, 122), (82, 70, 92), (92, 182, 255), (255, 212, 172), (255, 240, 212)),
 "hevia":        ((8, 36, 24), (42, 122, 72), (14, 50, 34), (82, 162, 112), (30, 80, 48), (18, 50, 30), (112, 132, 122), (56, 70, 64), (62, 255, 240), (152, 255, 240), (202, 255, 250)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[7], T_[1], .3))
def tcol(x, i): return np.array(mix(THEMES[x["th0"]][i], THEMES[x["th1"]][i], x["kb"]), float)

HZ = 200
XS = np.arange(RW)
SKYG = {k: F.make_grad(v[0], v[1], K=8, y0=0, y1=HZ) for k, v in THEMES.items()}
GRDG = {k: F.make_grad(v[4], v[5], K=6, y0=HZ, y1=RH - 30) for k, v in THEMES.items()}
SKYM = YY < HZ
def _cloud(seed, sig, thr, fall):
    n = np.random.default_rng(seed).random((HZ, 2*RW))
    d = gaussian_filter(n, sig, mode=("reflect", "wrap")); d = (d - d.mean())/d.std()
    d = d + fall*(1 - np.arange(HZ)[:, None]/HZ)
    return np.clip((d - thr)*.9, 0, 1)
CLOUD1, CLOUD2 = _cloud(3, (5, 16), .2, 1.), _cloud(8, (4, 9), .6, .8)
_hx = np.arange(RW)
HILL = (HZ - (5 + 4*np.sin(_hx*.03) + 3*np.sin(_hx*.071 + 1) + 2*np.sin(_hx*.17 + 2))).astype(int)
HILLM = (YY >= HILL[None, :]) & (YY < HZ)
_gr = np.random.default_rng(21)
GN = _gr.random((RH, RW))
SPEC_HI, SPEC_LO = (GN > .93) & (YY > HZ), (GN < .06) & (YY > HZ)
TUFTS = [(int(_gr.integers(0, RW)), int(HZ + 8 + _gr.random()**.7*(RH - HZ - 10)), _gr.random()*6.28) for _ in range(70)]
BGR = np.random.default_rng(31); NB = 170
RBX, RBY, RBS, RBL = BGR.random(NB)*(RW + 90), BGR.random(NB)*(RH + 40), .7 + BGR.random(NB)*.6, BGR.integers(4, 8, NB)
NF = 36
RFX, RFY, RFS, RFL = BGR.random(NF)*(RW + 90), BGR.random(NF)*(RH + 40), 1. + BGR.random(NF)*.5, BGR.integers(6, 10, NF)
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
BGCAM = P.Cam(DIST, 0, 1.0)
RING_R, RING_Y = 1.32, -.42
_sr = np.random.default_rng(17)
STONES = []
for _k, _a in enumerate((50, 82, 112, 140, 166, 194, 220, 248, 278, 310)):
    top = [(.44 + _sr.random()*.06, .72 + _sr.random()*.18), (.22, .95 + _sr.random()*.08), (-.12, 1.), (-.4, .86 + _sr.random()*.1), (-.48, .55)]
    STONES.append(dict(a=_a, h=.55 + _sr.random()*.35, w=.2 + _sr.random()*.1, poly=[(-.5, 0.), (.5, 0.)] + top, carve=_k % 3))
CARVE = {0: [(0, 0), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1), (2, -1), (2, 0), (2, 1), (2, 2), (1, 2), (0, 2), (-1, 2), (-2, 2)],
         1: [(0, 0)] + [(round(2.2*math.cos(a)), round(2.2*math.sin(a))) for a in np.linspace(0, 2*math.pi, 13)[:-1]],
         2: [(-2, -2), (-1, -1), (0, 0), (1, -1), (2, -2), (-2, 1), (-1, 2), (0, 3), (1, 2), (2, 1)]}

LIGHT_T, BOLTS = [], {}
def _bolt(seed):
    rg = np.random.default_rng(seed); x = 40 + rg.random()*190; y = 0.; main = [(x, y)]
    while y < HZ - 10:
        y += 6 + rg.random()*8; x += rg.normal(0, 5); main.append((x, y))
    br = []
    for _ in range(2):
        k = int(rg.integers(2, len(main) - 3)); bx, by = main[k]; seg = [(bx, by)]; dx = rg.choice([-1, 1])
        for _ in range(int(rg.integers(3, 6))):
            by += 5 + rg.random()*6; bx += dx*(3 + rg.random()*5) + rg.normal(0, 2); seg.append((bx, by))
        br.append(seg)
    return main, br
def flash(c, T):
    """lightning on the first downbeat and on every pipe-band roll: (flash strength, bolt index or None)"""
    if not LIGHT_T: LIGHT_T.extend(sorted({round(t, 4) for t, k, v in c.DRUMS if k in ("thunder", "crack")}))
    i = bisect.bisect_right(LIGHT_T, T + 1e-6) - 1
    if i < 0: return 0., None
    age = T - LIGHT_T[i]
    if age > .7: return 0., None
    fl = math.exp(-age*6)*(1. if (age < .07 or .12 < age < .17 or .25 < age < .29) else .4)*(1.25 if i == 0 else 1.)
    return fl, (i if age < .3 and fl > .35 else None)

def rain(img, T, X0, Y0, SP, L, col, alpha):
    x = (X0 - 380*SP*T) % (RW + 90) - 30; y = (Y0 + 560*SP*T) % (RH + 40) - 20; k = np.arange(10)[None, :]
    px = (x[:, None] + .56*k).astype(int); py = (y[:, None] - .83*k).astype(int)
    m = (k < L[:, None]) & (px >= 0) & (px < RW) & (py >= 0) & (py < RH)
    img[py[m], px[m]] = img[py[m], px[m]]*(1 - alpha) + col*alpha

def background(c, x):
    T, kb = x["T"], x["kb"]
    bg = np.where(SKYM[..., None], SKYG[x["th0"]]*(1 - kb) + SKYG[x["th1"]]*kb, GRDG[x["th0"]]*(1 - kb) + GRDG[x["th1"]]*kb)
    fl, bi = flash(c, T); bolt = tcol(x, 10)
    cl, chi = tcol(x, 2), tcol(x, 3)
    cl, chi = cl + (bolt - cl)*fl*.55, chi + (bolt - chi)*fl*.7
    o1, o2 = int(T*28) % (2*RW), int(T*64) % (2*RW)
    b = BAY[:HZ]
    l1 = np.floor(CLOUD1[:, (XS + o1) % (2*RW)]*3 + b*.999)[..., None]/3
    l2 = np.floor(CLOUD2[:, (XS + o2) % (2*RW)]*3 + b*.999)[..., None]/3
    sky = bg[:HZ]
    sky = sky + (bolt - sky)*fl*.4
    sky = sky*(1 - l1*.75) + chi*(l1*.75)
    sky = sky*(1 - l2) + cl*l2
    bg[:HZ] = sky
    gf = tcol(x, 4); bg[HILLM] = mix(tuple(gf), tuple(tcol(x, 1)), .35)
    gn = tcol(x, 5); st, sd, glow = tcol(x, 6), tcol(x, 7), tcol(x, 8)
    bg[SPEC_HI] = bg[SPEC_HI]*.6 + np.array(mix(tuple(gf), tuple(st), .5), float)*.4
    bg[SPEC_LO] *= .72
    bg[~SKYM] = bg[~SKYM]*(1 + fl*.35)
    im = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8)); d = ImageDraw.Draw(im)
    if bi is not None:                                              # the bolt itself
        if bi not in BOLTS: BOLTS[bi] = _bolt(bi*13 + 5)
        main, br = BOLTS[bi]; hc = tuple(int(v) for v in mix(tuple(bolt), tuple(tcol(x, 1)), .4))
        for seg in [main] + br: d.line(seg, fill=hc, width=3)
        for seg in [main] + br: d.line(seg, fill=tuple(int(v) for v in bolt), width=1)
    tc = tuple(int(v) for v in mix(tuple(gf), tuple(tcol(x, 1)), .25))
    for tx, ty, ph in TUFTS:                                        # heather tufts bending in the wind
        lean = -2 - 1.5*math.sin(T*5 + ph)
        d.line([(tx, ty), (tx + lean, ty - 4)], fill=tc); d.line([(tx + 1, ty), (tx + 1 + lean*.6, ty - 3)], fill=tc)
    # the stone ring (turns with the camera sway); carvings glow with each chanter note
    rot = SWAY*math.sin(2*math.pi*T/8)
    oi = bisect.bisect_right(c.ONSETS, T) - 1; oage = T - c.ONSETS[oi] if oi >= 0 else 9.
    loud = float(x["loud"])
    items = []
    for k, stn in enumerate(STONES):
        a = math.radians(stn["a"] + rot); px, pz = RING_R*math.sin(a), RING_R*math.cos(a)
        base = np.array([px, RING_Y, pz]); up = np.array([0, stn["h"], 0]); tg = np.array([math.cos(a), 0, -math.sin(a)])*stn["w"]
        Sc, dd = BGCAM.proj(np.stack([base, base + up, base + tg]))
        items.append((dd[0], k, Sc))
    for depth, k, Sc in sorted(items, key=lambda z: -z[0]):
        stn = STONES[k]; b0, t0, w0 = Sc; wv, hv = (w0 - b0), (t0 - b0)
        poly = [tuple(b0 + wv*u*2 + hv*v) for u, v in stn["poly"]]
        fog = clamp((depth - 3.6)/2.2)*.35
        c0 = tuple(int(v) for v in mix(tuple(st), tuple(tcol(x, 1)), fog)); c1 = tuple(int(v) for v in mix(tuple(sd), tuple(tcol(x, 1)), fog))
        d.polygon(poly, fill=c0)
        shade = [tuple(b0 + wv*max(u, .12)*2 + hv*v) for u, v in stn["poly"]]
        d.polygon(shade, fill=c1)
        d.line(poly + [poly[0]], fill=tuple(int(v*.45) for v in c1))
        if fl > .3: d.line([poly[-1], poly[-2], poly[-3]], fill=tuple(int(v) for v in bolt))
        hpx = float(np.hypot(*hv))
        if hpx > 26:
            gk = .15 + .3*loud + (1.3*math.exp(-oage*5) if oi >= 0 and oi % len(STONES) == k else 0.)
            gcol = tuple(int(v) for v in mix(tuple(sd), tuple(glow), clamp(gk)))
            cc = b0 + wv*(-.1)*2 + hv*.55; sc = 1 if hpx < 55 else 2
            for ux, uy in CARVE[stn["carve"]]:
                X_, Y_ = int(cc[0] + ux*sc), int(cc[1] + uy*sc)
                d.rectangle([X_, Y_, X_ + sc - 1, Y_ + sc - 1], fill=gcol)
    bg = np.asarray(im).astype(float)
    rain(bg, T, RBX, RBY, RBS, RBL, tcol(x, 9), .42)
    bg[TEXTB] *= .5; bg[TEXTE] *= .72
    return np.clip(bg, 0, 255).astype(np.uint8)

def render3d(c, ctx, s, t, scale, spin, fx):
    """the shared lit render, then a layer of sideways rain and the lightning flash over everything"""
    img, cam = pp_light.render_lit(c, ctx, s, t, scale, spin, fx)
    out = img.astype(float)
    rain(out, ctx["T"], RFX, RFY, RFS, RFL, tcol(ctx, 9), .35)
    fl, _ = flash(c, ctx["T"])
    if fl > 0: out = out + (tcol(ctx, 10) - out)*fl*.22
    return np.clip(out, 0, 255).astype(np.uint8), cam

if __name__ == "__main__":
    F.run(sys.modules[__name__])

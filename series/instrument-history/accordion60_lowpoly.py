"""Pixel Pitch: history of the accordion, 60-second cut.  python3 accordion60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys, types
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as TR
import piano_lowpoly as P
import cello_lowpoly as C
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, ease, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "accordion60_lowpoly.mp4", "ad"
BPM = 100
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.3, 44, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE ACCORDION", "2,450 YEARS", "TODAY", -500
INTRO_LINES = (("120 BASS BUTTONS,", "ALL FOR ONE HAND"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("2,450 YEARS FROM", "THE SHENG")
OUTRO_CREDIT = "DIGITAL CUMBIA"
LIGHTING = True

# ---------------------------------------------------------------- music (digital cumbia, 100 BPM)
CH = {"Am": (45, (57, 60, 64)), "Dm": (38, (57, 62, 65)), "G": (43, (55, 59, 62)), "E": (40, (56, 59, 64)), "E7": (40, (56, 59, 62)),
      "C": (36, (55, 60, 64)), "G7": (43, (53, 59, 62)), "A7": (45, (55, 61, 64)), "F": (41, (57, 60, 65)), "D7": (38, (54, 57, 60)),
      "C7": (36, (55, 58, 64))}
BARS = ["Am", "Dm", "G", "E",                                   # showcase
        "Am", ("G", "G", "Am", "Am"),                           # sheng
        "C", ("G7", "G7", "C", "C"),                            # Demian
        "Dm", ("A7", "A7", "Dm", "Dm"),                         # bandoneon
        "F", ("D7", "D7", "G7", "C7"),                          # 1910 rag
        "C", ("G", "G", "C", "C"),                              # Cajun
        "Am", ("E7", "E7", "Am", "Am"),                         # musette
        "F", ("C7", "C7", "F", "F"),                            # polka
        "G", ("D7", "D7", "G", "G"),                            # vallenato
        ("Am", "Am", "F", "F"), ("C", "C", "G", "G"),           # europop
        "Am", "Dm",                                             # modern again
        ("G", "G", "E", "E"),                                   # outro
        "Am"]                                                   # final chord
def r(items, v=.85): return [(b, m, d, v) for b, m, d in items]
C1, C2, C3, G1, G2 = (60, 64, 67), (64, 67, 72), (67, 72, 76), (59, 62, 65, 67), (62, 65, 67, 71)
SHOW = r([(0, [72, 76], .5), (.75, [72, 76], .25), (1, [74, 77], .5), (1.5, [72, 76], .5), (2, [71, 74], .5), (2.5, [69, 72], .5),
          (3, [71, 74], .25), (3.25, [72, 76], .25), (3.5, [69, 72], .5),
          (4, [74, 77], .5), (4.75, [74, 77], .25), (5, [76, 79], .5), (5.5, [77, 81], .5), (6, [76, 79], .5), (6.5, [74, 77], .5),
          (7, [72, 76], .5), (7.5, [74, 77], .5),
          (8, [71, 74], .5), (8.75, [71, 74], .25), (9, [74, 77], .5), (9.5, [71, 74], .5), (10, [67, 71], .5), (10.5, [71, 74], .5),
          (11, [74, 79], .5), (11.5, [71, 74], .5),
          (12, [68, 71], .5), (12.75, [68, 71], .25), (13, [71, 74], .5), (13.5, [68, 71], .5), (14, [64, 68], .5), (14.5, [68, 71], .25),
          (14.75, [71, 74], .25), (15, [71, 76], .5), (15.5, [68, 74], .5)], .85)
OUTRO_N = r([(8, [71, 74], .5), (8.75, [71, 74], .25), (9, [74, 79], .5), (9.5, [71, 74], .5), (10, [68, 71], .5), (10.5, [71, 74], .5),
             (11, [74, 76], .25), (11.25, [71, 74], .25), (11.5, [68, 71], .5)], .85) + [(12, [69, 72, 76, 81], 4.3, .95)]

SCENES = [
 dict(key="morino", theme="intro", name="HOHNER MORINO+ V", credit="DIGITAL CUMBIA", notes=SHOW),
 dict(key="sheng", theme="sheng", year=-433, ys="433 BC", name="SHENG", credit="ANCIENT CHINESE COURT", size=1.2,
      fact=("FREE REEDS SET IN", "BAMBOO PIPES"),
      notes=r([(0, [69, 76], 1.5), (1.5, [72, 79], .5), (2, [74, 81], 1), (3, [72, 79], 1), (4, [67, 74], 1), (5, [69, 76], .5),
               (5.5, [72, 76], .5), (6, [69, 76], 2)], .85)),
 dict(key="demian", theme="demian", year=1829, ys="1829", name="DEMIAN'S ACCORDION", credit="VIENNESE SALON", size=1.35,
      fact=("ONE BUTTON PLAYS", "A WHOLE CHORD"),
      notes=r([(0, C1, 1), (1, C2, .5), (1.5, C3, .5), (2, C2, 1), (3, C1, 1), (4, G1, 1), (5, G2, .5), (5.5, G2, .5), (6, C2, 1), (7, C3, 1)], .8)),
 dict(key="bandoneon", theme="bandoneon", year=1870, ys="1870s", name="BANDONEON", credit="ARGENTINE TANGO", size=1.15,
      fact=("MADE FOR HYMNS,", "STOLEN BY TANGO"),
      notes=r([(0, [50, 62], .25), (.75, [50, 62], .25), (1.5, [57, 65], .25), (2, [62, 69], .75), (3, [61, 69], .25), (3.5, [62, 65], .5),
               (4, [57, 61, 64], .25), (4.75, [57, 61, 64], .25), (5.5, [57, 61, 67], .25), (6, [62, 65, 69], .5), (6.5, [64, 67], .25),
               (6.75, [65, 69], .25), (7, [62, 74], .75)], .9)),
 dict(key="piano1910", theme="piano1910", year=1910, ys="1910", name="PIANO ACCORDION", credit="VAUDEVILLE RAG", size=1.12,
      fact=("COINED THE NAME", "PIANO-ACCORDION"),
      notes=r([(0, 77, .5), (.5, 76, .25), (.75, 77, .5), (1.25, 81, .25), (1.5, 84, .5), (2, 81, .5), (2.5, 77, .25), (2.75, 74, .5),
               (3.25, 72, .25), (3.5, 69, .5), (4, 78, .5), (4.5, 81, .5), (5, 84, .5), (5.5, 78, .5), (6, 79, .5), (6.5, 77, .25),
               (6.75, 74, .5), (7.25, 72, .25), (7.5, 76, .5)], .85)),
 dict(key="cajun", theme="cajun", year=1928, ys="1928", name="CAJUN ACCORDION", credit="CAJUN TWO-STEP", size=1.2,
      fact=("1ST CAJUN RECORD", "STARS AN ACCORDION"),
      notes=r([(0, [72, 76], .5), (.5, [72, 76], .5), (1, [76, 79], .5), (1.5, [72, 76], .5), (2, [74, 77], .25), (2.25, [76, 79], .25),
               (2.5, [79, 84], .5), (3, [76, 79], .5), (3.5, [72, 76], .5), (4, [71, 74], .5), (4.5, [74, 79], .5), (5, [71, 74], .5),
               (5.5, [67, 71], .5), (6, [72, 76], .5), (6.5, [76, 79], .25), (6.75, [79, 84], .25), (7, [72, 76, 84], 1)], .9)),
 dict(key="musette", theme="musette", year=1934, ys="1930s", name="MUSETTE ACCORDION", credit="BAL-MUSETTE WALTZ", size=1.04,
      fact=("3 REEDS DETUNED", "FOR THAT WOBBLE"),
      notes=r([(0, 76, .75), (.75, 75, .25), (1, 76, .5), (1.5, 81, .5), (2, 79, .25), (2.25, 77, .25), (2.5, 76, .5), (3, 72, .5),
               (3.5, 69, .5), (4, 71, .75), (4.75, 70, .25), (5, 71, .5), (5.5, 74, .25), (5.75, 76, .25), (6, 72, .5), (6.5, 76, .5),
               (7, 81, 1)], .85)),
 dict(key="golden", theme="golden", year=1955, ys="1955", name="GOLDEN-AGE ACCORDION", credit="POLKA",
      fact=("ON THE WELK SHOW", "FOR 27 YEARS"),
      notes=r([(0, [69, 72], .5), (.5, [72, 77], .5), (1, [69, 72], .5), (1.5, [65, 69], .5), (2, [67, 70], .25), (2.25, [69, 72], .25),
               (2.5, [70, 74], .25), (2.75, [69, 72], .25), (3, [67, 70], .5), (3.5, [65, 69], .5), (4, [64, 67], .5), (4.5, [67, 70], .5),
               (5, [64, 67], .5), (5.5, [60, 64], .5), (6, [65, 69], .5), (6.5, [69, 72], .5), (7, [72, 77], .75)], .9)),
 dict(key="vallenato", theme="vallenato", year=1968, ys="1968", name="VALLENATO ACCORDION", credit="VALLENATO", size=1.25,
      fact=("THREE ROWS, THREE", "REEDS PER NOTE"),
      notes=r([(0, [71, 74], .5), (.5, [71, 74], .25), (.75, [74, 79], .25), (1, [74, 79], .5), (1.5, [71, 74], .25), (1.75, [69, 72], .25),
               (2, [67, 71], .5), (2.5, [71, 74], .5), (3, [74, 79], .25), (3.25, [72, 76], .25), (3.5, [71, 74], .5),
               (4, [66, 69], .5), (4.5, [66, 69], .25), (4.75, [69, 72], .25), (5, [69, 74], .5), (5.5, [66, 72], .25), (5.75, [69, 72], .25),
               (6, [71, 74], .5), (6.5, [74, 79], .5), (7, [71, 79], .75)], .9)),
 dict(key="roland", theme="roland", year=2005, ys="2005", name="ROLAND V-ACCORDION", credit="EUROPOP",
      fact=("NO REEDS AT ALL:", "IT'S ALL DIGITAL"),
      notes=r([(0, 76, .5), (.5, 76, .25), (.75, 79, .5), (1.25, 76, .25), (1.5, 74, .5), (2, 72, .5), (2.5, 72, .25), (2.75, 77, .5),
               (3.25, 72, .25), (3.5, 69, .5), (4, 72, .5), (4.5, 72, .25), (4.75, 76, .5), (5.25, 79, .25), (5.5, 76, .5), (6, 74, .5),
               (6.5, 74, .25), (6.75, 79, .5), (7.25, 74, .25), (7.5, 71, .5)], .85)),
 dict(key="morino", theme="intro", year=2020, ys="2020s", name="HOHNER MORINO+ V", credit="DIGITAL CUMBIA",
      fact=("13 REGISTERS IN", "A 12.4 KG BOX"),
      notes=[e for e in SHOW if e[0] < 8] + OUTRO_N),
]
TRIM = {}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)

class Fr:
    """a plane frame: origin o, axis u (normal, toward the bass end), s (up the plane), w (depth)"""
    def __init__(s_, o, u, sv, w=(0, 0, 1.)):
        s_.o = np.asarray(o, float); s_.u = np.asarray(u, float); s_.s = np.asarray(sv, float); s_.w = np.asarray(w, float)
    def p(s_, u, sv, w): return s_.o + u*s_.u + sv*s_.s + w*s_.w

def fan(xa, y0, g0, th, q):
    """bellows frame q in [0,1]: the bellows open as a fan (g0 at the bottom, rotating by th about the bottom edge)"""
    a = th*q; c, sn = math.cos(a), math.sin(a)
    return Fr((xa + g0*q, y0, 0), (c, -sn, 0), (sn, c, 0))

_BF = {"w0": (0, 1, 3, 2), "w1": (4, 5, 7, 6), "u0": (0, 2, 6, 4), "u1": (1, 3, 7, 5), "s0": (0, 1, 5, 4), "s1": (2, 3, 7, 6)}
def tbox(g, fr, u0, u1, s0, s1, w0, w1, col, only=None, **kw):
    v = [fr.p(u, sv, w) for w in (w0, w1) for sv in (s0, s1) for u in (u0, u1)]
    c = fr.p((u0 + u1)/2, (s0 + s1)/2, (w0 + w1)/2)
    for k, idx in _BF.items():
        if only is None or k in only: g.face([v[i] for i in idx], col, ref=c, **kw)

def disc(g, c, nrm, rad, col, n=6, **kw):
    c = np.asarray(c, float); nrm = unit(nrm)
    g.face(S.ring(c, nrm, rad, n, .3), col, ref=c - nrm*.01, **kw)

def quad_w(g, fr, w, u0, u1, s0, s1, col, out=1, **kw):
    """flat decal on a w-plane of a frame, facing +w (out=1) or -w"""
    g.face([fr.p(u0, s0, w), fr.p(u1, s0, w), fr.p(u1, s1, w), fr.p(u0, s1, w)], col, ref=fr.p((u0 + u1)/2, (s0 + s1)/2, w - out*.01), **kw)

def quad_u(g, fr, u, s0, s1, w0, w1, col, out=1, **kw):
    g.face([fr.p(u, s0, w0), fr.p(u, s1, w0), fr.p(u, s1, w1), fr.p(u, s0, w1)], col, ref=fr.p(u - out*.01, (s0 + s1)/2, (w0 + w1)/2), **kw)

def path(g, pts, r0, col, r1=None, n=8, spec=.3):
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
    return P_[-1]

def bellows(g, frames, poly, dep, col, col2, tape=None, corner=None):
    """folded bellows through 2N+1 frames: even frames are the outer ridges, odd frames are inset by dep"""
    ss = [p[0] for p in poly]; ww = [p[1] for p in poly]
    sc, wc, hs, hw = (min(ss) + max(ss))/2, (min(ww) + max(ww))/2, (max(ss) - min(ss))/2, (max(ww) - min(ww))/2
    def ins(d): return [(sc + (s_ - sc)*(1 - d/hs), wc + (w_ - wc)*(1 - d/hw)) for s_, w_ in poly]
    pin = ins(dep)
    rings = [[fr.p(0, s_, w_) for s_, w_ in (pin if k % 2 else poly)] for k, fr in enumerate(frames)]
    axis = [fr.p(0, sc, wc) for fr in frames]
    m = len(poly)
    for k in range(len(frames) - 1):
        A, B = rings[k], rings[k + 1]; ref = (axis[k] + axis[k + 1])/2; cc = col if k % 2 == 0 else col2
        for i in range(m):
            j = (i + 1) % m
            g.face([A[i], A[j], B[j], B[i]], cc, ref=ref)
    if tape:
        for k in range(2, len(frames) - 2, 2):
            R = rings[k]
            for i in range(m):
                j = (i + 1) % m
                if poly[i][1] > min(ww) + 1e-6 or poly[j][1] > min(ww) + 1e-6: g.line(R[i], R[j], tape)
    if corner:
        s0, s1, w1 = min(ss), max(ss), max(ww)
        for k in range(2, len(frames) - 2, 2):
            fr = frames[k]
            tbox(g, fr, -.007, .007, s1 - .026, s1 + .003, w1 - .026, w1 + .003, corner, only=("w1", "s1"), spec=.6)
            tbox(g, fr, -.007, .007, s0 - .003, s0 + .026, w1 - .026, w1 + .003, corner, only=("w1", "s0"), spec=.6)

def breath(t, ph=0.):
    """bellows opening 0..1: pulled open over one bar, pushed closed over the next"""
    x = (t/BAR_S + ph) % 2
    return ease(x) if x < 1 else 1 - ease(x - 1)

def live_set(ev): return {e[1] for e in ev if e[0] < e[3] + .03}

def fold(m, lo, hi):
    while m < lo: m += 12
    while m > hi: m -= 12
    return m

_BASS = []
def bass_live(T):
    if not _BASS: _BASS.extend(bass_events(types.SimpleNamespace(BAR=BAR_S, BEAT=BEAT_S)))
    best = None
    for T0, m, dur, v in _BASS:
        if T0 <= T < T0 + dur + .04: best = m
    return best

# ---------------------------------------------------------------- instruments (modelled on reference photos; metres-ish, y up, treble box on the left = player's right)
CHROME, GOLDC, BRASSC = (222, 224, 232), (236, 196, 96), (214, 172, 80)
ACC = {
 "morino":    dict(kind="piano", H=.96, D=.40, Wt=.21, Wb=.19, N=14, g=(.1, .28), th=(6, 22), body=(26, 26, 30), spec=.55,
                   bel=(40, 40, 46), bel2=(12, 12, 15), tape=(118, 120, 128), low=53, nwhite=24, nat=(242, 240, 232), sharp=(20, 20, 24),
                   regs=13, cheek=(14, 14, 16), grille="morino", bass=(6, 20), bbtn=(238, 236, 228), strap=(34, 30, 30)),
 "golden":    dict(kind="piano", H=.96, D=.40, Wt=.21, Wb=.19, N=14, g=(.1, .28), th=(6, 22), body=(238, 232, 216), spec=.3,
                   bel=(30, 30, 34), bel2=(16, 16, 20), tape=(240, 240, 240), corner=CHROME, low=53, nwhite=24, nat=(246, 242, 230),
                   sharp=(26, 22, 26), cheek=(30, 28, 34), grille="golden", bass=(6, 20), bbtn=(30, 28, 32), strap=(150, 30, 44),
                   band=(36, 32, 40)),
 "roland":    dict(kind="piano", H=.94, D=.40, Wt=.21, Wb=.19, N=14, g=(.1, .28), th=(6, 22), body=(74, 76, 84), spec=.3,
                   bel=(28, 28, 32), bel2=(14, 14, 18), tape=(120, 124, 136), low=53, nwhite=24, nat=(240, 240, 240), sharp=(22, 22, 26),
                   cheek=(40, 42, 48), grille="roland", bass=(6, 20), bbtn=(28, 28, 32), strap=(30, 30, 34)),
 "piano1910": dict(kind="piano", H=.8, D=.36, Wt=.19, Wb=.15, N=10, g=(.08, .22), th=(5, 18), body=(96, 42, 26), spec=.2,
                   bel=(42, 22, 18), bel2=(24, 12, 10), tape=BRASSC, corner=BRASSC, low=60, nwhite=15, nat=(236, 222, 186),
                   sharp=(28, 20, 16), kd=.045, kframe=(118, 58, 34), cheek=(118, 58, 34), grille="pierced", pattern=0,
                   bass=(3, 8), bbtn=(236, 222, 186), strap=(110, 60, 30), brass=BRASSC),
 "musette":   dict(kind="buttons", H=.9, D=.38, Wt=.2, Wb=.18, N=13, g=(.09, .26), th=(5, 20), body=(30, 124, 90), spec=.4,
                   bel=(28, 28, 32), bel2=(14, 14, 18), tape=CHROME, corner=CHROME, rows=5, per=17, btn=(242, 238, 226),
                   board=(18, 16, 20), low=53, grille="pierced", pattern=1, bass=(5, 16), bbtn=(242, 238, 226), strap=(60, 40, 30),
                   chrome=CHROME),
 "vallenato": dict(kind="dia3", H=.7, D=.34, Wt=.17, Wb=.13, N=10, g=(.07, .2), th=(6, 24), body=(182, 28, 40), spec=.4,
                   bel=(232, 184, 72), bel2=(198, 148, 46), tape=(40, 24, 24), corner=(40, 24, 24), per=(10, 11, 10),
                   btn=(246, 244, 238), board=(18, 16, 20), grille="corona", bass=(2, 6), bbtn=(246, 244, 238), strap=(30, 24, 22),
                   gold=GOLDC),
 "cajun":     dict(kind="single", H=.74, D=.34, Wt=.17, Wb=.14, N=9, g=(.07, .18), th=(5, 17), body=(24, 22, 24), spec=.45,
                   bel=(26, 26, 28), bel2=(12, 12, 14), tape=BRASSC, corner=BRASSC, btn=(244, 240, 228), grille="cajun",
                   bass=(1, 2), bbtn=(244, 240, 228), strap=(96, 56, 30), gold=BRASSC, pink=(234, 150, 162)),
}

def piano_keys(g, xf, y0, s0, s1, zb, zf, low, nwhite, act, nat, sharp, kd=.03):
    """treble keyboard on the left face (x = xf), keys running front-to-back, lowest note at the top"""
    km = P.keymap(low, nwhite); kw = (s1 - s0)/nwhite; ytop = y0 + s1; xs = xf - kd
    g.box(xs, xf, ytop - nwhite*kw, ytop, zb, zf, nat)
    for m, (blk, pos) in km.items():
        if not blk and m in act:
            ya, yb = ytop - (pos + 1)*kw, ytop - pos*kw
            g.face([(xs - .0015, ya + .1*kw, zb), (xs - .0015, yb - .1*kw, zb), (xs - .0015, yb - .1*kw, zf), (xs - .0015, ya + .1*kw, zf)],
                   HOT, ref=(xf, (ya + yb)/2, (zb + zf)/2), emit=True)
    sep = mix(nat, (0, 0, 0), .5)
    for i in range(1, nwhite):
        y = ytop - i*kw; g.line((xs - .001, y, zb), (xs - .001, y, zf), sep)
    for m, (blk, pos) in km.items():
        if blk:
            yc = ytop - pos*kw; w = .58*kw; on = m in act
            g.box(xs - .018 + (.007 if on else 0), xs, yc - w/2, yc + w/2, zf - .6*(zf - zb), zf, HOT if on else sharp, emit=on)
    return km, kw, ytop

def key_y(km, kw, ytop, m):
    blk, pos = km[m]; return ytop - pos*kw if blk else ytop - (pos + .5)*kw

def grille(g, c, fr0, Wt, H, D, T, act):
    """front (+z) face of the treble box"""
    zf = D/2; u0, u1 = -Wt + .016, -.016; kind = c["grille"]
    if kind == "morino":                                     # two panels of vertical chrome bars that hook toward the keys at the top
        for sa, sb in ((.1*H, .52*H), (.55*H, .95*H)):
            quad_w(g, fr0, zf + .004, u0, u1, sa, sb, (34, 34, 40), spec=.3)
            for k in range(6):
                u = lerp(u0 + .02, u1 - .012, k/5)
                TR.tube(g, fr0.p(u, sa + .015, zf + .011), fr0.p(u, sb - .05 + .006*k, zf + .011), .0055, col=CHROME, n=5, spec=.9)
                TR.tube(g, fr0.p(u, sb - .05 + .006*k, zf + .011), fr0.p(u - .03, sb - .02 + .004*k, zf + .011), .0055, col=CHROME, n=5, spec=.9)
    elif kind == "golden":                                   # chrome grille, vertical slots, a gold diamond with rhinestones; black lower band
        quad_w(g, fr0, zf + .002, -Wt + .004, -.004, .02*H, .18*H, c["band"])
        quad_w(g, fr0, zf + .004, u0, u1, .22*H, .86*H, (206, 210, 222), spec=.9)
        for k in range(7):
            u = lerp(u0 + .02, u1 - .02, k/6)
            quad_w(g, fr0, zf + .0065, u - .004, u + .004, .25*H, .83*H, (70, 72, 86))
        cu, cs = (u0 + u1)/2, .54*H; ru, rs = (u1 - u0)*.36, .14*H
        dia = [fr0.p(cu, cs + rs, zf + .012), fr0.p(cu + ru, cs, zf + .012), fr0.p(cu, cs - rs, zf + .012), fr0.p(cu - ru, cs, zf + .012)]
        for k in range(4): TR.tube(g, dia[k], dia[(k + 1) % 4], .006, col=GOLDC, n=5, spec=.8)
        for k in range(12):
            a, b = dia[k % 4], dia[(k + 1) % 4]; p = a + (b - a)*((k//4 + 1)/4)
            tw = .5 + .5*math.sin(T*9 + k*1.7)
            disc(g, p + np.array([0, 0, .006]), (0, 0, 1), .007, mix((255, 120, 190) if k % 2 else (180, 220, 255), (255, 255, 255), tw), n=4, emit=True)
        for k in range(7):                                    # register tabs above the grille
            u = lerp(u0 + .01, u1 - .01, k/6)
            tbox(g, fr0, u - .009, u + .009, .885*H, .955*H, zf, zf + .014, (246, 244, 238), spec=.3)
    elif kind == "roland":                                   # dark panel with angled slots, a register column with LEDs, a small display
        quad_w(g, fr0, zf + .004, u0, u1, .1*H, .96*H, (46, 48, 56), spec=.2)
        quad_w(g, fr0, zf + .007, u0 + .05, u1 - .02, .885*H, .935*H, (80, 190, 255), emit=True)
        lit = int(T/BAR_S) % 12
        for k in range(12):
            s_ = lerp(.82*H, .14*H, k/11)
            tbox(g, fr0, u0 + .005, u0 + .035, s_ - .014, s_ + .014, zf + .004, zf + .014, (34, 34, 40))
            quad_w(g, fr0, zf + .0145, u0 + .01, u0 + .016, s_ - .005, s_ + .005, (140, 220, 255) if k == lit else (60, 64, 74), emit=k == lit)
        for j in range(14):
            s_ = lerp(.15*H, .84*H, j/13); off = .03*abs(j - 6.5)/6.5
            for k in range(3):
                u = u0 + .06 + off + k*.034
                quad_w(g, fr0, zf + .006, u, u + .024, s_ - .008, s_ + .008, (98, 104, 116))
    elif kind == "pierced":                                  # ornate pierced silver plate (1910 floral / 1930s lattice)
        quad_w(g, fr0, zf + .004, u0, u1, .26*H, .95*H, (204, 204, 212), spec=.7)
        nu, ns = 6, 18
        for i in range(nu):
            for j in range(ns):
                if c["pattern"] == 0: hole = (i + j) % 3 == 0 or (j % 5 == 2 and i in (1, 4))
                else: hole = (i + j) % 2 == 0 and j % 6 != 0
                if not hole: continue
                u = lerp(u0 + .014, u1 - .014, i/(nu - 1)); s_ = lerp(.29*H, .92*H, j/(ns - 1))
                quad_w(g, fr0, zf + .0065, u - .006, u + .006, s_ - .006, s_ + .006, (26, 20, 22))
        cu, cs = (u0 + u1)/2, .6*H
        jewel = (210, 36, 40) if c["pattern"] == 0 else (230, 232, 240)
        g.face([fr0.p(cu + .045*math.cos(a), cs + .07*math.sin(a), zf + .009) for a in np.linspace(0, 2*math.pi, 11)[:-1]],
               jewel, ref=fr0.p(cu, cs, 0), spec=.8)
        if c.get("brass"):
            for sv in (.02*H, .98*H):
                for uu in (-Wt + .012, -.012): quad_w(g, fr0, zf + .002, uu - .012, uu + .012, sv - .012, sv + .012, c["brass"], spec=.7)
        if c.get("chrome"):
            for sv in (.06*H, .22*H): quad_w(g, fr0, zf + .002, -Wt + .006, -.006, sv - .008, sv + .008, c["chrome"], spec=.9)
    elif kind == "corona":                                   # horizontal chrome bars over a dark panel; gold scrolls below
        quad_w(g, fr0, zf + .003, u0, u1, .42*H, .96*H, (40, 18, 22))
        for k in range(9):
            s_ = lerp(.45*H, .93*H, k/8)
            TR.tube(g, fr0.p(u0 + .004, s_, zf + .01), fr0.p(u1 - .004, s_, zf + .01), .0065, col=CHROME, n=5, spec=.9)
        for k in range(3):
            pts = [fr0.p(lerp(u0, u1, q), (.12 + .1*k)*H + .025*math.sin(q*9 + k), zf + .002) for q in np.linspace(0, 1, 9)]
            for a, b in zip(pts, pts[1:]): g.line(a, b, c["gold"])
    elif kind == "cajun":                                    # gold-trimmed front with ten pink key levers
        for a, b in (((u0, .03*H), (u1, .03*H)), ((u1, .03*H), (u1, .97*H)), ((u1, .97*H), (u0, .97*H)), ((u0, .97*H), (u0, .03*H))):
            g.line(fr0.p(a[0], a[1], zf + .002), fr0.p(b[0], b[1], zf + .002), c["gold"])
        on = c["_on"]
        for j in range(10):
            s_ = lerp(.88*H, .12*H, j/9)
            tbox(g, fr0, u0 + .02, u1 - .01, s_ - .013, s_ + .013, zf, zf + .012, HOT if j in on else c["pink"], emit=j in on, spec=.3)

def bass_side(g, c, fr1, Wb, H, D, T):
    """bass box: buttons on the outer end face (back part) with the strap across the front part; a decal on its front face"""
    zf = D/2; rows, cols = c["bass"]
    tbox(g, fr1, Wb, Wb + .012, .12*H, .88*H, zf - .13, zf - .07, c["strap"])
    lit = bass_live(T); pos = ((lit % 12)*7) % 12 if lit is not None else -1
    wa, wb = -zf + .03, zf - .15
    for i in range(rows):
        for j in range(cols):
            w = lerp(wa, wb, i/(max(1, rows - 1))) if rows > 1 else (wa + wb)/2
            sv = lerp(.88*H, .12*H, (j + .5*(i % 2))/max(1, cols)) if cols > 2 else lerp(.86*H, .74*H, j)
            on = i == min(1, rows - 1) and (j % 12) == pos
            disc(g, fr1.p(Wb + .004, sv, w), fr1.u, .011 if rows > 2 else .018, HOT if on else c["bbtn"], emit=on)
    if c.get("band"): quad_w(g, fr1, zf + .002, .004, Wb - .004, .02*H, .18*H, c["band"])
    if c["grille"] == "morino":
        for k in range(4): quad_w(g, fr1, zf + .002, .03 + k*.035, .045 + k*.035, .2*H, .8*H, (40, 40, 46))
    if c["grille"] == "corona":
        pts = [fr1.p(lerp(.01, Wb - .01, q), .5*H + .05*math.sin(q*8), zf + .002) for q in np.linspace(0, 1, 7)]
        for a, b in zip(pts, pts[1:]): g.line(a, b, c["gold"])
    if c.get("brass"):
        for sv in (.02*H, .98*H):
            for uu in (.012, Wb - .012): quad_w(g, fr1, zf + .002, uu - .012, uu + .012, sv - .012, sv + .012, c["brass"], spec=.7)

DIA = {0: 0, 1: 0, 2: 1, 3: 1, 4: 2, 5: 3, 6: 3, 7: 4, 8: 4, 9: 5, 10: 5, 11: 6}
def accordion(g, t, ev, fx, s, c):
    H, D, Wt, Wb, N = c["H"], c["D"], c["Wt"], c["Wb"], c["N"]
    T = s["t0"] + t; y0 = 0.
    f = breath(t)
    g0 = lerp(c["g"][0], c["g"][1], f); th = math.radians(lerp(c["th"][0], c["th"][1], f))
    gm, thm = (c["g"][0] + c["g"][1])/2, math.radians((c["th"][0] + c["th"][1])/2)
    xa = -(gm + .5*H*math.sin(thm) + Wb - Wt)/2
    frames = [fan(xa, y0, g0, th, k/(2*N)) for k in range(2*N + 1)]
    fr0, fr1 = frames[0], frames[-1]
    act = live_set(ev)
    tbox(g, fr0, -Wt, 0, 0, H, -D/2, D/2, c["body"], spec=c["spec"])
    e = .014
    bellows(g, frames, [(e, -D/2 + e), (H - e, -D/2 + e), (H - e, D/2 - e), (e, D/2 - e)], .03, c["bel"], c["bel2"], c.get("tape"), c.get("corner"))
    tbox(g, fr1, 0, Wb, 0, H, -D/2, D/2, c["body"], spec=c["spec"])
    xf = xa - Wt; zf = D/2; hot_pts = []
    if c["kind"] == "piano":
        kd = c.get("kd", .03); zb, zk = -D/2 + .02, D/2 - .055; m0, m1 = .06*H, .95*H
        if c.get("kframe"): g.box(xf - kd*.6, xf, y0 + m0 - .02, y0 + m1 + .02, zb - .012, D/2 - .01, c["kframe"])
        hi = c["low"] + {15: 24, 24: 40}[c["nwhite"]]
        a2 = {fold(m, c["low"], hi) for m in act}
        km, kw, ytop = piano_keys(g, xf, y0, m0, m1, zb, zk, c["low"], c["nwhite"], a2, c["nat"], c["sharp"], kd)
        g.box(xf - .012, xf, y0 + m0, y0 + m1, zk, D/2 - .004, c["cheek"])
        for m in a2: hot_pts.append((xf - kd - .02, key_y(km, kw, ytop, m), (zb + zk)/2))
        nreg = c.get("regs", 0)
        for k in range(nreg):
            y = y0 + lerp(m1 - .03, m0 + .03, k/max(1, nreg - 1))
            g.box(xf - .026, xf - .012, y - .013, y + .013, zk + .008, D/2 - .012, (18, 18, 20), spec=.4)
            g.face([(xf - .0265, y - .004, D/2 - .03), (xf - .0265, y + .004, D/2 - .03), (xf - .0265, y + .004, D/2 - .02), (xf - .0265, y - .004, D/2 - .02)],
                   (236, 236, 236), ref=(xf, y, D/2 - .025))
    elif c["kind"] in ("buttons", "dia3"):
        g.box(xf - .02, xf, y0 + .05*H, y0 + .95*H, -D/2 + .02, D/2 - .02, c["board"], spec=.3)
        if c["kind"] == "buttons":
            rows, per = c["rows"], c["per"]; counts = [per]*rows
            onb = set()
            for m in act:
                k = fold(m, c["low"], c["low"] + 3*per - 1) - c["low"]; rr, j = k % 3, k//3
                onb |= {(rr, j)} | ({(rr + 3, j)} if rr + 3 < rows else set())
        else:
            counts = list(c["per"]); rows = 3; onb = set()
            for m in act:
                k = (m*7) % 31; rr = 0 if k < 10 else (1 if k < 21 else 2); onb.add((rr, k - (0, 10, 21)[rr]))
        for rr in range(rows):
            n = counts[rr]; w = lerp(-D/2 + .045, D/2 - .045, rr/(rows - 1))
            for j in range(n):
                sv = lerp(.9*H, .1*H, (j + (.5 if n < max(counts) or (rows > 3 and rr % 2) else 0))/max(counts))
                on = (rr, j) in onb
                p = np.array([xf - .026, y0 + sv, w])
                disc(g, p, (-1, 0, 0), .013 if rows == 3 else .0115, HOT if on else c["btn"], emit=on)
                if on: hot_pts.append(tuple(p))
    elif c["kind"] == "single":                              # Cajun: one row of ten buttons on the keyboard edge
        on = set()
        for m in act: on.add((DIA[m % 12] + 7*(m//12) - (DIA[64 % 12] + 7*5)) % 10)
        c["_on"] = on
        for j in range(10):
            y = y0 + lerp(.88*H, .12*H, j/9); z = D/2 - .045
            g.box(xf - .024 + (.006 if j in on else 0), xf, y - .014, y + .014, z - .014, z + .014, HOT if j in on else c["btn"], emit=j in on, spec=.4)
            if j in on: hot_pts.append((xf - .03, y, z))
        for k in range(4):                                    # four turned stop knobs on top
            z = lerp(-D/2 + .06, D/2 - .06, k/3); x = xa - Wt/2
            TR.tube(g, (x, y0 + H, z), (x, y0 + H + .045, z), .008, col=(30, 26, 26), n=6, spec=.5)
            TR.tube(g, (x, y0 + H + .045, z), (x, y0 + H + .08, z), .019, .014, col=(20, 18, 18), n=8, spec=.7)
    grille(g, c, fr0, Wt, H, D, T, act)
    bass_side(g, c, fr1, Wb, H, D, T)
    top = tuple(fr1.p(Wb + .16, .74*H, 0.)); front = fr1.p(.5*Wb, .6*H, D/2)
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", top, age))
        if tp and age < .6: fx.append(("wave", tuple(front + np.array([.08, 0, 0])), age, 0))
        if tp and age < .06:
            for p in hot_pts[:2]: fx.append(("spark", p, age))

def sheng(g, t, ev, fx, s):
    """Zeng tomb style: a lacquered gourd wind chest, bamboo pipes standing in a ring, a long curved neck as the mouthpiece"""
    lac, red, bam = (52, 28, 24), (186, 52, 32), (178, 96, 52)
    prof = [(.0, .0), (.02, .07), (.06, .13), (.075, .145), (.19, .178), (.25, .162), (.265, .152), (.33, .108)]
    n = 14
    rings = [[(r_*math.cos(2*math.pi*k/n), y, r_*math.sin(2*math.pi*k/n)) for k in range(n)] for y, r_ in prof]
    for i in range(len(prof) - 1):
        A, B = rings[i], rings[i + 1]; ref = (0, (prof[i][0] + prof[i + 1][0])/2, 0)
        for k in range(n):
            col = red if i in (2, 5) else lac                       # black lacquer with painted red bands
            g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col, ref=ref, spec=.35)
    g.face(rings[-1], (66, 36, 28), ref=(0, 0, 0), spec=.3)
    notes = sorted({e[1] for e in s["ev"]}); act = live_set(ev)
    NP, RR = 14, .1
    hts = []
    for k in range(NP):                                         # two 'wings': tallest pipes at the sides, shortest at the front
        a = 2*math.pi*k/NP + math.pi/NP
        hts.append(.3 + .32*abs(math.cos(a)) + .1*max(0., -math.sin(a)))
    order = sorted(range(NP), key=lambda k: -hts[k])
    pipe_of = {m: order[min(NP - 1, 2*i)] for i, m in enumerate(notes)}
    lit = {pipe_of[m] for m in act if m in pipe_of}
    ytop = prof[-1][0]
    for k in range(NP):
        a = 2*math.pi*k/NP + math.pi/NP; x, z = RR*math.cos(a), RR*math.sin(a); h = hts[k]
        on = k in lit
        base = bam if k % 2 else dim(bam, .86)
        TR.tube(g, (x, ytop - .01, z), (x, ytop + h, z), .0135, col=mix(base, (255, 230, 170), .35) if on else base, n=6, spec=.25)
        for q in (.33, .66):
            TR.tube(g, (x, ytop + h*q - .006, z), (x, ytop + h*q + .006, z), .0152, col=dim(bam, .7), n=6, spec=.2)
        g.face(S.ring((x, ytop + h + .001, z), (0, 1, 0), .009, 6), HOT if on else (30, 18, 12), ref=(x, ytop, z), emit=on)
        hp = np.array([x, ytop + .06, z]) + unit((x, 0, z))*.0128
        disc(g, hp, (x, 0, z), .005 if not on else .008, (230, 186, 150) if on else (24, 14, 10))
        if on:
            for age, m, v, dur, tp, cs in ev:
                if m in pipe_of and pipe_of[m] == k and age < .9 and age < dur + .1:
                    fx.append(("note", (.26, .5, 0.), age))
                    if age < .06: fx.append(("spark", (x, ytop + h + .02, z), age))
    d = unit((1, 0, .6)); p0 = d*.15 + np.array([0, .17, 0]); p1 = p0 + d*.08
    c0 = p1 + np.array([0, .2, 0])
    arc = [c0 + d*.2*math.sin(a) - np.array([0, .2, 0])*math.cos(a) for a in np.linspace(0, math.pi/2, 7)]
    neck = [p0, p1] + arc[1:] + [arc[-1] + np.array([.012, .12, 0]), arc[-1] + np.array([.02, .2, 0])]
    path(g, neck, .036, lambda i: red if i in (1, 8) else lac, r1=.024, n=8, spec=.35)
    tip = neck[-1]
    g.face(S.ring(tip + np.array([0, .001, 0]), (0, 1, 0), .016, 8), (20, 12, 10), ref=tip - np.array([0, .05, 0]))
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .5: fx.append(("wave", tuple(tip + np.array([.06, .02, 0])), age, 0))

def demian(g, t, ev, fx, s):
    """1829 Vienna: a small mahogany box, five chord keys on the top board, a short paper bellows, a plain bottom board"""
    wood, wood2, ivory, paper = (128, 60, 36), (96, 42, 26), (238, 228, 202), (52, 82, 60)
    L, W = .3, .12
    g.box(-L, L, 0, .045, -W, W, wood, spec=.2)
    f = breath(t); gap = lerp(.06, .17, f); N = 6
    frames = [Fr((0, .045 + gap*k/(2*N), 0), (0, 1, 0), (1, 0, 0)) for k in range(2*N + 1)]
    bellows(g, frames, [(-L + .02, -W + .015), (L - .02, -W + .015), (L - .02, W - .015), (-L + .02, W - .015)], .02, paper, dim(paper, .75),
            tape=(214, 180, 96))
    yb = .045 + gap
    g.box(-L, L, yb, yb + .1, -W, W, wood, spec=.2)
    for y in (yb + .004, yb + .096):
        for a, b in (((-L, -W), (L, -W)), ((L, -W), (L, W)), ((L, W), (-L, W)), ((-L, W), (-L, -W))):
            g.line((a[0]*1.002, y, a[1]*1.002), (b[0]*1.002, y, b[1]*1.002), BRASSC)
    g.box(-L + .03, L - .03, yb + .1, yb + .17, -W, -W + .06, wood2, spec=.2)
    g.face([(-.06, yb + .03, W + .002), (.06, yb + .03, W + .002), (.06, yb + .07, W + .002), (-.06, yb + .07, W + .002)], BRASSC,
           ref=(0, yb + .05, 0), spec=.8)
    chords = []
    for e in s["ev"]:
        key = tuple(sorted(x[1] for x in s["ev"] if abs(x[0] - e[0]) < 1e-6))
        if key not in chords: chords.append(key)
    notes = live_set(ev); on = set()                     # the key whose chord is sounding
    for i, ck in enumerate(chords):
        if notes and set(ck) <= notes: on.add(i % 5)
    for i, x in enumerate(np.linspace(-.2, .2, 5)):
        down = i in on; y1 = yb + .125 - (.01 if down else 0)
        g.box(x - .032, x + .032, yb + .1, y1, -W + .06, W - .015, HOT if down else ivory, emit=down, spec=.3)
        if down:
            for age, m, v, dur, tp, cs in ev:
                if tp and age < .06: fx.append(("spark", (x, y1 + .02, 0), age))
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", (.12, yb + .3, 0), age))
        if tp and age < .6: fx.append(("wave", (L + .05, .045 + gap/2, 0), age, 0))

def oprism(g, fr, u0, u1, poly, col, spec=0.):
    ss = [p[0] for p in poly]; ww = [p[1] for p in poly]; sc, wc = (min(ss) + max(ss))/2, (min(ww) + max(ww))/2
    A = [fr.p(u0, s_, w_) for s_, w_ in poly]; B = [fr.p(u1, s_, w_) for s_, w_ in poly]; cen = fr.p((u0 + u1)/2, sc, wc)
    g.face(A, col, ref=cen, spec=spec); g.face(B, col, ref=cen, spec=spec)
    for i in range(len(poly)):
        j = (i + 1) % len(poly); g.face([A[i], A[j], B[j], B[i]], col, ref=cen, spec=spec)
    return A, B

def bandoneon(g, t, ev, fx, s):
    """square ends with chamfered corners, buttons on BOTH end faces, a strap across each end, carved fretwork on the front"""
    wood, pearl, cream = (84, 34, 24), (226, 222, 210), (240, 228, 196)
    Sz, ch, Wt, Wb, N = .44, .075, .13, .13, 14
    poly = [(ch, -Sz/2), (Sz - ch, -Sz/2), (Sz, -Sz/2 + ch), (Sz, Sz/2 - ch), (Sz - ch, Sz/2), (ch, Sz/2), (0, Sz/2 - ch), (0, -Sz/2 + ch)]
    f = breath(t); g0 = lerp(.14, .36, f); th = math.radians(lerp(0, 6, f))
    xa = -(.25 + Wb - Wt)/2 - .02
    frames = [fan(xa, 0., g0, th, k/(2*N)) for k in range(2*N + 1)]
    fr0, fr1 = frames[0], frames[-1]
    A0, B0 = oprism(g, fr0, -Wt, 0, poly, wood, .25)
    A1, B1 = oprism(g, fr1, 0, Wb, poly, wood, .25)
    e = .012; pin = [(sv*(1 - 2*e/Sz) + e, w*(1 - 2*e/Sz)) for sv, w in poly]
    bellows(g, frames, pin, .026, (24, 24, 26), (14, 14, 16), tape=(228, 228, 232), corner=CHROME)
    for R in (A0, B0, A1, B1):
        for i in range(len(R)): g.line(R[i], R[(i + 1) % len(R)], BRASSC)
    act = live_set(ev); hot = []
    for side, fr, u, nrm in ((0, fr0, -Wt - .005, -fr0.u), (1, fr1, Wb + .005, fr1.u)):
        rows = (7, 8, 8, 8, 7) if side == 0 else (6, 7, 7, 7, 6)
        btns = []
        for rr, nn in enumerate(rows):
            for j in range(nn):
                q = (j - (nn - 1)/2)/4.2; btns.append((Sz*.86 - rr*.052 - .03*q*q, q*Sz*.36 + (rr % 2)*.012))
        on = {(m*5) % len(btns) for m in act if (m >= 60) == (side == 0)}
        for k, (sv, w) in enumerate(btns):
            p = fr.p(u, sv, w); disc(g, p, nrm, .0115, HOT if k in on else cream, emit=k in on)
            if k in on: hot.append(tuple(p))
        uo = -Wt if side == 0 else Wb; sg = -1 if side == 0 else 1
        tbox(g, fr, uo + sg*.0, uo + sg*.016, .08*Sz, .36*Sz, -Sz*.36, Sz*.36, (112, 62, 32))
        quad_u(g, fr, uo + sg*.0165, .18*Sz, .26*Sz, -.02, .02, BRASSC, out=sg, spec=.8)
    for fr, ua, ub in ((fr0, -Wt + .015, -.015), (fr1, .015, Wb - .015)):     # fretwork + inlay on the front faces
        cu, cv = (ua + ub)/2, Sz*.48                           # carved rosette: a ring of slots round a centre
        for k in range(8):
            a = k*math.pi/4; u = cu + .034*math.cos(a); sv = cv + .07*math.sin(a)
            quad_w(g, fr, Sz/2 + .002, u - .005, u + .005, sv - .008, sv + .008, (20, 12, 10))
        quad_w(g, fr, Sz/2 + .002, cu - .006, cu + .006, cv - .006, cv + .006, (20, 12, 10))
        quad_w(g, fr, Sz/2 + .0025, (ua + ub)/2 - .02, (ua + ub)/2 + .02, Sz/2 - .012, Sz/2 + .012, pearl, spec=.5)
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", (xa - Wt/2 + .1, Sz + .14, 0), age))
        if tp and age < .6: fx.append(("wave", tuple(fr1.p(Wb + .06, Sz/2, 0)), age, 0))
        if tp and age < .06:
            for p in hot[:2]: fx.append(("spark", p, age))

def build(g, t, ev, fx, s):
    k = s["key"]
    if k == "sheng": sheng(g, t, ev, fx, s)
    elif k == "demian": demian(g, t, ev, fx, s)
    elif k == "bandoneon": bandoneon(g, t, ev, fx, s)
    else: accordion(g, t, ev, fx, s, ACC[k])

# ---------------------------------------------------------------- sound: free reeds
_rng = np.random.default_rng(29)
def _t(n): return np.arange(n)/SR
_WT = {}
def reed_wt(K, tilt, nas):
    key = (K, tilt, nas)
    if key not in _WT:
        x = np.arange(2048)/2048; k = np.arange(1, K + 1)
        amp = k**(-tilt)*(1 + nas*np.exp(-((k - 4)/2.2)**2))
        tb = (amp[:, None]*np.sin(2*np.pi*k[:, None]*x[None, :])).sum(0); _WT[key] = tb/np.abs(tb).max()
    return _WT[key]
def reed(f, dur, v, voices=((0, 1.),), tilt=1.1, nas=.6, attack=.012, rel=.05, breath_=0., lpf=None, stacc=1., vib=0., seed=0, clean=False, lock=False):
    d = max(.04, dur*stacc); n = int((d + rel*6 + .02)*SR); t = _t(n); rg = np.random.default_rng(seed)
    x = np.zeros(n)
    for cents, a in voices:
        fr = f*2**(cents/1200); K = int(max(1, min(40, 10000/fr))); tb = reed_wt(K, tilt, nas)
        ph = np.cumsum(fr*(1 + vib*np.sin(2*np.pi*4.8*t)))/SR if vib else fr*t
        ph = (ph + (0. if (clean or lock) else rg.random())) % 1.
        x += a*tb[(ph*2048).astype(int) & 2047]
    env = np.minimum(1, t/attack)*np.where(t < d, 1., np.exp(-(t - d)/rel))
    if not clean: env = env*(1 + .05*np.sin(2*np.pi*1.1*t + seed))
    if breath_: x = x + breath_*D.bp(D.nz(n), 1200, 6000)*np.minimum(1, t/.08)
    if lpf: x = D.lp(x, lpf)
    return x*env*v*.3
TONES = {
 "morino":    dict(voices=((0, 1.), (2, .3)), tilt=1.0, nas=.5, attack=.01, rel=.05),
 "sheng":     dict(voices=((0, 1.), (3, .55)), tilt=1.55, nas=.15, attack=.06, rel=.12, breath_=.22, vib=.0015),
 "demian":    dict(voices=((0, 1.),), tilt=1.3, nas=.7, attack=.02, rel=.07),
 "bandoneon": dict(voices=((0, 1.), (1200, .4)), tilt=1.45, nas=.3, attack=.007, rel=.03, lpf=2400, stacc=.65),
 "piano1910": dict(voices=((-6, 1.), (6, 1.)), tilt=1.1, nas=.6, attack=.012, rel=.05),
 "cajun":     dict(voices=((-11, 1.), (11, 1.), (1200, .5)), tilt=1.0, nas=.6, attack=.01, rel=.04),
 "musette":   dict(voices=((-18, 1.), (0, 1.), (18, 1.)), tilt=1.05, nas=.55, attack=.015, rel=.07, lock=True),   # wet musette: 3 reeds, outer two +/-18 cents
 "golden":    dict(voices=((-9, 1.), (0, 1.), (9, 1.), (-1200, .4)), tilt=1.05, nas=.55, attack=.012, rel=.05),
 "vallenato": dict(voices=((-6, 1.), (0, 1.), (6, 1.)), tilt=.95, nas=.7, attack=.008, rel=.04),
 "roland":    dict(voices=((0, 1.),), tilt=1.25, nas=.4, attack=.003, rel=.025, clean=True),
}
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    x = reed(midi_hz(m), dur, v*(1 if tp else .8), seed=idx*7 + m, **TONES[key])
    if key == "morino":                                    # dotted-eighth echo for the digital cumbia lead
        dl = int(.75*BEAT_S*SR); y = np.zeros(len(x) + 2*dl); y[:len(x)] += x; y[dl:dl + len(x)] += .28*x; y[2*dl:2*dl + len(x)] += .1*x; x = y
    return x

# ---------------------------------------------------------------- backing: digital cumbia (guacharaca 8ths, caja on 2 and 4, congas, kick on 1 and 3)
def guach(long):
    def fn(v):
        dur = .13 if long else .05; n = int(dur*SR); t = _t(n)
        teeth = .35 + .65*(np.sin(2*np.pi*(62 if long else 95)*t) > .2)
        return D.bp(D.nz(n), 2600, 9000)*teeth*np.minimum(1, t/.004)*np.exp(-t*(16 if long else 45))*v*.9
    return fn
def conga(f0, slap=False):
    def fn(v):
        n = int(.3*SR); t = _t(n); fr = f0*(1 + .1*np.exp(-t*30))
        tone = np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*(22 if slap else 13))
        return (tone + D.bp(D.nz(n), 900, 4000)*np.exp(-t*(30 if slap else 70))*(.7 if slap else .3))*v
    return fn
def caja(v):
    a = D.snare_s(320, 26, 36, 1800, 8000, .6, .25); b = C.rim(1.)*.35; n = max(len(a), len(b))
    return (np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))))*v
KIT = {"kick": lambda v: D.kick_s(50, 120, 7, .12, .45, 1.6, .3)*v, "caja": caja, "gL": guach(True), "gS": guach(False),
       "conga_o": conga(300), "conga_l": conga(205), "conga_s": conga(330, True), "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("caja",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "caja", .9)]; continue
        ev += [(T0 + k*.5*Q, "gL" if k % 2 == 0 else "gS", .5 if k % 2 == 0 else .38) for k in range(8)]
        ev += [(T0 + (k + .75)*Q, "gS", .22) for k in (1, 3)]
        if b == 0: ev += [(T0, "kick", .75), (T0 + 2*Q, "kick", .7)]; continue
        ev += [(T0, "kick", 1.), (T0 + 2*Q, "kick", .92), (T0 + Q, "caja", .9), (T0 + 3*Q, "caja", .9), (T0 + 1.75*Q, "caja", .28),
               (T0 + 1.5*Q, "conga_o", .7), (T0 + 3.5*Q, "conga_o", .7), (T0 + .5*Q, "conga_s", .4), (T0 + 2.5*Q, "conga_s", .4),
               (T0 + 2.75*Q, "conga_l", .55)]
        if b % 2: ev.append((T0 + 3.5*Q, "kick", .55))
        if b in (4, 22): ev.append((T0, "crash", .65))
    return ev
def _chord(b, beat):
    ch = BARS[b]; return ch[beat] if isinstance(ch, tuple) else ch
def _root(name):
    rt = CH[name][0]
    while rt > 45: rt -= 12
    while rt < 33: rt += 12
    return rt
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord(b, 0)), 2.4, 1.)); continue
        out += [(T0, _root(_chord(b, 0)), .85*c.BEAT, .95), (T0 + 1.5*c.BEAT, _root(_chord(b, 1)), .35*c.BEAT, .55),
                (T0 + 2*c.BEAT, _root(_chord(b, 2)) + 7, .85*c.BEAT, .85), (T0 + 3.5*c.BEAT, _root(_chord(b, 3)) + 12, .35*c.BEAT, .55)]
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph) + .35*np.sin(2*ph + .3) + .18*np.sin(3*ph)
    env = np.minimum(1, t/.004)*np.where(t < dur, np.exp(-t*2.2), math.exp(-dur*2.2)*np.exp(-(t - dur)/.04))
    return np.tanh(1.4*x*env)*v
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); t = _t(n); out = np.zeros(n)
    for m in notes:
        f = midi_hz(m)
        for det in (-.004, .004):
            ph = 2*np.pi*f*(1 + det)*t + _rng.random()*6.28
            out += sum(np.sin(k*ph)/k for k in range(1, 7))
    att = .02 if dur < 1 else .08
    env = np.minimum(1, t/att)*np.where(t < dur, 1., np.exp(-(t - dur)/.3))
    return D.lp(out*env, 1800)*v*.12
MIX = dict(drums=-21, bass=-23, pad=-31, verb=-23, verb_len=1.4)

# ---------------------------------------------------------------- background: a river cruise at night (paddle-steamer deck, string lights, the bank scrolling past)
THEMES = {   # sky top, sky bottom, string lights, skyline, windows, water, deck, rail, paddle
 "intro":     ((6, 22, 30), (22, 74, 86), (255, 236, 196), (10, 30, 38), (255, 222, 156), (8, 40, 52), (112, 78, 52), (196, 176, 146), (200, 64, 54)),
 "sheng":     ((4, 4, 8), (36, 22, 26), (255, 76, 42), (16, 10, 12), (255, 130, 64), (14, 12, 16), (98, 54, 36), (176, 56, 38), (190, 44, 30)),
 "demian":    ((34, 12, 38), (98, 46, 92), (255, 202, 112), (30, 12, 34), (255, 212, 132), (40, 20, 46), (120, 80, 60), (222, 192, 142), (150, 60, 110)),
 "bandoneon": ((30, 6, 12), (102, 26, 40), (234, 190, 98), (28, 8, 14), (242, 198, 112), (36, 12, 20), (102, 64, 44), (204, 162, 92), (176, 34, 44)),
 "piano1910": ((36, 26, 16), (132, 98, 62), (255, 242, 192), (44, 30, 18), (255, 228, 162), (52, 38, 24), (120, 88, 56), (212, 188, 142), (170, 80, 50)),
 "cajun":     ((10, 16, 6), (62, 76, 36), (232, 255, 92), (16, 22, 10), (212, 240, 100), (20, 30, 14), (98, 72, 44), (172, 162, 112), (150, 50, 36)),
 "musette":   ((14, 22, 42), (72, 92, 126), (232, 42, 54), (18, 24, 40), (255, 212, 152), (20, 30, 50), (102, 78, 62), (202, 62, 66), (210, 40, 52)),
 "golden":    ((56, 18, 48), (222, 122, 172), (232, 238, 252), (60, 24, 54), (255, 240, 202), (70, 30, 66), (132, 98, 82), (226, 230, 240), (230, 70, 120)),
 "vallenato": ((4, 28, 18), (32, 112, 72), (255, 170, 42), (8, 34, 22), (255, 202, 92), (8, 40, 34), (112, 82, 50), (242, 172, 62), (230, 120, 30)),
 "roland":    ((38, 42, 54), (152, 162, 182), (64, 164, 255), (34, 38, 50), (122, 202, 255), (30, 36, 56), (92, 92, 102), (172, 202, 242), (60, 130, 230)),
}
KIND = {"intro": "modern", "sheng": "pagoda", "demian": "vienna", "bandoneon": "docks", "piano1910": "theatre", "cajun": "bayou",
        "musette": "paris", "golden": "boardwalk", "vallenato": "magdalena", "roland": "glass"}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[6], T_[1], .3))

HZ, RAIL_T, DECK_Y = 214, 270, 292
SW, SH, PADX = 540, 90, 120
def _strip(kind):
    im = Image.new("L", (SW + PADX, SH), 0); d = ImageDraw.Draw(im); rg = np.random.default_rng(sum(map(ord, kind)))
    B = SH - 1
    def rect(x0, h, x1, val=1): d.rectangle([x0, B - h, x1, B], fill=val)
    def wins(x0, x1, h, p=.45, val=2, sx=4, sy=5, top=3):
        for y in range(B - h + top, B - 3, sy):
            for x in range(int(x0) + 2, int(x1) - 1, sx):
                if rg.random() < p: d.rectangle([x, y, x, y + 1], fill=val)
    def hills(base, amp, f1, f2, val=4):
        for x in range(SW + PADX):
            h = base + amp*math.sin(x*f1) + amp*.6*math.sin(x*f2 + 1.3); d.line([(x, B), (x, B - h)], fill=val)
    if kind == "modern":
        x = 0
        while x < SW:
            w = int(rg.integers(14, 30)); h = int(rg.integers(14, 34)); rect(x, h, x + w); wins(x, x + w, h, .35); x += w + int(rg.integers(0, 4))
        for _ in range(9):
            x = int(rg.integers(0, SW)); w = int(rg.integers(12, 22)); h = int(rg.integers(40, 76)); rect(x, h, x + w); wins(x, x + w, h, .5)
            d.point((x + w//2, B - h - 1), fill=3)
        px = 330
        d.line([(px - 14, B), (px, B - 82)], fill=1, width=2); d.line([(px + 14, B), (px, B - 82)], fill=1, width=2)
        d.rectangle([px - 90, B - 17, px + 90, B - 14], fill=1)
        for k in range(9):
            d.line([(px, B - 80 + k*5), (px - 88 + k*9, B - 17)], fill=1); d.line([(px, B - 80 + k*5), (px + 88 - k*9, B - 17)], fill=1)
        for x in range(px - 88, px + 90, 8): d.point((x, B - 18), fill=3)
    elif kind == "pagoda":
        hills(34, 12, .021, .057)
        x = 0
        while x < SW:
            w = int(rg.integers(14, 24)); h = int(rg.integers(7, 12)); rect(x, h, x + w)
            d.polygon([(x - 3, B - h), (x + w//2, B - h - 6), (x + w + 3, B - h)], fill=1); x += w + int(rg.integers(4, 12))
        for px in (70, 250, 430):
            for k in range(5):
                w = 30 - k*5; yb = B - 6 - k*13
                d.rectangle([px - w//2, yb - 9, px + w//2, yb], fill=1)
                d.polygon([(px - w//2 - 9, yb - 9), (px - w//2 - 12, yb - 14), (px - w//2 - 3, yb - 11), (px + w//2 + 3, yb - 11),
                           (px + w//2 + 12, yb - 14), (px + w//2 + 9, yb - 9)], fill=1)
                d.rectangle([px - 1, yb - 6, px + 1, yb - 3], fill=2)
                for sg in (-1, 1): d.rectangle([px + sg*(w//2 + 12), yb - 11, px + sg*(w//2 + 12), yb - 9], fill=3)
            d.line([(px, B - 6 - 5*13 - 9), (px, B - 6 - 5*13 - 24)], fill=1, width=2)
    elif kind == "vienna":
        x = 0
        while x < SW:
            w = int(rg.integers(16, 30)); h = int(rg.integers(16, 26)); rect(x, h, x + w); wins(x, x + w, h, .55, sx=3, sy=4)
            d.polygon([(x, B - h), (x + 3, B - h - 4), (x + w - 3, B - h - 4), (x + w, B - h)], fill=1); x += w + 1
        cx = 120
        rect(cx - 30, 30, cx + 30); rect(cx - 14, 44, cx + 14)
        d.pieslice([cx - 18, B - 64, cx + 18, B - 26], 180, 360, fill=1); d.rectangle([cx - 3, B - 72, cx + 3, B - 62], fill=1)
        d.line([(cx, B - 72), (cx, B - 80)], fill=1); d.line([(cx - 3, B - 77), (cx + 3, B - 77)], fill=1)
        for px in (cx - 48, cx + 48):
            rect(px - 4, 58, px + 4); d.ellipse([px - 6, B - 64, px + 6, B - 56], fill=1)
        wins(cx - 28, cx + 28, 26, .5)
        gx = 340
        rect(gx - 50, 26, gx + 50); d.polygon([(gx - 50, B - 26), (gx, B - 44), (gx + 50, B - 26)], fill=1)
        rect(gx - 10, 52, gx + 8); d.polygon([(gx - 12, B - 52), (gx - 1, B - 88), (gx + 10, B - 52)], fill=1)
        wins(gx - 48, gx + 48, 24, .4)
        for x in range(6, SW, 24): d.rectangle([x, B - 7, x, B - 6], fill=3)
    elif kind == "docks":
        x = 0
        while x < 210:
            w = 34; h = int(rg.integers(14, 20)); rect(x, h, x + w)
            for k in range(4): d.polygon([(x + k*8, B - h), (x + k*8 + 8, B - h - 6), (x + k*8 + 8, B - h)], fill=1)
            wins(x, x + w, h - 2, .25); x += w + 4
        for cx in (240, 470):
            d.line([(cx, B), (cx + 4, B - 58)], fill=1); d.line([(cx + 12, B), (cx + 8, B - 58)], fill=1)
            for k in range(5): d.line([(cx + 1, B - k*11), (cx + 11, B - k*11 - 11)], fill=1)
            d.line([(cx + 6, B - 58), (cx - 44, B - 64)], fill=1, width=2); d.line([(cx + 6, B - 58), (cx + 24, B - 55)], fill=1, width=2)
            d.line([(cx - 44, B - 64), (cx - 44, B - 32)], fill=1); d.rectangle([cx + 2, B - 62, cx + 10, B - 56], fill=1)
            d.point((cx + 6, B - 66), fill=3)
        d.polygon([(286, B - 14), (392, B - 14), (384, B), (294, B)], fill=1); d.rectangle([310, B - 24, 360, B - 14], fill=1)
        d.rectangle([340, B - 36, 348, B - 24], fill=1); d.line([(300, B - 14), (300, B - 44)], fill=1); d.line([(376, B - 14), (376, B - 40)], fill=1)
        for x in range(298, 384, 7): d.point((x, B - 9), fill=2)
        x = 396
        while x < SW + 20:
            w = int(rg.integers(10, 16)); h = int(rg.integers(14, 24)); rect(x, h, x + w); wins(x, x + w, h, .6, sx=3, sy=4); x += w + 1
        for x in range(10, SW, 30): d.rectangle([x, B - 5, x, B - 4], fill=3)
    elif kind == "theatre":
        x = 0
        while x < SW:
            w = int(rg.integers(16, 28)); h = int(rg.integers(18, 40)); rect(x, h, x + w); d.rectangle([x - 1, B - h - 1, x + w + 1, B - h], fill=1)
            wins(x, x + w, h, .5, sx=3, sy=4)
            if rg.random() < .3:
                d.rectangle([x + 4, B - h - 9, x + 10, B - h - 4], fill=1); d.line([(x + 5, B - h - 4), (x + 5, B - h)], fill=1)
                d.polygon([(x + 3, B - h - 9), (x + 7, B - h - 12), (x + 11, B - h - 9)], fill=1)
            x += w + 1
        tx = 200
        rect(tx - 12, 62, tx + 12); d.polygon([(tx - 12, B - 62), (tx, B - 80), (tx + 12, B - 62)], fill=1); d.line([(tx, B - 80), (tx, B - 88)], fill=1)
        wins(tx - 12, tx + 12, 62, .55, sx=3, sy=4)
        mx = 340
        rect(mx - 36, 36, mx + 36); d.rectangle([mx - 40, B - 24, mx + 40, B - 15], fill=1); d.rectangle([mx - 7, B - 60, mx + 7, B - 36], fill=1)
        for x in range(mx - 40, mx + 41, 3): d.point((x, B - 24), fill=3); d.point((x, B - 15), fill=3)
        for y in range(B - 60, B - 35, 3): d.point((mx - 7, y), fill=3); d.point((mx + 7, y), fill=3)
    elif kind == "bayou":
        hills(22, 4, .09, .23)
        for tx in (40, 150, 232, 380, 470, 520):
            h = int(rg.integers(52, 72))
            d.polygon([(tx - 7, B), (tx - 2, B - 8), (tx - 2, B - h + 12), (tx + 2, B - h + 12), (tx + 2, B - 8), (tx + 7, B)], fill=1)
            for k in range(5):
                ex, ey = tx + int(rg.integers(-16, 16)), B - h + int(rg.integers(-4, 14))
                d.ellipse([ex - 12, ey - 5, ex + 12, ey + 5], fill=1)
            for k in range(9):
                mx = tx + int(rg.integers(-18, 18)); my = B - h + int(rg.integers(6, 16)); d.line([(mx, my), (mx, my + int(rg.integers(5, 15)))], fill=4)
        d.rectangle([292, B - 24, 330, B - 10], fill=1); d.polygon([(288, B - 24), (311, B - 34), (334, B - 24)], fill=1)
        for x in (295, 310, 326): d.line([(x, B - 10), (x, B)], fill=1)
        d.rectangle([300, B - 20, 303, B - 16], fill=2); d.rectangle([316, B - 20, 319, B - 16], fill=2)
        for _ in range(60):
            x = int(rg.integers(0, SW)); d.line([(x, B), (x + int(rg.integers(-2, 3)), B - int(rg.integers(3, 9)))], fill=1)
    elif kind == "paris":
        x = 0
        while x < SW:
            w = int(rg.integers(20, 34)); h = int(rg.integers(20, 28)); rect(x, h, x + w); wins(x, x + w, h, .6, sx=3, sy=5, top=4)
            d.polygon([(x, B - h), (x + 4, B - h - 7), (x + w - 4, B - h - 7), (x + w, B - h)], fill=1)
            for k in range(x + 6, x + w - 4, 6): d.point((k, B - h - 4), fill=2)
            x += w + 1
        ex = 420
        L = [(ex - 26, B), (ex - 15, B - 24), (ex - 8, B - 48), (ex - 3, B - 70), (ex - 1, B - 80)]
        d.polygon(L + [(ex, B - 89)] + [(2*ex - px, py) for px, py in L[::-1]], fill=1)
        d.ellipse([ex - 15, B - 14, ex + 15, B + 14], fill=0)
        d.rectangle([ex - 17, B - 26, ex + 17, B - 23], fill=1); d.rectangle([ex - 9, B - 50, ex + 9, B - 48], fill=1)
        d.point((ex, B - 90), fill=3)
        d.rectangle([0, B - 20, 260, B], fill=1)
        for px in range(0, 260, 52): d.ellipse([px + 10, B - 14, px + 50, B + 14], fill=0)
        for px in range(4, 260, 26): d.rectangle([px, B - 25, px, B - 21], fill=1); d.point((px, B - 26), fill=3)
    elif kind == "boardwalk":
        d.rectangle([0, B - 6, SW + PADX, B - 4], fill=1)
        for x in range(0, SW + PADX, 9): d.line([(x, B - 4), (x, B)], fill=1)
        cx, cy, rr = 130, B - 46, 36
        d.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], outline=1, width=2); d.ellipse([cx - 30, cy - 30, cx + 30, cy + 30], outline=1)
        for k in range(12):
            a = k*math.pi/6; d.line([(cx, cy), (cx + rr*math.cos(a), cy + rr*math.sin(a))], fill=1)
            gx, gy = cx + rr*math.cos(a), cy + rr*math.sin(a); d.rectangle([gx - 2, gy, gx + 2, gy + 4], fill=1)
        for k in range(24):
            a = k*math.pi/12; d.point((cx + (rr + 1)*math.cos(a), cy + (rr + 1)*math.sin(a)), fill=3)
        d.line([(cx, cy), (cx - 22, B - 6)], fill=1, width=2); d.line([(cx, cy), (cx + 22, B - 6)], fill=1, width=2)
        pts = [(x, B - 22 - 30*abs(math.sin((x - 250)*.022))) for x in range(250, SW + 40, 3)]
        d.line(pts, fill=1, width=2)
        for x, y in pts[::4]: d.line([(x, y), (x, B - 6)], fill=1)
        for x in (20, 200, 330):
            rect(x, 16, x + 30); d.rectangle([x + 4, B - 14, x + 26, B - 9], outline=3)
    elif kind == "magdalena":
        hills(30, 9, .015, .043)
        x = 0
        while x < SW:
            w = int(rg.integers(12, 22)); h = int(rg.integers(9, 15)); rect(x, h, x + w)
            d.polygon([(x - 2, B - h), (x + w//2, B - h - 5), (x + w + 2, B - h)], fill=1); wins(x, x + w, h, .35, sx=4, sy=4); x += w + int(rg.integers(2, 10))
        cx = 210
        rect(cx - 20, 32, cx + 20); d.polygon([(cx - 20, B - 32), (cx, B - 42), (cx + 20, B - 32)], fill=1)
        for px in (cx - 26, cx + 26):
            rect(px - 5, 50, px + 5); d.pieslice([px - 6, B - 58, px + 6, B - 44], 180, 360, fill=1); d.line([(px, B - 58), (px, B - 63)], fill=1)
            d.rectangle([px - 1, B - 44, px + 1, B - 40], fill=2)
        d.rectangle([cx - 3, B - 18, cx + 3, B - 8], fill=2)
        for px in (60, 330, 470, 560):
            top = (px + 10, B - 56); d.line([(px, B), (px + 4, B - 30), top], fill=1, width=2)
            for k in range(7):
                a = -math.pi + k*math.pi/6; d.line([top, (top[0] + 16*math.cos(a), top[1] + 8*math.sin(a) + 6)], fill=1, width=2)
        d.polygon([(100, B - 3), (140, B - 3), (136, B), (104, B)], fill=1)
    elif kind == "glass":
        for _ in range(16):
            x = int(rg.integers(0, SW)); w = int(rg.integers(14, 26)); h = int(rg.integers(36, 84))
            rect(x, h, x + w)
            if rg.random() < .4: d.polygon([(x, B - h), (x + w, B - h - 10), (x + w, B - h)], fill=1)
            for y in range(B - h + 3, B - 3, 3):
                for xx in range(x + 2, x + w - 1, 3):
                    if rg.random() < .55: d.point((xx, y), fill=2)
            d.point((x + w//2, B - h - 2), fill=3)
        tx = 440
        d.line([(tx, B), (tx, B - 88)], fill=1, width=2); d.ellipse([tx - 7, B - 62, tx + 7, B - 52], fill=1); d.point((tx, B - 89), fill=3)
    a = np.asarray(im).copy(); m = a[:, :SW].copy(); ext = a[:, SW:]
    m[:, :PADX] = np.where(ext > 0, ext, m[:, :PADX])
    return m
STRIPS = {k: _strip(k) for k in set(KIND.values())}
SKYG = {k: F.make_grad(v[0], v[1], y0=0, y1=HZ) for k, v in THEMES.items()}
_r = np.random.default_rng(5)
STARS = (YY < 160) & (_r.random((RH, RW)) < .005)
MOON = (np.hypot(XX - 36, YY - 124) < 9) & (np.hypot(XX - 41, YY - 121) >= 8)
XS = np.arange(RW)
WK = np.arange(HZ, DECK_Y) - HZ
WSRC = SH - 1 - (WK*1.5).astype(int)
_dy = np.maximum(YY - 150, 1)
_u = (XX - CX)/_dy*330/30.
PLANK = np.floor(_u + .5).astype(int)
_fu = np.abs(_u + .5 - np.floor(_u + .5) - .5)
_dep = 330/_dy
DECKM = YY >= DECK_Y
SEAM = DECKM & ((_fu < .045*(_dep*.5 + .5)) | (np.abs(((_dep*1.2 + (PLANK % 3)*.37) % 1) - .5) < .04))
DSH = np.clip(1.08 - (YY - DECK_Y)/420., .7, 1.1)*(1 + .06*((PLANK % 2) - .5))
WX, WY, WR = 254, 234, 46
def cat(xa_, ya_, xb_, yb_, sag):
    xs = np.arange(xa_, xb_ + 1); q = (xs - xa_)/(xb_ - xa_); return xs, (ya_ + (yb_ - ya_)*q + 4*sag*q*(1 - q)).astype(int)
STRA = cat(5, 100, 265, 104, 40); STRB = cat(5, 160, 265, 156, 16)
BULBS = [(int(x), int(y), i, 0) for i, (x, y) in enumerate(zip(STRA[0][6::15], STRA[1][6::15]))] + \
        [(int(x), int(y), i, 1) for i, (x, y) in enumerate(zip(STRB[0][9::18], STRB[1][9::18]))]
GLOW = [(dy, dx) for dy in range(-3, 4) for dx in range(-3, 4) if abs(dx) + abs(dy) <= 3 and (dx, dy) != (0, 0)]
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
_fr = np.random.default_rng(9)
FLIES = [(_fr.random()*RW, 150 + _fr.random()*110, _fr.random()*6.28) for _ in range(22)]
LANTS = [(_fr.random()*(RW + 20), k) for k in range(6)]

def strip_rgb(m, th, pulse, beat_i):
    T_ = THEMES[th]; out = np.zeros(m.shape + (3,))
    out[m == 1] = T_[3]; out[m == 4] = mix(T_[3], T_[1], .45); out[m == 2] = T_[4]
    out[m == 3] = np.clip(np.array(T_[2], float)*(.6 + .5*pulse), 0, 255)
    return out, m > 0

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return np.array(mix(A_[i], B_[i], kb), float)
    pulse, bi, bph = x["pulse"], x["beat_i"], x["beat_ph"]
    bg = SKYG[th0]*(1 - kb) + SKYG[th1]*kb
    bg[STARS & (np.sin(T*2.3 + XX*.9 + YY*.37) > -.2)] = (236, 236, 255)
    bg[MOON] = (244, 238, 214)
    ox = int(T*7) % SW; cols = (XS + ox) % SW
    m0 = STRIPS[KIND[th0]][:, cols]; rgb, val = strip_rgb(m0, th0, pulse, bi)
    if th1 != th0:
        m1 = STRIPS[KIND[th1]][:, cols]; rgb1, val1 = strip_rgb(m1, th1, pulse, bi)
        sky = bg[HZ - SH:HZ]
        a0 = np.where(val[..., None], rgb, sky); a1 = np.where(val1[..., None], rgb1, sky)
        bg[HZ - SH:HZ] = a0*(1 - kb) + a1*kb
    else:
        reg = bg[HZ - SH:HZ]; reg[val] = rgb[val]
    # water with the bank's lights reflected in broken ripples
    wat = col(5); bg[HZ:DECK_Y] = wat[None, None, :]*(1 - .25*(WK/(DECK_Y - HZ)))[:, None, None]
    msk = STRIPS[KIND[th1 if kb > .5 else th0]]
    wob = (2*np.sin(WK*.8 + T*2.6)).astype(int)
    ok = WSRC >= 0
    rows = np.where(ok)[0]
    src = msk[WSRC[rows][:, None], (XS[None, :] + ox + wob[rows][:, None]) % SW]
    lit = (src == 2) | (src == 3)
    brk = ((rows[:, None] + int(T*5)) % 3 != 2) & ((XS[None, :] + rows[:, None]*7 + int(T*9)) % 11 > 2)
    sub = bg[HZ:HZ + len(rows)]
    sub[(src == 1) | (src == 4)] *= .72
    sub[lit & brk] = sub[lit & brk]*.4 + np.array(mix(A_[4], B_[4], kb), float)*.6
    rip = ((XX*3 + (YY - HZ)*17 + int(T*24)) % 40 < 3) & ((YY - HZ) % 4 == 1) & (YY >= HZ) & (YY < DECK_Y)
    bg[rip] = bg[rip]*.6 + col(1)*.4
    lc = col(2)
    for bx, by, i, row in BULBS:
        if row == 0:
            for k in range(6, 40, 3):
                if 0 <= bx < RW: bg[HZ + k, bx] = bg[HZ + k, bx]*.6 + lc*.4*(1 - k/40)
    if KIND[th1] == "pagoda" or KIND[th0] == "pagoda":
        wgt = kb if KIND[th1] == "pagoda" else 1 - kb
        for x0, k in LANTS:
            lx = int((x0 - T*9) % (RW + 20)) - 10; ly = HZ + 10 + k*7
            if 1 <= lx < RW - 2:
                bg[ly - 1:ly + 2, lx - 1:lx + 2] = bg[ly - 1:ly + 2, lx - 1:lx + 2]*(1 - wgt) + np.array((255, 90, 50), float)*wgt
                for kk in range(3, 14, 2): bg[ly + kk, lx] = bg[ly + kk, lx]*(1 - .4*wgt) + np.array((255, 90, 50), float)*.4*wgt
    # paddle wheel (one paddle per beat) with splashes
    im = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8)); d = ImageDraw.Draw(im)
    ang = (bi + ease(clamp(bph*3)))*math.pi/4
    wc_, pc_ = tuple(int(v) for v in dim(col(7), .8)), tuple(int(v) for v in col(8))
    d.arc([WX - WR - 9, WY - WR - 9, WX + WR + 9, WY + WR + 9], 180, 360, fill=tuple(int(v) for v in col(6)), width=4)
    for k in range(8):
        a = ang + k*math.pi/4; ca, sa = math.cos(a), math.sin(a)
        d.line([(WX, WY), (WX + WR*ca, WY + WR*sa)], fill=wc_, width=1)
        d.line([(WX + WR*.6*ca, WY + WR*.6*sa), (WX + WR*1.08*ca, WY + WR*1.08*sa)], fill=pc_, width=5)
    d.ellipse([WX - WR, WY - WR, WX + WR, WY + WR], outline=wc_, width=2); d.ellipse([WX - WR*.6, WY - WR*.6, WX + WR*.6, WY + WR*.6], outline=wc_)
    d.ellipse([WX - 5, WY - 5, WX + 5, WY + 5], fill=wc_)
    rs = np.random.default_rng(bi)
    for k in range(10):
        sp = .5 + rs.random(); x0 = WX - 30 + rs.random()*30
        px_, py_ = x0 - 40*bph*sp, 262 - 34*bph*sp + 46*bph*bph
        if bph < .85: d.point((px_, py_), fill=(236, 244, 250))
    bg = np.asarray(im).astype(float)
    # railing
    rl = col(7)
    bg[RAIL_T:RAIL_T + 4] = rl*.85; bg[RAIL_T] = rl
    for xx in range(1, RW, 6): bg[RAIL_T + 4:RAIL_T + 18, xx] = rl*.62
    for xx in range(10, RW, 24): bg[RAIL_T + 4:DECK_Y, xx:xx + 2] = rl*.78
    bg[RAIL_T + 17:RAIL_T + 19] = rl*.7
    # deck planks
    dk = col(6)
    bg[DECKM] = (dk[None, :]*DSH[DECKM][:, None])
    bg[SEAM] = dk*.55
    # string lights + poles
    pole = col(7)*.45
    for px in (3, 265): bg[96:DECK_Y + 8, px:px + 2] = pole
    for xs_, ys_ in (STRA, STRB):
        okk = (xs_ >= 0) & (xs_ < RW) & (ys_ >= 0) & (ys_ < RH); bg[ys_[okk], xs_[okk]] = col(1)*.3
    for bx, by, i, row in BULBS:
        chase = 1. if (i + bi + row) % 3 == 0 else 0.
        b = .5 + .3*pulse + .25*chase
        hc = np.clip(mix(tuple(lc), (255, 255, 255), .3), 0, 255)
        for dy, dx in GLOW:
            yy, xx = by + 1 + dy, bx + dx
            if 0 <= yy < RH and 0 <= xx < RW and BAY[yy, xx] < .55 - .1*(abs(dx) + abs(dy)): bg[yy, xx] = bg[yy, xx]*.55 + lc*.45*b
        bg[by + 1:by + 3, bx:bx + 2] = np.clip(np.array(hc, float)*min(1.1, b + .2), 0, 255)
    if KIND[th1] == "bayou" or KIND[th0] == "bayou":
        wgt = kb if KIND[th1] == "bayou" else 1 - kb; fc = np.array((232, 255, 96), float)
        for fx_, fy_, p in FLIES:
            if math.sin(T*3 + p*5) > .1:
                xx = int(fx_ + 10*math.sin(T*.7 + p)) % RW; yy = int(fy_ + 6*math.sin(T*1.1 + p))
                bg[yy, xx] = bg[yy, xx]*(1 - wgt) + fc*wgt
                for dy, dx in ((0, 1), (1, 0), (0, -1), (-1, 0)):
                    if 0 <= xx + dx < RW: bg[yy + dy, xx + dx] = bg[yy + dy, xx + dx]*(1 - .45*wgt) + fc*.45*wgt
    bg[TEXTB] *= .5; bg[TEXTE] *= .72
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

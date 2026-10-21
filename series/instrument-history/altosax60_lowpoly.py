"""Pixel Pitch: history of the alto saxophone, 60-second cut.  python3 altosax60_lowpoly.py [out.mp4] | --still t ...

Reference photos checked before modelling (Claude in Chrome, 2026-10-21):
  YAS-62  Yamaha product photos: gold lacquer, engraved bell, high F# + front F, low B/Bb bell keys on the
          front face of the bell between bell and body (modern "right-side" bell keys).
  1846    The Met 2005.82.1 (Adolphe Sax alto, ca. 1855, 59.7 cm, 14 cm bell): plain brass, separate keys on
          posts, no pearls, long neck run, two octave keys, one bell key on the back of the bell.
  1888    NMM 13158 (Conn "Wonder" no. 16, 1892): gold plate, rose-gold engraved bell + bow, pearl touchpieces,
          single octave key, low keys on the LEFT side of the bell (back face, away from the pearls), 12.5 cm bell.
  1920s   Conn New Wonder: satin silver plate, gold-washed bell interior, left-side bell keys.
  1940s   Conn 6M: gold lacquer, engraved cartouche on the bell, micro-tuner ring on the neck, left-side bell keys.
  1953    Grafton: cream acrylic body, bow and bell, brass neck, keywork and bell rim, white touchpieces.
  1959    Selmer Super Balanced Action: darker lacquer, right-side bell keys.
  1967    Selmer Varitone: pickup in the neck, control box (blue-grey panel, 4 sliders, 3 knobs) on the lower bell.
  1987    Akai EWI1000 (SMEM collection): straight silver square tube, black mouthpiece + underside, touch plates
          that never move, roller octave switches, cable to the EWV2000 rack module (silver, blue knobs).
Local model frame: y up from the pedestal top, +x toward the bell, +z toward the camera (pearl side).
"""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import cello_lowpoly as C
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "altosax60_lowpoly.mp4", "as"
BPM = 96
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 2.15, 0, 11, .25
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE ALTO SAX", "180 YEARS", "TODAY", 1830
INTRO_LINES = (("ABOUT 600 PARTS,", "AND JUST ONE REED"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("180 YEARS FROM", "ADOLPHE SAX")
OUTRO_CREDIT = "BOOM-BAP JAZZ"
LIGHTING = True

# ---------------------------------------------------------------- music (boom bap, 96 BPM)
CH = {"Cm9": (36, (51, 58, 62, 67)), "Abmaj9": (44, (55, 60, 63, 70)), "Fm9": (41, (51, 56, 60, 67)), "G7b9": (43, (53, 59, 62, 68)),
      "Bb": (46, (53, 58, 62, 65)), "F7": (41, (51, 57, 60, 65)), "Eb": (39, (55, 58, 63, 67)), "Bb7": (46, (50, 56, 62, 65)),
      "F": (41, (53, 57, 60, 65)), "G7": (43, (53, 59, 62, 65)), "C7": (36, (52, 58, 60, 64)), "Dbmaj7": (37, (53, 56, 60, 65)),
      "Ebm7": (39, (54, 58, 61, 66)), "Ab7": (44, (54, 60, 63, 66)), "Fm7": (41, (51, 56, 60, 63)), "Ebmaj7": (39, (55, 58, 62, 67)),
      "Bbm7": (46, (53, 56, 61, 65)), "Em9": (40, (55, 59, 62, 66)), "Cmaj7": (36, (55, 59, 64, 67)), "D": (38, (54, 57, 62, 66))}
BARS = ["Cm9", "Abmaj9", "Fm9", "G7b9",                     # showcase
        "Bb", ("F7", "F7", "Bb", "Bb"),                     # 1846
        "Eb", ("Bb7", "Bb7", "Eb", "Eb"),                   # 1888
        "F", ("G7", "G7", "C7", "C7"),                      # 1920s
        "Dbmaj7", ("Ebm7", "Ebm7", "Ab7", "Ab7"),           # 1940s
        ("Fm7", "Fm7", "Bb7", "Bb7"), "Ebmaj7",             # 1953
        "Ebm7", "Bbm7",                                     # 1959
        "F7", "Bb7",                                        # 1967
        "Em9", ("Cmaj7", "Cmaj7", "D", "D"),                # 1987
        "Cm9", "Abmaj9",                                    # 2020s
        ("Fm9", "Fm9", "G7b9", "G7b9"),                     # outro
        "Cm9"]                                              # final chord
SWG = {0.: 0., .25: .3, .5: .6, .75: .8}                    # lazy boom-bap swing
def sw(b):
    i = math.floor(b + 1e-9); f = round(b - i, 4); return i + SWG.get(f, f)
def swing(items): return [(sw(b), m, sw(b + d) - sw(b)) for b, m, d in items]
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]

RIFF_A = [(.5, 67, .5), (1, 70, .5), (1.5, 72, 1.25), (2.75, 70, .25), (3, 67, .5), (3.5, 65, .5),
          (4, 63, 1.5), (5.5, 67, .5), (6, 72, .75), (6.75, 74, .25), (7, 75, .5), (7.5, 72, .5)]
RIFF_B = [(8, 68, .75), (8.75, 67, .25), (9, 65, .5), (9.5, 63, .5), (10, 60, 1.5), (11.5, 63, .5),
          (12, 62, .5), (12.5, 65, .5), (13, 68, .5), (13.5, 71, .5), (14, 74, .75), (14.75, 75, .25), (15, 74, .5), (15.5, 71, .5)]
RIFF_END = [(8, 68, .75), (8.75, 67, .25), (9, 65, .5), (9.5, 63, .5), (10, 62, .5), (10.5, 65, .5), (11, 71, .5), (11.5, 74, .5)]

SCENES = [
 dict(key="modern", theme="intro", name="YAMAHA YAS-62", credit="BOOM-BAP JAZZ", notes=tn(swing(RIFF_A + RIFF_B))),
 dict(key="sax1846", theme="sax1846", year=1846, ys="1846", name="ADOLPHE SAX ALTO", credit="FRENCH MILITARY BAND",
      fact=("BRASS POWER WITH", "WOODWIND SPEED"),
      notes=tn([(0, 65, .75), (.75, 65, .25), (1, 70, .75), (1.75, 74, .25), (2, 77, 1), (3, 74, .5), (3.5, 70, .5),
                (4, 72, .75), (4.75, 69, .25), (5, 65, .5), (5.5, 75, .5), (6, 74, 1), (7, 70, 1)], .88)),
 dict(key="conn", theme="conn", year=1888, ys="1888", name="CONN'S FIRST SAX", credit="GILDED AGE CONCERT BAND",
      fact=("MADE IN ELKHART,", "COPIED FROM A SAX"),
      notes=tn([(0, 70, .5), (.5, 67, .5), (1, 63, .5), (1.5, 67, .5), (2, 70, .75), (2.75, 72, .25), (3, 70, .5), (3.5, 67, .5),
                (4, 68, .5), (4.5, 65, .5), (5, 62, .5), (5.5, 65, .5), (6, 67, .5), (6.5, 70, .5), (7, 75, 1)], .86)),
 dict(key="wonder", theme="wonder", year=1922, ys="1920s", name="CONN NEW WONDER", credit="1920s DANCE BAND",
      fact=("OVER 1 MILLION", "SOLD BY 1925"),
      notes=tn([(0, 69, .3), (2/3, 72, .25), (1, 74, .3), (1 + 2/3, 72, .25), (2, 69, .55), (2 + 2/3, 65, .25), (3, 67, .3), (3 + 2/3, 69, .25),
                (4, 71, .3), (4 + 2/3, 74, .25), (5, 77, .45), (5 + 2/3, 74, .25), (6, 76, .55), (6 + 2/3, 72, .25), (7, 70, .7)], .88)),
 dict(key="c6m", theme="c6m", year=1940, ys="1940s", name="CONN 6M", credit="SWING ERA BALLAD", gl={1: (-5, .3), 4: (-3, .2), 7: (-2, .18)},
      fact=("JOHNNY HODGES,", "ELLINGTON'S ALTO"),
      notes=tn(swing([(0, 68, .5), (.5, 65, .5), (1, 73, 2.0), (3, 72, .5), (3.5, 70, .5),
                      (4, 70, 1.5), (5.5, 66, .5), (6, 72, .5), (6.5, 70, .5), (7, 68, 1.0)]), .86)),
 dict(key="grafton", theme="grafton", year=1953, ys="1953", name="GRAFTON ACRYLIC", credit="BEBOP",
      fact=("PARKER PLAYS A", "PLASTIC ALTO"),
      notes=tn([(0, 72, .25), (.25, 75, .25), (.5, 74, .25), (.75, 72, .25), (1, 68, .25), (1.25, 65, .25), (1.5, 67, .25), (1.75, 68, .25),
                (2, 69, .25), (2.25, 70, .25), (2.5, 74, .25), (2.75, 77, .25), (3, 80, .25), (3.25, 77, .25), (3.5, 74, .25), (3.75, 71, .25),
                (4, 70, .5), (4.5, 67, .25), (4.75, 70, .25), (5, 74, .5), (5.5, 72, .25), (5.75, 70, .25), (6, 67, .5), (6.5, 66, .25),
                (6.75, 67, .25), (7, 70, .25), (7.5, 67, .4)], .86)),
 dict(key="sba", theme="sba", year=1959, ys="1959", name="SELMER SUPER ACTION", credit="COOL JAZZ: TAKE FIVE",
      fact=("1ST JAZZ SINGLE TO", "SELL A MILLION"),
      notes=tn(swing([(0, 73, 1.0), (1, 70, .5), (1.5, 68, .5), (2, 66, 1.5), (3.5, 65, .5),
                      (4, 68, .75), (4.75, 66, .25), (5, 65, 1.0), (6, 61, .5), (6.5, 63, 1.5)]), .84)),
 dict(key="varitone", theme="varitone", year=1967, ys="1967", name="SELMER VARITONE", credit="60s SOUL JAZZ", gl={6: (-2, .12), 1: (-1, .08)},
      fact=("A MIC IN THE NECK,", "PLUS SUB-OCTAVES"),
      notes=tn(swing([(0, 65, .5), (.5, 68, .25), (.75, 69, .25), (1, 72, .5), (1.5, 75, .5), (2, 72, .75), (3, 68, .25), (3.25, 65, .75),
                      (4, 65, .5), (4.5, 68, .5), (5, 70, .5), (5.5, 74, .5), (6, 77, .75), (6.75, 75, .25), (7, 74, .5), (7.5, 70, .5)]), .88)),
 dict(key="ewi", theme="ewi", year=1987, ys="1987", name="AKAI EWI1000", credit="80s FUSION",
      fact=("KEYS SENSE TOUCH,", "THEY NEVER MOVE"),
      notes=tn([(0, 76, .25), (.25, 78, .25), (.5, 79, .25), (.75, 83, .25), (1, 81, .5), (1.5, 78, .5), (2, 76, .25), (2.25, 74, .25),
                (2.5, 71, .5), (3, 74, .25), (3.25, 76, .75), (4, 79, .5), (4.5, 78, .25), (4.75, 76, .25), (5, 83, 1.0),
                (6, 81, .25), (6.25, 79, .25), (6.5, 78, .25), (6.75, 74, .25), (7, 76, 1.0)], .86)),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="YAMAHA YAS-62", credit="BOOM-BAP JAZZ",
      fact=("25 TONE HOLES,", "LEATHER PADS"),
      notes=tn(swing(RIFF_A + RIFF_END)) + [(12, 72, 4.3, .9)]),
]
TRIM = {"modern": .3, "sax1846": .2, "conn": 0., "wonder": -.3, "c6m": .6, "grafton": -.6, "sba": .5, "varitone": -.6, "ewi": -1.}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers
BLACK, NICKEL, CORK, REED, STAND = (24, 22, 26), (214, 216, 226), (176, 136, 96), (226, 202, 144), (44, 42, 50)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
def xform(g, f0, l0, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]; g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]; g.L[i] = (a + t, b + t, col, gid)
def box_at(g, p, s, col, emit=False, spec=0.):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit, spec=spec)

def sweep(g, pts, radii, col, n=10, spec=.85, inner=None, inner_from=10**6, two=False):
    """tube along a polyline with per-point radius (rotation-minimising frame). inner: colour of an inward-facing liner
    from ring `inner_from` on (the inside of a bell). Returns (rings, axis)."""
    P_ = np.array([np.asarray(p, float) for p in pts]); N = len(P_)
    tg = np.gradient(P_, axis=0); tg /= np.linalg.norm(tg, axis=1)[:, None]
    a = np.array([0, 0, 1.]) if abs(tg[0][2]) < .9 else np.array([1., 0, 0])
    u = np.cross(tg[0], a); u /= np.linalg.norm(u); rings = []
    for i in range(N):
        u = u - np.dot(u, tg[i])*tg[i]; u /= np.linalg.norm(u); v = np.cross(tg[i], u)
        rings.append([P_[i] + radii[i]*(math.cos(2*math.pi*k/n)*u + math.sin(2*math.pi*k/n)*v) for k in range(n)])
    for i in range(N - 1):
        ref = (P_[i] + P_[i + 1])/2; A, B = rings[i], rings[i + 1]
        for k in range(n):
            k2 = (k + 1) % n
            g.face([A[k], A[k2], B[k2], B[k]], col, ref=ref, spec=spec, two=two)
            if inner is not None and i >= inner_from:
                q = [P_[i] + (A[k] - P_[i])*.975, P_[i] + (A[k2] - P_[i])*.975, P_[i + 1] + (B[k2] - P_[i + 1])*.975, P_[i + 1] + (B[k] - P_[i + 1])*.975]
                cen = sum(q)/4; g.face(q, inner, ref=cen + (cen - ref), spec=spec*.5)
    return rings, P_

def rsurf(rings, axis, fi, fk, lift=1.03):
    """point on a swept surface at fractional ring fi / fractional segment fk, pushed out from the axis by `lift`"""
    fi = clamp(fi, 0, len(rings) - 1.001); i0 = int(fi); i1 = i0 + 1; ti = fi - i0
    n = len(rings[0]); k0 = int(math.floor(fk)); tk = fk - k0; k0 %= n; k1 = (k0 + 1) % n
    p = (rings[i0][k0]*(1 - tk) + rings[i0][k1]*tk)*(1 - ti) + (rings[i1][k0]*(1 - tk) + rings[i1][k1]*tk)*ti
    c = axis[i0]*(1 - ti) + axis[i1]*ti
    return c + (p - c)*lift

def k_facing(rings, axis, i, d):
    """fractional ring segment whose outward direction is closest to d"""
    d = unit(d); best, bk = -9, 0
    for k, p in enumerate(rings[i]):
        s_ = np.dot(unit(p - axis[i]), d)
        if s_ > best: best, bk = s_, k
    return bk

def slab(g, a, b, side, w, col, two=True):
    a, b, side = np.asarray(a, float), np.asarray(b, float), unit(side)*w/2
    g.face([a - side, b - side, b + side, a + side], col, two=two)

# ---------------------------------------------------------------- the alto (all eras share one skeleton)
YB, YT, BOWR, RB, RT, RBELL0 = .118, .608, .05, .029, .013, .031
XB = 2*BOWR
def rbody(y): return RB + (RT - RB)*(y - YB)/(YT - YB)
def bpt(f, phi, lift=0.):
    """point on the body at fraction f (0 = bow joint, 1 = neck socket) and azimuth phi (0 = front +z, 90 = bell side +x)"""
    y = lerp(YB, YT, f); a = math.radians(phi); n = np.array([math.sin(a), 0, math.cos(a)])
    return np.array([0, y, 0]) + n*(rbody(y) + lift), n

def bell_path(R, flare=1.0, lean=1.0):
    """(points, radii) for the bell from the bow joint to the rim; opening up, forward (+x) and a little toward camera"""
    ps = np.array([XB + .01, YB + .15, 0]); rs = .042
    pts, rad = [], []
    for k in range(5):
        u = k/4; pts.append(np.array([XB + .01*u, YB + .15*u, 0])); rad.append(lerp(RBELL0, rs, u))
    pc = ps + np.array([0, .055, 0]); pr = ps + np.array([.042*lean, .1, .02*lean])
    bz = []
    for k in range(1, 9):
        u = 1 - (1 - k/8)**1.7
        bz.append((1 - u)**2*ps + 2*(1 - u)*u*pc + u*u*pr)
    L = [0.]
    for a_, b_ in zip(bz[::-1], bz[::-1][1:]): L.append(L[-1] + np.linalg.norm(a_ - b_))
    L = L[::-1]                                            # remaining arclength to the rim, per Bezier point
    x0 = .05*flare
    for p, rem, k in zip(bz, L, range(1, 9)):
        lin = lerp(rs, rs*1.12, k/8)
        pts.append(p); rad.append(max(lin, R*(x0/(rem + x0))**1.0))
    return pts, rad

def mouthpiece(g, p, d, lig=NICKEL, body=BLACK, metal_mp=False):
    p, d = np.asarray(p, float), unit(d); dn = unit(np.cross(d, [0, 0, 1.]))
    if dn[1] > 0: dn = -dn                                 # reed side faces down
    T.tube(g, p - d*.004, p + d*.024, .0088, col=CORK, n=8, spec=0)
    mc = NICKEL if metal_mp else body
    T.tube(g, p - d*.006, p + d*.05, .0118, .0112, col=mc, n=10, spec=.7)
    T.tube(g, p + d*.05, p + d*.086, .0112, .006, col=mc, n=10, spec=.7)
    T.tube(g, p + d*.036, p + d*.052, .0124, col=lig, n=10, spec=.9)
    slab(g, p + d*.03 + dn*.0095, p + d*.088 + dn*.0055, [0, 0, 1], .012, REED)
    return p + d*.086

def cup(g, pt, nrm, r, col, shut, h=.004):
    cen = pt + nrm*(h/2 + (.0006 if shut else .0045))
    if not shut: g.face(S.ring(pt + nrm*.0007, nrm, r*.8, 8), (20, 14, 10), ref=pt - nrm, emit=True)
    S.prism(g, cen, nrm, r, h, 8, col, dim(col, .8))
    return cen

def touch(g, pt, nrm, c, on, r=.0088):
    if c.get("pearls", True): S.prism(g, pt, nrm, r, .003, 6, HOT if on else c["pearl"], dim(c["keys"], .8), emit=on)
    else: S.prism(g, pt, nrm, r*.9, .0025, 6, HOT if on else c["keys"], dim(c["keys"], .8), emit=on)

FING = {58: (set(range(6)), "bb"), 59: (set(range(6)), "b"), 60: (set(range(6)), "c"), 61: (set(range(6)), "cs"),
        62: (set(range(6)), None), 63: (set(range(6)), "eb"), 64: ({0, 1, 2, 3, 4}, None), 65: ({0, 1, 2, 3}, None),
        66: ({0, 1, 2, 4}, None), 67: ({0, 1, 2}, None), 68: ({0, 1, 2}, "gs"), 69: ({0, 1}, None), 70: ({0}, "bis"),
        71: ({0}, None), 72: ({1}, None), 73: (set(), None)}
def sax_fing(w):
    """written pitch -> (closed main keys 0..5 = LH1 LH2 LH3 RH1 RH2 RH3, octave register 0/1/2, extra key, palm keys)"""
    if w >= 86: return set(), 2, None, True
    reg = 0
    if w >= 74: w -= 12; reg = 1 + (w >= 69)
    while w < 58: w += 12
    while w > 73: w -= 12
    closed, extra = FING[w]
    return closed, reg, extra, False

# main stack: (fraction, cup azimuth, pearl azimuth, cup radius)
MAIN = [(.80, -24, 24, .0105), (.735, -24, 24, .011), (.67, -24, 24, .0115), (.48, -24, 24, .013), (.415, -24, 24, .0135), (.35, -24, 24, .014)]

def engrave(g, rings, axis, pattern, col, i0, i1, kf, seed=3):
    rng = np.random.default_rng(seed); n = len(rings[0])
    def curl(fi, fk, s_, turns=1.4, m=7):
        pts = [rsurf(rings, axis, fi + s_*.35*math.sin(a)*a/6, fk + s_*math.cos(a)*a/6) for a in np.linspace(0, turns*2*math.pi, m)]
        for a_, b_ in zip(pts, pts[1:]): g.line(a_, b_, col)
    if pattern in ("floral", "small", "leaves"):
        span = {"floral": 2.2, "small": 1.2, "leaves": 3.4}[pattern]
        nv = {"floral": 2, "small": 1, "leaves": 4}[pattern]
        for v_ in range(nv):
            off = (v_ - (nv - 1)/2)*span*.6
            pts = [rsurf(rings, axis, lerp(i0, i1, u), kf + off + .7*math.sin(u*9 + v_)) for u in np.linspace(0, 1, 9)]
            for a_, b_ in zip(pts, pts[1:]): g.line(a_, b_, col)
            for u in np.linspace(.1, .9, 4 if pattern != "small" else 2):
                curl(lerp(i0, i1, u), kf + off + .7*math.sin(u*9 + v_), 1 if rng.random() < .5 else -1)
        if pattern == "leaves":                                # dense rose-gold foliage all over
            for _ in range(10):
                fi, fk = lerp(i0, i1, rng.random()), kf + (rng.random() - .5)*span*2.2
                c0 = rsurf(rings, axis, fi, fk)
                for dk, di in ((.5, .25), (0, .5), (-.5, .25), (0, 0)):
                    c1 = rsurf(rings, axis, fi + di, fk + dk); g.line(c0, c1, col); c0 = c1
    elif pattern == "lady":                                    # oval cartouche with a figure-like S curve inside
        fc = (i0 + i1)/2; ri, rk = (i1 - i0)*.36, 1.25
        pts = [rsurf(rings, axis, fc + ri*math.sin(a), kf + rk*math.cos(a)) for a in np.linspace(0, 2*math.pi, 13)]
        for a_, b_ in zip(pts, pts[1:]): g.line(a_, b_, col)
        s_ = [rsurf(rings, axis, fc - ri*.6 + ri*1.2*u, kf + .45*math.sin(u*6)) for u in np.linspace(0, 1, 6)]
        for a_, b_ in zip(s_, s_[1:]): g.line(a_, b_, col)
        for sg in (-1, 1): curl(fc + ri*1.2, kf + sg*1.6, sg, 1.0, 6)

SAX = {   # metal, keys, pearl, bell colour, inner bell, keyset, bell rim radius, bell keys side, engraving
 "modern":   dict(metal=(234, 188, 86), keys=(240, 196, 98), pearl=(246, 242, 232), R=.061, bk="front", engr="floral", keyset="full",
                  frontF=True, highFs=True, spec=.9),
 "sax1846":  dict(metal=(196, 160, 86), keys=(206, 170, 92), pearl=(206, 170, 92), pearls=False, R=.07, bk="back1", engr=None,
                  keyset="sparse", neck="long", flare=1.35, lean=1.1, spec=.6, oct2=True),
 "conn":     dict(metal=(236, 182, 100), keys=(240, 192, 108), pearl=(238, 234, 222), bell=(224, 150, 112), R=.0625, bk="back",
                  engr="leaves", engr_col=(150, 84, 64), keyset="mid", spec=.95, bowbell=True),
 "wonder":   dict(metal=(198, 200, 206), keys=(214, 216, 222), pearl=(238, 234, 222), inner=(232, 186, 96), R=.062, bk="back",
                  engr="small", engr_col=(130, 132, 140), keyset="mid", spec=.5),
 "c6m":      dict(metal=(218, 166, 70), keys=(226, 176, 80), pearl=(240, 236, 224), R=.0625, bk="back", engr="lady",
                  engr_col=(140, 96, 38), keyset="full", microtuner=True, spec=.85),
 "grafton":  dict(metal=(236, 226, 198), keys=(224, 178, 88), pearl=(248, 248, 244), neckc=(226, 176, 88), rim=(226, 176, 88),
                  inner=(226, 214, 184), R=.06, bk="front", engr=None, keyset="full", frontF=True, spec=.3),
 "sba":      dict(metal=(206, 156, 64), keys=(216, 166, 72), pearl=(242, 238, 226), R=.06, bk="front", engr="small",
                  engr_col=(140, 100, 40), keyset="full", frontF=True, spec=.85),
 "varitone": dict(metal=(228, 180, 78), keys=(236, 190, 90), pearl=(244, 240, 230), R=.06, bk="front", engr="small",
                  engr_col=(150, 106, 40), keyset="full", frontF=True, varitone=True, spec=.9),
}

def sax_stand(g):
    """low cradle under the bow + a backrest post (the sax sits close to the pedestal)"""
    cy = YB - BOWR - RBELL0 - .004
    for z in (-.045, .045):
        S.rod(g, (BOWR - .05, .004, z), (BOWR + .05, .004, z), .006, STAND)
        S.rod(g, (BOWR, .004, z), (BOWR, cy, z*.8), .006, STAND)
    S.rod(g, (BOWR, cy, -.04), (BOWR, cy, .04), .006, STAND)
    S.rod(g, (BOWR, .004, -.045), (0, .3, -.036), .0055, STAND)
    S.rod(g, (-.018, .3, -.034), (.018, .3, -.034), .005, STAND)

def alto(g, c, m, t):
    met, kc = c["metal"], c["keys"]; bellc = c.get("bell", met); sp = c.get("spec", .85)
    ks = c["keyset"]; full = ks == "full"; sparse = ks == "sparse"
    closed, reg, extra, palm = sax_fing(m + 9)
    sax_stand(g)
    # body, bow, bell
    ys = np.linspace(YB, YT, 9)
    body_r, body_ax = sweep(g, [(0, y, 0) for y in ys], [rbody(y) for y in ys], met, n=10, spec=sp)
    ang = np.linspace(math.pi, 2*math.pi, 9)
    bow_r, bow_ax = sweep(g, [(BOWR + BOWR*math.cos(a), YB + BOWR*math.sin(a), 0) for a in ang], list(np.linspace(RB, RBELL0, 9)),
                          bellc if c.get("bowbell") else met, n=10, spec=sp)
    bp, br = bell_path(c["R"], c.get("flare", 1.), c.get("lean", 1.))
    nfl = 6
    bell_r, bell_ax = sweep(g, bp, br, bellc, n=12, spec=sp, inner=c.get("inner", dim(bellc, .78)), inner_from=len(bp) - nfl - 1)
    rim = bell_ax[-1]; d_rim = unit(bell_ax[-1] - bell_ax[-2])
    T.tube(g, rim - d_rim*.005, rim + d_rim*.002, c["R"]*1.05, col=c.get("rim", bellc), n=14, spec=sp, two=True)
    thr = len(bp) - nfl - 1
    g.face(S.ring(bell_ax[thr], unit(bell_ax[thr + 1] - bell_ax[thr]), br[thr]*.97, 10), (26, 18, 12), ref=bell_ax[thr] - d_rim, emit=True, two=True)
    for y0, x0, r0 in ((YB + .004, 0, RB), (YB + .004, XB, RBELL0)):
        T.tube(g, (x0, y0, 0), (x0, y0 + .012, 0), r0*1.1, col=met if x0 == 0 else bellc, n=10, spec=sp)
    T.tube(g, (0, YT - .016, 0), (0, YT + .002, 0), RT*1.4, col=met, n=10, spec=sp)               # neck receiver
    box_at(g, (0, YT - .008, -RT*1.5), .003, kc)
    a_, _ = bpt(.5, 88, .001); T.tube(g, a_, (XB + .006 - .036, a_[1], 0), .0025, col=kc, n=4)     # bell brace
    # neck + mouthpiece
    if c.get("neck") == "long":
        neck = [(0, YT, 0), (0, YT + .022, 0), (-.007, YT + .042, 0), (-.026, YT + .056, 0), (-.058, YT + .06, 0), (-.1, YT + .056, 0), (-.142, YT + .049, 0)]
    else:
        neck = [(0, YT, 0), (0, YT + .03, 0), (-.006, YT + .055, 0), (-.02, YT + .075, 0), (-.042, YT + .088, 0), (-.068, YT + .092, 0),
                (-.095, YT + .088, 0), (-.118, YT + .08, 0)]
    neck_r, neck_ax = sweep(g, neck, list(np.linspace(.0118, .0086, len(neck))), c.get("neckc", met), n=8, spec=sp)
    nd = unit(neck_ax[-1] - neck_ax[-2])
    mouthpiece(g, neck_ax[-1], nd, metal_mp=c.get("metal_mp", False))
    if c.get("microtuner"): T.tube(g, neck_ax[-1] - nd*.034, neck_ax[-1] - nd*.016, .0118*1.2, col=(236, 200, 120), n=8, spec=.95)
    # octave keys
    up_n = lambda i: unit(np.array([0, 1., 0]) - np.dot([0, 1., 0], unit(neck_ax[min(i + 1, len(neck_ax) - 1)] - neck_ax[max(i - 1, 0)]))
                          *unit(neck_ax[min(i + 1, len(neck_ax) - 1)] - neck_ax[max(i - 1, 0)]))
    ki = len(neck_ax) - 3; kp = neck_ax[ki] + up_n(ki)*.013
    on_neck = (reg >= 1) if not c.get("oct2") else (reg == 2)
    S.prism(g, kp, up_n(ki), .0055, .003, 6, HOT if on_neck else kc, dim(kc, .8), emit=on_neck)
    lv = [kp] + [neck_ax[i] + up_n(i)*.0145 for i in range(ki - 1, 0, -1)] + [np.array([0, YT - .02, -RT - .007])]
    for a_, b_ in zip(lv, lv[1:]): g.line(a_, b_, kc)
    th_, thn = bpt(.80, 180, .008); S.prism(g, th_, thn, .0055, .003, 6, HOT if reg else kc, dim(kc, .8), emit=bool(reg))
    g.line(lv[-1], th_, kc)
    if c.get("oct2"):                                                    # Sax's second (body) octave key
        p2, n2 = bpt(.95, 140, .004); on2 = reg == 1
        S.prism(g, p2, n2, .006, .003, 6, HOT if on2 else kc, dim(kc, .8), emit=on2); g.line(p2, th_, kc)
    tr, trn = bpt(.42, 180, .006); g.box(tr[0] - .006, tr[0] + .006, tr[1] - .003, tr[1] + .004, tr[2] - .004, tr[2] + .002, kc)
    sr, srn = bpt(.70, 180, .004); T.tube(g, sr, sr + srn*.006, .0035, col=kc, n=5)
    # main stack
    for i, (f, cphi, pphi, cr) in enumerate(MAIN):
        shut = i in closed
        pt, nrm = bpt(f, cphi); cen = cup(g, pt, nrm, cr, kc, shut)
        tp_, tn_ = bpt(f - .01, pphi, .013); touch(g, tp_, tn_, c, shut)
        g.line(cen, tp_, kc)
    # auxiliary keys
    gs_on, low_on = extra == "gs", extra in ("b", "bb", "c", "cs", "eb")
    pt, nrm = bpt(.62, -62); cup(g, pt, nrm, .009, kc, gs_on)
    if not sparse:
        pt, nrm = bpt(.705, -4); cup(g, pt, nrm, .006, kc, extra == "bis")
        for f_ in ((.56, .60) if full else (.58,)):
            pt, nrm = bpt(f_, 84); cen = cup(g, pt, nrm, .008, kc, False)
            tp_, tn_ = bpt(f_ - .12, 70, .008); g.line(cen, tp_, kc); S.prism(g, tp_, tn_, .004, .006, 4, kc, dim(kc, .8))
        for pf in ((.86, .9, .94) if full else (.88, .93)):           # palm keys
            pt, nrm = bpt(pf + .03, 104); cen = cup(g, pt, nrm, .0085, kc, not palm)
            tp_, tn_ = bpt(pf, 72, .009); touch(g, tp_, tn_, c, palm, .005); g.line(cen, tp_, kc)
        tbl, tbn = bpt(.55, 50, .009)                                    # LH pinky table
        for k in range(3):
            on = (k == 0 and gs_on) or (k == 1 and extra == "cs") or (k == 2 and extra in ("b", "bb"))
            q = tbl + np.array([0, -.008*k, 0]); S.prism(g, q, tbn, .0042, .003, 4, HOT if on else kc, dim(kc, .8), emit=on)
        rp, rpn = bpt(.26, 48, .01)                                      # RH pinky Eb / C
        for k, nm in enumerate(("eb", "c")):
            S.prism(g, rp + np.array([0, -.01*k, 0]), rpn, .0048, .003, 4, HOT if extra == nm else kc, dim(kc, .8), emit=extra == nm)
        for phi, l0, f0_, f1_ in ((-78, .006, .3, .86), (-98, .005, .36, .8), (76, .006, .4, .62)):   # key rods
            a_, _ = bpt(f0_, phi, l0); b_, _ = bpt(f1_, phi, l0); g.line(a_, b_, kc)
            if phi < 0 or full: g.line(a_ + [0, 0, -.001], b_ + [0, 0, -.001], dim(kc, .7))
    else:
        for pf in (.89, .94):
            pt, nrm = bpt(pf + .03, 100); cen = cup(g, pt, nrm, .008, kc, not palm)
            tp_, tn_ = bpt(pf, 70, .008); touch(g, tp_, tn_, c, palm, .005); g.line(cen, tp_, kc)
        for (f, cphi, pphi, cr) in MAIN:                                 # each key on its own posts, no rods
            a_, _ = bpt(f + .02, cphi - 30, .004); b_, _ = bpt(f - .02, cphi - 30, .004); g.line(a_, b_, kc)
    if c.get("frontF"):
        p_, n_ = bpt(.845, 18, .008); S.prism(g, p_, n_, .0045, .0025, 6, HOT if palm else c["pearl"], dim(kc, .8), emit=palm)
    if c.get("highFs"):
        p_, n_ = bpt(.53, 112); cup(g, p_, n_, .0075, kc, True)
    # low keys: Eb + C on the body under a guard, C# on the bow, B + Bb on the bell
    for (f, phi, r), nm in (((.2, 2, .015), "eb"), ((.13, -26, .017), "c")):
        pt, nrm = bpt(f, phi); cup(g, pt, nrm, r, kc, extra == nm)
    if sparse:
        a_, _ = bpt(.06, -40, .02); b_, _ = bpt(.26, -40, .02); g.line(a_, b_, kc); g.line(b_, bpt(.26, 20, .02)[0], kc)
    else:
        gp = [bpt(.25, -48, .016)[0], bpt(.09, -48, .018)[0], bpt(.06, -12, .02)[0], bpt(.09, 22, .018)[0], bpt(.25, 24, .016)[0]]
        for a_, b_ in zip(gp, gp[1:]): g.line(a_, b_, kc)
        q_ = rsurf(bow_r, bow_ax, 5.6, k_facing(bow_r, bow_ax, 5, (-.3, -.2, 1)), 1.0)
        qn = unit(q_ - bow_ax[5]); cup(g, q_, qn, .013, kc, extra == "cs")
    bkd = {"front": unit((-.62, 0, .78)), "back": unit((.55, 0, -.83)), "back1": unit((.55, 0, -.83))}[c["bk"]]
    nbk = 1 if c["bk"] == "back1" else 2
    for j in range(nbk):
        fi = 1.2 + 1.3*j; ax_ = bell_ax[0]*(1 - fi/4) + bell_ax[4]*(fi/4); r_ = lerp(RBELL0, .042, fi/4)
        pt = ax_ + bkd*r_; nm = ("bb", "b")[j]
        cup(g, pt, bkd, .0155 - .001*j, kc, extra == nm or (nbk == 1 and extra in ("b", "bb")))
        gd = [pt + bkd*.014 + np.array([0, .012, 0]) + np.cross(bkd, [0, 1, 0])*.014*s_ for s_ in (-1, 1)]
        g.line(gd[0], gd[1], kc); g.line(gd[0], gd[0] - [0, .022, 0], kc); g.line(gd[1], gd[1] - [0, .022, 0], kc)
    # engraving
    if c.get("engr"):
        ec = c.get("engr_col", dim(bellc, .66))
        kf = k_facing(bell_r, bell_ax, len(bp) - 4, (.15, 0, 1))
        top = len(bp) - 1.4
        engrave(g, bell_r, bell_ax, c["engr"], ec, 5.0 if c["engr"] != "small" else top - 2.2, top, kf)
        if c["engr"] == "leaves":
            engrave(g, bell_r, bell_ax, "leaves", ec, 1.0, 5.0, kf, seed=8)
            engrave(g, bow_r, bow_ax, "small", ec, 2.0, 7.0, k_facing(bow_r, bow_ax, 4, (0, 0, 1)), seed=5)
    # Varitone: neck pickup, control box on the lower bell, cables, a small amp at the back
    if c.get("varitone"):
        pk = neck_ax[2] - up_n(2)*.014; T.tube(g, pk, pk + np.array([.006, -.014, .006]), .0055, col=NICKEL, n=6, spec=.9)
        fi = 2.4; ax_ = bell_ax[0]*(1 - fi/4) + bell_ax[4]*(fi/4); r_ = lerp(RBELL0, .042, fi/4)
        bxc = ax_ + np.array([-.012, 0, r_ + .011])
        g.box(bxc[0] - .027, bxc[0] + .027, bxc[1] - .04, bxc[1] + .04, bxc[2] - .009, bxc[2] + .009, (206, 210, 220), spec=.9)
        fz = bxc[2] + .0092
        g.face([(bxc[0] - .024, bxc[1] - .036, fz), (bxc[0] + .006, bxc[1] - .036, fz), (bxc[0] + .006, bxc[1] + .036, fz),
                (bxc[0] - .024, bxc[1] + .036, fz)], (112, 142, 170), ref=bxc)
        g.face([(bxc[0] + .007, bxc[1] - .036, fz), (bxc[0] + .024, bxc[1] - .036, fz), (bxc[0] + .024, bxc[1] + .036, fz),
                (bxc[0] + .007, bxc[1] + .036, fz)], (22, 22, 26), ref=bxc)
        for k in range(4):
            y_ = bxc[1] + .024 - .016*k
            g.box(bxc[0] - .02, bxc[0] - .004, y_ - .002, y_ + .002, fz, fz + .002, (16, 16, 20))
        for k in range(3):
            S.prism(g, np.array([bxc[0] + .0155, bxc[1] + .022 - .022*k, fz + .003]), (0, 0, 1), .0052, .006, 8, (220, 224, 232), (150, 154, 164))
        cab = [pk, np.array([-RT - .006, YT - .04, .004]), np.array([-.024, .45, .01]), np.array([-.03, .3, .02]),
               np.array([-.02, .2, .03]), bxc + np.array([-.027, 0, 0])]
        for a_, b_ in zip(cab, cab[1:]): g.line(a_, b_, (30, 30, 36))
        out = [bxc + np.array([0, -.04, 0]), bxc + np.array([.01, -.09, .0]), np.array([.2, .006, .02]), np.array([.26, .006, -.1]), np.array([.27, .05, -.14])]
        for a_, b_ in zip(out, out[1:]): g.line(a_, b_, (30, 30, 36))
        g.box(.19, .37, 0, .2, -.32, -.14, (44, 38, 36))                                  # amp
        g.face([(.2, .02, -.1395), (.36, .02, -.1395), (.36, .16, -.1395), (.2, .16, -.1395)], (92, 76, 62), ref=(.28, .1, -.2))
        for k in range(6): g.line((.2, .03 + .022*k, -.139), (.36, .03 + .022*k, -.139), (70, 58, 48))
        g.box(.2, .36, .17, .195, -.1395, -.1385, (196, 198, 206), spec=.6)
        for k in range(3): S.prism(g, np.array([.25 + .04*k, .183, -.137]), (0, 0, 1), .005, .004, 6, (230, 230, 236), (120, 120, 130))
    return rim

# ---------------------------------------------------------------- 1987: EWI1000 + EWV2000
SILV, EBLK, PLATE = (204, 206, 212), (36, 36, 40), (232, 234, 242)
EWI_MAIN = [(.58, 0), (.545, 0), (.51, 0), (.40, 0), (.365, 0), (.33, 0)]
def ewi(g, m, t):
    w = m
    while w > 85: w -= 12
    closed, reg, extra, palm = sax_fing(w); roll = int(clamp((m - 58)//12, 0, 3))
    # stand: tripod + cup holding the bottom end
    hub = np.array([0, .04, 0])
    for k in range(3):
        a = 2*math.pi*k/3 + .5; S.rod(g, hub, hub + np.array([.12*math.cos(a), -.036, .12*math.sin(a)]), .0055, STAND)
    T.tube(g, (0, .06, 0), (0, .1, 0), .03, .028, col=STAND, n=8, spec=.3); S.rod(g, hub, (0, .06, 0), .007, STAND)
    # body
    g.box(-.018, .018, .095, .66, -.014, .016, SILV, spec=.6)
    g.box(-.016, .016, .095, .66, -.022, -.014, EBLK)                                    # black underside
    g.box(-.017, .017, .095, .27, -.036, -.014, EBLK)                                    # thumb / roller housing
    md = unit((-.22, 1, .12)); mb = np.array([0, .66, 0])
    T.tube(g, mb - md*.01, mb + md*.03, .016, .014, col=EBLK, n=8, spec=.4)
    T.tube(g, mb + md*.03, mb + md*.095, .012, .005, col=EBLK, n=8, spec=.4)
    for i, (y, _) in enumerate(EWI_MAIN):                                                  # touch plates: glow, never move
        on = i in closed
        g.box(-.008, .008, y - .006, y + .006, .016, .0175, HOT if on else PLATE, emit=on, spec=.9)
    for k, y in enumerate((.47, .44, .29, .26)):                                           # side plates
        on = (k == 0 and extra == "gs") or (k == 2 and extra in ("eb", "c")) or (k == 3 and extra in ("b", "bb", "cs"))
        g.box(.018, .0195, y - .008, y + .008, -.004, .01, HOT if on else PLATE, emit=on, spec=.9)
    for k in range(4):                                                                     # roller octave switches
        y = .2 - .022*k; on = k == roll
        T.tube(g, (-.016, y, -.04), (.016, y, -.04), .0065, col=HOT if on else (56, 56, 62), n=6, spec=.2)
        if on: g.face(S.ring((.0165, y, -.04), (1, 0, 0), .006, 6), HOT, ref=(0, y, -.04), emit=True)
    g.box(-.01, .01, .6, .62, -.03, -.022, EBLK)                                          # bite / bend sensor block
    # cable to the EWV2000
    cab = [np.array([0, .095, -.01]), np.array([.012, .05, -.03]), np.array([.03, .008, -.07]), np.array([.07, .008, -.12]),
           np.array([.08, .03, -.175])]
    for a_, b_ in zip(cab, cab[1:]): T.tube(g, a_, b_, .0035, col=(30, 30, 34), n=4, spec=.2)
    # EWV2000 sound module behind
    x0, x1, y0, y1, z0, z1 = -.24, .24, 0., .11, -.44, -.18
    g.box(x0, x1, y0, y1, z0, z1, (190, 192, 198), spec=.5)
    fz = z1 + .0008
    for xa, xb in ((x0, x0 + .018), (x1 - .018, x1)):
        g.face([(xa, y0 + .004, fz), (xb, y0 + .004, fz), (xb, y1 - .004, fz), (xa, y1 - .004, fz)], (166, 168, 176), ref=(0, .05, -.3))
    g.face([(-.03, .072, fz), (.11, .072, fz), (.11, .094, fz), (-.03, .094, fz)], (14, 16, 18), ref=(0, .05, -.3), emit=True)
    nb = int(clamp((m - 60)/2.2, 1, 12))
    for k in range(nb):                                                                    # display: a level bar for the note
        xa = -.024 + k*.011; g.face([(xa, .077, fz + .0004), (xa + .008, .077, fz + .0004), (xa + .008, .089, fz + .0004), (xa, .089, fz + .0004)],
                                    (120, 230, 220), ref=(0, .05, -.3), emit=True)
    for k in range(4): S.prism(g, np.array([-.19, .022 + .022*k, fz + .004]), (0, 0, 1), .0075, .008, 8, (70, 110, 200), (40, 70, 140))
    g.face([(-.145, .02, fz + .0003), (-.138, .02, fz + .0003), (-.138, .09, fz + .0003), (-.145, .09, fz + .0003)], (20, 20, 24), ref=(0, .05, -.3))
    for r_ in range(2):
        for k in range(8):
            xa = -.02 + k*.017; ya = .022 + r_*.022
            g.box(xa, xa + .011, ya, ya + .012, fz, fz + .004, (226, 226, 230))
    for k in range(2): g.box(.125 + k*.02, .138 + k*.02, .074, .086, fz, fz + .004, (214, 50, 60))
    for k, cc in enumerate(((210, 50, 60), (70, 110, 200), (70, 110, 200))):
        S.prism(g, np.array([.2, .085 - .03*k, fz + .004]), (0, 0, 1), .007, .008, 8, cc, dim(cc, .6))
    return np.array([0, .14, -.3])

# ---------------------------------------------------------------- scene assembly
def cur_note(s, t, ev):
    live = [e for e in ev if e[0] < e[3] + .03]
    return min(live)[1] if live else (min(ev)[1] if ev else s["ev"][0][1])

def build(g, t, ev, fx, s):
    k = s["key"]; m = cur_note(s, t, ev)
    if k == "ewi": mouth = ewi(g, m, t); off = (.12, .04, 0)
    else: mouth = alto(g, SAX[k], m, t); off = (.08, .03, 0)
    mouth = np.asarray(mouth, float)
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(mouth + off), age))
        if tp and age < .6: fx.append(("wave", tuple(mouth + [.03, .01, 0]), age, 0))

# ---------------------------------------------------------------- sound: alto voice, EWI lead
def _t(n): return np.arange(n)/SR
_RN = np.random.default_rng(23)
def sax_tone(f, dur, v, bright=.6, vib=.005, vrate=5.2, odd=.12, scoop=.03, att=.025, formant=1150., fboost=.8, breath=.05,
             growl=0., slap=0., gl=None):
    n = int((dur + .16)*SR); t = _t(n)
    env = np.minimum(1, t/att)**1.4*np.where(t < dur, 1, np.exp(-(t - dur)/.05))*(.86 + .14*np.exp(-t*5))
    fr = f*(1 - scoop*np.exp(-t/.05))*(1 + vib*np.sin(2*np.pi*vrate*t)*np.clip((t - .16)/.3, 0, 1))
    if gl:
        semis, secs = gl; u = np.clip(t/secs, 0, 1); fr = fr*2**(semis*(1 - u*u*(3 - 2*u))/12)
    ph = 2*np.pi*np.cumsum(fr)/SR
    dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
    for k in range(1, 48):
        fk = f*k
        if fk > 9000: break
        base = (1/k**.85)*((1 - odd) if k % 2 == 0 else 1)
        base *= 1 + fboost*math.exp(-((fk - formant)/650)**2) + .6*bright*math.exp(-((fk - 2.4*formant)/900)**2)
        a1 = base*math.exp(-k/3.); a2 = base*math.exp(-k/(5 + 11*bright))
        s_ = np.sin(k*ph + .4*k); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
    mb = np.clip(env*v, 0, 1)**1.2
    x = (dark/sd*(1 - mb) + brt/sb*mb)*env
    if growl: x = x*(1 + growl*np.sin(2*np.pi*29*t + 3*np.sin(2*np.pi*7*t)))
    if breath:
        nz = D.bp(_RN.normal(0, 1, n), 1600, 6500); x = x + breath*nz*env*(.45 + 1.6*np.exp(-t*18))
    if slap:
        k = int(.03*SR); tt = t[:k]
        x[:k] += slap*(np.sin(2*np.pi*170*tt)*np.exp(-tt*110) + D.bp(_RN.normal(0, 1, k), 800, 4000)*np.exp(-tt*300)*.6)
    return x*v

def ewi_lead(f, dur, v, f_prev=None):
    n = int((dur + .18)*SR); t = _t(n)
    fr = np.full(n, f, float)
    if f_prev: u = np.clip(t/.055, 0, 1); fr = f*(f_prev/f)**(1 - u*u*(3 - 2*u))
    fr = fr*(1 + .007*np.sin(2*np.pi*5.6*t)*np.clip((t - .22)/.3, 0, 1))
    env = np.minimum(1, t/.018)*np.where(t < dur, 1, np.exp(-(t - dur)/.07))
    out = np.zeros(n)
    for det in (-.0035, .0035):
        ph = 2*np.pi*np.cumsum(fr*(1 + det))/SR; dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
        for k in range(1, 40):
            fk = f*k
            if fk > 9000: break
            a = 1/k*(1 if k % 2 else .55)
            a1 = a/(1 + (fk/1100)**4); a2 = a/(1 + (fk/3800)**4)
            s_ = np.sin(k*ph); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
        op = np.clip(env*np.exp(-t*2.2)*1.2 + .25, 0, 1)
        out += dark/sd*(1 - op) + brt/sb*op
    return out*.5*env*v

TONES = {"modern":   dict(bright=.62, vib=.004, vrate=5.0, formant=1100, fboost=.9, breath=.045, scoop=.035, att=.03),
         "sax1846":  dict(bright=.5, vib=0., formant=950, fboost=1.1, breath=.03, scoop=.015, att=.035, odd=.25),
         "conn":     dict(bright=.55, vib=.002, formant=1000, fboost=.9, breath=.035, scoop=.02, att=.03),
         "wonder":   dict(bright=.72, vib=.009, vrate=6.6, formant=1250, fboost=.8, breath=.03, scoop=.02, att=.012, slap=.55),
         "c6m":      dict(bright=.32, vib=.013, vrate=5.3, formant=800, fboost=1.0, breath=.06, scoop=.04, att=.06),
         "grafton":  dict(bright=.82, vib=.003, vrate=5.6, formant=1400, fboost=1.0, breath=.04, scoop=.03, att=.014),
         "sba":      dict(bright=.38, vib=.002, vrate=5.0, formant=1250, fboost=.4, breath=.08, scoop=.02, att=.035, odd=.3),
         "varitone": dict(bright=.68, vib=.005, vrate=5.4, formant=1150, fboost=.9, breath=.04, scoop=.04, att=.02, growl=.12)}

def _echo(x, dl, taps):
    d = int(dl*SR); y = np.zeros(len(x) + d*len(taps)); y[:len(x)] += x
    for j, a in enumerate(taps, 1): y[d*j:d*j + len(x)] += a*x
    return y
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    beat = round(nt/BEAT_S, 3); gl = next((v_ for b, v_ in s.get("gl", {}).items() if abs(b - beat) < 1e-3), None)
    if key == "ewi":
        prev = s["ev"][idx - 1] if idx > 0 else None
        fp = midi_hz(prev[1]) if prev is not None and nt - prev[0] < .45 else None
        return _echo(ewi_lead(midi_hz(m), dur, v, fp), .75*BEAT_S, (.22, .08))
    x = sax_tone(midi_hz(m), dur, v, gl=gl, **TONES[key])
    if key == "varitone":                                           # octave divider + echo
        n = len(x); t = _t(n); f2 = midi_hz(m)/2; sub = np.zeros(n)
        for k in range(1, 14, 2):
            if f2*k > 3000: break
            sub += np.sin(2*np.pi*f2*k*t)/k
        envs = np.minimum(1, t/.02)*np.where(t < dur, 1, np.exp(-(t - dur)/.05))
        x = _echo(x + .42*sub*envs*v, .19, (.38, .18, .08))
    elif key == "modern":
        x = _echo(x, .75*BEAT_S, (.22, .08))
    return x

# ---------------------------------------------------------------- backing: boom bap (dusty kick, lazy swung snare, vinyl crackle, upright bass, Rhodes)
LAZY = .022
def _kick(v): return D.lp(D.kick_s(47, 108, 6.5, .06, .55, 1.6, .35), 2400)*v
def _snare(v):
    a = D.snare_s(196, 13, 26, 500, 4200, .7, .45); b = C.clap(1.)*.3; n = max(len(a), len(b))
    return D.lp(np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))), 5500)*v
def _hat(v): return D.lp(P.hat(1.), 6500)*.7*v
def _vinyl(v):
    n = int((BAR_S + .03)*SR); x = D.bp(_RN.normal(0, 1, n), 900, 7000)*.035
    for p in _RN.integers(0, n - 200, 26):
        k = int(_RN.integers(20, 120)); x[p:p + k] += _RN.normal(0, 1, k)*np.exp(-np.arange(k)/18)*_RN.uniform(.1, .5)
    return x*v
KIT = {"kick": _kick, "snare": _snare, "ghost": lambda v: _snare(v)*.5, "hat": _hat, "vinyl": _vinyl,
       "crash": lambda v: D.lp(P.crash(1.), 6000)*.55*v}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("snare",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B; ev.append((T0, "vinyl", 1.))
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", .8)]; continue
        if b == 0:
            ev += [(T0, "kick", .9), (T0 + 3*Q + LAZY, "snare", .55), (T0 + sw(3.75)*Q, "ghost", .4)]
            ev += [(T0 + sw(k*.5)*Q, "hat", .3 if k % 2 else .45) for k in range(8)]; continue
        kicks = [0, 1.75, 2.5] if b % 2 else [0, .75, 2.25, 2.5]
        ev += [(T0 + sw(k)*Q, "kick", 1. if k == 0 else .82) for k in kicks]
        ev += [(T0 + Q + LAZY, "snare", .9), (T0 + 3*Q + LAZY, "snare", .92)]
        ev += [(T0 + sw(k*.5)*Q, "hat", .5 if k % 2 == 0 else .3) for k in range(8)]
        if b % 4 == 3: ev.append((T0 + sw(3.75)*Q, "ghost", .35))
        if b in (4, 20): ev.append((T0, "crash", .5))
    return ev
def _chord_at(b, beat):
    ch = BARS[min(b, len(BARS) - 1)]; return ch[int(beat)] if isinstance(ch, tuple) else ch
def _root(name):
    r = CH[name][0]
    while r > 45: r -= 12
    while r < 33: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS); Q = c.BEAT
    for b in range(1, nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord_at(b, 0)), 2.4, 1.)); continue
        r0, r2, nx = _root(_chord_at(b, 0)), _root(_chord_at(b, 2)), _root(_chord_at(b + 1, 0))
        out += [(T0, r0, 1.3*Q, .95), (T0 + sw(1.5)*Q, r0 + (7 if r2 == r0 else 12), .35*Q, .7),
                (T0 + 2*Q, r2, 1.2*Q, .9), (T0 + sw(3.5)*Q, nx + (1 if nx < r2 else -1), .35*Q, .72)]
    return out
def _pluck(f, dur, v, bright=.3, pos=.25, sus=2.2):
    ring = dur + .25; n = int(ring*SR); t = _t(n); x = np.zeros(n)
    for k in range(1, 18):
        if f*k > 5000: break
        a = (abs(math.sin(math.pi*k*pos)) + .04)/k**(1.5 - .8*bright)
        x += a*np.sin(2*np.pi*f*k*t + k)*np.exp(-t*(1.5 + .7*(k - 1))*(.6 + f/500)/sus)
    return x*np.minimum(1, t/.003)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))*v
def bass_voice(m, dur, v):
    f = midi_hz(m); x = _pluck(f, dur, v); t = _t(len(x))
    return x + .8*np.sin(2*np.pi*f*t)*np.exp(-t*2.6)*np.minimum(1, t/.004)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))*v
def _rhodes(f, dur, v):
    n = int((dur + .35)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph)*np.exp(-t*1.0) + .28*np.sin(2*ph)*np.exp(-t*4) + .1*np.sin(3*ph + .5)*np.exp(-t*7) + .05*np.sin(7.1*ph)*np.exp(-t*16)
    env = np.minimum(1, t/.005)*np.where(t < dur, 1, np.exp(-(t - dur)/.09))
    return x*env*(1 + .12*np.sin(2*np.pi*4.6*t))*v
_LASTPAD = [None]
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); out = np.zeros(n)
    if dur > 2.2: hits = [(0, dur/BEAT_S)]
    elif abs(dur - (BAR_S + .05)) < .1: hits = [(0, 1.45), (sw(2.5), .85), (sw(3.5), .3)]
    else:                                                          # one beat of a split bar: only on a chord change
        if _LASTPAD[0] == tuple(notes): return out
        hits = [(0, 1.8)]
    _LASTPAD[0] = tuple(notes)
    for b, d in hits:
        for mm in notes:
            x = _rhodes(midi_hz(mm), d*BEAT_S, .55*v); a = int(b*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return D.lp(out, 5000)
MIX = dict(drums=-20, bass=-22, pad=-27, verb=-25, verb_len=1.2)

# ---------------------------------------------------------------- background: a subway platform at night
THEMES = {   # wall tile, accent, ceiling, floor
 "intro":    ((232, 232, 226), (250, 204, 40), (30, 32, 38), (118, 120, 124)),
 "sax1846":  ((226, 208, 166), (255, 168, 58), (34, 24, 16), (96, 80, 60)),
 "conn":     ((124, 36, 42), (214, 172, 84), (30, 12, 14), (82, 54, 46)),
 "wonder":   ((56, 96, 74), (240, 168, 60), (14, 24, 20), (72, 86, 72)),
 "c6m":      ((228, 214, 180), (128, 30, 36), (30, 22, 20), (112, 96, 80)),
 "grafton":  ((168, 222, 196), (214, 44, 50), (20, 34, 30), (100, 122, 110)),
 "sba":      ((36, 50, 98), (130, 196, 240), (8, 12, 26), (60, 66, 92)),
 "varitone": ((198, 98, 38), (102, 62, 34), (34, 18, 10), (112, 78, 52)),
 "ewi":      ((128, 130, 134), (255, 64, 170), (22, 22, 26), (78, 78, 82)),
}
LAMP = {"intro": "tube", "sax1846": "gas", "conn": "gas", "wonder": "bulb", "c6m": "bulb", "grafton": "tube", "sba": "tube",
        "varitone": "tube", "ewi": "tube"}
WORD = {"intro": "LIVE", "sax1846": "BAND", "conn": "MARCH", "wonder": "DANCE", "c6m": "SWING", "grafton": "BEBOP", "sba": "COOL",
        "varitone": "SOUL", "ewi": "FUSION"}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[3], T_[1], .3))

LY0, FRZ0, FRZ1, WALL1 = 104, 109, 113, 186
LEDGE1, PIT1, EDGE1, SAFE1 = 190, 216, 219, 223
WALL = (YY >= 36) & (YY < WALL1)
_wy = YY - 113
MORTAR = WALL & (((_wy % 5) == 4) | ((((XX + ((_wy//5) % 2)*4) % 9) == 8)))
WALLSH = np.clip(.64 + .36*np.exp(-np.maximum(YY - 107, 0)/36.), 0, 1)
CEIL = YY < 36
CEILG = .55 + .45*(YY/36.)
FRIEZE = (YY >= FRZ0) & (YY < FRZ1)
TUNNEL = np.zeros((RH, RW), bool); ARCH = np.zeros((RH, RW), bool); TDEPTH = np.zeros((RH, RW))
TUN_C, TUN_HW, TUN_SP = (18, 252), 34, 160
for _c in TUN_C:
    _d = np.hypot(XX - _c, YY - TUN_SP)
    _in = (np.abs(XX - _c) < TUN_HW) & ((YY >= TUN_SP) | (_d < TUN_HW)) & (YY < PIT1)
    _rim = (np.abs(XX - _c) < TUN_HW + 4) & ((YY >= TUN_SP) | (_d < TUN_HW + 4)) & (YY < LEDGE1) & ~_in
    TUNNEL |= _in; ARCH |= _rim
    TDEPTH = np.where(_in, np.clip(1 - np.maximum(np.abs(XX - _c)/TUN_HW, np.clip((TUN_SP + 30 - YY)/60, 0, 1)), 0, 1), TDEPTH)
TRAIL = np.zeros((RH, RW), bool)
for _c in TUN_C:
    for _sx in (-1, 1):
        for _y in range(172, PIT1):
            u = (_y - 172)/(PIT1 - 172); _x = int(round(_c + _sx*(3 + 19*u)))
            if 0 <= _x < RW: TRAIL[_y, _x] = True
TRINGS = np.zeros((RH, RW), bool)
for _c in TUN_C:
    for _r in (12, 20, 28):
        TRINGS |= (np.abs(np.hypot(XX - _c, (YY - TUN_SP)*1.1) - _r) < .6) & (YY < TUN_SP + _r*.8)
TRINGS &= TUNNEL
LEDGE = (YY >= WALL1) & (YY < LEDGE1) & ~TUNNEL
PIT = (YY >= LEDGE1) & (YY < PIT1)
RAILS = PIT & ((YY == 205) | (YY == 212))
SLEEPERS = PIT & (YY >= 204) & (YY <= 213) & ((XX % 6) < 3) & ~RAILS
EDGE = (YY >= PIT1) & (YY < EDGE1)
SAFE = (YY >= EDGE1) & (YY < SAFE1)
DOTS = SAFE & ((XX % 3) == 1) & ((YY % 2) == 0)
FLOOR = YY >= SAFE1
_fd = np.log1p(np.maximum(YY - SAFE1, 0)/4.)/np.log(1.45)
_fx = CX + (XX - CX)*30/np.maximum(YY - 170, 1)
FLINES = FLOOR & ((np.abs(_fd - np.round(_fd)) < .09) | (np.abs(((_fx + 4) % 14) - 7) > 6.4))
POS_A, POS_B, PW, PH = (66, 120), (168, 120), 36, 54
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)

def _poster(th, which):
    tile, acc, ceil, floor = THEMES[th]
    im = Image.new("RGB", (PW, PH)); d = ImageDraw.Draw(im)
    def rect(x0, y0, x1, y1, c_): d.rectangle([x0, y0, x1, y1], fill=c_)
    if th == "intro":
        if which == 0:
            rect(0, 0, PW, PH, (244, 242, 236)); d.ellipse([7, 6, 29, 28], fill=acc); rect(5, 40, 30, 41, (30, 30, 34)); rect(5, 44, 24, 45, (30, 30, 34))
        else:
            rect(0, 0, PW, PH, (24, 24, 28))
            for r in (15, 12, 9): d.ellipse([18 - r, 22 - r, 18 + r, 22 + r], outline=(70, 70, 76))
            d.ellipse([15, 19, 21, 25], fill=acc); rect(6, 44, 30, 45, acc)
    elif th == "sax1846":
        rect(0, 0, PW, PH, (236, 220, 178)); d.rectangle([2, 2, PW - 3, PH - 3], outline=(150, 96, 40)); d.rectangle([4, 4, PW - 5, PH - 5], outline=acc)
        for k in range(4): rect(8, 32 + 4*k, 27 - (k % 2)*5, 32 + 4*k, (110, 80, 50))
        if which: d.ellipse([10, 8, 26, 26], outline=(150, 96, 40)); d.ellipse([14, 12, 22, 22], fill=acc)
    elif th == "conn":
        rect(0, 0, PW, PH, (96, 24, 30) if which == 0 else (60, 14, 20)); d.rectangle([1, 1, PW - 2, PH - 2], outline=acc)
        d.pieslice([6, 6, 30, 34], 180, 360, fill=acc); rect(6, 20, 30, 34, (96, 24, 30) if which == 0 else (60, 14, 20))
        for k in range(3): rect(8, 38 + 4*k, 28, 38 + 4*k, dim(acc, .8))
        if which: d.ellipse([12, 10, 24, 22], fill=(240, 220, 160))
    elif th == "wonder":
        bgc, fg = ((48, 86, 64), acc) if which == 0 else (acc, (40, 76, 56))
        rect(0, 0, PW, PH, bgc); d.ellipse([20, 4, 32, 16], fill=fg)
        for k in range(3):
            pts = [(x, 22 + 7*k + 4*math.sin(x/4 + k)) for x in range(0, PW, 2)]; d.line(pts, fill=fg)
    elif th == "c6m":
        bgc, fg = ((236, 222, 188), acc) if which == 0 else (acc, (236, 222, 188))
        rect(0, 0, PW, PH, bgc)
        for k in range(7):
            a = math.pi*(k + .5)/7; d.line([(18, 36), (18 - 22*math.cos(a), 36 - 22*math.sin(a))], fill=fg)
        for k in range(3): rect(6 + 4*k, 40 + 4*k, PW - 7 - 4*k, 42 + 4*k, fg)
    elif th == "grafton":
        bgc, fg = ((182, 230, 206), acc) if which == 0 else (acc, (182, 230, 206))
        rect(0, 0, PW, PH, bgc)
        for (x, y, r) in ((10, 10, 5), (26, 22, 6), (12, 36, 4)):
            for k in range(4): a = k*math.pi/4; d.line([(x - r*math.cos(a), y - r*math.sin(a)), (x + r*math.cos(a), y + r*math.sin(a))], fill=fg)
        d.arc([4, 18, 30, 50], 200, 320, fill=(30, 40, 40))
    elif th == "sba":
        bgc, fg = ((28, 40, 84), acc) if which == 0 else (acc, (28, 40, 84))
        rect(0, 0, PW, PH, bgc); rect(4, 6, 16, 24, fg); d.ellipse([18, 10, 32, 24], outline=fg); rect(20, 30, 31, 33, fg); rect(4, 30, 14, 31, fg)
    elif th == "varitone":
        bgc, fg = ((222, 116, 44), (110, 64, 34)) if which == 0 else ((110, 64, 34), (230, 130, 50))
        rect(0, 0, PW, PH, bgc)
        for r in (16, 12, 8, 4): d.ellipse([18 - r, 20 - r, 18 + r, 20 + r], outline=fg)
        for k in range(2):
            pts = [(x, 40 + 5*k + 2*math.sin(x/3 + k*2)) for x in range(0, PW, 2)]; d.line(pts, fill=fg)
    elif th == "ewi":
        rect(0, 0, PW, PH, (20, 18, 30))
        for k in range(5): d.line([(0, 30 + 4*k), (PW, 30 + 4*k)], fill=(60, 220, 230) if which == 0 else (120, 60, 200))
        d.polygon([(6, 26), (18, 6), (30, 26)], outline=acc if which == 0 else (60, 220, 230))
        if which: d.line([(2, 48), (10, 40), (18, 48), (26, 40), (34, 48)], fill=acc)
    if which == 0:
        wd = WORD[th]; ink = (30, 24, 20) if th in ("intro", "sax1846", "c6m", "grafton") else (240, 236, 224)
        P.draw_text(d, (PW - P.text_w(wd, 1))//2, PH - 12 if th != "intro" else 30, wd, ink, 1, shadow=False)
    return np.asarray(im).astype(float)
POSTERS = {k: (_poster(k, 0), _poster(k, 1)) for k in THEMES}

def _graffiti():
    """invented spray-paint tags (outlined two-tone pixel letters with drips), no real words or brands"""
    im = Image.new("RGBA", (RW, RH), (0, 0, 0, 0)); d = ImageDraw.Draw(im); r = np.random.default_rng(87)
    def tag(x, y, word, sc, fill, fill2, out=(18, 14, 22)):
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if dx or dy: P.draw_text(d, x + dx*max(1, sc//2 + 1), y + dy*max(1, sc//2 + 1), word, out + (255,), sc, shadow=False)
        P.draw_text(d, x, y, word, fill + (255,), sc, shadow=False)
        h = 7*sc; lo = Image.new("RGBA", (RW, RH), (0, 0, 0, 0)); dl = ImageDraw.Draw(lo)
        P.draw_text(dl, x, y, word, fill2 + (255,), sc, shadow=False)
        a = np.asarray(lo).copy(); a[:y + h//2] = 0; im.alpha_composite(Image.fromarray(a))
        for k in range(len(word)):
            if r.random() < .55:
                xx = x + k*6*sc + int(r.integers(0, 4*sc)); d.line([(xx, y + h), (xx, y + h + int(r.integers(3, 9)))], fill=fill2 + (255,))
    tag(112, 132, "PX87", 2, (255, 64, 170), (255, 150, 40))
    tag(70, 177, "ZAP", 1, (70, 230, 120), (40, 170, 240))
    tag(176, 150, "OK", 2, (60, 200, 255), (150, 90, 255))
    tag(118, 168, "YO", 1, (255, 220, 40), (255, 120, 30))
    d.arc([200, 112, 222, 128], 200, 340, fill=(255, 220, 40, 255), width=2)
    a = np.asarray(im).astype(float); return a[..., :3], a[..., 3]/255.
GRAF_RGB, GRAF_A = _graffiti()

def _lamp_layer(style, tile, acc, fl, T_):
    """returns (rgb add, mask) for light fixtures"""
    out = np.zeros((RH, RW, 3)); m = np.zeros((RH, RW), bool)
    if style == "tube":
        lit = np.array(mix((246, 250, 255), tile, .15), float)*min(1.25, fl)
        housing = (YY >= LY0) & (YY < LY0 + 4); tube = (YY >= LY0 + 1) & (YY < LY0 + 3) & ((XX % 46) < 38)
        out[housing] = (52, 52, 58); out[tube] = lit; m |= housing
    else:
        pts = (58, 212) if style == "gas" else (40, 104, 166, 230)
        warm = np.array((255, 196, 110) if style == "gas" else (255, 232, 170), float)
        for k, x in enumerate(pts):
            y = 118 if style == "gas" else LY0 + 3
            flk = fl*(1 + (.1*math.sin(T_*23 + k*2.1)*math.sin(T_*7.3 + k) if style == "gas" else 0))
            rr = np.hypot(XX - x, YY - y); halo = (rr < 11) & (BAY < np.clip(.75 - rr/14, 0, 1)*min(1.3, flk))
            out[halo] = warm*.8; m |= halo
            core = (np.abs(XX - x) <= 1) & (np.abs(YY - y) <= 1); out[core] = np.minimum(255, warm*1.15*flk); m |= core
            if style == "gas":
                br = (YY == y + 3) & (XX >= x - 4) & (XX <= x + 1) | ((XX == x - 4) & (YY >= y + 3) & (YY <= y + 7))
                out[br] = (40, 30, 24); m |= br
            else:
                cord = (XX == x) & (YY >= 96) & (YY < y - 1); out[cord] = (30, 28, 30); m |= cord
    return out, m

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return np.array(mix(A_[i], B_[i], kb), float)
    tile, acc, ceil, floor = col(0), col(1), col(2), col(3)
    th = th1 if kb >= .5 else th0; style = LAMP[th]
    age = -math.log(max(x["accent"], 1e-6))/6
    fl = 1 + .5*math.exp(-age*10)
    if .03 < age < .07: fl *= .45                                   # lights flicker on the snare
    warm = {"tube": 1., "gas": .82, "bulb": .9}[style]
    bg = np.zeros((RH, RW, 3))
    bg[CEIL] = ceil*CEILG[CEIL][:, None]
    sh = (WALLSH*warm*(.9 + .1*min(fl, 1.3)))[..., None]
    bg[WALL] = (tile[None, None]*sh)[WALL]; bg[MORTAR] = (tile[None, None]*sh*.74)[MORTAR]
    if style != "tube": bg[WALL] = bg[WALL]*.85 + np.array((60, 40, 10))*.15*warm
    if th == "ewi" or th0 == "ewi" or th1 == "ewi":
        wg = (1 - kb if th0 == "ewi" else 0) + (kb if th1 == "ewi" else 0)
        a = GRAF_A*wg; bg = bg*(1 - a[..., None]) + GRAF_RGB*a[..., None]
    bg[FRIEZE] = acc; bg[(YY == FRZ0) | (YY == FRZ1 - 1)] = acc*.7
    for (px, py), j in ((POS_A, 0), (POS_B, 1)):
        art = POSTERS[th0][j]*(1 - kb) + POSTERS[th1][j]*kb
        bg[py - 2:py + PH + 2, px - 2:px + PW + 2] = ceil*.7 + 20
        bg[py:py + PH, px:px + PW] = art*(.86 + .14*min(fl, 1.2))
    if th == "ewi" or th0 == "ewi" or th1 == "ewi":
        wg = (1 - kb if th0 == "ewi" else 0) + (kb if th1 == "ewi" else 0)
        reg = (YY >= POS_A[1]) & (YY < POS_A[1] + PH); a = GRAF_A*wg*reg; bg = bg*(1 - a[..., None]) + GRAF_RGB*a[..., None]
    bg[ARCH] = np.array(mix(tuple(acc.astype(int)), tuple(tile.astype(int)), .45), float)*.8
    tn_ = np.clip(TDEPTH, 0, 1)[..., None]
    bg[TUNNEL] = (np.array((10, 10, 14))[None, None]*(1 - tn_) + ceil[None, None]*.6*tn_)[TUNNEL]
    bg[TRINGS] = ceil*.9 + 12; bg[TRAIL & TUNNEL] = (120, 120, 130)
    sig = (232, 40, 40) if int(T_*2) % 2 else (90, 20, 20)
    for _c in TUN_C: bg[166:168, _c + 9:_c + 11] = sig
    bg[LEDGE] = np.array(mix(tuple(tile.astype(int)), (150, 150, 150), .6), float)*.9; bg[(YY == LEDGE1 - 1) & ~TUNNEL] = acc*.6
    bg[PIT] = np.array((34, 32, 36))*(1 + .3*(YY[PIT] < 200))[:, None]
    bg[SLEEPERS] = (62, 46, 36); bg[RAILS] = (176, 176, 186)
    bg[EDGE] = (176, 176, 170); bg[SAFE] = acc; bg[DOTS] = acc*.72
    bg[FLOOR] = floor*(.75 + .25*np.clip((YY[FLOOR] - SAFE1)/120, 0, 1))[:, None]; bg[FLINES] = floor*.62
    lamp, lm = _lamp_layer(style, tile, acc, fl, T_); bg[lm] = lamp[lm]
    # train headlights rushing past on every bar's downbeat
    ph = T_ % BAR_S; bi = int(T_/BAR_S); dur = .95
    if ph < dur and bi < len(BARS) - 1:
        u = ph/dur; dirn = 1 if bi % 2 == 0 else -1
        hx = -40 + u*(RW + 80) if dirn > 0 else RW + 40 - u*(RW + 80)
        behind = (hx - XX)*dirn
        band = (YY >= 203) & (YY <= 209) & (behind >= 0) & (behind < 220)
        k_ = np.exp(-np.maximum(behind, 0)/70.)*(1 - .25*u)
        bg[band] = bg[band]*(1 - k_[band, None]) + np.array((255, 240, 200))*k_[band, None]
        win = (YY >= 194) & (YY <= 199) & (behind > 12) & (behind < 240) & ((((behind - 12)//9) % 3) != 0)
        kw = .65*np.exp(-np.maximum(behind - 12, 0)/170.)
        bg[win] = bg[win]*(1 - kw[win, None]) + np.array((255, 226, 160))*kw[win, None]
        glow = np.exp(-(((XX - hx)/34.)**2 + ((YY - 206)/26.)**2))*.55
        gm = (YY > 140) & (YY < 240)
        bg[gm] = bg[gm] + (np.array((255, 236, 190))[None]*glow[gm, None])*.6
        hd = (np.abs(XX - hx) <= 2) & (YY >= 204) & (YY <= 207); bg[hd] = (255, 252, 230)
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

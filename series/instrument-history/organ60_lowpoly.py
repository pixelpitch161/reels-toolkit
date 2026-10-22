"""Pixel Pitch: history of the pipe organ, 60-second cut.  python3 organ60_lowpoly.py [out.mp4] | --still t ...

Showcase: the Halberstadt Cage organ (St. Burchardi), which has been playing John Cage's ORGAN2/ASLSP since 2001 and is due to
finish in 2640. Then 250 BC hydraulis -> 2016 Elbphilharmonie -> back to the Cage organ.
Backdrop: a limestone cavern (stalactites that glow with every note, drifting mist, drips that land on the hi-hats, shafts of
light from above). Beat: industrial techno at 120 BPM (distorted rumble kick, anvil hits, choir pad, organ stabs).
Models were built from reference photos (museum and maker pages, Praetorius's Syntagma Musicum plate XXV, church photos).
"""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, ease, midi_hz, SR, RW, RH, CX, YY, XX, BAY, HOT

P._G.setdefault("È", ".##.." "#####" "#...." "####." "#...." "#...." "#####")

OUT, TAG = "organ60_lowpoly.mp4", "po"
BPM = 120
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.0, -12, 11, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE PIPE ORGAN", "2,275 YEARS", "TODAY", -300
INTRO_LINES = (("ONE ORGAN PIECE", "LASTS 639 YEARS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("2,275 YEARS FROM", "A WATER ORGAN")
OUTRO_CREDIT = "ORGAN TECHNO"
LIGHTING = True

# ---------------------------------------------------------------- music (D minor, industrial techno)
CH = {"Dm": (38, (57, 62, 65)), "Bb": (34, (58, 62, 65)), "C": (36, (55, 60, 64)), "A": (33, (57, 61, 64)),
      "Gm": (43, (55, 58, 62)), "F": (41, (57, 60, 65)), "D": (38, (57, 62, 66))}
BARS = ["Dm", "Bb", "C", "A",          # showcase: the Cage organ
        "Dm", "A",                     # hydraulis
        "Dm", "C",                     # portative
        "Dm", "Gm",                    # Halberstadt 1361
        "F", "C",                      # Valere
        "Dm", "Dm",                    # Arnstadt
        "Bb", "C",                     # Saint-Sulpice
        "Dm", "A",                     # Wurlitzer
        "F", "C",                      # Boardwalk
        "Bb", "F",                     # Hammond
        "Dm", "A",                     # Sydney
        "Bb", "Gm",                    # Elbphilharmonie
        "Dm", "Bb",                    # the Cage organ again
        "A",                           # outro
        "D"]                           # final chord
HORN = 1                               # marker pitch: the Wurlitzer's car horn
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]

STAB_POS = (0, 3, 6, 10, 12, 14)
STAB_V = (.95, .8, .9, .85, .75, .9)
STAB_CH = {"Dm": (69, 74, 77), "Bb": (70, 74, 77), "C": (72, 76, 79), "A": (69, 73, 76)}
def stabs(b0, ch): return [(b0 + p/4, STAB_CH[ch], .2, v) for p, v in zip(STAB_POS, STAB_V)]
def cage_drone(bars):
    """ASLSP-style: two held drones and two voices that change one note at a time, offset by half a bar"""
    nb = len(bars); L = 4*nb
    out = [(0, 50, L, .55), (0, 57, L, .55)]
    low = {"Dm": 62, "Bb": 62, "C": 60, "A": 61}
    hi = {"Dm": 64, "Bb": 65, "C": 64, "A": 64}
    for k, ch in enumerate(bars): out.append((4*k, low[ch], 4, .5))
    out.append((0, 65, 2, .45))
    for k, ch in enumerate(bars): out.append((4*k + 2, hi[ch], min(4, L - 4*k - 2), .45))
    return out
SHOW = cage_drone(["Dm", "Bb", "C", "A"]) + stabs(0, "Dm") + stabs(4, "Bb") + stabs(8, "C") + stabs(12, "A")
LAST = (cage_drone(["Dm", "Bb", "A"]) + stabs(0, "Dm") + stabs(4, "Bb") + stabs(8, "A") +
        [(12, (50, 57, 62, 64, 66, 69, 74), 5.0, .9)])

HYD = tn([(0, 74, 1.5), (1.5, 75, .5), (2, 74, .5), (2.5, 72, .5), (3, 70, 1), (4, 73, 1), (5, 70, .5), (5.5, 72, .5), (6, 69, 2)], .85)
PORT = tn([(0, 74, .5), (.5, 76, .5), (1, 77, .5), (1.5, 79, .5), (2, 81, 1), (3, 79, .5), (3.5, 77, .5),
           (4, 79, .5), (4.5, 77, .5), (5, 76, .5), (5.5, 74, .5), (6, 76, .75), (6.75, 72, .25), (7, 74, 1)], .85)
def organum(items): return [(b, (m - 12, m - 7, m), d, .9) for b, m, d in items]
HALB = organum([(0, 62, 1), (1, 65, 1), (2, 67, 1), (3, 69, 1), (4, 67, 1), (5, 65, 1), (6, 64, 1), (7, 62, 1)])
VAL = (tn([(0, 77, 2), (2, 76, 1), (3, 74, 1), (4, 72, 1.5), (5.5, 74, .5), (6, 76, 2)], .8) +
       tn([(0, 69, 1), (1, 72, 1), (2, 72, 1), (3, 70, 1), (4, 67, 2), (6, 67, 2)], .7) +
       tn([(0, 53, 2), (2, 58, 1), (3, 57, 1), (4, 55, 2), (6, 48, 2)], .7))
def toccata(b0, o):
    run = [(0, 81, .25), (.25, 79, .25), (.5, 81, 1.), (1.5, 79, .25), (1.75, 77, .25), (2, 76, .25), (2.25, 74, .25),
           (2.5, 73, .5)]
    return [(b0 + b, (m - 12 + o, m + o), d, .9) for b, m, d in run]
BACH = toccata(0, 0) + [(3, (62, 74), 1., .9)] + toccata(4, -12) + [(7, (38, 50, 57, 62, 65, 69), 1., .95)]
def widor():
    out = []
    for bar, tones in ((0, (65, 70, 74, 77)), (4, (64, 67, 72, 76))):
        seqn = [tones[0], tones[1], tones[2], tones[1], tones[0], tones[1], tones[2], tones[3]]
        for k in range(16):
            b = bar + k*.25
            if k % 2 == 0: out.append((b, seqn[(k//2) % 8], .22, .78))
            else: out.append((b, (tones[2] + 12, tones[3] + 12) if k % 4 == 3 else (tones[1] + 12, tones[2] + 12), .2, .66))
    return out
SULP = widor() + tn([(0, 46, 1), (1, 46, .5), (1.5, 48, .5), (2, 50, 1), (3, 48, 1), (4, 48, 1), (5, 52, .5), (5.5, 50, .5), (6, 48, 2)], .95)
WURL = tn([(0, 74, .25), (.25, 73, .25), (.5, 74, .25), (.75, 77, .25), (1, 81, .5), (1.5, 80, .25), (1.75, 81, .25), (2, 77, .25),
           (2.25, 76, .25), (2.5, 74, .25), (2.75, 73, .25), (3, 74, .5),
           (4, 76, .25), (4.25, 75, .25), (4.5, 76, .25), (4.75, 79, .25), (5, 81, .5), (5.5, 82, .25), (5.75, 81, .25), (6, 79, .25),
           (6.25, 77, .25), (6.5, 76, .25), (6.75, 73, .25), (7, 69, .5)], .82) + [(3.45, HORN, .55, .95), (7.45, HORN, .55, .95)]
BOARD = (tn([(0, 72, .75), (.75, 74, .25), (1, 77, 1), (2, 81, .75), (2.75, 80, .25), (3, 81, 1),
             (4, 79, .75), (4.75, 77, .25), (5, 76, .5), (5.5, 74, .5), (6, 76, .75), (6.75, 79, .25), (7, 84, 1)], .9) +
         [(b, ch, .45, .8) for b, ch in ((0, (57, 60, 65)), (1.5, (57, 60, 65)), (2.5, (57, 60, 65)), (4, (55, 60, 64)),
                                         (5.5, (55, 60, 64)), (6.5, (55, 58, 64)))])
HAMM = tn([(0, 73, .1), (.1, (65, 70, 74), .9), (1, (67, 70, 75), .5), (1.5, (65, 70, 74), .5), (2, 77, .25), (2.25, 75, .25),
           (2.5, 74, .25), (2.75, 72, .25), (3, 70, 1), (4, 68, .1), (4.1, (65, 69, 72), .9), (5, (65, 70, 74), .5),
           (5.5, (65, 69, 72), .5), (6, 72, .25), (6.25, 74, .25), (6.5, 75, .25), (6.75, 76, .25), (7, 77, 1)], .85)
PROG_A = [62, 65, 69, 62, 65, 69, 62, 67, 62, 65, 70, 62, 65, 70, 62, 69]
PROG_B = [61, 64, 69, 61, 64, 69, 61, 67, 61, 64, 69, 61, 64, 69, 74, 73]
SYD = [(k*.25 + 4*h, (m - 12, m), .23, .95 if k in (0, 3, 6, 8, 11, 14) else .72)
       for h, row in enumerate((PROG_A, PROG_B)) for k, m in enumerate(row)]
ELB = ([(k*.5, (62, 65, 70, 65)[k % 4], .45, .6) for k in range(8)] + [(4 + k*.5, (62, 67, 70, 67)[k % 4], .45, .6) for k in range(8)] +
       tn([(0, 77, 4), (4, 79, 2), (6, 74, 2)], .75) + tn([(0, 46, 4), (4, 43, 4)], .6))

SCENES = [
 dict(key="cage", theme="cage", name="HALBERSTADT CAGE ORGAN", credit="ORGAN TECHNO", size=.56, notes=SHOW),
 dict(key="hyd", theme="hyd", year=-250, ys="250 BC", name="HYDRAULIS", credit="ANCIENT GREEK MODE", size=.76,
      fact=("WATER KEEPS ITS", "AIR PRESSURE EVEN"), notes=HYD),
 dict(key="port", theme="port", year=1330, ys="1300s", name="PORTATIVE ORGAN", credit="MEDIEVAL ESTAMPIE", size=1.75,
      fact=("ONE HAND PLAYS,", "ONE HAND PUMPS"), notes=PORT),
 dict(key="halb", theme="halb", year=1361, ys="1361", name="HALBERSTADT ORGAN", credit="GOTHIC ORGANUM", size=.74,
      fact=("FIRST PERMANENT", "ORGAN ON RECORD"), notes=HALB),
 dict(key="val", theme="val", year=1435, ys="1435", name="VALÈRE ORGAN, SION", credit="RENAISSANCE MOTET", size=.6,
      fact=("OLDEST ORGAN", "STILL PLAYABLE"), notes=VAL),
 dict(key="bach", theme="bach", year=1703, ys="1703", name="BACH'S ARNSTADT ORGAN", credit="BAROQUE TOCCATA", size=.74,
      fact=("BACH, AGED 18,", "HIRED TO PLAY IT"), notes=BACH),
 dict(key="sulp", theme="sulp", year=1862, ys="1862", name="SAINT-SULPICE ORGAN", credit="FRENCH ORGAN TOCCATA", size=.58,
      fact=("WIDOR PLAYS IT", "FOR 63 YEARS"), notes=SULP),
 dict(key="wurl", theme="wurl", year=1925, ys="1920s", name="MIGHTY WURLITZER", credit="SILENT-FILM CHASE", size=.8,
      fact=("CAR HORNS, DRUMS", "FOR SILENT FILMS"), notes=WURL),
 dict(key="board", theme="board", year=1932, ys="1932", name="BOARDWALK HALL ORGAN", credit="ART DECO SHOWTUNE", size=.72,
      fact=("33,112 PIPES:", "LOUDEST EVER BUILT"), notes=BOARD),
 dict(key="hamm", theme="hamm", year=1935, ys="1935", name="HAMMOND ORGAN", credit="GOSPEL", size=1.12,
      fact=("50,000 CHURCHES", "HAD ONE BY 1966"), notes=HAMM),
 dict(key="syd", theme="syd", year=1979, ys="1979", name="SYDNEY OPERA HOUSE", credit="70s PROG ROCK", size=.56,
      fact=("LARGEST TRACKER", "ORGAN EVER BUILT"), notes=SYD),
 dict(key="elb", theme="elb", year=2016, ys="2016", name="ELBPHILHARMONIE ORGAN", credit="CONTEMPORARY CLASSICAL", size=.6,
      fact=("4,765 PIPES,", "MOSTLY HIDDEN"), notes=ELB),
 dict(key="cage", theme="cage", year=2020, ys="2020s", name="HALBERSTADT CAGE ORGAN", credit="ORGAN TECHNO", size=.56,
      fact=("SANDBAGS HOLD", "THE NOTES DOWN"), notes=LAST),
]
TRIM = {}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers (metres; x right, y up, z toward the viewer)
TIN, BRONZE, PEWTER, INKC = (204, 208, 218), (222, 164, 92), (164, 164, 172), (34, 26, 22)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)

def beam(g, p0, p1, w, col, h=None, spec=0., emit=False):
    """square-section beam between two points"""
    p0, p1 = np.asarray(p0, float), np.asarray(p1, float); d = unit(p1 - p0)
    a = np.array([0, 1., 0]) if abs(d[1]) < .9 else np.array([0, 0, 1.])
    u = unit(np.cross(d, a)); v = np.cross(u, d); hw, hh = w/2, (h or w)/2
    cs = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
    A = [p0 + cu*u + cv*v for cu, cv in cs]; B = [p1 + cu*u + cv*v for cu, cv in cs]; mid = (p0 + p1)/2
    for k in range(4): g.face([A[k], A[(k + 1) % 4], B[(k + 1) % 4], B[k]], col, ref=mid, spec=spec, emit=emit)
    g.face(A, col, ref=mid, spec=spec, emit=emit); g.face(B, col, ref=mid, spec=spec, emit=emit)

def quad(g, a, b, c, d, col, ref=None, emit=False, two=False, spec=0.):
    pts = [np.asarray(p, float) for p in (a, b, c, d)]
    g.face(pts, col, ref=ref, emit=emit, two=two, spec=spec)

def frect(g, x0, x1, y0, y1, z, col, emit=False, spec=0.):
    """flat rectangle facing +z"""
    g.face([(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)], col, ref=((x0 + x1)/2, (y0 + y1)/2, z - 1), emit=emit, spec=spec)

def pipe(g, x, z, y0, y1, r, col=TIN, glow=0., n=6, foot=None, spec=.55, mouth=True):
    """flue pipe standing at (x, z): conical foot, body, mouth on the front, open top"""
    fl = foot if foot is not None else min(.3*(y1 - y0), 7*r)
    yb = y0 + fl
    T.tube(g, (x, y0, z), (x, yb, z), r*.25, r, col=dim(col, .82), n=n, spec=spec)
    body = mix(col, HOT, .5*glow) if glow > .05 else col
    T.tube(g, (x, yb, z), (x, y1, z), r, col=body, n=n, spec=spec)
    if mouth:
        ym, h, zf = yb + .15*r, max(.8*r, .008), z + r*1.02
        g.face([(x - .5*r, ym, zf), (x + .5*r, ym, zf), (x + .5*r, ym + h, zf), (x - .5*r, ym + h, zf)],
               mix((26, 20, 26), HOT, glow), ref=(x, ym, z - 1), emit=glow > .05)
    g.face(S.ring((x, y1 + .001, z), (0, 1, 0), r*.8, n), (18, 14, 18), ref=(x, y1 - 1, z))
    return (x, y1, z)

def put(g, sub, yaw=0., off=(0, 0, 0)):
    """copy a sub-model into g, turned by yaw degrees and moved by off"""
    a = math.radians(yaw); c, s_ = math.cos(a), math.sin(a)
    R = np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]]); o = np.asarray(off, float)
    for Pp, n, col, two, emit, gid, spec in sub.F: g.F.append((Pp @ R.T + o, R @ n, col, two, emit, gid, spec))
    for a_, b_, col, gid in sub.L: g.L.append((a_ @ R.T + o, b_ @ R.T + o, col, gid))
    return R, o

def note_glow(ev, tail=.3):
    out = {}
    for age, m, v, dur, tp, cs in ev:
        if m == HORN: continue
        gl = 1. if age < dur else max(0., 1 - (age - dur)/tail)
        if gl > out.get(m, 0): out[m] = gl
    return out

def prange(s):
    ps = [e[1] for e in s["ev"] if e[1] != HORN]
    return min(ps), max(ps)

def slot_glow(ev, s, n):
    lo, hi = prange(s); out = np.zeros(n)
    for m, gl in note_glow(ev).items():
        k = int(round((m - lo)/max(1, hi - lo)*(n - 1))); out[k] = max(out[k], gl)
    return out

def tops_fx(fx, ev, s, tops, cap=4):
    lo, hi = prange(s); n = len(tops); cand = []
    for age, m, v, dur, tp, cs in ev:
        if m == HORN or not tp or v < .6 or age > .9: continue
        k = int(round((m - lo)/max(1, hi - lo)*(n - 1)))
        cand.append((age, tuple(np.asarray(tops[k]) + np.array([0, .04, 0]))))
    for age, pos in sorted(cand)[:cap]: fx.append(("note", pos, age))

def manual(g, x0, x1, zf, y, depth, nwhite, lit=(), nat=(236, 230, 214), sharp=(28, 24, 26), low=36, spec=0.):
    """cheap keyboard: one slab for the naturals, single raised quads for the sharps; lit = set of white-key indices to glow"""
    kw = (x1 - x0)/nwhite; zb = zf - depth
    g.box(x0, x1, y - .012, y, zb, zf, nat)
    for i in range(1, nwhite): g.line((x0 + i*kw, y + .001, zb), (x0 + i*kw, y + .001, zf), mix(nat, (0, 0, 0), .45))
    for i in lit:
        if 0 <= i < nwhite:
            xa = x0 + i*kw
            g.face([(xa + .1*kw, y + .002, zb), (xa + .9*kw, y + .002, zb), (xa + .9*kw, y + .002, zf), (xa + .1*kw, y + .002, zf)],
                   HOT, ref=(xa, y - 1, zf), emit=True)
    for i in range(nwhite):
        if (i % 7) in (0, 1, 3, 4, 5):
            xc = x0 + (i + 1)*kw; w = .3*kw
            if xc >= x1 - .1*kw: continue
            g.face([(xc - w, y + .008, zb), (xc + w, y + .008, zb), (xc + w, y + .008, zb + depth*.62), (xc - w, y + .008, zb + depth*.62)],
                   sharp, ref=(xc, y - 1, zb), spec=spec)
            g.face([(xc - w, y, zb + depth*.62), (xc + w, y, zb + depth*.62), (xc + w, y + .008, zb + depth*.62), (xc - w, y + .008, zb + depth*.62)],
                   dim(sharp, .8), ref=(xc, y, zb - 1))
    return kw

WHITE_STEP = {0: 0, 2: 1, 4: 2, 5: 3, 7: 4, 9: 5, 11: 6, 1: 0, 3: 1, 6: 3, 8: 4, 10: 5}
def white_idx(m, low=36):
    return (m//12 - low//12)*7 + WHITE_STEP[m % 12] - WHITE_STEP[low % 12]

def lit_keys(ev, low, nwhite):
    out = set()
    for m, gl in note_glow(ev, .08).items():
        if gl > .5: out.add(white_idx(m, low) % nwhite)
    return out

# ---------------------------------------------------------------- 0 / 12: the Halberstadt Cage organ (St. Burchardi; acrylic box since 2011)
OAK, OAK_D, PLINTH, BAG = (206, 132, 62), (164, 98, 44), (232, 212, 146), (238, 234, 222)
def cage(g, t, ev, fx, s):
    y0 = .55
    g.box(-.6, .6, 0, y0, -.4, .4, PLINTH)                                            # plinth
    g.box(-.7, .7, y0, y0 + .03, -.3, .3, OAK_D)                                       # base board
    W = .045
    def post(x, z, ya, yb): beam(g, (x, ya, z), (x, yb, z), W, OAK)
    ytop = y0 + 2.0
    for sx in (-1, 1):                                                                # the two towers
        xa, xb = sorted((sx*.32, sx*.64))
        for x in (xa, xb):
            for z in (-.15, .15): post(x, z, y0 + .03, ytop - (.0 if sx < 0 else .05))
        for k, yy in enumerate((y0 + .62, y0 + .82, ytop - .32, ytop - .2, ytop - .08) if sx > 0 else (y0 + .62, y0 + .82, ytop - .25, ytop - .06)):
            yy = min(yy, ytop - (.0 if sx < 0 else .05) - .02)
            for z in (-.15, .15): beam(g, (xa, yy, z), (xb, yy, z), W*.8, OAK)
            for x in (xa, xb): beam(g, (x, yy, -.15), (x, yy, .15), W*.8, OAK)
        beam(g, (xb, y0 + .5, .15), (sx*.18, y0 + .12, .15), W*.8, OAK)              # V struts under the chest
        beam(g, (xb, y0 + .5, -.15), (sx*.18, y0 + .12, -.15), W*.8, OAK)
    g.box(-.64, .64, y0 + .62, y0 + .8, -.12, .12, OAK)                                # wind chest with three drawers
    for k in range(3):
        xc = -.4 + .4*k; frect(g, xc - .14, xc + .14, y0 + .66, y0 + .77, .121, OAK_D)
        g.box(xc - .02, xc + .02, y0 + .7, y0 + .73, .121, .14, (70, 44, 24))
    for x in (-.18, .18): post(x, .0, y0 + .8, y0 + 1.36)                              # the gabled middle
    beam(g, (-.18, y0 + 1.36, 0), (0, y0 + 1.62, 0), W*.9, OAK); beam(g, (.18, y0 + 1.36, 0), (0, y0 + 1.62, 0), W*.9, OAK)
    beam(g, (-.18, y0 + 1.36, 0), (.18, y0 + 1.36, 0), W*.7, OAK)
    for x in (-.18, .18): post(x, .0, y0 + .03, y0 + .62)
    gl = slot_glow(ev, s, 5)
    tops = [pipe(g, -.8, 0, y0 + .03, y0 + 1.95, .045, glow=gl[0], foot=.32),
            pipe(g, -.48, 0, y0 + .8, y0 + 1.72, .028, glow=gl[1], foot=.18),
            pipe(g, 0, 0, y0 + .8, y0 + 1.42, .018, glow=gl[2], foot=.12),
            pipe(g, .48, 0, y0 + .8, y0 + 1.86, .028, glow=gl[3], foot=.18),
            pipe(g, .8, 0, y0 + .03, y0 + 1.9, .045, glow=gl[4], foot=.32)]
    tops_fx(fx, ev, s, tops)
    # the little keyboard at the bottom, held down by sandbags
    g.box(-.2, .2, y0 + .16, y0 + .2, -.02, .16, OAK_D)
    act = {k for k, v in enumerate(gl) if v > .5}
    for k in range(8):
        x = -.18 + .045*k + .0225; down = k in (1, 3, 4, 6)
        g.box(x - .019, x + .019, y0 + .2 - (.012 if down else 0), y0 + .225 - (.012 if down else 0), .02, .17,
              mix((240, 236, 224), HOT, .7) if (down and act) else (240, 236, 224))
    hop = .012*math.exp(-((t % BEAT_S)/BEAT_S)*5)
    for k, x in enumerate((-.12, -.02, .03, .12)):
        yb = y0 + .215 + (hop if k % 2 else 0)
        T.tube(g, (x, yb, .1), (x, yb + .07, .1), .034, .026, col=BAG, n=6, spec=.05)
        T.tube(g, (x, yb + .07, .1), (x, yb + .1, .1), .026, .008, col=BAG, n=6, spec=.05)
        T.tube(g, (x, yb + .1, .1), (x, yb + .12, .1), .008, .012, col=dim(BAG, .8), n=4)
    # acrylic box: edges, plus a couple of glints sliding across the front pane
    ac = (190, 236, 248); bx0, bx1, by0, by1, bz0, bz1 = -.93, .93, y0, y0 + 2.1, -.36, .36
    for a, b in (((bx0, by0, bz0), (bx1, by0, bz0)), ((bx0, by1, bz0), (bx1, by1, bz0)), ((bx0, by0, bz1), (bx1, by0, bz1)),
                 ((bx0, by1, bz1), (bx1, by1, bz1)), ((bx0, by0, bz0), (bx0, by1, bz0)), ((bx1, by0, bz0), (bx1, by1, bz0)),
                 ((bx0, by0, bz1), (bx0, by1, bz1)), ((bx1, by0, bz1), (bx1, by1, bz1)), ((bx0, by0, bz0), (bx0, by0, bz1)),
                 ((bx1, by0, bz0), (bx1, by0, bz1)), ((bx0, by1, bz0), (bx0, by1, bz1)), ((bx1, by1, bz0), (bx1, by1, bz1))):
        g.line(a, b, ac)
    for k in range(2):
        u = ((t*.35 + .5*k) % 1.4) - .2
        xa = bx0 + (bx1 - bx0)*u
        g.line((xa, by0 + .3, bz1 + .002), (xa + .35, by0 + 1.0, bz1 + .002), (226, 248, 255))
        g.line((xa + .05, by0 + .3, bz1 + .002), (xa + .4, by0 + 1.0, bz1 + .002), (170, 210, 226))
    for x, z in ((bx0, bz0), (bx1, bz0), (bx0, bz1), (bx1, bz1)):
        g.box(x - .012, x + .012, by1 - .012, by1 + .012, z - .012, z + .012, ac, spec=.6)

# ---------------------------------------------------------------- 250 BC: hydraulis (Dion / Aquincum reconstructions)
PINE, PINE_D, WATER = (206, 166, 104), (168, 126, 72), (70, 206, 214)
def octo(r, ph=math.pi/8, n=8): return [(r*math.cos(ph + 2*math.pi*k/n), r*math.sin(ph + 2*math.pi*k/n)) for k in range(n)]
def _hydraulis(g, t, ev, fx, s):
    P.prism(g, octo(.5), 0, .08, PINE_D, PINE)
    P.prism(g, octo(.42), .08, .9, PINE, PINE)
    P.prism(g, octo(.47), .9, .98, PINE_D, mix(PINE, (255, 255, 255), .1))
    pts = octo(.42)
    for k in range(8):                                                                # raised panels on each face
        (xa, za), (xb, zb) = pts[k], pts[(k + 1) % 8]
        mx_, mz = (xa + xb)/2, (za + zb)/2
        if mz > .3: continue                                                          # the front face is the water window
        for (u0, v0), (u1, v1) in (((.15, .2), (.85, .2)), ((.85, .2), (.85, .82)), ((.85, .82), (.15, .82)), ((.15, .82), (.15, .2))):
            g.line((lerp(xa, xb, u0)*1.01, v0, lerp(za, zb, u0)*1.01), (lerp(xa, xb, u1)*1.01, v1, lerp(za, zb, u1)*1.01), PINE_D)
    # front window into the cistern: water, and the bronze bell (pnigeus) standing in it
    zf = .42*math.cos(math.pi/8) + .003; hw = .42*math.sin(math.pi/8) - .02
    frect(g, -hw, hw, .16, .84, zf, (30, 22, 16))
    gl = note_glow(ev); lvl = .1*max(gl.values(), default=0)
    wy = .44 + .03*math.sin(t*7) + lvl*.3
    frect(g, -hw, hw, .16, wy, zf + .002, mix(dim(WATER, .65), WATER, lvl*3), emit=True)
    for k in range(3):
        y = wy - .05 - .09*k; ph = t*5 + k
        g.line((-hw + .01, y, zf + .004), (hw - .01, y + .006*math.sin(ph), zf + .004), mix(WATER, (255, 255, 255), .4))
    bell = [(-.09, .2), (.09, .2), (.09, .5), (.06, .58), (0, .61), (-.06, .58), (-.09, .5)]
    g.face([(x, y, zf + .006) for x, y in bell], BRONZE, ref=(0, .4, 0), spec=.5)
    T.tube(g, (0, .61, zf + .006), (0, .84, zf + .006), .012, col=BRONZE, n=4)
    # wind chest, keys, one rank of bronze pipes
    g.box(-.42, .42, .98, 1.13, -.16, .12, (156, 100, 52))
    for k in range(9): g.line((-.4 + .1*k, 1.0, .121), (-.4 + .1*k, 1.11, .121), (220, 178, 90))
    act = slot_glow(ev, s, 18); tops = []
    for k in range(18):
        x = -.36 + k*.72/17; h = .1 + .92*(1 - k/17)**1.7
        tops.append(pipe(g, x, -.03, 1.13, 1.13 + h, .017, col=BRONZE, glow=act[k], foot=.05, n=5, spec=.8))
        press = act[k] > .5
        g.box(x - .015, x + .015, 1.0, 1.035, .12, .15 if press else .19, HOT if press else (224, 196, 140), emit=press)
    g.box(-.42, .42, 1.3, 1.33, -.07, -.04, (156, 100, 52))
    tops_fx(fx, ev, s, tops)
    # two pumps, one on each side: bronze cylinders, pistons and rocking levers
    ph = (t/BEAT_S) % 2
    for sx in (-1, 1):
        x = sx*.64
        g.box(x - .1, x + .1, 0, .28, -.1, .1, PINE_D)
        T.tube(g, (x, .28, 0), (x, .82, 0), .07, col=BRONZE, n=8, spec=.7)
        T.tube(g, (x, .3, 0), (sx*.4, .3, 0), .02, col=BRONZE, n=5, spec=.5)                         # air pipe into the cistern
        beam(g, (x + sx*.12, .28, 0), (x + sx*.12, 1.0, 0), .04, PINE_D)                            # lever post
        k = .5 + .5*math.cos(math.pi*(ph + (0 if sx < 0 else 1)))
        ang = math.radians(-14 + 28*k)*sx
        piv = np.array([x + sx*.12, 1.0, 0])
        arm_in = piv + np.array([-sx*.2*math.cos(ang), -.2*math.sin(ang)*sx, 0])
        arm_out = piv + np.array([sx*.36*math.cos(ang), .36*math.sin(ang)*sx, 0])
        beam(g, arm_in, arm_out, .035, PINE)
        T.tube(g, (x, .82, 0), (arm_in[0] + sx*.0, arm_in[1], 0), .012, col=(120, 120, 128), n=4)
        T.tube(g, arm_out, arm_out + np.array([0, .1, 0]), .018, col=PINE_D, n=5)

def hydraulis(g, t, ev, fx, s):
    sub = P.Geo(); _hydraulis(sub, t, ev, fx, s)
    for Pp, n, col, two, emit, gid, spec in sub.F:                    # squash the cistern a little (keeps the pipes in frame)
        Q = Pp.copy(); Q[:, 1] = np.where(Q[:, 1] <= .98, Q[:, 1]*.84, Q[:, 1] - .157); g.F.append((Q, n, col, two, emit, gid, spec))
    for a, b, col, gid in sub.L:
        a2, b2 = a.copy(), b.copy()
        for q in (a2, b2): q[1] = q[1]*.84 if q[1] <= .98 else q[1] - .157
        g.L.append((a2, b2, col, gid))
    for i, f in enumerate(fx):
        if f[0] == "note": fx[i] = (f[0], (f[1][0], f[1][1] - .157, f[1][2])) + tuple(f[2:])

# ---------------------------------------------------------------- 1300s: portative organ (strap, button keys, bellows on the back)
WALNUT, WALNUT_D, LEATHER = (178, 112, 60), (120, 70, 36), (176, 62, 40)
def _portative(g, t, ev, fx, s):
    g.box(-.2, .2, 0, .09, -.07, .07, WALNUT)
    act = slot_glow(ev, s, 15); tops = []
    for k in range(15):
        x = -.18 + k*.36/14; h = .1 + .5*(1 - k/14)**1.25
        tops.append(pipe(g, x, 0, .09, .09 + h, .011, glow=act[k], foot=.035, n=5))
    beam(g, (-.205, .0, -.06), (-.205, .74, -.06), .022, WALNUT_D); beam(g, (-.205, .0, .06), (-.205, .74, .06), .022, WALNUT_D)
    g.box(-.215, -.195, .09, .74, -.06, .06, WALNUT)
    g.box(.195, .215, .09, .25, -.06, .06, WALNUT)
    prev = None
    for k in range(9):                                                               # curved top rail following the pipe tops
        u = k/8; x = lerp(-.205, .205, u); y = .09 + .1 + .5*(1 - u)**1.25 + .04
        if prev is not None: beam(g, (prev[0], prev[1], 0), (x, y, 0), .025, WALNUT, h=.06)
        prev = (x, y)
    lit = {k for k in range(15) if act[k] > .5}
    for k in range(9):                                                               # two rows of button keys
        x = -.17 + k*.34/8; on = any(abs(j*15/9 - k*1.6) < 1 for j in lit) and k in {int(j*9/15) for j in lit}
        g.box(x - .013, x + .013, .055, .075 - (.006 if on else 0), .07, .1, HOT if on else (232, 214, 170), emit=on)
    for k in range(5):
        x = -.15 + k*.3/4
        g.box(x - .011, x + .011, .075, .095, .055, .075, (60, 36, 20))
    tops_fx(fx, ev, s, tops)
    # wedge bellows on the left end, hinged at the bottom, pumped by the left hand
    pump = .5 + .5*math.sin(2*math.pi*t/(2*BEAT_S))
    ang = math.radians(6 + 24*pump)
    xb, h, dz, yb0 = -.23, .36, .12, .03
    g.box(xb - .014, xb, yb0, yb0 + h, -dz, dz, WALNUT_D)                               # fixed board against the case
    hinge = np.array([xb - .014, yb0, 0.]); up = np.array([-math.sin(ang), math.cos(ang), 0.]); top = hinge + up*h
    beam(g, hinge - up*0 + np.array([-.007*math.cos(ang), -.007*math.sin(ang), 0]), top + np.array([-.007*math.cos(ang), -.007*math.sin(ang), 0]),
         .014, WALNUT, h=2*dz)
    ftop = np.array([xb - .014, yb0 + h, 0.])
    for z in (-dz, dz):                                                               # leather gussets with folds
        zz = np.array([0, 0, z])
        g.face([hinge + zz, ftop + zz, top + zz], LEATHER, ref=hinge + zz - np.array([0, 0, z*3]), two=True)
        for f in (.25, .5, .75): g.line(hinge + zz*1.01, ftop + (top - ftop)*f + zz*1.01, dim(LEATHER, .55))
    quad(g, ftop + np.array([0, 0, -dz]), ftop + np.array([0, 0, dz]), top + np.array([0, 0, dz]), top + np.array([0, 0, -dz]), LEATHER,
         ref=hinge, two=True)
    out_ = np.array([-math.cos(ang), -math.sin(ang), 0])
    T.tube(g, top - up*.08 + out_*.03, top - up*.2 + out_*.03, .01, col=WALNUT_D, n=4)
    for yy in (.08, .2): beam(g, top - up*yy, top - up*yy + out_*.03, .012, WALNUT_D)
    # the carrying strap, lying on the pedestal
    pts = [(-.21, .62, 0), (-.3, .34, .04), (-.34, .03, .12), (-.12, .01, .22), (.18, .01, .2), (.3, .06, .08), (.215, .2, 0)]
    for a, b in zip(pts, pts[1:]): beam(g, a, b, .02, (112, 64, 34), h=.007)
def portative(g, t, ev, fx, s):
    sub = P.Geo(); fx2 = []
    _portative(sub, t, ev, fx2, s)
    R, o = put(g, sub, 32.)
    for f in fx2: fx.append((f[0], tuple(np.asarray(f[1]) @ R.T + o)) + tuple(f[2:]))

# ---------------------------------------------------------------- 1361: Halberstadt cathedral keyboards (Praetorius, Syntagma Musicum II, plate XXV)
WOODCUT, WOODCUT_D = (226, 206, 160), (176, 150, 108)
def _key_lines(g, x0, x1, y, z0, z1):
    for a, b in (((x0, y, z0), (x1, y, z0)), ((x1, y, z0), (x1, y, z1)), ((x1, y, z1), (x0, y, z1)), ((x0, y, z1), (x0, y, z0))):
        g.line((a[0], a[1] + .001, a[2]), (b[0], b[1] + .001, b[2]), INKC)
def halberstadt(g, t, ev, fx, s):
    sub = P.Geo(); fx2 = []; _halberstadt(sub, t, ev, fx2, s); R, o = put(g, sub, 0., (0, 0, -.22))
    for f in fx2: fx.append((f[0], tuple(np.asarray(f[1]) + o)) + tuple(f[2:]))
def _halberstadt(g, t, ev, fx, s):
    gl = note_glow(ev, .1)
    def lit(ms): return max([gl.get(m, 0) for m in ms] + [0])
    steps = {0: 0, 2: 1, 3: 2, 5: 3, 7: 4, 8: 5, 9: 5, 10: 6, 11: 6, 1: 0, 4: 2, 6: 3}
    def deg(m): return (m//12)*7 + steps[m % 12]
    # side cheeks: a stepped wooden frame, hatched like the woodcut
    prof = [(1.0, 0), (1.0, .12), (.5, .4), (.12, .62), (.12, .9), (-.42, 1.02), (-.42, 0)]
    for sx in (-1, 1):
        x0 = sx*.78 - .03; x1 = sx*.78 + .03
        P.prism(g, [(x0, -.42), (x1, -.42), (x1, 1.0), (x0, 1.0)], 0, .02, WOODCUT_D, WOODCUT_D)
        pts = [(sx*.78, y, z) for z, y in prof]
        g.face(pts, WOODCUT_D, ref=(sx*.78 - sx, .4, .3), two=True)
        for k in range(7): g.line((sx*.781, .08 + .12*k, -.4), (sx*.781, .08 + .12*k, min(.95, .5 - .7*(.08 + .12*k - .4))), INKC)
    # IV. pedal clavier: long bars on the floor, tall blocks for the raised keys
    g.box(-.76, .76, 0, .05, .4, 1.0, WOODCUT_D)
    for k in range(9):
        x = -.64 + k*.16; m_set = [m for m in gl if m < 52 and deg(m) % 9 == k]
        on = lit(m_set) > .5; dy = -.02 if on else 0
        g.box(x - .045, x + .045, .05 + dy, .09 + dy, .44, .98, HOT if on else WOODCUT, emit=on); _key_lines(g, x - .045, x + .045, .09 + dy, .44, .98)
        if k % 2: g.box(x + .045, x + .115, .05, .2, .44, .54, WOODCUT); _key_lines(g, x + .045, x + .115, .2, .44, .54)
    # III. clavier: long planks with raised blocks
    g.box(-.76, .76, .36, .42, .1, .5, WOODCUT_D)
    for k in range(9):
        x = -.64 + k*.16; m_set = [m for m in gl if 52 <= m < 62 and deg(m) % 9 == k]
        on = lit(m_set) > .5; dy = -.02 if on else 0
        g.box(x - .055, x + .055, .42 + dy, .46 + dy, .14, .5, HOT if on else WOODCUT, emit=on); _key_lines(g, x - .055, x + .055, .46 + dy, .14, .5)
        if k in (1, 2, 4, 5, 6): g.box(x + .03, x + .1, .46, .52, .14, .24, WOODCUT); _key_lines(g, x + .03, x + .1, .52, .14, .24)
    # I. and II. discant claviers: short, round-ended keys like fists
    g.box(-.76, .76, .64, .7, -.36, .12, WOODCUT_D)
    for row, (zf, zb_) in enumerate(((.12, -.1), (-.12, -.34))):
        yy = .7 + .05*row
        if row: g.box(-.76, .76, .7, .75, -.36, -.12, WOODCUT_D)
        for k in range(10):
            x = -.66 + k*.147; m_set = [m for m in gl if m >= 62 and deg(m) % 10 == k] if row == 0 else []
            on = lit(m_set) > .5; dy = -.02 if on else 0
            col = HOT if on else WOODCUT
            P.prism(g, [(x - .05, zb_), (x + .05, zb_), (x + .05, zf - .04), (x + .03, zf), (x - .03, zf), (x - .05, zf - .04)],
                    yy + dy, yy + .06 + dy, dim(col, .85), col, emit=on)
            _key_lines(g, x - .05, x + .05, yy + .06 + dy, zb_, zf - .04)
            if k in (1, 3, 5, 6, 8): g.box(x + .04, x + .1, yy + .06, yy + .12, zb_, zb_ + .1, WOODCUT)
    # the great Blockwerk behind: a screen of big pipes under a Gothic arch
    act = slot_glow(ev, s, 9); tops = []
    for k in range(9):
        x = -.64 + k*.16; h = 1.0 - .35*abs(k - 4)/4
        tops.append(pipe(g, x, -.52, .78, .78 + h, .05, col=PEWTER, glow=act[k], foot=.18))
    g.box(-.78, .78, .7, .8, -.6, -.44, WOODCUT_D)
    for sx in (-1, 1):
        beam(g, (sx*.76, .8, -.52), (sx*.76, 1.6, -.52), .05, WOODCUT_D)
        beam(g, (sx*.76, 1.6, -.52), (sx*.3, 1.9, -.52), .05, WOODCUT_D)
    beam(g, (-.3, 1.9, -.52), (.3, 1.9, -.52), .05, WOODCUT_D)
    tops_fx(fx, ev, s, tops)

# ---------------------------------------------------------------- 1435: the Valere swallow's-nest organ, Sion
STONE, CASE, CASE_D, FRESCO, GILT = (196, 192, 182), (150, 72, 42), (104, 46, 26), (186, 52, 38), (226, 176, 72)
def valere(g, t, ev, fx, s):
    sub = P.Geo(); fx2 = []; _valere(sub, t, ev, fx2, s); R, o = put(g, sub, 0., (0, -.6, 0))
    for f in fx2: fx.append((f[0], tuple(np.asarray(f[1]) + o)) + tuple(f[2:]))
def _valere(g, t, ev, fx, s):
    g.box(-.62, .62, .6, 3.0, -.42, -.24, STONE)                                        # the church wall
    for k in range(3, 12):
        y = .25*k + .25; g.line((-.62, y, -.239), (.62, y, -.239), dim(STONE, .7))
        for j in range(3):
            x = -.42 + .42*j + (.21 if k % 2 else 0); g.line((x, y - .25, -.239), (x, y, -.239), dim(STONE, .7))
    plan = [(-.52, -.24), (-.48, .02), (-.24, .2), (.24, .2), (.48, .02), (.52, -.24)]
    P.prism(g, plan, 1.45, 1.86, (196, 112, 54), (210, 132, 70))                        # the nest: a polygonal balcony...
    for (xa, za), (xb, zb) in zip(plan, plan[1:]):
        for f in (.25, .5, .75):
            g.line((lerp(xa, xb, f)*1.01, 1.48, lerp(za, zb, f)*1.01 + .002), (lerp(xa, xb, f)*1.01, 1.83, lerp(za, zb, f)*1.01 + .002), (120, 60, 26))
    mid = [(x*.62, z*.62 - .08) for x, z in plan]
    for k in range(len(plan) - 1):                                                     # ...narrowing to a point below
        (xa, za), (xb, zb) = plan[k], plan[k + 1]; (xc, zc), (xd, zd) = mid[k], mid[k + 1]
        quad(g, (xa, 1.45, za), (xb, 1.45, zb), (xd, 1.12, zd), (xc, 1.12, zc), (190, 104, 48), ref=(0, 1.3, -.4))
        g.face([(xc, 1.12, zc), (xd, 1.12, zd), (0, .8, -.15)], (170, 88, 40), ref=(0, 1.1, -.4))
    S.prism(g, (0, .76, -.15), (0, 1, 0), .035, .08, 6, GILT, GILT)
    # the Gothic case
    g.box(-.42, .42, 1.86, 2.04, -.2, .12, CASE)
    for k in range(10):
        x = -.38 + .084*k; g.line((x, 1.9, .121), (x + .04, 2.0, .121), GILT); g.line((x + .04, 2.0, .121), (x + .08, 1.9, .121), GILT)
    g.box(-.42, .42, 2.04, 2.6, -.2, -.12, CASE_D)
    gl = slot_glow(ev, s, 17); tops = []; k = 0
    for i in range(5):                                                                # left flat
        x = -.38 + i*.045; tops.append(pipe(g, x, -.05, 2.04, 2.56, .019, glow=gl[k], foot=.1)); k += 1
    for i in range(7):                                                                # middle flat, rising to the gable
        x = -.135 + i*.045; h = .42 + .16*(1 - abs(i - 3)/3); tops.append(pipe(g, x, -.02, 2.04, 2.04 + h, .019, glow=gl[k], foot=.1)); k += 1
    for i in range(5):
        x = .2 + i*.045; tops.append(pipe(g, x, -.05, 2.04, 2.56, .019, glow=gl[k], foot=.1)); k += 1
    tops_fx(fx, ev, s, tops)
    for x in (-.42, -.16, .16, .42): beam(g, (x, 2.04, .02), (x, 2.82 if abs(x) > .3 else 2.62, .02), .04, CASE)
    beam(g, (-.16, 2.5, .03), (0, 2.82, .03), .04, CASE); beam(g, (.16, 2.5, .03), (0, 2.82, .03), .04, CASE)
    S.prism(g, (0, 2.9, .03), (0, 1, 0), .03, .14, 4, GILT, GILT)
    for sx in (-1, 1):                                                                 # side towers with battlements
        x0, x1 = sorted((sx*.2, sx*.44))
        g.box(x0, x1, 2.58, 2.66, -.2, .06, CASE)
        for j in range(4): g.box(x0 + j*.065, x0 + j*.065 + .035, 2.66, 2.72, -.06, .06, CASE)
        for j in range(3): g.line((x0 + .02 + j*.08, 2.58, .061), (x0 + .06 + j*.08, 2.5, .061), GILT)
    # the painted wings, opened outward
    for sx, robe in ((-1, (64, 112, 86)), (1, (176, 44, 34))):
        hx = sx*.42; ang = math.radians(150)
        dx, dz = sx*math.cos(math.pi - ang)*-1, math.sin(math.pi - ang)
        wv = .4
        p0 = np.array([hx, 1.98, .12]); p1 = p0 + np.array([sx*wv*.92, 0, wv*.38])
        A, B, C, Dd = p0, p1, p1 + np.array([0, .8, 0]), p0 + np.array([0, .8, 0])
        g.face([A, B, C, Dd], CASE_D, ref=p0 + np.array([0, .4, -1]) , two=False)
        n_ = unit(np.cross(B - A, Dd - A))
        if n_[2] < 0: n_ = -n_
        off = n_*.004
        g.face([A + off, B + off, C + off, Dd + off], (232, 214, 172), ref=p0 + np.array([0, .4, 0]) - n_)
        # the saint: robe, head and halo painted on the panel
        def L(u, v): return A + (B - A)*u + (Dd - A)*v + off*2
        g.face([L(.2, .06), L(.8, .06), L(.66, .62), L(.34, .62)], robe, ref=L(.5, .3) - n_)
        g.face([L(.5 + .2*math.cos(a), .76 + .14*math.sin(a)) for a in np.linspace(0, 2*math.pi, 9)[:-1]], GILT, ref=L(.5, .7) - n_)
        g.face([L(.5 + .11*math.cos(a), .75 + .08*math.sin(a)) for a in np.linspace(0, 2*math.pi, 7)[:-1]], (236, 196, 160), ref=L(.5, .7) - n_)
        g.face([L(0, .95), L(1, .95), L(1, 1), L(0, 1)], FRESCO, ref=L(.5, .9) - n_)

# ---------------------------------------------------------------- 1703: Bach's Wender organ, Arnstadt (white and gold, third gallery)
BWHITE, BGOLD, KNOB = (238, 234, 224), (228, 182, 68), (132, 62, 38)
def balustrade(g, x0, x1, y0, y1, z, col, rail, n=16):
    g.box(x0, x1, y1 - .04, y1, z - .05, z + .02, rail)
    g.box(x0, x1, y0, y0 + .04, z - .05, z + .02, col)
    for k in range(n):
        x = lerp(x0, x1, (k + .5)/n); w = (x1 - x0)/n*.28
        g.face([(x - w, y0 + .04, z), (x + w, y0 + .04, z), (x + w*.6, y1 - .04, z), (x - w*.6, y1 - .04, z)], col, ref=(x, y0, z - 1))
def bach(g, t, ev, fx, s):
    for i, (y0, y1, zf) in enumerate(((0, .2, .44), (.2, .4, .32), (.4, .6, .2))):       # three gallery tiers; the organ is on the third
        g.box(-.86, .86, y0, y1 - .1, -.3, zf - .06, mix(BWHITE, (0, 0, 0), .08))
        balustrade(g, -.86, .86, y1 - .12, y1, zf, BWHITE, BGOLD, 18)
    sub = P.Geo(); fx2 = []; _bach_case(sub, t, ev, fx2, s); R, o = put(g, sub, 0., (0, -.3, 0))
    for f in fx2: fx.append((f[0], tuple(np.asarray(f[1]) + o)) + tuple(f[2:]))
def _bach_case(g, t, ev, fx, s):
    g.box(-.64, .64, .9, 1.32, -.34, .1, BWHITE)                                        # lower case with the console
    frect(g, -.2, .2, .96, 1.3, .101, (60, 44, 34))
    gl = note_glow(ev, .1)
    for row, y in enumerate((1.03, 1.1)):
        lit = {white_idx(m, 36) % 28 for m, v in gl.items() if v > .5 and ((m >= 62) == bool(row))}
        manual(g, -.18, .18, .12 + .0 - .05*row, y, .08, 28, lit)
    g.face([(-.16, 1.16, .03), (.16, 1.16, .03), (.16, 1.28, .0), (-.16, 1.28, .0)], BWHITE, ref=(0, 1.2, -1))
    for sx in (-1, 1):                                                                  # brown stop knobs on both jambs
        for col_ in range(2):
            for r in range(5 + (sx > 0 and col_ == 1)):
                x = sx*(.25 + .065*col_); y = 1.0 + .056*r
                S.prism(g, (x, y, .125), (0, 0, 1), .018, .05, 6, mix(KNOB, (255, 230, 200), .15), KNOB)
    # upper case: round middle tower, two flats, two pointed outer towers
    secs = [(-.62, -.42, 2.3, 5, 'tow'), (-.42, -.16, 2.0, 6, 'flat'), (-.16, .16, 2.42, 7, 'mid'), (.16, .42, 2.0, 6, 'flat'), (.42, .62, 2.3, 5, 'tow')]
    npipe = sum(sc[3] for sc in secs); act = slot_glow(ev, s, npipe); k = 0; tops = []
    for x0, x1, top, n, kind in secs:
        zc = .02 if kind == 'mid' else -.04 if kind == 'tow' else -.12
        g.box(x0, x1, 1.32, top - .2, -.34, zc - .06, BWHITE)
        for i in range(n):
            u = (i + .5)/n; x = lerp(x0 + .02, x1 - .02, u)
            z = zc + (.05*math.sin(math.pi*u) if kind != 'flat' else 0)
            tops.append(pipe(g, x, z, 1.34, top - .32 - (.06*abs(u - .5) if kind != 'flat' else 0), .022, glow=act[k], foot=.12)); k += 1
        g.face([(x0, top - .32, zc + .07), (x1, top - .32, zc + .07), (x1, top - .2, zc + .07), (x0, top - .2, zc + .07)],
               BGOLD, ref=((x0 + x1)/2, top - .26, zc - 1), spec=.5)                                          # carved pipe shades
        for j in range(4): g.line((lerp(x0, x1, j/4), top - .32, zc + .072), (lerp(x0, x1, (j + 1)/4), top - .2, zc + .072), (150, 100, 30))
        g.box(x0 - .02, x1 + .02, top - .2, top - .12, -.36, zc + .09, BWHITE)
        g.line((x0 - .02, top - .12, zc + .091), (x1 + .02, top - .12, zc + .091), BGOLD)
        if kind != 'flat':
            S.prism(g, ((x0 + x1)/2, top - .06, zc), (0, 1, 0), .035, .12, 6, BGOLD, BGOLD)
    tops_fx(fx, ev, s, tops)
    for sx in (-1, 1):                                                                 # gilded carved "ears"
        x = sx*.64
        g.face([(x, 1.4, -.1), (x + sx*.16, 1.55, -.1), (x + sx*.12, 1.8, -.1), (x + sx*.2, 2.0, -.1), (x, 2.1, -.1)], BGOLD, ref=(x - sx, 1.7, -.1), two=True, spec=.5)

# ---------------------------------------------------------------- 1862: Saint-Sulpice, Paris (Chalgrin case, statues, Cavaille-Coll organ)
SOAK, SOAK_D, SCREAM = (96, 60, 38), (64, 38, 24), (216, 206, 184)
def statue(g, x, y, z, col):
    T.tube(g, (x, y, z), (x, y + .2, z), .068, .045, col=col, n=6, spec=.15)
    T.tube(g, (x, y + .2, z), (x, y + .25, z), .045, .03, col=col, n=6, spec=.15)
    S.prism(g, (x, y + .285, z), (0, 1, 0), .034, .065, 6, mix(col, (255, 255, 255), .1), col)
    beam(g, (x + .035, y + .2, z + .02), (x + .07, y + .27, z + .05), .03, col)
def sulpice(g, t, ev, fx, s):
    for x in (-.9, -.45, 0, .45, .9):                                                  # the tribune: columns and balustrade
        g.box(x - .07, x + .07, 0, .05, .12, .26, SCREAM)
        T.tube(g, (x, .05, .19), (x, .46, .19), .05, .045, col=SCREAM, n=8, spec=.1)
        g.box(x - .07, x + .07, .46, .52, .12, .26, SCREAM)
    g.box(-1.06, 1.06, .52, .6, -.3, .3, SCREAM)
    balustrade(g, -1.06, 1.06, .6, .78, .3, SCREAM, SCREAM, 22)
    g.box(-1.02, 1.02, .6, 1.0, -.34, .02, SOAK_D)
    gl = slot_glow(ev, s, 34); k = 0; tops = []
    for x0, x1, n, top in ((-1.0, -.62, 5, 1.86), (-.62, -.2, 6, 1.94), (-.2, .2, 6, 1.98), (.2, .62, 6, 1.94), (.62, 1.0, 5, 1.86)):
        frect(g, x0, x1, 1.0, 2.0, -.2, (30, 20, 16))
        for i in range(n):
            u = (i + .5)/n; x = lerp(x0 + .05, x1 - .05, u)
            tops.append(pipe(g, x, -.14, 1.04, top - .05*abs(u - .5), .028, glow=gl[k], foot=.16)); k += 1
    for x in (-1.0, -.62, -.2, .2, .62, 1.0):                                            # big columns with statues in front
        g.box(x - .06, x + .06, 1.0, 1.08, -.12, .02, SOAK)
        T.tube(g, (x, 1.08, -.05), (x, 1.96, -.05), .045, col=SOAK, n=8, spec=.2)
        g.box(x - .065, x + .065, 1.96, 2.04, -.12, .02, SOAK)
        g.box(x - .06, x + .06, 1.12, 1.16, .0, .12, SOAK_D)
        statue(g, x, 1.16, .07, (156, 116, 80))
    g.box(-1.1, 1.1, 2.02, 2.16, -.36, .06, SOAK)                                      # entablature and cornice
    g.box(-1.14, 1.14, 2.16, 2.2, -.38, .1, SOAK_D)
    for k2 in range(22): g.line((-1.08 + .1*k2, 2.04, .061), (-1.08 + .1*k2, 2.14, .061), SOAK_D)
    for cx, sc in ((0, 1.), (-.98, .62), (.98, .62)):                                   # sculpture groups on top: angels with spread wings
        g.box(cx - .24*sc, cx + .24*sc, 2.2, 2.3, -.2, .02, SOAK)
        S.prism(g, (cx, 2.36, -.08), (0, 1, 0), .14*sc, .12, 6, (118, 80, 54), (104, 70, 46))
        statue(g, cx, 2.42, -.06, (124, 86, 58))
        for sx in (-1, 1):
            g.face([(cx + sx*.03, 2.6*1 + .0, -.08), (cx + sx*.3*sc, 2.74, -.1), (cx + sx*.26*sc, 2.6, -.1), (cx + sx*.2*sc, 2.5, -.09)],
                   (134, 94, 62), ref=(cx, 2.6, -1), two=True)
    g.box(-.3, .3, .6, 1.0, .02, .3, SOAK)                                              # the positif, with the clock above
    for i in range(7):
        x = -.24 + .08*i; pipe(g, x, .26, .66, .9 - .05*abs(i - 3), .02, glow=gl[min(33, 12 + i*2)], foot=.08)
    g.box(-.32, .32, .98, 1.04, .0, .32, SOAK_D)
    S.prism(g, (0, 1.2, .2), (0, 0, 1), .13, .04, 12, (196, 160, 80), (150, 110, 50))
    S.prism(g, (0, 1.2, .225), (0, 0, 1), .105, .01, 12, (238, 232, 216), (238, 232, 216))
    ang = -t*1.2; g.line((0, 1.2, .232), (.08*math.sin(ang), 1.2 + .08*math.cos(ang), .232), (20, 16, 14))
    g.line((0, 1.2, .232), (.05*math.sin(ang/12), 1.2 + .05*math.cos(ang/12), .232), (20, 16, 14))
    for sx in (-1, 1): statue(g, sx*.22, 1.04, .22, (156, 116, 80))
    tops_fx(fx, ev, s, tops)

# ---------------------------------------------------------------- 1920s: Mighty Wurlitzer (horseshoe console + toy counter: drum and car horn)
IVORY, WGOLD = (234, 218, 182), (214, 168, 76)
TAB_COLS = [(214, 40, 40), (242, 238, 226), (30, 26, 24), (232, 156, 40)]
def wurlitzer(g, t, ev, fx, s):
    g.box(-.55, .55, 0, .05, .12, .5, (40, 30, 26))                                     # pedalboard
    for k in range(12): g.box(-.5 + k*.09, -.46 + k*.09, .05, .07, .14, .5, (236, 228, 210))
    g.box(-.6, .6, .05, .76, -.36, .12, IVORY)                                          # console body
    for x0, x1 in ((-.55, -.2), (-.17, .17), (.2, .55)):
        for a_, b_ in (((x0, .16), (x1, .16)), ((x1, .16), (x1, .7)), ((x1, .7), (x0, .7)), ((x0, .7), (x0, .16))):
            g.line((a_[0], a_[1], .121), (b_[0], b_[1], .121), WGOLD)
    gl = note_glow(ev, .08)
    for i in range(3):
        y = .82 + .05*i; zf = .16 - .085*i
        lit = {white_idx(m, 36) % 30 for m, v in gl.items() if v > .5 and i == (2 if m >= 76 else 1 if m >= 70 else 0)}
        manual(g, -.42, .42, zf, y, .08, 30, lit)
        g.box(-.46, -.42, y - .04, y + .01, zf - .085, zf, IVORY); g.box(.42, .46, y - .04, y + .01, zf - .085, zf, IVORY)
    # the horseshoe of stop tabs, leaning back toward the player
    def hs(x, y, zc=-.18): return (x, y, zc - (y - .86)*.75)
    pts = [hs(.78*math.cos(a), .86 + .5*math.sin(a)) for a in np.linspace(0, math.pi, 15)]
    inner = [hs(.44*math.cos(a), .86 + .16*math.sin(a)) for a in np.linspace(0, math.pi, 15)]
    for j in range(14):
        quad(g, inner[j], pts[j], pts[j + 1], inner[j + 1], IVORY if j % 2 else mix(IVORY, WGOLD, .25), ref=(0, .5, 1.5))
    g.line(pts[0], pts[-1], WGOLD)
    for j in range(14): g.line(pts[j], pts[j + 1], WGOLD)
    bt = int(t/BEAT_S)
    for r in range(3):
        rx, ry = .5 + .085*r, .22 + .085*r; n = 20 + 3*r
        for j in range(n):
            a = math.pi*(.03 + .94*j/(n - 1)); x, y = rx*math.cos(a), .86 + ry*math.sin(a)
            col = TAB_COLS[(j + r) % 4]
            if (j*7 + r*3 + bt) % 23 == 0: col = mix(col, HOT, .6)
            c0 = hs(x - .016, y - .022, -.17); c1 = hs(x + .016, y - .022, -.17); c2 = hs(x + .016, y + .022, -.17); c3 = hs(x - .016, y + .022, -.17)
            g.face([c0, c1, c2, c3], col, ref=(x, y, -1.5))
    g.face([(-.2, .9, -.2), (.2, .9, -.2)] + [(.2*math.cos(a), 1.08 + .12*math.sin(a), -.32) for a in np.linspace(0, math.pi, 9)],
           (150, 100, 50), ref=(0, 1.0, -1), spec=.3)                                                       # arched music desk
    for k in range(4): g.line((-.15 + .1*k, .9, -.198), (-.15 + .1*k, 1.14, -.318), (220, 180, 100))
    for sx in (-1, 1):                                                                   # painted side wings with scroll tops
        x0, x1 = sorted((sx*.6, sx*.76))
        g.box(x0, x1, .05, 1.06, -.36, .1, IVORY)
        g.face([(x0, 1.06, .101), (x1, 1.06, .101), (x1 if sx > 0 else x0, 1.2, .101)], WGOLD, ref=(sx*.68, 1.1, -1))
        frect(g, x0 + .025, x1 - .025, .4, .98, .102, (96, 150, 176))
        g.face([((x0 + x1)/2 - .03, .45, .104), ((x0 + x1)/2 + .03, .45, .104), ((x0 + x1)/2 + .015, .78, .104), ((x0 + x1)/2 - .015, .78, .104)],
               (232, 170, 150), ref=(0, .6, -1))
        S.prism(g, ((x0 + x1)/2, .84, .104), (0, 0, 1), .03, .01, 6, (232, 170, 150), (232, 170, 150))
    # toy counter: bass drum on the left, car horn on a stand on the right
    hit = 0.
    for T0, k_, v_ in DRUMS:
        if k_ == "kick":
            a = s["t0"] + t - T0
            if 0 <= a < .25: hit = max(hit, 1 - a/.25)
    S.drum(g, (-.96, .3, .12), (0, 0, 1), .28, .2, (176, 30, 36), (236, 226, 200), hit=hit)
    beam(g, (-.96, 0, .0), (-.96, .04, .0), .2, (60, 50, 40))
    honk = max([1 - (age - dur)/.2 if age > dur else 1. for age, m, v, dur, tp, cs in ev if m == HORN and age < dur + .2] + [0])
    T.tube(g, (.95, 0, .05), (.95, .62, .05), .015, col=(60, 60, 66), n=4)
    T.tube(g, (.86, .66, .05), (1.02, .66, .05), .06, col=(40, 40, 44), n=8, spec=.5)
    mouth = S.bell_dir(g, (.86, .66, .05), (-1, .25, .55), .3, .03, .11 + .02*honk, mix((214, 214, 222), HOT, honk*.6), segs=5)
    for age, m, v, dur, tp, cs in ev:
        if m == HORN and age < dur: fx.append(("wave", tuple(mouth), age, 0)); fx.append(("wavel", tuple(mouth), age))
    tops_fx(fx, ev, s, [(x, 1.36, -.5) for x in np.linspace(-.5, .5, 7)])

# ---------------------------------------------------------------- 1932: Boardwalk Hall, Atlantic City (the 7-manual console in its round booth)
MAHOG, MAHOG_D, RED_P, TAB = (82, 42, 28), (52, 26, 18), (176, 32, 38), (234, 228, 208)
def boardwalk(g, t, ev, fx, s):
    R = .86; n = 14
    angs = np.linspace(math.pi, 2*math.pi, n + 1)
    for k in range(n):                                                                  # back half of the round booth
        a0, a1 = angs[k], angs[k + 1]
        p = lambda a, y: (R*math.cos(a), y, R*math.sin(a)*.8)
        quad(g, p(a0, 0), p(a1, 0), p(a1, 1.86), p(a0, 1.86), MAHOG, ref=(0, .9, 0), two=True)
        quad(g, p(a0, 1.2), p(a1, 1.2), p(a1, 1.52), p(a0, 1.52), RED_P, ref=(0, 1.3, -5), two=True)
        g.line(p(a0, 1.2), p(a0, 1.52), (206, 164, 70))
        g.line(p(a0, 1.62), p(a1, 1.78), (206, 164, 70)); g.line(p(a0, 1.78), p(a1, 1.62), (206, 164, 70))
    for k in range(n + 1):
        a = angs[k]
        if k % 2 == 0:                                                                  # flame lamps on the rim
            x, z = R*math.cos(a), R*math.sin(a)*.8
            T.tube(g, (x, 1.86, z), (x, 1.92, z), .025, .03, col=(206, 164, 70), n=5, spec=.6)
            fl = .7 + .3*math.sin(t*9 + k)
            T.tube(g, (x, 1.92, z), (x, 2.04 + .02*fl, z), .028, .004, col=mix((255, 150, 60), (255, 236, 170), fl), n=5)
            g.F[-5:] = [(f[0], f[1], f[2], f[3], True, f[5], f[6]) for f in g.F[-5:]]
    for sx in (-1, 1): beam(g, (sx*R, 0, 0), (sx*R, 1.9, 0), .1, MAHOG_D)
    g.box(-.56, .56, 0, .6, -.3, .2, MAHOG)                                            # console
    g.box(-.5, .5, 0, .04, .2, .58, MAHOG_D)
    for k in range(13): g.box(-.46 + k*.075, -.43 + k*.075, .04, .06, .22, .58, (230, 220, 200))
    gl = note_glow(ev, .08); lo, hi = prange(s)
    for i in range(7):
        y = .64 + .052*i; zf = .2 - .062*i
        lit = {white_idx(m, 36) % 30 for m, v in gl.items() if v > .5 and int((m - lo)/max(1, hi - lo + 1)*7) == i}
        manual(g, -.4, .4, zf, y, .07, 30, lit)
        g.box(-.44, -.4, y - .04, y + .005, zf - .07, zf, MAHOG); g.box(.4, .44, y - .04, y + .005, zf - .07, zf, MAHOG)
    bt = int(t/BEAT_S)
    for sx in (-1, 1):                                                                 # two great curved jambs of stop tabs
        cols_ = 7
        for j in range(cols_):
            a0, a1 = math.radians(8 + j*11), math.radians(8 + (j + 1)*11)
            px = lambda a: sx*(.44 + .42*math.sin(a)); pz = lambda a: -.3 + .42*(1 - math.cos(a))*.9 + .05
            quad(g, (px(a0), .6, pz(a0) - .02), (px(a1), .6, pz(a1) - .02), (px(a1), 1.55, pz(a1) - .14), (px(a0), 1.55, pz(a0) - .14),
                 MAHOG_D, ref=(sx*1.6, 1., -1.))
            for r in range(12):
                am = (a0 + a1)/2; xx = px(am); y = .64 + .075*r; zz = pz(am) - .015 - .12*(y - .6)/.95
                col = TAB if (r + j) % 6 else ((40, 30, 26) if r % 2 else (196, 40, 40))
                if (r*7 + j*3 + bt*5) % 31 == 0: col = HOT
                tx, tz = math.cos(am)*.022, math.sin(am)*.022*sx
                g.face([(xx - tx, y, zz + tz*sx*0 - .0 + tz), (xx + tx, y, zz - tz), (xx + tx, y + .052, zz - tz - .006), (xx - tx, y + .052, zz + tz - .006)],
                       col, ref=(sx*1.6, y, -1.), emit=col == HOT)
    g.box(-.42, .42, 1.06, 1.3, -.3, -.24, MAHOG_D)                                    # music rack with a gilded grille
    for k in range(9):
        g.line((-.4 + .1*k, 1.08, -.239), (-.35 + .1*k, 1.28, -.239), (206, 164, 70)); g.line((-.3 + .1*k, 1.08, -.239), (-.35 + .1*k, 1.28, -.239), (206, 164, 70))
    tops_fx(fx, ev, s, [(x, 1.36, -.24) for x in np.linspace(-.4, .4, 7)])

# ---------------------------------------------------------------- 1935: Hammond Model A (two 61-key manuals, 25 pedals, drawbars; no pipes)
HWAL, HWAL_D = (124, 70, 36), (86, 46, 22)
DB_COL = [(110, 60, 30), (110, 60, 30), (240, 236, 226), (240, 236, 226), (24, 22, 22), (240, 236, 226), (24, 22, 22), (24, 22, 22), (240, 236, 226)]
def turned_leg(g, x, z, y0, y1):
    T.tube(g, (x, y0, z), (x, y0 + .12, z), .028, .02, col=HWAL_D, n=6)
    T.tube(g, (x, y0 + .12, z), (x, y0 + .28, z), .02, .036, col=HWAL, n=6, spec=.3)
    T.tube(g, (x, y0 + .28, z), (x, y1 - .1, z), .036, .022, col=HWAL, n=6, spec=.3)
    T.tube(g, (x, y1 - .1, z), (x, y1, z), .034, .034, col=HWAL_D, n=6)
def hammond(g, t, ev, fx, s):
    sub = P.Geo(); fx2 = []; _hammond(sub, t, ev, fx2, s); R, o = put(g, sub, 0., (0, 0, -.12))
    for f in fx2: fx.append((f[0], tuple(np.asarray(f[1]) + o)) + tuple(f[2:]))
def _hammond(g, t, ev, fx, s):
    g.box(-.62, .62, 0, .05, -.3, .54, HWAL_D)                                          # base frame + 25-note pedalboard
    gl = note_glow(ev, .08)
    for k in range(15):
        x = -.49 + k*.07
        g.box(x - .025, x + .025, .05, .075, .12, .52, (176, 120, 70))
    for k in range(15):
        if k % 7 in (2, 6) or k == 14: continue
        x = -.49 + k*.07 + .035; g.box(x - .014, x + .014, .075, .1, .12, .28, (36, 26, 20))
    for x in (-.58, .58):
        for z in (-.26, .08): turned_leg(g, x, z, .05, .62)
    g.box(-.62, .62, .62, .88, -.32, .1, HWAL)                                          # cabinet
    g.line((-.62, .7, .101), (.62, .7, .101), HWAL_D)
    for sx in (-1, 1):                                                                 # key cheeks
        x0, x1 = sorted((sx*.6, sx*.53))
        P.prism(g, [(x0, -.32), (x1, -.32), (x1, .2), (x0, .2)], .88, 1.0, HWAL, HWAL)
    lit_lo = {white_idx(m, 36) for m, v in gl.items() if v > .5 and m < 68}
    lit_hi = {white_idx(m, 36) for m, v in gl.items() if v > .5 and m >= 68}
    for row, (y, zf, lit) in enumerate(((.9, .2, lit_lo), (.97, .08, lit_hi))):
        manual(g, -.33, .53, zf, y, .11, 36, lit)                                     # 61 playing keys, waterfall fronts
        manual(g, -.53, -.33, zf, y, .11, 7, (), nat=(30, 26, 24), sharp=(236, 230, 214))   # reverse-coloured preset keys
        g.box(-.53, .53, y - .03, y - .012, zf, zf + .012, (236, 230, 214))
    g.box(-.53, .53, .97, 1.06, -.3, -.04, HWAL)                                        # back rail with the drawbars
    bar_i = int(t/BAR_S)
    regs = ["888000000", "888400000", "868000000", "888800000"]
    for gi, x0 in enumerate((-.42, -.23, .07, .26)):
        reg = regs[(gi + bar_i) % 4]
        for j in range(9):
            x = x0 + j*.018; out_ = int(reg[j])/8*.06
            g.box(x - .006, x + .006, 1.06, 1.075, -.04 - .0, -.04 + out_ + .015, DB_COL[j])
    for j in range(2): g.box(-.02 + .024*j, -.008 + .024*j, 1.06, 1.075, -.04, .0, (110, 60, 30))
    g.face([(-.4, 1.08, -.06), (.4, 1.08, -.06), (.4, 1.32, -.16), (-.4, 1.32, -.16)], HWAL, ref=(0, 1.2, -1))      # music rack
    g.box(-.42, .42, 1.06, 1.09, -.1, -.04, HWAL_D)
    tops_fx(fx, ev, s, [(x, 1.12, .0) for x in np.linspace(-.3, .5, 7)])

# ---------------------------------------------------------------- 1979: Sydney Opera House Grand Organ (timber case, plain pipe towers, acrylic rings)
TIMBER, TIMBER_D, RECESS = (216, 172, 118), (174, 128, 82), (46, 34, 30)
def sydney(g, t, ev, fx, s):
    g.box(-1.12, 1.12, 0, .48, -.34, .12, TIMBER)                                       # timber platform wall
    for k in range(10): g.line((-1.1 + .22*k, .02, .121), (-1.1 + .22*k, .46, .121), TIMBER_D)
    flats = [(-1.08, -.82, .5, 2.68, 4), (-.82, -.56, 1.0, 1.8, 5), (-.56, -.3, 1.0, 2.32, 5), (-.3, .3, .96, 2.02, 9),
             (.3, .56, 1.0, 2.32, 5), (.56, .82, 1.0, 1.8, 5), (.82, 1.08, .5, 2.68, 4)]
    npipe = sum(f[4] for f in flats); act = slot_glow(ev, s, npipe); k = 0; tops = []
    for fi, (x0, x1, yb, top, n) in enumerate(flats):
        g.box(x0, x1, yb - .06, top + .04, -.32, -.2, RECESS)
        g.box(x0, x1, yb - .08, yb, -.2, .02, TIMBER)
        for i in range(n):
            u = (i + .5)/n; x = lerp(x0 + .03, x1 - .03, u)
            tt = top - (.22*abs(u - .5) if fi == 3 else (.1*u if x0 < 0 else .1*(1 - u)) if n > 4 else 0)
            tops.append(pipe(g, x, -.1, yb, tt, .028 if n > 4 else .036, glow=act[k], foot=.2)); k += 1
    for x in (-1.08, -.82, -.56, -.3, .3, .56, .82, 1.08):                               # timber mullions
        beam(g, (x, .48, -.06), (x, 2.0 if abs(x) < .9 else 2.76, -.06), .05, TIMBER_D)
    g.box(-.34, .34, .46, .86, .05, .4, TIMBER)                                          # the Rueckpositiv box with the console below it
    for i in range(9): pipe(g, -.28 + .07*i, .34, .86, .86 + .26 - .08*abs(i - 4)/4, .018, glow=act[min(npipe - 1, 13 + i)], foot=.06)
    g.box(-.3, .3, .2, .46, .1, .38, (64, 46, 36))
    manual(g, -.26, .26, .39, .3, .08, 30, ()); manual(g, -.26, .26, .33, .36, .07, 30, ())
    tops_fx(fx, ev, s, tops)

# ---------------------------------------------------------------- 2016: Elbphilharmonie (pipes rising between the white terraces of the Grand Hall)
GYPSUM, SEAT, ELB_WOOD = (236, 234, 228), (42, 50, 76), (186, 146, 104)
def elbphil(g, t, ev, fx, s):
    g.box(-1.0, 1.0, 1.1, 2.42, -.62, -.5, GYPSUM)                                      # the "white skin" wall
    for r in range(9):
        for c_ in range(10):
            x = -.92 + .2*c_ + (.1 if r % 2 else 0); y = 1.18 + .14*r
            if x > .95: continue
            g.face([(x - .045, y, -.499), (x + .045, y, -.499), (x + .03, y + .06, -.499), (x - .03, y + .06, -.499)], (204, 202, 194), ref=(x, y, -1))
    groups = [(-.78, -.46, .62, 2.25, 8, -.2), (.16, .5, .94, 2.36, 8, -.36), (-.22, .06, 1.26, 2.12, 6, -.44), (.68, .94, .3, 1.96, 6, .1)]
    npipe = sum(gr[4] for gr in groups); act = slot_glow(ev, s, npipe); k = 0; tops = []
    for x0, x1, yb, top, n, z in groups:
        for i in range(n):
            u = (i + .5)/n; x = lerp(x0, x1, u); tp = top - .35*abs(u - .45) - .1*((i*7) % 3)
            tops.append(pipe(g, x, z, yb, tp, .02, glow=act[k], foot=.12, n=5)); k += 1
    tops_fx(fx, ev, s, tops)
    levels = [(0, .3, .5, (-1.0, 1.0)), (.3, .62, .24, (-1.0, .62)), (.62, .94, -.02, (-.96, .98)), (.94, 1.26, -.28, (-.94, .7))]
    for li, (y0, y1, zf, (xa, xb)) in enumerate(levels):                                 # wavy terraces with rows of seats
        xs = np.linspace(xa, xb, 11); zfr = [zf + .07*math.sin(3.2*x + li*1.7) for x in xs]
        for j in range(10):
            quad(g, (xs[j], y0, zfr[j]), (xs[j + 1], y0, zfr[j + 1]), (xs[j + 1], y1 + .1, zfr[j + 1]), (xs[j], y1 + .1, zfr[j]), GYPSUM, ref=(0, y0, -5))
            quad(g, (xs[j], y1 + .1, zfr[j]), (xs[j + 1], y1 + .1, zfr[j + 1]), (xs[j + 1], y1 + .1, zfr[j + 1] - .04), (xs[j], y1 + .1, zfr[j] - .04),
                 mix(GYPSUM, (255, 255, 255), .1), ref=(0, 0, 0))
            quad(g, (xs[j], y1, zfr[j] - .04), (xs[j + 1], y1, zfr[j + 1] - .04), (xs[j + 1], y1, zf - .3), (xs[j], y1, zf - .3), ELB_WOOD, ref=(0, 0, zf - .15))
        for row in range(2):
            zz = zf - .1 - .1*row
            for j in range(9):
                x = lerp(xa + .08, xb - .08, (j + .5)/9); z2 = zz + .07*math.sin(3.2*x + li*1.7)
                g.box(x - .035, x + .035, y1, y1 + .06 + .03*row, z2 - .04, z2, SEAT)

BUILD = {"cage": cage, "hyd": hydraulis, "port": portative, "halb": halberstadt, "val": valere, "bach": bach, "sulp": sulpice,
         "wurl": wurlitzer, "board": boardwalk, "hamm": hammond, "syd": sydney, "elb": elbphil}
def build(g, t, ev, fx, s): BUILD[s["key"]](g, t, ev, fx, s)

# ---------------------------------------------------------------- sound: every scene in that organ's own voice
def _t(n): return np.arange(n)/SR
HMAX = SR*.42
PRIN = (1, .55, .32, .2, .14, .1, .07, .05, .035, .025, .018, .012)
FLUTE = (1, .26, .09, .045, .02, .01)
STOPPED = (1, .03, .2, .02, .06, .01, .02)
REED = (1, .95, .86, .78, .7, .63, .56, .5, .45, .4, .36, .32, .28, .25, .22, .2, .18, .16, .14, .12)
STRING = (1, .72, .6, .54, .48, .43, .39, .35, .32, .29, .26, .24, .22, .2)
TIBIA = (1, .1, .04, .015)
_bp_cache = {}
def bpf(x, lo, hi, order=2):
    lo, hi = max(20., lo), min(SR*.45, hi)
    key = (round(lo), round(hi), order)
    if key not in _bp_cache: _bp_cache[key] = butter(order, [lo/(SR/2), hi/(SR/2)], "band")
    return lfilter(*_bp_cache[key], x)
def lpf(x, fc):
    key = ("lp", round(fc))
    if key not in _bp_cache: _bp_cache[key] = butter(2, min(fc, SR*.45)/(SR/2))
    return lfilter(*_bp_cache[key], x)

def organ(f, dur, v, ranks, att=.03, rel=.12, chiff=0., wind=0., trem=None, vib=None, seed=0, drift=0., t_abs=0.):
    """additive pipe-organ voice. ranks: (spectrum, pitch multiple, level, detune in cents)"""
    n = int((dur + 5*rel + .03)*SR); t = _t(n)
    fr = np.full(n, f)
    if vib: fr = fr*(1 + vib[1]*np.sin(2*np.pi*vib[0]*(t + t_abs)))
    if drift: fr = fr*(1 + drift*np.sin(2*np.pi*.7*t + seed))
    ph = 2*np.pi*np.cumsum(fr)/SR
    x = np.zeros(n)
    for spec, mult, amp, cents in ranks:
        d = mult*2**(cents/1200)
        for k, a in enumerate(spec, 1):
            if k*d*f > HMAX: break
            x += amp*a*np.sin(k*d*ph + .7*k)
    e = np.where(t < att, .5 - .5*np.cos(np.pi*t/att), 1.)*np.where(t < dur, 1., np.exp(-(t - dur)/rel))
    x *= e
    rng = np.random.default_rng(seed)
    if chiff:
        nz = rng.normal(0, 1, n); k = int(.06*SR)
        c = bpf(nz[:k], min(2.5*f, 7000), min(5.5*f, 12000))*np.exp(-t[:k]/.014)
        x[:k] += chiff*c*3
    if wind:
        nz = rng.normal(0, 1, n)
        x += wind*bpf(nz, min(f*.8, 2000), min(f*6, 9000))*e*2.2
    if trem: x *= 1 - trem[1]*(.5 + .5*np.sin(2*np.pi*trem[0]*(t + t_abs)))
    return x*v

def v_cage(f, dur, v, seed):           # the ASLSP organ: wheezy, piercing, beating ranks, lots of wind
    long_ = dur > 1
    x = organ(f, dur, v, [(PRIN, 1, 1., 0), (STRING, 1, .45, 7), (PRIN, 2, .35, -4)], att=.25 if long_ else .012,
              rel=.35 if long_ else .06, chiff=0 if long_ else .25, wind=.22, seed=seed)
    return x*(.55 if long_ else 1.)
def v_hyd(f, dur, v, seed):            # breathy bronze pipes, the water pressure gives a slight waver
    return organ(f, dur, v, [(FLUTE, 1, 1., 0), (PRIN, 2, .12, 0)], att=.06, rel=.1, chiff=.35, wind=.55, seed=seed, drift=.004)
def v_port(f, dur, v, seed):           # a small flute; the hand-pumped bellows make it tremble
    return organ(f, dur, v, [(FLUTE, 1, 1., 0), (FLUTE, 2, .3, 3)], att=.015, rel=.06, chiff=.5, wind=.15, trem=(5.2, .12), seed=seed)
def v_halb(f, dur, v, seed):           # the Blockwerk: every rank at once, rough medieval tuning
    return organ(f, dur, v, [(PRIN, 1, 1., 0), (PRIN, 2, .7, 9), (PRIN, 3, .45, -11), (PRIN, 4, .38, 6), (PRIN, 6, .25, -8),
                             (PRIN, 8, .2, 12), (PRIN, .5, .5, -5)], att=.07, rel=.16, chiff=.2, wind=.2, seed=seed)
def v_val(f, dur, v, seed):            # sweet Gothic principal and octave
    return organ(f, dur, v, [(PRIN, 1, 1., 0), (PRIN, 2, .4, 2), (FLUTE, 4, .12, 0)], att=.03, rel=.14, chiff=.3, wind=.06, seed=seed)
def v_bach(f, dur, v, seed):           # full baroque plenum: 16' to mixture
    return organ(f, dur, v, [(PRIN, .5, .45, 0), (PRIN, 1, 1., 0), (PRIN, 2, .7, 1), (PRIN, 3, .3, -2), (PRIN, 4, .45, 2),
                             (PRIN, 6, .25, 0), (PRIN, 8, .2, -1)], att=.012, rel=.12, chiff=.45, wind=.04, seed=seed)
def v_sulp(f, dur, v, seed):           # Cavaille-Coll reeds: trompette, bombarde, clairon
    return np.tanh(1.3*organ(f, dur, v, [(REED, 1, 1., 0), (REED, .5, .55, 2), (REED, 2, .45, -2), (PRIN, 1, .35, 0)],
                             att=.01, rel=.1, chiff=.1, seed=seed))
def v_wurl(f, dur, v, seed, t_abs):    # tibia clausa + string, deep theatre tremulant
    return organ(f, dur, v, [(TIBIA, 1, 1., 0), (TIBIA, 2, .35, 0), (STRING, 1, .18, 6)], att=.02, rel=.14,
                 trem=(6.8, .38), vib=(6.8, .006), seed=seed, t_abs=t_abs)
def v_horn(dur, v):                    # electric car horn: "ah-OO-ga"
    n = int((dur + .1)*SR); t = _t(n)
    fr = np.where(t < .1, 330 + 900*t, np.where(t < .32, 420, 420 - 140*np.clip((t - .32)/.12, 0, 1)))
    ph = 2*np.pi*np.cumsum(fr)/SR
    x = np.sign(np.sin(ph))*.6 + .4*np.sin(2*ph) + .3*np.sin(3*ph)
    x = bpf(x, 380, 2600)*np.minimum(1, t/.01)*np.where(t < dur, 1., np.exp(-(t - dur)/.03))
    x *= np.where((t > .3) & (t < .33), .4, 1.)
    return np.tanh(2.2*x)*v
def v_board(f, dur, v, seed):          # high-pressure reeds (tuba mirabilis, ophicleide) over a big principal chorus
    return np.tanh(1.8*organ(f, dur, v, [(REED, 1, 1., 0), (REED, 2, .5, 3), (PRIN, 1, .5, 0), (PRIN, 2, .3, 0), (REED, .5, .4, -2)],
                             att=.008, rel=.14, chiff=.05, seed=seed))
def v_hamm(f, dur, v, seed, t_abs):    # tonewheels at 16' 5 1/3' 8' (888000000), key click, through a fast Leslie
    n = int((dur + .25)*SR); t = _t(n); tt = t + t_abs
    fm = 1 + .0035*np.sin(2*np.pi*6.7*tt)
    ph = 2*np.pi*np.cumsum(np.full(n, f)*fm)/SR
    x = sum(np.sin(m_*ph + i) for i, m_ in enumerate((.5, 1.5, 1.)) if m_*f < HMAX) + .25*np.sin(2*ph)
    e = np.minimum(1, t/.004)*np.where(t < dur, 1., np.exp(-(t - dur)/.02))
    x = x*e
    k = int(.004*SR); x[:k] += np.random.default_rng(seed).normal(0, .9, k)*np.linspace(1, 0, k)
    x *= 1 - .3*(.5 + .5*np.sin(2*np.pi*6.7*tt))
    return np.tanh(1.5*x)/1.5*v
def v_syd(f, dur, v, seed):            # tracker full organ: instant speech, mixtures and reeds
    return organ(f, dur, v, [(PRIN, 1, 1., 0), (PRIN, 2, .65, 0), (PRIN, 4, .45, 1), (PRIN, 6, .3, -1), (PRIN, 8, .25, 0),
                             (REED, 1, .45, 2), (PRIN, .5, .4, 0)], att=.006, rel=.09, chiff=.55, wind=.03, seed=seed)
def v_elb(f, dur, v, seed):            # soft flutes with a voix celeste beating against the strings
    return organ(f, dur, v, [(FLUTE, 1, 1., 0), (FLUTE, 2, .35, 0), (STRING, 1, .3, 0), (STRING, 1, .3, 9)],
                 att=.05, rel=.35, chiff=.12, seed=seed)

_NOTE = {}
def synth(s, idx):
    key = (round(s["t0"], 3), idx)
    if key in _NOTE: return _NOTE[key]
    nt, m, v, dur, tp, cs = s["ev"][idx]; k = s["key"]; seed = idx + int(s["t0"]*10)
    if m == HORN: x = v_horn(dur, v)
    else:
        f = midi_hz(m); ta = s["t0"] + nt
        x = {"cage": lambda: v_cage(f, dur, v, seed), "hyd": lambda: v_hyd(f, dur, v, seed), "port": lambda: v_port(f, dur, v, seed),
             "halb": lambda: v_halb(f, dur, v, seed), "val": lambda: v_val(f, dur, v, seed), "bach": lambda: v_bach(f, dur, v, seed),
             "sulp": lambda: v_sulp(f, dur, v, seed), "wurl": lambda: v_wurl(f, dur, v, seed, ta), "board": lambda: v_board(f, dur, v, seed),
             "hamm": lambda: v_hamm(f, dur, v, seed, ta), "syd": lambda: v_syd(f, dur, v, seed), "elb": lambda: v_elb(f, dur, v, seed)}[k]()
    _NOTE[key] = x; return x

# ---------------------------------------------------------------- backing: industrial techno (rumble kick, anvils, closed hats, choir, organ stabs)
_rng = np.random.default_rng(23)
def _kick(v):
    x = D.kick_s(46, 175, 6.5, .4, .42, 3.4, .3); return x*v
def _anvil(v):
    n = int(.6*SR); t = _t(n); x = np.zeros(n)
    for r, d, a in ((1, 11, 1.), (2.31, 14, .7), (2.97, 16, .55), (4.43, 21, .4), (5.71, 26, .3)):
        x += a*np.sin(2*np.pi*1180*r*t + r)*np.exp(-t*d)
    x += .5*np.sin(2*np.pi*395*t)*np.exp(-t*18)
    k = int(.003*SR); x[:k] += _rng.normal(0, 1.5, k)
    return np.tanh(1.6*x)*.55*v
def _hat(v):
    return D.metal_s([3120, 4250, 5530, 6890, 7740, 9100], 70, .35, .07, 7000)*v*.9
def _slam(v):
    n = int(1.4*SR); t = _t(n); f = 34 + 70*np.exp(-t*9)
    x = np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*3.2) + lpf(_rng.normal(0, 1, n), 2400)*np.exp(-t*4.5)*.9
    return np.tanh(2.0*x)*.9*v
_STABS = {}
def _stab(ch, v):
    if ch not in _STABS:
        notes = [n_ + 12 for n_ in CH[ch][1]] + [CH[ch][1][0] + 24]
        x = sum(organ(midi_hz(m), .16, .7, [(PRIN, 1, 1., 0), (PRIN, 2, .6, 0), (REED, 1, .4, 3)], att=.004, rel=.05, chiff=.3, seed=m)
                for m in notes)
        _STABS[ch] = np.tanh(1.4*x)
    return _STABS[ch]*v
KIT = {"kick": _kick, "anvil": _anvil, "hat": _hat, "slam": _slam}
for _ch in CH: KIT["stab_" + _ch] = (lambda v, ch=_ch: _stab(ch, v))
KICK_KINDS, ACCENT_KINDS = ("kick",), ("anvil",)
HAT_V = (.24, .16, .5, .16)
def scene_start_bars(): return {0} | {4 + 2*k for k in range(12)}
def drum_events(c):
    ev = []; s16 = c.BEAT/4; nb = len(BARS); starts = scene_start_bars()
    for b in range(nb):
        T0 = b*c.BAR
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "slam", 1.)]; continue
        ev += [(T0 + k*c.BEAT, "kick", 1. if k == 0 else .92) for k in range(4)]
        ev += [(T0 + p*s16, "hat", HAT_V[p % 4]) for p in range(16)]
        ev += [(T0 + 4*s16, "anvil", .9), (T0 + 12*s16, "anvil", .95)]
        if b in starts: ev.append((T0, "slam", 1. if b == 0 else .72))
        if (b + 1) in starts or b == nb - 2:
            ev += [(T0 + p*s16, "anvil", v) for p, v in ((13, .45), (14, .55), (15, .7))]
            if b >= 3: ev.append((T0 + 14*s16, "stab_" + BARS[b + 1], .75))
    return ev
def bass_events(c):
    out = []; s16 = c.BEAT/4; nb = len(BARS)
    for b in range(nb - 1):
        r = CH[BARS[b]][0]
        for k in range(4): out.append((b*c.BAR + k*c.BEAT + s16, r, 3*s16, .9))
    out.append(((nb - 1)*c.BAR, CH[BARS[-1]][0], 2.4, 1.))
    return out
def bass_voice(m, dur, v):             # the techno rumble: a low sine and sub-bass noise, driven hard, ducked by the kick
    f = midi_hz(m); n = int((dur + .04)*SR); t = _t(n)
    x = .9*np.sin(2*np.pi*f*t) + .5*np.sin(2*np.pi*f*.5*t) + 1.4*lpf(_rng.normal(0, 1, n), 160)
    env = np.minimum(1, t/.03)*np.where(t < dur, 1 - .35*t/max(dur, .01), np.exp(-(t - dur)/.02))
    return np.tanh(2.4*x*env)*v
def pad_voice(notes, dur, v):          # choir "aah": detuned voices through formant filters
    n = int((dur + .6)*SR); t = _t(n); out = np.zeros(n)
    for j, m in enumerate(notes):
        f = midi_hz(m)
        for d in (-.006, 0, .006):
            ph = 2*np.pi*f*(1 + d)*(t + .004*np.sin(2*np.pi*5.1*t + j + d*300)/(2*np.pi*5.1))
            K = int(min(40, 4200/f)); out += sum(np.sin(k*ph)/k for k in range(1, K + 1))
    y = bpf(out, 600, 900)*1.0 + bpf(out, 1000, 1300)*.6 + bpf(out, 2400, 2900)*.3
    env = np.minimum(1, t/.35)*np.where(t < dur, 1., np.exp(-(t - dur)/.35))
    return y*env*v
MIX = dict(drums=-18, bass=-23.5, pad=-30, verb=-21, verb_len=2.3)

# ---------------------------------------------------------------- background: a limestone cavern
THEMES = {   # stone, glow (era palettes: stone colour + glow colour)
 "cage":  ((214, 222, 214), (176, 255, 196)),      # acrylic-clear white + pale green
 "hyd":   ((200, 146, 76), (80, 232, 222)),        # ochre + aqua
 "port":  ((226, 204, 160), (244, 74, 40)),        # parchment + vermilion
 "halb":  ((104, 112, 130), (255, 178, 72)),       # Gothic slate + candle amber
 "val":   ((176, 178, 182), (218, 62, 50)),        # alpine grey + fresco red
 "bach":  ((236, 232, 222), (240, 192, 72)),       # baroque white + gilt
 "sulp":  ((124, 138, 160), (255, 216, 140)),      # Paris blue-grey + gaslight
 "wurl":  ((158, 34, 48), (255, 204, 84)),         # cinema red velvet + marquee gold
 "board": ((44, 138, 138), (226, 236, 246)),       # Art Deco teal + chrome
 "hamm":  ((156, 108, 64), (240, 60, 80)),         # church oak + stained-glass jewels
 "syd":   ((46, 96, 166), (246, 246, 240)),        # harbour blue + sail white
 "elb":   ((228, 226, 218), (150, 214, 255)),      # white limestone + ice blue
}
JEWELS = [(236, 52, 72), (52, 130, 240), (252, 196, 44), (64, 200, 100), (180, 76, 214)]
def ped_cols(th): st, gl = THEMES[th]; return F.ped_cols_from(mix(dim(st, .62), gl, .06))
def mv(a, b, k): return np.asarray(a, float)*(1 - k) + np.asarray(b, float)*k

_r = np.random.default_rng(1361)
def noise2(scale, seed, w=RW, h=RH):
    r = np.random.default_rng(seed); g_ = r.random((h//scale + 3, w//scale + 3))
    im = Image.fromarray((g_*255).astype(np.uint8)).resize(((w//scale + 3)*scale, (h//scale + 3)*scale), Image.BICUBIC)
    return np.asarray(im, float)[:h, :w]/255.
def _smooth_noise(n, scale, seed):
    r = np.random.default_rng(seed); k = max(2, n//scale + 2); pts = r.random(k)
    xs = np.linspace(0, k - 1, n); return np.interp(xs, np.arange(k), pts)
# luminance (0..1) and class per pixel: 0 = far cave, 1 = rock, 2 = stalactite, 3 = floor, 4 = pool
LUM = np.zeros((RH, RW)); CLS = np.zeros((RH, RW), np.int8)
HORIZON = 290
# far cave: brighter toward a distant chamber behind the pedestal
d_far = np.sqrt(((XX - 135)/150.)**2 + ((YY - 250)/190.)**2)
LUM[:] = np.clip(.62 - .55*d_far, .04, .62)
# distant formations on the horizon (irregular, lighter than the near rock)
far = np.zeros((RH, RW), bool)
fh = 34*_smooth_noise(RW, 9, 3) + 12*_smooth_noise(RW, 3, 4)
for x in range(RW):
    top = int(HORIZON - fh[x]); far[top:HORIZON, x] = True
CLS[far] = 1; LUM[far] = .34 + .12*_smooth_noise(RW*RH, 40, 5).reshape(RH, RW)[far]
# floor: wet stone with curved strata
fl = YY >= HORIZON
CLS[fl] = 3
fy = (YY - HORIZON)/(RH - HORIZON)
LUM[fl] = (.2 + .26*np.clip(1 - np.abs(XX - 135)/170., 0, 1) - .1*fy + .1*noise2(9, 21) + .02*np.sin((YY + .002*(XX - 135)**2)*.4))[fl]
# rimstone pools near the bottom (the drips land in them)
POOLS = [(58, 448, 40, 11), (210, 452, 42, 10), (134, 470, 46, 8)]
M_POOL = np.zeros((RH, RW), bool); M_RIM = np.zeros((RH, RW), bool)
for cx, cy, rx, ry in POOLS:
    e = ((XX - cx)/rx)**2 + ((YY - cy)/ry)**2
    M_POOL |= e < 1; M_RIM |= (e >= 1) & (e < 1.35)
CLS[M_POOL] = 4; LUM[M_POOL] = .12 + .26*np.clip(1 - ((YY[M_POOL] - 440)/40.)**2, 0, 1)*(.6 + .4*noise2(5, 31)[M_POOL])
LUM[M_RIM] = .5
# side walls: rough rock with strata and a lit inner edge
wl = 22 + 16*np.sin(np.arange(RH)*.021 + 1) + 9*np.sin(np.arange(RH)*.07) + 10*_smooth_noise(RH, 7, 8)
wr = 22 + 16*np.sin(np.arange(RH)*.019 + 2.5) + 9*np.sin(np.arange(RH)*.08 + 1) + 10*_smooth_noise(RH, 7, 9)
wl += np.clip((np.arange(RH) - 300)/3., 0, 60); wr += np.clip((np.arange(RH) - 310)/3., 0, 55)
rockN = .65*noise2(13, 10) + .35*noise2(4, 12)
for y in range(RH):
    a, b = int(wl[y]), RW - int(wr[y])
    for x0, x1, edge in ((0, a, a), (b, RW, b)):
        if x1 <= x0: continue
        CLS[y, x0:x1] = 1
        xs = np.arange(x0, x1); dist = np.abs(xs - edge)
        LUM[y, x0:x1] = .08 + .2*rockN[y, x0:x1] + .03*np.sin(y*.25 + xs*.12) + .3*np.exp(-dist/2.5)
# the ceiling, and the stalactites hanging from it like a rank of organ pipes (but conical and rough)
ce = 24 + 10*np.sin(np.arange(RW)*.045) + 8*_smooth_noise(RW, 6, 11)
for x in range(RW):
    c = int(ce[x]); CLS[:c, x] = 1; LUM[:c, x] = .08 + .1*rockN[:c, x] + .22*np.exp(-(c - np.arange(c))/2.)
STAL_ID = np.full((RH, RW), -1, np.int16); TIPW = np.zeros((RH, RW))
N_ST = 15; STALS = []
for i in range(N_ST):
    x = 10 + i*(250/(N_ST - 1)) + _r.uniform(-3, 3)
    d = abs(x - 135)/125.
    tip = 116 + 150*d**1.7 + _r.uniform(-8, 8); w = 8 + 7*d + _r.uniform(-1.5, 2)
    base = int(ce[int(np.clip(x, 0, RW - 1))]) - 2
    STALS.append((x, tip, w, base))
for i in range(22):                                                     # smaller ones in between
    x = _r.uniform(4, 266); d = abs(x - 135)/125.
    STALS.append((x, ce[int(x)] + _r.uniform(10, 24 + 40*d), _r.uniform(3, 5.5), int(ce[int(x)]) - 1))
for i, (x, tip, w, base) in enumerate(STALS):
    for y in range(base, int(tip) + 1):
        u = (y - base)/max(1, tip - base)
        hw = w/2*max(0., 1 - u)**.85 + .45*math.sin(y*.37 + i)*(1 - u) + (.6 if u < .05 else 0)
        if hw < .35: hw = .35 if u < .999 else 0
        xa, xb = int(round(x - hw)), int(round(x + hw))
        for xx in range(max(0, xa), min(RW, xb + 1)):
            f = clamp((xx - (x - hw))/max(.8, 2*hw))
            CLS[y, xx] = 2
            LUM[y, xx] = .2 + .34*(1 - f)**1.3 + .06*math.sin(y*.5 + i) - .1*u*(f > .6)
            if i < N_ST: STAL_ID[y, xx] = i; TIPW[y, xx] = u**1.6
STAL_TIPS = [(x, tip) for x, tip, w, base in STALS[:N_ST]]
# stalagmites rising from the floor at the sides
for x, h, w in ((30, 120, 22), (52, 70, 12), (236, 110, 20), (214, 64, 12), (12, 60, 10), (258, 76, 12)):
    yb = 420 if x < 135 else 424
    for y in range(yb - h, yb):
        u = (yb - y)/h; hw = w/2*max(0., 1 - u)**.8 + .5*math.sin(y*.4 + x)
        for xx in range(max(0, int(x - hw)), min(RW, int(x + hw) + 1)):
            f = clamp((xx - (x - hw))/max(.8, 2*hw)); CLS[y, xx] = 1; LUM[y, xx] = .16 + .3*(1 - f) + .05*math.sin(y*.6)
LUM = np.clip(LUM, 0, 1)
K_ = 7
LIDX = np.clip(np.floor(LUM*(K_ - 1) + BAY), 0, K_ - 1).astype(int)

def _pal(th):
    st, gl = THEMES[th]; st = np.array(st, float); gl = np.array(gl, float)
    deep = mix(st, (6, 4, 10), .9); far_lo = mix(st, (5, 4, 9), .94)
    ramps = []
    for cls in range(5):
        lo, hi = {0: (far_lo, mix(mix(st, gl, .3), (0, 0, 0), .5)), 1: (deep, mix(st, gl, .06)),
                  2: (mix(st, (6, 4, 10), .82), mix(st, (255, 255, 255), .12)), 3: (deep, mix(st, (0, 0, 0), .2)),
                  4: (mix(gl, (0, 0, 0), .88), mix(mix(gl, st, .45), (0, 0, 0), .25))}[cls]
        ramps.append([mix(lo, hi, (k/(K_ - 1))**1.15) for k in range(K_)])
    return np.array(ramps, float)
BASE = {k: _pal(k)[CLS, LIDX] for k in THEMES}
# light shafts from two openings in the ceiling
SHAFT = np.zeros((RH, RW))
for x0, x1, w0, w1, a in ((92, 50, 9, 42, .9), (186, 222, 7, 34, .75)):
    u = YY/RH; cx = x0 + (x1 - x0)*u; hw = w0 + (w1 - w0)*u
    SHAFT = np.maximum(SHAFT, a*np.clip(1 - np.abs(XX - cx)/hw, 0, 1)**.7*np.clip(1.15 - u, 0, 1))
SHAFT[CLS == 2] *= .5
# mist: a soft noise texture that drifts sideways
def _mist(seed):
    r = np.random.default_rng(seed); m = np.zeros((RH, RW))
    for k in range(7):                                                   # periodic in x, so it can scroll forever
        fx_ = int(r.integers(2, 7))*r.choice([-1, 1]); fy_ = r.uniform(.03, .09); ph = r.uniform(0, 6.28)
        m += np.sin(2*np.pi*fx_*XX/RW + fy_*YY + ph)*(1 + .3*np.sin(.05*YY + k))
    return (m - m.min())/(m.max() - m.min())
MIST1, MIST2 = _mist(5), _mist(6)
MIST_BAND = np.clip(1 - np.abs(YY - 318)/60., 0, 1)**1.2*.9 + np.clip(1 - np.abs(YY - 430)/50., 0, 1)*.6
M_TXT = ((YY >= 38) & (YY < 102)) | ((YY >= 330) & (YY < 382))
M_TXTE = (((YY >= 34) & (YY < 38)) | ((YY >= 102) & (YY < 106)) | ((YY >= 326) & (YY < 330)) | ((YY >= 382) & (YY < 386))) & (BAY < .5)
DRIP_STALS = [0, 1, 2, 12, 13, 14]
G_PX = 1700.
def _drip_targets():
    out = []
    for i in DRIP_STALS:
        x, tip = STAL_TIPS[i]
        cx, cy, rx, ry = min(POOLS, key=lambda p: abs(p[0] - x))
        xl = int(np.clip(x, cx - rx + 4, cx + rx - 4)); out.append((int(x), tip + 2, xl, cy))
    return out
DRIPS = _drip_targets()
_HATS = None
def background(c, x):
    global _HATS
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    bg = BASE[th0]*(1 - kb) + BASE[th1]*kb
    st = mv(THEMES[th0][0], THEMES[th1][0], kb); glc = mv(THEMES[th0][1], THEMES[th1][1], kb)
    # shafts of light, a little brighter on each downbeat
    bar_ph = (T % c.BAR)/c.BAR
    sh = SHAFT*(.55 + .25*math.exp(-bar_ph*5) + .15*x["accent"])
    lit = sh > BAY*1.05
    bg[lit] = bg[lit]*.58 + mv(mv(st, (255, 255, 255), .65), glc, .2)*.42
    # mist drifting across the floor
    m = (np.roll(MIST1, int(T*7) % RW, 1)*.6 + np.roll(MIST2, -int(T*4) % RW, 1)*.4)*MIST_BAND
    mm = m > .5 + BAY*.42
    bg[mm] = bg[mm]*.78 + mv(st, (255, 255, 255), .55)*.22
    # stalactites glow with the notes of the current scene
    s = c.SCENES[x["si"]]; t = T - s["t0"]
    lo, hi = prange(s); gls = np.zeros(N_ST); cols = np.tile(glc, (N_ST, 1))
    for e in s["ev"]:
        age = t - e[0]
        if age < 0 or e[1] == HORN: continue
        gl = 1. if age < e[3] else math.exp(-(age - e[3])/.25)
        if gl < .02: continue
        k = int(round((e[1] - lo)/max(1, hi - lo)*(N_ST - 1)))
        gls[k] = max(gls[k], gl*(1.25 if age < .08 else 1.))
    if s["theme"] == "hamm" or (x["th1"] == "hamm" and kb > .5):
        cols = np.array([JEWELS[i % len(JEWELS)] for i in range(N_ST)], float)
    sm = STAL_ID >= 0
    ids = STAL_ID[sm]; w = np.clip(gls[ids]*(.25 + .75*TIPW[sm]), 0, 1)
    w = np.floor(w*4 + BAY[sm]*.999)/4
    bg[sm] = bg[sm]*(1 - w[:, None]*.85) + cols[ids]*w[:, None]*.85 + 30*w[:, None]
    for i in range(N_ST):                                                       # a halo round each glowing tip
        if gls[i] < .1: continue
        tx, ty = STAL_TIPS[i]; r = 7
        y0, y1, x0, x1 = int(ty - r), int(ty + r + 1), int(tx - r), int(tx + r + 1)
        y0, x0 = max(0, y0), max(0, x0); y1, x1 = min(RH, y1), min(RW, x1)
        dd = np.sqrt((XX[y0:y1, x0:x1] - tx)**2 + (YY[y0:y1, x0:x1] - ty)**2)
        a = np.clip(1 - dd/r, 0, 1)*min(1, gls[i])*.7
        a = (a > BAY[y0:y1, x0:x1]*.9)*.5
        bg[y0:y1, x0:x1] = bg[y0:y1, x0:x1]*(1 - a[..., None]) + cols[i]*a[..., None]
    # the pools catch the glow
    pg = gls.mean()*1.5
    if pg > .02:
        pm = M_POOL & (BAY < min(.9, pg)*.5)
        bg[pm] = bg[pm]*.7 + glc*.3
    # drips fall from the side stalactites and land on the loud hi-hats, with a splash ring in the pool
    if _HATS is None: _HATS = [tt for tt, k, v in c.DRUMS if k == "hat" and v >= .45]
    for j, th_ in enumerate(_HATS):
        if not (T - .4 < th_ < T + .6): continue
        sx, sy, lx, ly = DRIPS[j % len(DRIPS)]
        fall = math.sqrt(2*(ly - sy)/G_PX); rel = th_ - fall
        if rel <= T < th_:
            tt = T - rel; xx = sx + (lx - sx)*tt/fall; yy = sy + .5*G_PX*tt*tt
            yi, xi = int(yy), int(xx)
            if 0 <= yi < RH - 2 and 0 <= xi < RW: bg[yi:yi + 2, xi] = mv(glc, (255, 255, 255), .6)
        elif th_ <= T < th_ + .45:
            a = (T - th_)/.45; rr = 2 + 11*a; ry = rr*.35
            ang = np.linspace(0, 2*np.pi, 28, endpoint=False)
            px = np.clip((lx + rr*np.cos(ang)).astype(int), 0, RW - 1); py = np.clip((ly + ry*np.sin(ang)).astype(int), 0, RH - 1)
            bg[py, px] = bg[py, px]*a + mv(glc, (255, 255, 255), .5)*(1 - a)
            if a < .5:
                for dx in (-2, 0, 2):
                    yy = int(ly - 7*math.sin(math.pi*a*2)*(1.2 if dx == 0 else .8)); xx2 = lx + dx*(1 + 3*a)
                    if 0 <= yy < RH and 0 <= int(xx2) < RW: bg[yy, int(xx2)] = mv(glc, (255, 255, 255), .7)
    bg[M_TXT] *= .5; bg[M_TXTE] *= .72
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

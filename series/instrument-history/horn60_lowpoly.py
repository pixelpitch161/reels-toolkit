"""Pixel Pitch: history of the French horn, 60-second cut.  python3 horn60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "horn60_lowpoly.mp4", "fh"
BPM = 120
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.75, -8, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE FRENCH HORN", "1,000 YEARS", "TODAY", 1000
INTRO_LINES = (("NEARLY 4 METRES OF", "COILED BRASS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("1,000 YEARS FROM", "A KNIGHT'S OLIFANT")
OUTRO_CREDIT = "EPIC TRAILER HORNS"
T3 = 1/3

# ---------------------------------------------------------------- music (epic trailer groove: taiko + string ostinato)
CH = {"Dm": (38, (57, 62, 65)), "Bb": (46, (58, 62, 65)), "F": (41, (57, 60, 65)), "C": (36, (55, 60, 64)), "D5": (38, (57, 62, 69)),
      "D": (38, (57, 62, 66)), "C7": (36, (55, 58, 64)), "Eb": (39, (55, 58, 63)), "Bb7": (46, (56, 62, 65)), "G": (43, (55, 59, 62)),
      "A7": (45, (55, 61, 64)), "Ab": (44, (56, 60, 63)), "Bbm": (46, (58, 61, 65)), "Ebm": (39, (58, 63, 66)), "F7": (41, (57, 60, 63))}
BARS = ["Dm", "Bb", "F", "C",
        "D5", "D5", "D", "D", "F", ("C7", "C7", "F", "F"), "Eb", ("Bb7", "Bb7", "Eb", "Eb"), "D", ("G", "G", "A7", "D"),
        "Eb", ("Ab", "Ab", "Bb7", "Eb"), "Bbm", ("Ebm", "Ebm", "F", "F"), "F", ("C7", "C7", "F", "F"), "F7", ("Bb7", "Bb7", "F7", "C7"),
        "Bb", ("Eb", "Eb", "Bb", "Bb"), "C", ("Bb", "Bb", "C", "C"), "Dm", "Bb", "F", "Dm"]
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
HERO = [(0, 62, 1), (1, 69, 2), (3, 67, .5), (3.5, 65, .5), (4, 62, 1.5), (5.5, 65, .5), (6, 70, 2)]
HERO2 = [(0, 69, 1), (1, 72, 1.5), (2.5, 70, .5), (3, 69, 1)]

SCENES = [
 dict(key="modern", theme="intro", name="MODERN DOUBLE HORN", credit="EPIC TRAILER HORNS",
      notes=tn(HERO) + tn(HERO2, b0=8) + tn([(12, 67, 1.5), (13.5, 65, .5), (14, 64, 2)])),
 dict(key="olifant", theme="olifant", year=1050, ys="1000s", name="OLIFANT", credit="MEDIEVAL BATTLE CALL", size=1.0,
      fact=("KNIGHTS CARRIED", "CARVED IVORY HORNS"),
      notes=tn([(0, 50, 1.5), (1.5, 57, .5), (2, 57, 2), (4, 50, 1), (5, 57, 1), (6, 50, 2)], .9)),
 dict(key="trompe", theme="trompe", year=1680, ys="1680s", name="TROMPE DE CHASSE", credit="FRENCH HUNTING FANFARE",
      fact=("A BIG HOOP TO WEAR", "ON HORSEBACK"),
      notes=tn([(0, 62, 2*T3), (2*T3, 66, T3), (1, 69, 2*T3), (1 + 2*T3, 66, T3), (2, 69, 1), (3, 74, 1),
                (4, 74, 2*T3), (4 + 2*T3, 69, T3), (5, 66, 2*T3), (5 + 2*T3, 69, T3), (6, 62, 2)], .88)),
 dict(key="baroque", theme="baroque", year=1717, ys="1717", name="BAROQUE HORN", credit="IN THE STYLE OF HANDEL",
      fact=("HANDEL'S WATER MUSIC", "SHOWED OFF ITS HORNS"),
      notes=seq(0, [(65, .5), (69, .5), (72, 1), (69, .5), (72, .5), (77, 1), (76, .5), (74, .5), (72, .5), (70, .5), (69, 1), (65, 1)], .85)),
 dict(key="hand", theme="hand", year=1780, ys="1780s", name="HAND HORN", credit="IN THE STYLE OF MOZART",
      fact=("A HAND IN THE BELL", "BENDS THE PITCH"),
      notes=tn([(0, 63, 2*T3), (2*T3, 63, T3), (1, 67, 2*T3), (1 + 2*T3, 70, T3), (2, 72, 2*T3), (2 + 2*T3, 70, T3), (3, 67, 1),
                (4, 68, 2*T3), (4 + 2*T3, 67, T3), (5, 65, 2*T3), (5 + 2*T3, 63, T3), (6, 67, 1), (7, 63, 1)], .85)),
 dict(key="valve1818", theme="valve1818", year=1818, ys="1818", name="EARLY VALVE HORN", credit="ROMANTIC HUNTING CHORUS",
      fact=("VALVES, PATENTED 1818,", "FILL IN THE GAPS"),
      notes=seq(0, [(62, .75), (62, .25), (66, .5), (69, .5), (74, 1.5), (73, .5), (71, .5), (69, .5), (67, .5), (66, .5), (64, 1), (62, 1)], .85)),
 dict(key="vienna", theme="vienna", year=1830, ys="1830s", name="VIENNA HORN", credit="IN THE STYLE OF BRUCKNER",
      fact=("VIENNA STILL PLAYS", "THIS 1830S DESIGN"),
      notes=seq(0, [(58, 1), (63, 1), (67, 1.5), (65, .5), (63, 1), (60, 1), (62, 1), (63, 1)], .85)),
 dict(key="wagner", theme="wagner", year=1876, ys="1876", name="WAGNER TUBA", credit="IN THE STYLE OF WAGNER", size=.8,
      fact=("WAGNER HAD A HORN", "BUILT FOR THE RING"),
      notes=seq(0, [(53, 1.5), (58, .5), (61, 2), (60, 1), (58, 1), (53, 2)], .9)),
 dict(key="double", theme="double", year=1897, ys="1897", name="DOUBLE HORN", credit="IN THE STYLE OF RICHARD STRAUSS",
      fact=("KRUSPE JOINED F AND", "B-FLAT IN ONE HORN"),
      notes=seq(0, [(60, .5), (65, .5), (69, .5), (72, .5), (77, 1), (76, .5), (74, .5), (72, .5), (69, .5), (70, .5), (67, .5), (65, 2)], .85)),
 dict(key="jazz", theme="jazz", year=1955, ys="1950s", name="JAZZ FRENCH HORN", credit="IN THE STYLE OF JULIUS WATKINS",
      fact=("JULIUS WATKINS TOOK", "IT INTO BEBOP"),
      notes=tn([(0, 65, 2*T3), (2*T3, 68, T3), (1, 69, 2*T3), (1 + 2*T3, 72, T3), (2, 74, 1), (3, 72, 2*T3), (3 + 2*T3, 70, T3),
                (4, 68, 2*T3), (4 + 2*T3, 67, T3), (5, 65, 1), (6, 63, 2*T3), (6 + 2*T3, 65, T3), (7, 67, 1)], .88)),
 dict(key="mello", theme="mello", year=1965, ys="1960s", name="MELLOPHONE", credit="DRUM CORPS FANFARE",
      fact=("MARCHING BANDS POINT", "THE BELL FORWARD"),
      notes=seq(0, [(70, .5), (70, .25), (70, .25), (74, .5), (77, .5), (82, 1.5), (79, .5), (77, .5), (75, .5), (74, .5), (72, .5), (70, 2)], .88)),
 dict(key="film", theme="film", year=1977, ys="1970s", name="FILM SCORE HORN", credit="CINEMATIC HERO THEME",
      fact=("HOLLYWOOD'S SOUND", "OF HEROES"),
      notes=seq(0, [(60, 1), (67, 2), (65, .5), (64, .5), (62, .5), (72, 1.5), (67, 2)], .9)),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="MODERN DOUBLE HORN", credit="EPIC TRAILER HORNS",
      fact=("THE HAND STAYS IN", "THE BELL TO SHAPE TONE"),
      notes=tn(HERO) + tn(HERO2, b0=8) + [(12, 62, 4.3, .95)]),
]

TRIM = {"olifant": 1.0, "trompe": -.6, "valve1818": -.6, "wagner": -.7, "mello": -.4, "double": -.2}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], -.3 if _s is SCENES[-1] else 0.)

# ---------------------------------------------------------------- instruments (modelled on reference photos, metres)
# Double horn, front view: tubing wound into a coil ~31 cm across; the bell (30 cm, as wide as the coil)
# grows out of the bottom of the coil and flares out to the left; four rotary valves sit in a column on the
# opposite side, their slides running across the inside of the coil, lever "spatulas" sticking out past
# the rim; the leadpipe drops in from the top on the valve side.
BRASS, NICKEL, SKIN = (228, 178, 86), (214, 216, 226), (230, 186, 150)
HN = {
 "modern":    dict(kind="double", col=(226, 172, 90), bell=.152),
 "double":    dict(kind="double", col=(232, 194, 96), bell=.148),
 "jazz":      dict(kind="double", col=(220, 172, 84), bell=.15, mic=True),
 "film":      dict(kind="double", col=(240, 202, 114), bell=.152, clapper=True),
 "vienna":    dict(kind="double", col=(232, 190, 100), bell=.15, vienna=True),
 "hand":      dict(kind="natural", col=(224, 182, 92), loops=(.165, .155, .145), bell=.14, crook=True, hand=True),
 "baroque":   dict(kind="natural", col=(212, 168, 82), loops=(.19, .18), bell=.12, crook=True, garland=(244, 210, 120)),
 "trompe":    dict(kind="natural", col=(224, 178, 86), loops=(.26, .25, .24), bell=.13, leather=(30, 40, 96)),
 "valve1818": dict(kind="natural", col=(222, 176, 86), loops=(.165, .155), bell=.14, crook=True, stolzel=True),
}
C0 = np.array([0, .56])
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
BDIR = unit((-.82, .42, .42))
def box_at(g, p, s, col, emit=False, spec=0.):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit, spec=spec)
def path(g, pts, r0, col, r1=None, n=8, spec=.9):
    pts = [np.asarray(p, float) for p in pts]; r1 = r0 if r1 is None else r1; N = len(pts) - 1
    for k, (a, b) in enumerate(zip(pts, pts[1:])):
        T.tube(g, a, b, lerp(r0, r1, k/N), lerp(r0, r1, (k + 1)/N), col=col, n=n, spec=spec)
def arc(c, R, z, a0, a1, segs=20):
    return [(c[0] + R*math.cos(math.radians(a)), c[1] + R*math.sin(math.radians(a)), z) for a in np.linspace(a0, a1, segs + 1)]
def bell(g, p0, d, L, r_back, R, col, x0=.05, m=.78, segs=14, garland=None):
    """Bessel-horn flare along direction d, rim at p0 + d*L; returns the rim centre"""
    p0, d = np.asarray(p0, float), unit(d)
    def rad(dd): return max(r_back, R*(x0/(dd + x0))**m)
    ds = [L*(k/segs)**2.2 for k in range(segs + 1)][::-1]
    for d0, d1 in zip(ds, ds[1:]):
        T.tube(g, p0 + d*(L - d0), p0 + d*(L - d1), rad(d0), rad(d1), col=col, n=14 if d1 < .1 else 10, spec=.9, two=d1 < .25)
    rim = p0 + d*L
    T.tube(g, rim - d*.006, rim + d*.002, R*1.04, R*1.04, col=col, n=14, two=True)
    g.face(S.ring(rim - d*.05, d, rad(.05)*.75, 12), (28, 18, 10), ref=rim - d, emit=True, two=True)
    if garland: T.tube(g, rim - d*.05, rim - d*.012, rad(.05)*1.05, rad(.012)*1.05, col=garland, n=14, two=True)
    return rim
def mouthpiece(g, p, d, r=.0065):
    p, d = np.asarray(p, float), unit(d)
    T.tube(g, p, p + d*.05, r*1.1, r*.9, col=NICKEL); T.tube(g, p + d*.05, p + d*.07, r*.9, r*2.6, col=NICKEL, n=10)
def bell_branch(g, col, R, r, bellR, garland=None):
    br = arc(C0, R, .008, 330, 222, 12)
    path(g, br, r*1.1, col, r1=.022)
    return bell(g, br[-1], BDIR, .26, .022, bellR, col, garland=garland)

def double_horn(g, c, m, t, ev):
    col, r = c["col"], .0062
    for R, z in ((.155, 0.), (.142, -.012)): path(g, arc(C0, R, z, 0, 360, 30), r, col)
    rim = bell_branch(g, col, .168, r, c["bell"])
    fing = T.fingering(m + 7); thumb = m >= 65; x = .07
    ys = [C0[1] + v for v in (.06, .025, -.01, -.045)]
    for i, y in enumerate(ys):
        on = (str(i + 1) in fing) if i < 3 else thumb
        if c.get("vienna"):
            if i == 3: continue
            for dy in (-.0075, .0075): T.tube(g, (x - .03, y + dy, .012), (x + .025, y + dy, .012), .0075, col=col, n=8)
            box_at(g, (x - .033, y, .012), .01, dim(col, .85)); box_at(g, (x + .028, y, .012), .01, dim(col, .85))
            if on: box_at(g, (x - .035 - .006, y, .012), .006, HOT, emit=True)
        else:
            T.tube(g, (x, y, -.012), (x, y, .03), .016, col=col, n=10)
            cap = np.array([x, y, .031]); g.face(S.ring(cap, (0, 0, 1), .0135, 10), dim(col, .82), ref=(x, y, 0))
            a = math.pi/2 if on else 0
            g.line(cap + [.011*math.cos(a), .011*math.sin(a), .001], cap - [.011*math.cos(a), .011*math.sin(a), -.001], HOT if on else (80, 60, 30))
        xl = (-.1, -.075, -.115, -.06)[i]
        for dy in (-.009, .009): T.tube(g, (x - .016, y + dy, 0), (xl, y + dy, 0), r*.85, col=col, n=6)
        T.bend(g, (xl, y, 0), .009, r*.85, 90, 270, col=col, segs=4)
        pad = np.array([.205, y - .022 - (.01 if on else 0), .035])
        g.line((x + .012, y, .03), pad, NICKEL); g.line((x + .012, y + .003, .03), pad + [0, .003, 0], NICKEL)
        g.box(pad[0] - .016, pad[0] + .016, pad[1] - .005, pad[1] + .005, pad[2] - .009, pad[2] + .009, HOT if on else (236, 230, 214), emit=on, spec=.6)
    for xx, h in ((-.015, .675), (.03, .668)):
        for dx in (-.009, .009): T.tube(g, (xx + dx, .6, -.004), (xx + dx, h, -.004), r*.9, col=col, n=6)
        T.bend(g, (xx, h, -.004), .009, r*.9, 0, 180, col=col, segs=4)
    path(g, arc((-.07, .47), .042, -.006, 0, 360, 14), r*.9, col)
    path(g, arc((-.035, .455), .03, -.01, 0, 360, 10), r*.9, col)
    mp = np.array([.175, .82, .02]); mouthpiece(g, mp, (.28, 1, 0))
    path(g, [mp, (.15, .74, .02), (.12, .67, .02), (.09, ys[0] + .015, .01)], r*.85, col, r1=r*1.05)
    for a in (40, 140, 230): p = C0 + .148*np.array([math.cos(math.radians(a)), math.sin(math.radians(a))]); g.line((*p, 0), (x, ys[1], 0), dim(col, .75))
    return rim

def natural_horn(g, c, m, t, ev):
    col, r = c["col"], .0055; loops = c["loops"]
    for k, R in enumerate(loops): path(g, arc(C0, R, -.01*k, 0, 360, 34), r, col)
    if c.get("leather"):
        for k, R in enumerate(loops[:2]): path(g, arc(C0, R, -.01*k, 95, 205, 10), r*1.7, c["leather"])
    rim = bell_branch(g, col, loops[0] + .012, r, c["bell"], garland=c.get("garland"))
    if c.get("crook"):
        cc = (.03, C0[1] + loops[-1] - .065)
        for k, R in enumerate((.055, .047)): path(g, arc(cc, R, .012 + .008*k, 0, 360, 16), r, col)
        mp0 = np.array([cc[0] + .045, cc[1] + .035, .02]); path(g, [(cc[0] + .05, cc[1], .015), mp0], r, col)
        mp = mp0 + unit((.45, 1, 0))*.12; path(g, [mp0, mp], r*.9, col); mouthpiece(g, mp, (.45, 1, 0))
    else:
        a = math.radians(60); p0 = np.array([C0[0] + loops[0]*math.cos(a), C0[1] + loops[0]*math.sin(a), 0])
        mp = p0 + unit((.5, 1, 0))*.1; path(g, [p0, mp], r, col); mouthpiece(g, mp, (.5, 1, 0))
    if c.get("stolzel"):
        fing = T.fingering(m + 7)
        for i, x in enumerate((.04, .08)):
            down = str(i + 1) in fing
            T.tube(g, (x, .47, .02), (x, .6, .02), .014, col=col, n=8)
            top = .64 - (.02 if down else 0)
            T.tube(g, (x, .6, .02), (x, top, .02), .004, col=NICKEL, n=6)
            T.tube(g, (x, top, .02), (x, top + .009, .02), .012, col=HOT if down else (236, 230, 214), n=8, spec=.4)
            path(g, [(x, .47, .02), (x, .43, .0), (x + .02, .43, 0), (x + .02, .47, 0)], r*.8, col)
        path(g, [(-.1, .53, 0), (.04, .53, .02)], r, col)
    if c.get("hand"):
        live = [e for e in ev if e[0] < e[3]]; push = .03 if (live and live[0][1] in (68, 65, 62)) else 0
        hp = rim - BDIR*(.07 + push)
        g.box(hp[0] - .03, hp[0] + .03, hp[1] - .045, hp[1] + .045, hp[2] - .016, hp[2] + .016, SKIN)
    return rim

def horn(g, c, m, t, ev):
    rim = double_horn(g, c, m, t, ev) if c["kind"] == "double" else natural_horn(g, c, m, t, ev)
    if c.get("mic"):
        S.rod(g, (-.45, 0, .2), (-.45, .4, .2), .008); g.ngon(-.45, .004, .2, .1, (40, 38, 46), n=8)
        g.box(-.49, -.41, .4, .52, .16, .24, (190, 194, 204), spec=.8); g.box(-.49, -.41, .45, .47, .155, .245, (40, 40, 48))
    if c.get("clapper"):
        b0 = np.array([.2, 0, .2]); g.box(b0[0], b0[0] + .2, 0, .12, b0[2], b0[2] + .015, (30, 30, 34))
        ang = .25 + .2*abs(math.sin(t*3)); tip = (b0[0] + .2*math.cos(ang), .12 + .2*math.sin(ang), b0[2] + .007)
        g.face([(b0[0], .12, b0[2] + .007), tip, (tip[0] + .015*math.sin(ang), tip[1] - .015*math.cos(ang) + .03, tip[2]), (b0[0], .15, b0[2] + .007)], (240, 240, 240), two=True)
        for k in range(4): g.box(b0[0] + .02 + .045*k, b0[0] + .04 + .045*k, .12, .14, b0[2], b0[2] + .016, (240, 240, 240))
    return rim

def olifant(g, m, t):
    """a gently curved, heavily carved elephant tusk: thin mouthpiece end, wide mouth, metal bands for a carrying strap"""
    ivory, metal = (234, 222, 190), (190, 150, 80)
    pts = [np.array([-.3 + .6*u, .5 + .05*math.sin(math.pi*u) + .03*u, 0]) for u in np.linspace(0, 1, 13)]
    rad = lambda u: .016 + .05*u**1.15
    for k, (a, b) in enumerate(zip(pts, pts[1:])):
        u0, u1 = k/12, (k + 1)/12
        T.tube(g, a, b, rad(u0), rad(u1), col=ivory if k % 2 else dim(ivory, .86), n=10, spec=.4)
    for u in (.14, .2, .8, .9, .96):
        k = int(u*12); p = pts[k] + (pts[min(12, k + 1)] - pts[k])*(u*12 - k)
        T.tube(g, p - [.008, 0, 0], p + [.008, 0, 0], rad(u)*1.06, col=dim(ivory, .75), n=10)
    for u in (.3, .72):
        k = int(u*12); p = pts[k]; T.tube(g, p - [.012, 0, 0], p + [.012, 0, 0], rad(u)*1.1, col=metal, n=10, spec=.7)
        g.line(p + [0, rad(u)*1.1, 0], p + [0, .2, 0], (80, 70, 60)); g.line(p + [.003, rad(u)*1.1, 0], p + [.003, .2, 0], (80, 70, 60))
    T.tube(g, pts[0] - [.03, 0, 0], pts[0], .009, .016, col=metal)
    d = unit(pts[-1] - pts[-2]); g.face(S.ring(pts[-1] + d*.001, d, rad(1)*.85, 12), (30, 22, 14), ref=pts[-2], emit=True)
    for x in (-.15, .18):
        g.box(x - .006, x + .006, 0, .5 + .05*math.sin(math.pi*(x + .3)/.6) - .04, -.006, .006, (200, 220, 230), spec=.8)
    return pts[-1]

def wagner(g, m, t):
    """upright oval body, four rotary valves inside it, a tall bell rising from the top"""
    col, r = (226, 182, 90), .0075; cc = (0, .44)
    for k, (rx, ry) in enumerate(((.085, .14), (.074, .128))):
        path(g, [(cc[0] + rx*math.cos(a), cc[1] + ry*math.sin(a), -.012*k) for a in np.linspace(0, 2*math.pi, 25)], r, col)
    path(g, [(-.06, .54, .008), (-.05, .62, .01), (-.03, .7, .012)], r*1.5, col, r1=.024)
    rim = bell(g, (-.03, .7, .012), (-.3, 1, .25), .2, .024, .105, col)
    fing = T.fingering(m + 14)
    for i in range(4):
        y = .5 - .03*i; p = np.array([.03, y, .01]); on = str(i + 1) in fing if i < 3 else m < 53
        T.tube(g, p - [0, 0, .02], p + [0, 0, .022], .014, col=col, n=10)
        g.face(S.ring(p + [0, 0, .023], (0, 0, 1), .012, 10), HOT if on else dim(col, .85), ref=p, emit=on)
        for dy in (-.008, .008): T.tube(g, (.017, y + dy, 0), (-.05, y + dy, 0), r*.8, col=col, n=6)
        pad = np.array([.12, y - .015 - (.008 if on else 0), .04]); g.line(p + [0, 0, .025], pad, NICKEL)
        box_at(g, pad, .009, HOT if on else (236, 230, 214), emit=on)
    mp = np.array([.1, .64, .02]); mouthpiece(g, mp, (.3, 1, 0), .007)
    path(g, [mp, (.08, .58, .02), (.05, .53, .01)], r*.9, col)
    S.rod(g, (0, 0, -.04), (0, .3, -.02), .014); g.ngon(0, .004, -.04, .14, (40, 38, 46), n=8)
    return rim

def mello(g, m, t):
    """bell-front marching horn: long straight leadpipe, three pistons on top, round wrap below, big forward bell"""
    col, r = (214, 216, 226), .0075
    mp = np.array([-.4, .6, 0]); mouthpiece(g, mp, (-1, 0, 0), .0075)
    path(g, [mp, (-.02, .6, 0)], r, col)
    fing = T.fingering(m)
    for i, x in enumerate((-.1, -.06, -.02)):
        T.tube(g, (x, .5, 0), (x, .62, 0), .016, col=col, n=10)
        T.tube(g, (x, .62, 0), (x, .63, 0), .019, col=dim(col, .9), n=10); T.tube(g, (x, .5, 0), (x, .49, 0), .019, col=dim(col, .9), n=10)
        down = str(i + 1) in fing; top = .665 - (.022 if down else 0)
        T.tube(g, (x, .63, 0), (x, top, 0), .004, col=col, n=6)
        T.tube(g, (x, top, 0), (x, top + .009, 0), .014, col=HOT if down else (240, 236, 226), n=8, spec=.4)
        path(g, arc((x, .455), .022, .015, 0, 360, 10), r*.8, col)
    path(g, [(cx_, cy_, -.01) for cx_, cy_ in [(-.16 + .2*math.cos(a), .5 + .075*math.sin(a)) for a in np.linspace(.4, 2*math.pi + .4, 25)]], r, col)
    path(g, [(.03, .53, -.01), (.1, .54, -.005), (.14, .56, 0)], r*1.2, col, r1=.022)
    rim = bell(g, (.14, .56, 0), (1, .12, .2), .28, .022, .13, col)
    S.rod(g, (-.1, 0, -.04), (-.1, .43, -.02), .014); g.ngon(-.1, .004, -.04, .14, (40, 38, 46), n=8)
    return rim

def cur(s, t, ev):
    live = [e for e in ev if e[0] < e[3] + .03]
    return min(live)[1] if live else (min(ev)[1] if ev else s["ev"][0][1])

SHIFT = np.array([.1, -.07, 0])
def build(g, t, ev, fx, s):
    k = s["key"]; m = cur(s, t, ev); f0, l0, x0 = len(g.F), len(g.L), len(fx)
    if k == "olifant": rim = olifant(g, m, t)
    elif k == "wagner": rim = wagner(g, m, t)
    elif k == "mello": rim = mello(g, m, t)
    else:
        c = HN[k]; rim = horn(g, c, m, t, ev)
        lo = c.get("loops", (.155,))[0]
        S.rod(g, (0, 0, -.06), (0, C0[1] - lo, -.02), .014); g.ngon(0, .004, -.06, .14, (40, 38, 46), n=8)
    rim = np.asarray(rim, float)
    sh = SHIFT if k not in ("olifant", "mello", "wagner") else np.zeros(3)
    for i in range(f0, len(g.F)): P_, *rest = g.F[i]; g.F[i] = (P_ + sh, *rest)
    for i in range(l0, len(g.L)): a, b, *rest = g.L[i]; g.L[i] = (a + sh, b + sh, *rest)
    rim = rim + sh; left = k not in ("mello", "olifant", "wagner")
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(rim + [-.04 if left else .04, .2, 0]), age))
        if tp and age < .6: fx.append(("wavel" if left else "wave", tuple(rim + [-.03 if left else .03, 0, 0]), age, 0))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
TONE = {"modern": dict(bright=.55, vib=.002, scoop=.02, tilt=1.0), "olifant": dict(bright=.5, vib=0, scoop=.06, rough=.2, tilt=.9),
        "trompe": dict(bright=1.0, vib=.012, scoop=.03, tilt=.85), "baroque": dict(bright=.7, vib=0, scoop=.03, tilt=.9),
        "hand": dict(bright=.45, vib=0, scoop=.03, tilt=1.0), "valve1818": dict(bright=.55, vib=.002, scoop=.02, tilt=1.0),
        "vienna": dict(bright=.65, vib=.002, scoop=.02, tilt=.95), "wagner": dict(bright=.3, vib=.002, scoop=.02, tilt=1.1),
        "double": dict(bright=.55, vib=.002, scoop=.02, tilt=1.0), "jazz": dict(bright=.5, vib=.005, scoop=.04, tilt=1.0),
        "mello": dict(bright=1.0, vib=.003, scoop=.03, tilt=.85), "film": dict(bright=.75, vib=.003, scoop=.02, tilt=.95)}
HARMONIC = {62, 66, 69, 72, 74, 77, 65, 67, 70, 75, 79, 82, 63, 58, 55, 60}
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]; tone = dict(TONE[key])
    if key == "hand" and m in (68, 65, 62): tone["bright"] = .18                                       # hand-stopped notes
    x = T.brass(midi_hz(m), dur, v, **tone)
    if key in ("modern", "film", "double"):
        y = T.brass(midi_hz(m)*1.002, dur, v, **tone); x = .65*x + .35*y[:len(x)]                    # a touch of section width
    return x

# ---------------------------------------------------------------- backing: epic trailer (taiko, shime, spiccato strings, low strings)
def taiko(v):
    a = D.tom_s(62, 4.5, .35, 1.0); b = D.kick_s(44, 80, 5, .04, 1.0, 1.2); n = max(len(a), len(b))
    return (np.pad(a, (0, n - len(a))) + .7*np.pad(b, (0, n - len(b))))*v
KIT = {"taiko": taiko, "taikoH": lambda v: D.tom_s(130, 8, .3, .5)*v, "shime": lambda v: D.wood_s(900, 45)*.6*v,
       "crash": lambda v: D.metal_s([400, 650, 1010, 1600, 2400, 3500, 5100], 1.6, .2, 2.4, 1500)*v}
KICK_KINDS, ACCENT_KINDS = ("taiko",), ("taikoH",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "taiko", 1.), (T0, "crash", 1.), (T0, "taikoH", .9)]; continue
        if b == 0: ev += [(T0, "taiko", .8), (T0 + 2*Q, "taiko", .6)]; continue
        ev += [(T0, "taiko", 1.), (T0 + 1.5*Q, "taiko", .65), (T0 + 2*Q, "taiko", .9), (T0 + 3.5*Q, "taiko", .6),
               (T0 + Q, "taikoH", .8), (T0 + 3*Q, "taikoH", .8), (T0 + 2.75*Q, "taikoH", .45), (T0 + 3.25*Q, "taikoH", .5)]
        ev += [(T0 + k*.5*Q, "shime", .5 if k % 2 else .3) for k in range(8)]
        if b in (4, 26): ev.append((T0, "crash", .6))
    return ev
def _chord_at(b, beat):
    ch = BARS[min(b, len(BARS) - 1)]; return ch[int(beat)] if isinstance(ch, tuple) else ch
def _root(name):
    r = CH[name][0]
    while r > 45: r -= 12
    while r < 33: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord_at(b, 0)), 2.4, 1.)); continue
        out += [(T0, _root(_chord_at(b, 0)), 1.9*c.BEAT, .9), (T0 + 2*c.BEAT, _root(_chord_at(b, 2)), 1.9*c.BEAT, .85)]
    return out
def strings(f, dur, v, att=.01):
    n = int((dur + .15)*SR); t = _t(n); x = np.zeros(n)
    for k in range(1, 14):
        if f*k > 7000: break
        x += np.sin(2*np.pi*f*k*t*(1 + .003*np.sin(2*np.pi*5.5*t)) + k)/k**1.3
    return x*np.minimum(1, t/att)*np.where(t < dur, 1, np.exp(-(t - dur)/.05))*v
def bass_voice(m, dur, v): return strings(midi_hz(m), dur, v, att=.06)*.8
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); out = np.zeros(n); lo = sorted(notes)
    if dur < 2: pat = [lo[0], lo[2], lo[1], lo[2], lo[0] + 12, lo[2], lo[1], lo[2]] if abs(dur - (BAR_S + .05)) < .1 else [lo[0], lo[2]]
    else: pat = None
    if pat is None:
        for m in lo: x = strings(midi_hz(m), 2.2, .5*v, att=.2); out[:len(x)] += x[:n]
        return out
    for k, m in enumerate(pat):
        x = strings(midi_hz(m), .16, .6*v); a = int(k*.5*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
MIX = dict(drums=-21, bass=-24, pad=-27, verb=-21, verb_len=2.0)

# ---------------------------------------------------------------- background: concert hall with a pipe organ that lights up with the music
THEMES = {   # wall top, wall bottom, organ case, pipe metal, pipe glow, stage floor
 "intro":     ((24, 10, 14), (110, 40, 40), (90, 44, 30), (210, 200, 190), (255, 200, 110), (70, 30, 24)),
 "olifant":   ((14, 14, 20), (70, 70, 90), (60, 54, 60), (180, 180, 196), (210, 220, 255), (40, 40, 50)),
 "trompe":    ((14, 28, 14), (70, 120, 60), (70, 60, 30), (210, 200, 160), (255, 240, 150), (40, 50, 26)),
 "baroque":   ((40, 24, 10), (170, 120, 60), (120, 70, 30), (240, 220, 170), (255, 230, 150), (80, 50, 24)),
 "hand":      ((30, 14, 30), (140, 80, 130), (90, 50, 80), (230, 210, 230), (255, 210, 240), (60, 30, 56)),
 "valve1818": ((10, 24, 36), (60, 110, 150), (50, 60, 80), (210, 220, 236), (180, 230, 255), (30, 40, 56)),
 "vienna":    ((34, 8, 12), (150, 40, 50), (110, 40, 34), (240, 210, 190), (255, 220, 160), (70, 20, 24)),
 "wagner":    ((6, 8, 20), (40, 50, 100), (40, 40, 60), (170, 180, 210), (140, 170, 255), (20, 24, 46)),
 "double":    ((30, 24, 8), (150, 130, 50), (100, 80, 30), (240, 230, 190), (255, 240, 140), (70, 56, 20)),
 "jazz":      ((10, 6, 20), (60, 30, 90), (40, 30, 60), (200, 190, 220), (255, 120, 200), (30, 20, 44)),
 "mello":     ((6, 24, 40), (30, 120, 160), (30, 70, 90), (210, 236, 246), (120, 240, 255), (20, 50, 64)),
 "film":      ((8, 8, 8), (70, 60, 50), (50, 40, 30), (220, 210, 190), (255, 230, 170), (36, 30, 26)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[5], T_[2], .4))
GY = 300
WALLG = {k: F.make_grad(v[0], v[1], y0=100, y1=GY) for k, v in THEMES.items()}
PIPES = []
for i in range(27):
    x = 14 + i*9; u = abs(i - 13)/13.
    top = int(178 - 56*(1 - u)**.6 + (14 if i % 2 else 0)) if i != 13 else 112
    top = int(126 + 50*abs(math.sin(i*.5))) if 0 else top
    PIPES.append((x, top, i % 14))
PIPE = np.zeros((RH, RW), int) - 1
for j, (x, top, band) in enumerate(PIPES): PIPE[top:262, x:x + 6] = j
CASE = (YY >= 254) & (YY < 300) & (XX >= 8) & (XX < RW - 8)
CASE |= (YY >= 104) & (YY < 112) & (((XX >= 8) & (XX < 40)) | ((XX >= RW - 40) & (XX < RW - 8)) | (np.abs(XX - CX) < 20))
GROUND = YY >= GY
BOARDS = GROUND & ((((YY - GY) % 8) == 0) | (((XX + ((YY - GY)//8)*17) % 44) == 0))
SPOTW = 20 + (YY - 100)*.3
BEAM = (YY >= 100) & (YY < GY + 60) & (np.abs(XX - CX) < SPOTW) & (BAY < .18)
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
SHADE = np.tile(np.array([.72, .9, 1.05, 1.1, .95, .78]), 46)[:RW]

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return np.array(mix(A_[i], B_[i], kb), float)
    bg = WALLG[th0]*(1 - kb) + WALLG[th1]*kb
    bg[:100] = col(0)*.85
    metal, glow, eq = col(3), col(4), x["eq"]
    for j, (px, top, band) in enumerate(PIPES):
        lv = eq[band]; sl = bg[top:262, px:px + 6]
        base = metal*(.55 + .25*lv) + glow*.35*lv
        sl[:] = base[None, None, :]*SHADE[:6][None, :, None]
        bg[top:top + 2, px:px + 6] = base*1.15
        my = 238 + (j % 3)*4; bg[my:my + 3, px + 1:px + 5] = (20, 14, 12)
        if lv > .55: bg[my - 3:my, px + 1:px + 5] = glow
    bg[CASE] = col(2); bg[(YY == 254) & CASE] = col(2)*1.3
    bg[GROUND] = col(5); bg[BOARDS] = col(5)*.75
    bg[BEAM] = bg[BEAM]*.6 + glow*.4*(.5 + .5*x["loud"])
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

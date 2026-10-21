"""Pixel Pitch: history of the harp, 60-second cut.  python3 harp60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import piano_lowpoly as P
import cello_lowpoly as C
import acoustic_lowpoly as A
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT, HOTSTR

OUT, TAG = "harp60_lowpoly.mp4", "hp"
BPM = 96                        # dotted-quarter pulse: a 12/8 groove, every beat splits in three
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .9, -6, 14, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE HARP", "15,000 YEARS", "TODAY", -13000
INTRO_LINES = (("47 STRINGS AND", "7 PEDALS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("15,000 YEARS FROM", "A HUNTER'S BOW")
OUTRO_CREDIT = "MODERN HARP POP"
T3 = 1/3

# ---------------------------------------------------------------- music
CH = {"D": (38, (57, 62, 66)), "Bm": (47, (54, 59, 62)), "G": (43, (55, 59, 62)), "A": (45, (57, 61, 64)), "A5": (45, (57, 64, 69)),
      "D5": (38, (57, 62, 69)), "Em": (40, (55, 59, 64)), "C": (36, (55, 60, 64)), "Gm": (43, (55, 58, 62)), "Cm": (36, (55, 60, 63)),
      "A7": (45, (55, 61, 64)), "E7": (40, (56, 59, 62)), "F7": (41, (57, 60, 63)), "Bb7": (46, (56, 58, 62))}
BARS = ["D", "Bm", "G", "A",
        "A5", "A5", "D5", "D5", "Em", ("D", "D", "Em", "Em"), "D5", ("C", "C", "D5", "D5"), "Gm", ("Cm", "Cm", "D", "Gm"),
        "D", ("A7", "A7", "A7", "D"), ("A", "A", "E7", "E7"), ("A", "A", "E7", "A"), "F7", ("Bb7", "Bb7", "F7", "F7"),
        "Bm", "G", "A", "D"]
def arp(b0, ms, v=.62, d=1.):
    return [(b0 + k*T3, m, d, v) for k, m in enumerate(ms)]
def tseq(b0, items, v=.85):
    out = []
    for b, m, d in items: out.append((b0 + b, m, d, v))
    return out
def up_down(root, ivs=(0, 7, 12, 16, 19, 24, 28)):
    u = [root + i for i in ivs[:6]]; return u + u[::-1][:6]
SHOW = (arp(0, [50, 57, 62, 66, 69, 74, 78, 74, 69, 66, 62, 57]) + [(0, 38, 3, .6)] +
        arp(4, [47, 54, 59, 62, 66, 71, 74, 71, 66, 62, 59, 54]) + [(4, 35, 3, .6)] +
        arp(8, [43, 50, 55, 59, 62, 67, 71, 67, 62, 59, 55, 50]) + [(8, 43, 3, .6)] +
        arp(12, [45, 52, 57, 61, 64, 69, 73, 69, 64, 61, 57, 52]) + [(12, 45, 3, .6)] +
        tseq(8, [(0, 79, 1), (1, 81, 1), (2, 83, 2), (4, 85, 1), (5, 81, 1), (6, 76, 2)], .92))
E7_ = [57, 61, 64, 69]

SCENES = [
 dict(key="modern", theme="intro", name="CONCERT HARP", credit="CINEMATIC HARP ARPEGGIOS", notes=SHOW),
 dict(key="bow", theme="bow", trim=-1.3, year=-13000, ys="13,000 BC", name="HUNTER'S BOW", credit="TAPPED BOW HARMONICS",
      fact=("HARPS GREW FROM", "THE HUNTER'S BOW"),
      notes=tseq(0, [(0, 57, 2*T3), (2*T3, 64, T3), (1, 57, 2*T3), (1 + 2*T3, 69, T3), (2, 64, 1), (3, 57, 2*T3), (3 + 2*T3, 64, T3),
                     (4, 69, 2*T3), (4 + 2*T3, 73, T3), (5, 69, 2*T3), (5 + 2*T3, 64, T3), (6, 57, 1), (7, 64, 2*T3), (7 + 2*T3, 57, T3)], .85)),
 dict(key="egypt", theme="egypt", year=-2500, ys="2500 BC", name="EGYPTIAN ARCHED HARP", credit="ANCIENT EGYPTIAN MODE",
      fact=("EGYPTIAN TOMBS SHOW", "HARPISTS PLAYING"),
      notes=tseq(0, [(0, 62, 1), (1, 63, 2*T3), (1 + 2*T3, 66, T3), (2, 67, 1), (3, 69, 1), (4, 67, 2*T3), (4 + 2*T3, 66, T3),
                     (5, 63, 1), (6, 62, 2)], .88) + [(0, 50, 4, .55), (4, 50, 4, .55)]),
 dict(key="irish", theme="irish", trim=.3, year=1350, ys="1300s", name="IRISH WIRE HARP", credit="OLD IRISH AIR",
      fact=("IRELAND'S NATIONAL", "SYMBOL IS A HARP"),
      notes=tseq(0, [(0, 64, 1), (1, 67, 2*T3), (1 + 2*T3, 69, T3), (2, 71, 1), (3, 74, 2*T3), (3 + 2*T3, 71, T3),
                     (4, 69, 2*T3), (4 + 2*T3, 67, T3), (5, 66, 1), (6, 64, 2)], .88) +
            [(0, 40, 2, .55), (2, 47, 2, .55), (4, 38, 2, .55), (6, 40, 2, .55)]),
 dict(key="gothic", theme="gothic", trim=.6, year=1450, ys="1400s", name="GOTHIC HARP", credit="MEDIEVAL ESTAMPIE",
      fact=("BRAY PINS GAVE IT", "A BUZZING VOICE"),
      notes=tseq(0, [(0, 62, 2*T3), (2*T3, 64, T3), (1, 65, 2*T3), (1 + 2*T3, 67, T3), (2, 69, 1), (3, 67, 2*T3), (3 + 2*T3, 65, T3),
                     (4, 64, 2*T3), (4 + 2*T3, 65, T3), (5, 64, 2*T3), (5 + 2*T3, 60, T3), (6, 62, 2)], .88) +
            [(0, 38, 4, .55), (4, 36, 2, .55), (6, 38, 2, .55)]),
 dict(key="triple", theme="triple", year=1630, ys="1600s", name="TRIPLE HARP", credit="BAROQUE GIGUE",
      fact=("THREE ROWS OF", "STRINGS FOR SHARPS"),
      notes=arp(0, [67, 70, 74], .85, .6) + tseq(0, [(1, 79, 2*T3), (1 + 2*T3, 74, T3)], .88) + arp(2, [75, 74, 72], .85, .6) +
            tseq(0, [(3, 70, 2*T3), (3 + 2*T3, 69, T3)], .88) + arp(4, [70, 67, 70], .85, .6) + tseq(0, [(5, 72, 2*T3), (5 + 2*T3, 70, T3)], .88) +
            arp(6, [69, 67, 66], .85, .6) + [(7, 67, 1, .9), (0, 43, 2, .55), (2, 50, 2, .55), (4, 48, 2, .55), (6, 50, 1, .55), (7, 43, 1, .55)]),
 dict(key="erard", theme="erard", year=1810, ys="1810", name="ERARD PEDAL HARP", credit="ROMANTIC HARP CADENZA",
      fact=("ERARD'S 7 PEDALS", "UNLOCK EVERY KEY"),
      notes=arp(0, [50, 54, 57, 62, 66, 69, 74, 78, 81, 86, 81, 78], .7, 1.2) + arp(4, [85, 81, 79, 76, 73, 69, 67, 64, 61], .7, 1.2) +
            [(7, [62, 66, 69, 74, 78], 1.2, .92), (0, 38, 3, .6), (4, 45, 3, .6)]),
 dict(key="paraguay", theme="paraguay", year=1945, ys="1940s", name="PARAGUAYAN HARP", credit="PARAGUAYAN POLCA",
      fact=("PARAGUAY'S NATIONAL", "INSTRUMENT"),
      notes=[(b, ms, d, .85) for b, ms, d in (
          (0, [76, 73], 2*T3), (2*T3, [78, 74], T3), (1, [76, 73], 2*T3), (1 + 2*T3, [73, 69], T3), (2, [74, 71], 2*T3), (2 + 2*T3, [76, 73], T3),
          (3, [74, 71], 2*T3), (3 + 2*T3, [71, 68], T3), (4, [73, 69], 2*T3), (4 + 2*T3, [71, 68], T3), (5, [69, 64], 2*T3), (5 + 2*T3, [71, 68], T3),
          (6, [68, 64], 1))] + [(7, E7_, 1, .9), (0, 45, 1, .55), (2, 40, 1, .55), (4, 45, 1, .55), (6, 40, 1, .55)]),
 dict(key="jazz", theme="jazz", trim=.4, year=1957, ys="1950s", name="JAZZ HARP", credit="IN THE STYLE OF DOROTHY ASHBY",
      fact=("DOROTHY ASHBY MADE", "THE HARP SWING"),
      notes=tseq(0, [(0, 65, 2*T3), (2*T3, 68, T3), (1, 69, 2*T3), (1 + 2*T3, 72, T3), (2, 75, 1), (3, 72, 2*T3), (3 + 2*T3, 69, T3),
                     (4, 70, 2*T3), (4 + 2*T3, 68, T3), (5, 65, 1), (7, 65, 1)], .88) +
            [(6, [63, 69, 72, 77], 1, .9), (0, 41, 2, .55), (2, 48, 2, .55), (4, 46, 2, .55), (6, 41, 2, .55)]),
 dict(key="electric", theme="intro", year=2005, ys="2000s", name="ELECTRIC HARP", credit="MODERN HARP POP",
      fact=("SOLID-BODY HARPS", "PLUG RIGHT IN"),
      notes=arp(0, [59, 62, 66, 71, 74, 78, 74, 71, 66, 62, 59, 62]) + arp(4, [55, 59, 62, 67, 71, 74, 71, 67, 62, 59, 55, 59]) +
            arp(8, [57, 61, 64, 69, 73, 76, 73, 69, 64, 61, 64, 69]) + [(0, 35, 3, .6), (4, 43, 3, .6), (8, 45, 3, .6)] +
            tseq(0, [(0, 83, 2), (2, 81, 1), (3, 78, 1), (4, 79, 2), (6, 78, 1), (7, 74, 1), (8, 76, 2), (10, 78, 1), (11, 81, 1)], .92) +
            [(12, [38, 50, 57, 62, 66, 69, 74, 78, 86], 4.3, .95)]),
]

# ---------------------------------------------------------------- strings
LET = {0: 0, 1: 0, 2: 1, 3: 2, 4: 2, 5: 3, 6: 3, 7: 4, 8: 4, 9: 5, 10: 6, 11: 6}
def dia(m): return (m//12)*7 + LET[m % 12]
def string_of(m, lo, n):
    i = dia(m) - dia(lo)
    while i < 0: i += 7
    while i >= n: i -= 7
    return max(0, i)
GUT, CRED, FBLU, WIRE, SKIN = (236, 226, 200), (222, 54, 54), (40, 44, 110), (236, 196, 96), (230, 186, 150)
def str_col(c, i):
    if c.get("scol"): return c["scol"]
    let = (dia(c["lo"]) + i) % 7
    return CRED if let == 0 else (FBLU if let == 3 else GUT)

def curve(p0, p1, bulge, u):
    p0, p1 = np.asarray(p0, float), np.asarray(p1, float); d = p1 - p0; nrm = np.array([-d[1], d[0]])/np.hypot(*d)
    return p0 + d*u + nrm*bulge*math.sin(math.pi*u)
def tube_path(g, pts, r0, r1, col, n=7, spec=.3, z=0.):
    for k in range(len(pts) - 1):
        ra, rb = lerp(r0, r1, k/(len(pts) - 1)), lerp(r0, r1, (k + 1)/(len(pts) - 1))
        T.tube(g, (*pts[k], z), (*pts[k + 1], z), ra, rb, col=col, n=n, spec=spec)
def box_at(g, p, s, col, emit=False):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit)

HARPS = {   # frame: column foot/top, column bulge, shoulder, soundbox foot, radii, neck S-amplitude, strings
 "modern":   dict(col=(226, 186, 96), box=(214, 160, 104), neck=(200, 146, 90), h=1.42, colx=-.42, bulge=0., sb0=(-.3, .14), sh=(.4, 1.02),
                  rb=(.13, .055), namp=.05, n=33, lo=36, pedals=True, capital=True),
 "erard":    dict(col=(230, 192, 104), box=(188, 112, 60), neck=(176, 110, 60), h=1.4, colx=-.42, bulge=0., sb0=(-.3, .14), sh=(.4, 1.0),
                  rb=(.12, .05), namp=.05, n=33, lo=36, pedals=True, capital=True, paint=(40, 90, 60)),
 "jazz":     dict(col=(232, 196, 110), box=(36, 30, 34), neck=(40, 32, 36), h=1.42, colx=-.42, bulge=0., sb0=(-.3, .14), sh=(.4, 1.02),
                  rb=(.13, .055), namp=.05, n=33, lo=36, pedals=True, capital=True, mic=True),
 "irish":    dict(col=(112, 70, 40), box=(126, 80, 46), neck=(112, 70, 40), h=1.12, colx=-.36, bulge=.12, sb0=(-.24, .14), sh=(.34, .86),
                  rb=(.13, .08), namp=.07, n=24, lo=43, scol=WIRE, rcol=.04, knots=True),
 "gothic":   dict(col=(206, 156, 96), box=(196, 146, 88), neck=(196, 146, 88), h=1.46, colx=-.32, bulge=.03, sb0=(-.22, .14), sh=(.28, 1.1),
                  rb=(.07, .04), namp=.09, n=24, lo=48, rcol=.02, bray=True, crown=True),
 "triple":   dict(col=(120, 70, 40), box=(150, 90, 48), neck=(120, 70, 40), h=1.44, colx=-.4, bulge=.02, sb0=(-.28, .14), sh=(.38, 1.0),
                  rb=(.11, .05), namp=.06, n=27, lo=43, rows=3, gild=(236, 196, 96)),
 "paraguay": dict(col=(206, 150, 90), box=(220, 168, 104), neck=(206, 150, 90), h=1.36, colx=-.38, bulge=.05, sb0=(-.26, .14), sh=(.38, .98),
                  rb=(.11, .05), namp=.07, n=30, lo=40, rcol=.025, holes=True, scol=(240, 236, 226), tail=True),
 "electric": dict(col=(34, 34, 44), box=(40, 40, 52), neck=(34, 34, 44), h=1.38, colx=-.4, bulge=0., sb0=(-.28, .14), sh=(.38, 1.0),
                  rb=(.045, .035), namp=.05, n=33, lo=36, scol=(150, 240, 255), led=True, rcol=.026),
}
ANCH = np.array([.55, .55, .5])

def frame_harp(g, c, t, ev, fx):
    colx, h = c["colx"], c["h"]; sb0, sh = np.array(c["sb0"]), np.array(c["sh"]); r0, r1 = c["rb"]
    crown = np.array([colx, h]); rc = c.get("rcol", .034)
    # column
    col_pts = [curve((colx, .12), crown, c["bulge"], u) for u in np.linspace(0, 1, 7)]
    tube_path(g, col_pts, rc, rc*.9, c["col"], spec=.7)
    if c.get("capital"):
        g.box(colx - .06, colx + .06, h - .02, h + .07, -.05, .05, c["col"], spec=.8)
        g.box(colx - .045, colx + .045, .1, .2, -.045, .045, c["col"], spec=.8)
    if c.get("crown"): T.tube(g, (colx, h, 0), (colx - .05, h + .12, 0), .03, .004, col=c["neck"], n=6)
    # soundbox (top surface follows the soundboard line)
    d = sh - sb0; L = np.hypot(*d); dn = d/L; nrm = np.array([-dn[1], dn[0]])
    a0, a1 = sb0 - nrm*r0*.8, sh - nrm*r1*.8
    T.tube(g, (*a0, 0), (*a1, 0), r0, r1, col=c["box"], n=6, spec=.3)
    g.face(S.ring((*a0, 0), (*dn, 0), r0, 6), dim(c["box"], .8), ref=(*a1, 0))
    if c.get("paint"):
        for u in (.2, .45, .7):
            p = a0 + (a1 - a0)*u; T.tube(g, (*(p - dn*.02), 0), (*(p + dn*.02), 0), lerp(r0, r1, u)*1.05, col=c["paint"], n=6)
    if c.get("holes"):
        for u in (.25, .5, .72):
            p = a0 + (a1 - a0)*u; g.face(S.ring((p[0], p[1], lerp(r0, r1, u)*.9 + .002), (0, 0, 1), lerp(r0, r1, u)*.35, 8), (30, 20, 12), ref=(p[0], p[1], 0))
    # neck: S-curve from crown to shoulder
    def yn(x):
        u = clamp((x - colx)/(sh[0] + .06 - colx)); return lerp(h, sh[1] + .05, u) + c["namp"]*math.sin(2*math.pi*u)
    nk = [(x, yn(x)) for x in np.linspace(colx, sh[0] + .06, 10)]
    tube_path(g, nk, .04, .035, c["neck"], spec=.4)
    if c.get("tail"): T.tube(g, (sh[0] + .06, yn(sh[0] + .06), 0), (sh[0] + .16, sh[1] + .16, 0), .03, .012, col=c["neck"], n=6)
    # base + pedals
    g.box(colx - .07, sb0[0] + .1, 0, .12, -.13, .13, dim(c["box"], .85), spec=.2)
    if c.get("pedals"):
        for k in range(7):
            x = colx - .03 + k*.055 + (.03 if k > 2 else 0)
            g.box(x - .015, x + .015, .02, .04, .13, .2, c["col"], spec=.8)
    if c.get("knots"):
        for u in (.3, .55, .8): box_at(g, (*curve((colx, .12), crown, c["bulge"], u), .035), .018, (236, 196, 96), True)
    if c.get("gild"):
        for u in np.linspace(.1, .9, 5): box_at(g, (*(a0 + (a1 - a0)*u + nrm*r0*.2), lerp(r0, r1, u)*.9), .016, c["gild"], True)
    if c.get("led"):
        hue = [(255, 60, 200), (60, 200, 255), (140, 90, 255)]
        for k in range(len(nk) - 1):
            cl = hue[(k + int(t*6)) % 3]; pa, pb = np.array(nk[k]), np.array(nk[k + 1])
            g.line((*(pa + [0, .045]), .02), (*(pb + [0, .045]), .02), cl)
        for k in range(len(col_pts) - 1):
            cl = hue[(k + int(t*6) + 1) % 3]; g.line((*(col_pts[k] - [.04, 0]), .02), (*(col_pts[k + 1] - [.04, 0]), .02), cl)
        g.box(.42, .58, 0, .12, -.08, .08, (30, 30, 36), spec=.3); g.box(.44, .56, .03, .1, .081, .082, (60, 220, 255), emit=True)
        cab = [(sb0[0] + .1, .04, .05), (.2, .01, .12), (.44, .05, .06)]
        for p_, q_ in zip(cab, cab[1:]): g.line(p_, q_, (20, 20, 24))
    if c.get("mic"):
        S.rod(g, (.4, 0, .45), (.25, .62, .3), .01); g.ngon(.4, .004, .45, .12, (40, 38, 46), n=8)
        g.box(.2, .3, .6, .74, .26, .34, (190, 194, 204), spec=.8)
    # strings
    n = c["n"]; xs = np.linspace(colx + .09, sh[0] - .06, n)
    def yb(x): return sb0[1] + (x - sb0[0])*d[1]/d[0] + .01
    tops = [yn(x) - .035 for x in xs]; bots = [yb(x) for x in xs]
    active = {}
    for age, m, v, dur, tp, cs in ev:
        if age < min(dur + .6, 1.8): active.setdefault(string_of(m, c["lo"], n), []).append((age, m, v, dur))
    FL = F.V.FLIP[0]; rows = c.get("rows", 1)
    for i, x in enumerate(xs):
        B, N = np.array([x, bots[i], 0.]), np.array([x, tops[i], 0.]); base = str_col(c, i)
        if i in active:
            age, m, v, dur = min(active[i]); amp = .012*math.exp(-age*4) + .0015
            col = HOTSTR if age < .4 else mix(HOTSTR, base, clamp((age - .4)/.8))
            pts = [B*(1 - u) + N*u + np.array([amp*math.sin(math.pi*u)*FL, 0, 0]) for u in np.linspace(0, 1, 9)]
            for a, b in zip(pts, pts[1:]): g.line(a, b, col)
        else:
            g.line(B, N, dim(base, .9) if c.get("led") is None else base)
        for rz in ((-.022, .022) if rows == 3 else ()):
            g.line(B + [.006, 0, rz], N + [.006, 0, rz], dim(GUT, .8))
        if c.get("bray"): g.box(x - .006, x + .006, bots[i] + .01, bots[i] + .03, -.006, .012, (60, 40, 24))
    # hands + fx
    recent = sorted((e for e in ev if e[0] < .3), key=lambda e: e[1])
    hands = [recent[0], recent[-1]] if len(recent) > 1 else recent
    for age, m, v, dur, tp, cs in hands:
        i = string_of(m, c["lo"], n); p = np.array([xs[i], lerp(bots[i], tops[i], .36), 0.])
        q = p + np.array([.02 + .05*age/.3, -.01, .04])
        g.box(q[0] - .022, q[0] + .022, q[1] - .03, q[1] + .03, q[2] - .016, q[2] + .016, SKIN)
    for age, m, v, dur, tp, cs in ev:
        i = string_of(m, c["lo"], n); p = (xs[i], lerp(bots[i], tops[i], .36), .03)
        if age < .06 and v >= .6: fx.append(("spark", p, age))
        if tp and v >= .8 and age < .9: fx.append(("note", (xs[i] + .08, tops[i] + .12, 0), age))

def bow(g, t, ev, fx, s):
    wood, dark = (150, 96, 50), (96, 60, 30)
    P0, P1 = np.array([-.1, .26, 0.]), np.array([-.1, 1.34, 0.])
    pts = [P0 + (P1 - P0)*u + np.array([.34*math.sin(math.pi*u), 0, 0]) for u in np.linspace(0, 1, 11)]
    for a, b in zip(pts, pts[1:]): T.tube(g, a, b, .022, col=wood, n=6, spec=.2)
    for p in (P0, P1): box_at(g, p, .026, dark)
    S.rod(g, (.17, 0, -.08), (.2, .8, -.04), .014, (70, 50, 34)); g.ngon(.17, .004, -.08, .14, (60, 44, 30), n=8)
    live = [e for e in ev if e[0] < e[3] + .3]
    if live:
        age = min(live)[0]; amp = .014*math.exp(-age*5) + .002
        sp = [P0*(1 - u) + P1*u + np.array([amp*math.sin(math.pi*u)*F.V.FLIP[0], 0, 0]) for u in np.linspace(0, 1, 9)]
        col = HOTSTR if age < .3 else mix(HOTSTR, GUT, clamp((age - .3)/.6))
        for a, b in zip(sp, sp[1:]): g.line(a, b, col)
    else:
        g.line(P0, P1, GUT)
    hp = np.array([-.1, .62, .02])
    hits = [(s["t0"]*0 + e[0], hp) for e in s["ev"]]
    tip, lift = S.stick_pos(hits, t, .1, .2)
    if tip is not None: S.draw_stick(g, tip + [lift*.6, lift*.3, .02], ANCH + [-.1, .1, 0], .42, (210, 180, 130))
    for age, m, v, dur, tp, cs in ev:
        if age < .06: fx.append(("spark", tuple(hp + [0, 0, .03]), age))
        if tp and age < .9: fx.append(("note", (.12, 1.2, 0), age))
    for k in range(3): g.line((-.1, P1[1] - .02 - .03*k, .01), (-.06, P1[1] - .05 - .03*k, .01), (200, 40, 40))

def egypt(g, t, ev, fx, s):
    wood, blue, gold = (176, 112, 56), (40, 84, 176), (236, 196, 90)
    T.tube(g, (-.44, .26, 0), (.24, .3, 0), .075, .05, col=wood, n=7, spec=.2)
    for u in (.2, .45, .7): x = lerp(-.44, .24, u); T.tube(g, (x - .02, lerp(.26, .3, u), 0), (x + .02, lerp(.26, .3, u), 0), lerp(.075, .05, u)*1.06, col=blue if u != .45 else gold, n=7)
    for x in (-.32, .12): g.box(x - .03, x + .03, 0, .22, -.04, .04, dim(wood, .8))
    def yt(x): return .26 + 1.04*math.sqrt(max(0, 1 - ((x + .44)/.68)**2))
    nk = [(-.44 + .68*math.cos(a), .26 + 1.04*math.sin(a)) for a in np.linspace(0, math.pi/2, 10)]
    tube_path(g, nk, .04, .03, wood, spec=.2)
    g.box(-.49, -.39, 1.28, 1.38, -.04, .04, gold, spec=.7); g.box(-.47, -.41, 1.38, 1.42, -.03, .03, blue)
    n = 10; xs = np.linspace(-.36, .12, n)
    c = dict(lo=50, n=n, scol=GUT)
    active = {}
    for age, m, v, dur, tp, cs in ev:
        if age < min(dur + .6, 1.8): active.setdefault(string_of(m, 50, n), []).append((age, m, v, dur))
    for i, x in enumerate(xs):
        B, N = np.array([x, .33, 0.]), np.array([x, yt(x) - .04, 0.])
        if i in active:
            age = min(active[i])[0]; amp = .012*math.exp(-age*4) + .0015
            col = HOTSTR if age < .4 else mix(HOTSTR, GUT, clamp((age - .4)/.8))
            pts = [B*(1 - u) + N*u + np.array([amp*math.sin(math.pi*u)*F.V.FLIP[0], 0, 0]) for u in np.linspace(0, 1, 9)]
            for a, b in zip(pts, pts[1:]): g.line(a, b, col)
        else: g.line(B, N, GUT)
    for age, m, v, dur, tp, cs in ev:
        i = string_of(m, 50, n)
        if age < .06: fx.append(("spark", (xs[i], .55, .03), age))
        if tp and v >= .8 and age < .9: fx.append(("note", (xs[i] + .08, yt(xs[i]) + .1, 0), age))
    recent = [e for e in ev if e[0] < .3]
    if recent:
        age, m = min(recent)[:2]; i = string_of(m, 50, n); q = np.array([xs[i] + .02 + .05*age/.3, .52, .04])
        g.box(q[0] - .022, q[0] + .022, q[1] - .03, q[1] + .03, q[2] - .016, q[2] + .016, SKIN)

SHIFT = np.array([.07, 0, 0])
def build(g, t, ev, fx, s):
    k = s["key"]; f0, l0, x0 = len(g.F), len(g.L), len(fx)
    if k == "bow": bow(g, t, ev, fx, s)
    elif k == "egypt": egypt(g, t, ev, fx, s)
    else: frame_harp(g, HARPS[k], t, ev, fx)
    for i in range(f0, len(g.F)): P_, *rest = g.F[i]; g.F[i] = (P_ + SHIFT, *rest)
    for i in range(l0, len(g.L)): a, b, *rest = g.L[i]; g.L[i] = (a + SHIFT, b + SHIFT, *rest)
    for i in range(x0, len(fx)): f = fx[i]; fx[i] = (f[0], tuple(np.asarray(f[1], float) + SHIFT)) + tuple(f[2:])

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def harp_tone(f, dur, v, bright=.5, sus=1., body=.3, buzz=0., wire=False, pos=.28):
    ring = min(2.6, max(1.3, dur + 1.0)); n = int(ring*SR); t = _t(n); x = np.zeros(n)
    for k in range(1, 26):
        fk = f*k*(1 + (.00015*k*k if wire else 0))
        if fk > 9000: break
        a = (abs(math.sin(math.pi*k*pos)) + .05)/k**(1.5 - .8*bright)
        dk = (1.0 + .5*(k - 1))*(.55 + f/650)/sus
        x += a*np.sin(2*np.pi*fk*t + k*.7)*np.exp(-t*dk)
    x = x + body*np.sin(2*np.pi*f*t)*np.exp(-t*(.8 + f/900))
    x *= np.minimum(1, t/.0015)*np.minimum(1, (ring - t)/.15)
    if buzz:
        x = x/(np.abs(x).max() + 1e-9); x = (1 - buzz)*x + buzz*np.tanh(5*x)*.7
    return x*v
TONES = {"modern": dict(bright=.55, sus=1.1, body=.3), "egypt": dict(bright=.3, sus=.8, body=.4), "irish": dict(bright=.95, sus=1.8, body=.15, wire=True),
         "gothic": dict(bright=.6, sus=.9, body=.25, buzz=.45), "triple": dict(bright=.55, sus=1.0, body=.3),
         "erard": dict(bright=.5, sus=1.1, body=.35), "paraguay": dict(bright=.75, sus=.9, body=.2), "jazz": dict(bright=.4, sus=1.1, body=.4),
         "electric": dict(bright=.65, sus=1.2, body=.3)}
def bow_tone(m, dur, v):
    f0 = midi_hz(45); f = midi_hz(m); n = int((dur + .6)*SR); t = _t(n)
    x = harp_tone(f0, .2, .5, bright=.3, sus=.5)[:n]; x = np.pad(x, (0, n - len(x)))
    x = x + .9*np.sin(2*np.pi*f*t)*np.exp(-t*3.5)*(1 - np.exp(-t*80)) + .3*np.sin(2*np.pi*2*f*t)*np.exp(-t*6)
    tick = int(.004*SR); x[:tick] += np.sin(2*np.pi*1800*t[:tick])*.4
    return x*v
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    if key == "bow": return bow_tone(m, dur, v)
    x = harp_tone(midi_hz(m), dur, v, **TONES[key])*(.8 if cs > 2 else 1.)
    if key == "electric":
        dl = int(2*T3*BEAT_S*SR); y = np.zeros(len(x) + 2*dl); y[:len(x)] += x; y[dl:dl + len(x)] += .3*x; y[2*dl:2*dl + len(x)] += .12*x
        x = np.tanh(1.3*y)
    return x

# ---------------------------------------------------------------- backing: 12/8 Celtic groove (bodhran, shaker, kick, snaps), sub bass, soft pad
def bod(f0, dec, skin):
    def fn(v):
        n = int(.5*SR); t = _t(n); fr = f0*(1 + .6*np.exp(-t*40))
        return (np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*dec) + D.bp(D.nz(n), 150, 1200)*np.exp(-t*60)*skin)*v
    return fn
def snap(v):
    n = int(.12*SR); t = _t(n)
    return (D.bp(D.nz(n), 1200, 5000)*np.exp(-t*90)*.7 + np.sin(2*np.pi*900*t)*np.exp(-t*80)*.3)*v
KIT = {"bodL": bod(82, 9, .25), "bodH": bod(150, 16, .3), "kick": lambda v: D.kick_s(48, 110, 6, .08, .5, 1.4, .3)*v,
       "snap": snap, "shaker": A.shaker, "crash": lambda v: D.metal_s([480, 760, 1210, 1890, 2770, 4000, 5600], 2.0, .25, 2.0, 1800)*v}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("snap", "bodL")
BODV = [1., .35, .5, .7, .35, .5, .9, .35, .5, .75, .4, .6]
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "bodL", 1.)]; continue
        ev += [(T0 + j*T3*Q, "shaker", .45 if j % 3 == 0 else .28) for j in range(12)]
        ev += [(T0 + j*T3*Q, "bodL" if j % 3 == 0 else "bodH", BODV[j]*(.6 if b == 0 else 1.)) for j in range(12)]
        if b == 0: continue
        ev += [(T0, "kick", .95), (T0 + 2*Q, "kick", .85), (T0 + (3 + 2*T3)*Q, "kick", .5), (T0 + Q, "snap", .7), (T0 + 3*Q, "snap", .7)]
        if b in (4, 20): ev.append((T0, "crash", .6))
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
        out += [(T0, _root(_chord_at(b, 0)), 1.6*c.BEAT, .9), (T0 + 2*c.BEAT, _root(_chord_at(b, 2)), 1.3*c.BEAT, .8),
                (T0 + (3 + 2*T3)*c.BEAT, _root(_chord_at(b + 1, 0)), .3*c.BEAT, .6)]
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .3)*SR); t = _t(n); ph = 2*np.pi*f*t
    env = np.minimum(1, t/.01)*np.exp(-t*.9)*np.where(t < dur, 1, np.exp(-(t - dur)/.08))
    return np.tanh(1.3*(np.sin(ph) + .25*np.sin(2*ph))*env)*v
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); t = _t(n); x = np.zeros(n)
    for m in notes:
        f = midi_hz(m)
        for det in (-1.5, 1.5):
            for k in (1, 2, 3): x += np.sin(2*np.pi*(f + det)*k*t + k)/k**1.8
    env = np.minimum(1, t/.12)*np.where(t < dur, 1, np.exp(-(t - dur)/.3))
    return x*env*v*.2
MIX = dict(drums=-22, bass=-25, pad=-31, verb=-20, verb_len=2.2)

# ---------------------------------------------------------------- background: aurora over a mirror lake
THEMES = {   # sky top, sky bottom, aurora low, aurora high, mountains, lake tint
 "intro":    ((4, 8, 30), (22, 42, 84), (80, 255, 170), (170, 90, 255), (14, 22, 46), (10, 24, 54)),
 "bow":      ((12, 4, 4), (74, 30, 22), (255, 150, 60), (255, 60, 90), (34, 16, 12), (44, 18, 12)),
 "egypt":    ((10, 10, 40), (70, 50, 96), (255, 220, 100), (90, 220, 255), (46, 34, 62), (24, 20, 56)),
 "irish":    ((2, 16, 14), (20, 70, 62), (70, 255, 130), (60, 170, 255), (8, 32, 26), (6, 32, 32)),
 "gothic":   ((10, 6, 26), (62, 42, 84), (210, 130, 255), (255, 130, 200), (26, 18, 42), (22, 16, 44)),
 "triple":   ((20, 10, 4), (96, 62, 30), (255, 210, 90), (255, 120, 50), (40, 24, 12), (44, 26, 14)),
 "erard":    ((6, 12, 34), (56, 86, 146), (150, 230, 255), (250, 250, 255), (20, 30, 58), (16, 30, 66)),
 "paraguay": ((30, 4, 20), (146, 44, 64), (255, 130, 70), (255, 236, 100), (46, 12, 26), (60, 14, 30)),
 "jazz":     ((4, 4, 12), (40, 30, 64), (90, 150, 255), (255, 100, 190), (14, 12, 28), (14, 12, 34)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[4], T_[1], .45))
HZ = 262
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=HZ) for k, v in THEMES.items()}
XS = np.arange(RW)
_r = np.random.default_rng(5)
STARS = np.zeros((RH, RW), bool); STAR_PH = _r.random((RH, RW))*6.28
for _ in range(110): STARS[_r.integers(0, 200), _r.integers(0, RW)] = True
def _ridge(lo, hi, step, seed):
    rr = np.random.default_rng(seed); kx = np.arange(0, RW + step, step); ky = rr.uniform(lo, hi, len(kx))
    return np.interp(XS, kx, ky)
RF, RN = _ridge(200, 236, 34, 3), _ridge(228, 254, 22, 4)
MTF = (YY >= RF[None, :]) & (YY < HZ); MTN = (YY >= RN[None, :]) & (YY < HZ)
SNOW = MTF & (YY < RF[None, :] + 3 + (216 - RF[None, :])*.5) & (RF[None, :] < 216) & ~MTN
PINES = np.zeros((RH, RW), bool)
for px, ph in ((10, 30), (24, 22), (36, 34), (232, 26), (246, 36), (260, 24)):
    for y in range(HZ - ph, HZ):
        w = (y - (HZ - ph))*.28 + 1; PINES[y, max(0, int(px - w)):min(RW, int(px + w) + 1)] = True
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
LY = np.arange(HZ, RH)
SRCY = np.clip(2*HZ - LY - 1, 0, HZ - 1)
DEPTH = ((LY - HZ)/(RH - HZ))[:, None, None]
UPY = YY[:HZ]

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return np.array(mix(A_[i], B_[i], kb), float)
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    eq, flash = x["eq"], max([math.exp(-a*5) for a in x["ages"]], default=0.)
    tw = np.sin(STAR_PH + T_*2.5) > -.3; bg[STARS & tw] = bg[STARS & tw]*.3 + 220*.7
    I = np.zeros((HZ, RW))
    for i, (base, amp, hh, sp) in enumerate(((108, 16, 70, .5), (138, 13, 56, -.4), (166, 9, 42, .3))):
        yc = base + amp*np.sin(XS*.025 + T_*sp + i*2) + 6*np.sin(XS*.07 - T_*sp*1.7)
        band = eq[np.clip(XS*14//RW, 0, 13)]
        rays = .55 + .45*np.sin(XS*.33 + T_*1.4*(i + 1) + 3*np.sin(XS*.05 + i))
        k = (.3 + .7*band + .35*flash)*(1 - .22*i)
        dd = UPY - yc[None, :]
        prof = np.where(dd < 0, np.clip(1 + dd/hh, 0, 1)**2, np.exp(-dd/3.))
        I = np.maximum(I, prof*(rays*k)[None, :])
    Iq = np.clip(np.floor(I*5 + BAY[:HZ])/5, 0, 1)[..., None]
    gy = np.clip((200 - UPY)/120, 0, 1)[..., None]
    acol = col(2)*(1 - gy) + col(3)*gy
    bg[:HZ] = bg[:HZ]*(1 - .8*Iq) + acol*.8*Iq
    mc = col(4); bg[MTF] = mc*.75 + col(1)*.25; bg[SNOW] = mc*.45 + col(3)*.25 + 150*.3; bg[MTN] = mc; bg[PINES] = mc*.6
    dx = np.round(np.sin(LY*.32 + T_*2.2)*(.6 + (LY - HZ)/90)).astype(int)
    src = bg[SRCY[:, None], (XS[None, :] + dx[:, None]) % RW]
    tint = col(5)
    lake = src*(.62 - .25*DEPTH) + tint*(.38 + .25*DEPTH)
    lake[(LY - HZ) % 6 == 0] *= .85
    bg[HZ:] = lake
    for a in x["ages"]:
        hsh = ((int(round((x["T"] - a)*100))*2654435761) % 997)/997.
        cx, cy = 20 + hsh*230, HZ + 14 + ((hsh*7.3) % 1)*150
        rx = 5 + a*70; ry = rx*.24
        ring = np.abs(((XX[HZ:] - cx)/rx)**2 + ((YY[HZ:] - cy)/ry)**2 - 1) < .14 + .05/(1 + a*4)
        k = (1 - a/1.6)*.7
        bg[HZ:][ring] = bg[HZ:][ring]*(1 - k) + col(2)*k
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

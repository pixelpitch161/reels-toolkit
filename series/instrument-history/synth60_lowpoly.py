"""Pixel Pitch: history of the synthesizer, 60-second cut.  python3 synth60_lowpoly.py [out.mp4] | --still t ...

Modern Korg minilogue xd showcase, then 1906 Telharmonium -> 1995 Eurorack -> back to the minilogue xd.
Backdrop: inside a circuit board (copper traces that carry a light pulse on every note, components standing like
buildings, an oscilloscope that draws the current instrument's real waveform). Beat: 808 electro at 116 BPM.
Models were built from reference photos (maker pages, museum collections, archive photos).
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
import cello_lowpoly as C
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, ease, midi_hz, SR, RW, RH, CX, YY, XX, BAY, HOT

P._G.setdefault("%", "##..." "##..#" "...#." "..#.." ".#..." "#..##" "...##")

OUT, TAG = "synth60_lowpoly.mp4", "sx"
BPM = 116
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.0, -14, 12, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE SYNTHESIZER", "120 YEARS", "TODAY", 1890
INTRO_LINES = (("THE FIRST SYNTH,", "WEIGHED 200 TONS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("120 YEARS FROM", "THE TELHARMONIUM")
OUTRO_CREDIT = "MODERN ELECTRO"
LIGHTING = True

# ---------------------------------------------------------------- music (A minor, 808 electro)
CH = {"Am": (45, (57, 60, 64)), "F": (41, (57, 60, 65)), "C": (36, (55, 60, 64)), "G": (43, (55, 59, 62)),
      "E": (40, (56, 59, 64)), "Dm": (38, (57, 62, 65))}
BARS = ["Am", "F", "C", "G",                                                     # showcase
        "Am", "E", "F", "E", "Dm", "Am", "F", "G", "Am", "Am", "Am", "E",        # 1906 .. 1964
        "F", "G", "Am", "F", "Am", "Am", "F", "G", "Am", "Am", "Am", "F",        # 1970 .. 2020s
        "G", "Am"]                                                               # outro, final hit
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
TONES = {"Am": (57, 60, 64, 69), "F": (53, 57, 60, 65), "C": (55, 60, 64, 67), "G": (55, 59, 62, 67)}
ARP = (0, 1, 2, 3, 1, 2, 3, 2, 0, 1, 2, 3, 2, 3, 1, 2)
ACC = {0: .95, 3: .85, 6: .9, 10: .9, 12: .8}
def arp_bar(b0, ch):
    return [(b0 + .25*k, TONES[ch][i] + (12 if k in (6, 14) else 0), .22, ACC.get(k, .62)) for k, i in enumerate(ARP)]
SHOW = arp_bar(0, "Am") + arp_bar(4, "F") + arp_bar(8, "C") + arp_bar(12, "G")
TEL = (tn([(0, 76, 1), (1, 77, .5), (1.5, 76, .5), (2, 72, 1), (3, 69, 1), (4, 71, 1), (5, 74, 1), (6, 80, 1), (7, 76, 1)], .86) +
       tn([(1, (60, 64), .45), (2, (60, 64), .45), (4, (62, 68), .45), (5, (62, 68), .45), (7, (62, 68), .45)], .4))
THER = tn([(0, 72, 1.5), (1.5, 77, 1), (2.5, 76, .5), (3, 74, 1), (4, 71, 1.5), (5.5, 68, .5), (6, 76, 2)], .86)
ONDES = tn([(0, 69, .5), (.5, 74, .5), (1, 77, .5), (1.5, 81, 1.5), (3, 79, .5), (3.5, 76, .5), (4, 72, 1), (5, 76, .5),
            (5.5, 79, .5), (6, 84, 1), (7, 81, 1)], .84)
NOVA = (tn([(0, (53, 57, 60), 3.9), (4, (55, 59, 62), 3.9)], .5) +
        tn([(0, 72, 1.5), (1.5, 74, .5), (2, 77, 2), (4, 79, 1.5), (5.5, 77, .5), (6, 74, 1), (7, 71, 1)], .86))
RCA = tn([(0, 69, .25), (.5, 82, .25), (1.25, 63, .25), (1.75, 76, .5), (2.5, 61, .25), (3, 84, .25), (3.75, 66, .25),
          (4.5, 79, .5), (5, 62, .25), (5.75, 77, .25), (6.5, 59, .25), (7, 80, .75)], .8)      # a twelve-tone row
MOOG = tn([(0, 69, .25), (.25, 72, .25), (.5, 76, .25), (.75, 72, .25), (1, 69, .25), (1.25, 72, .25), (1.5, 76, .25), (1.75, 81, .25),
           (2, 79, .5), (2.5, 77, .5), (3, 76, .5), (3.5, 74, .5),
           (4, 68, .25), (4.25, 71, .25), (4.5, 74, .25), (4.75, 71, .25), (5, 68, .25), (5.25, 71, .25), (5.5, 74, .25), (5.75, 80, .25),
           (6, 83, .5), (6.5, 80, .5), (7, 76, .5), (7.5, 71, .5)], .82)
MINI = tn([(0, 72, .5), (.5, 77, .5), (1, 81, 1), (2, 79, .5), (2.5, 77, .5), (3, 76, 1), (4, 74, .5), (4.5, 79, .5), (5, 83, 1.5),
           (6.5, 81, .5), (7, 79, 1)], .86)
_AM, _FA = (57, 60, 64, 69), (57, 60, 65, 69)
PRO = tn([(0, _AM, .4), (.75, _AM, .4), (1.5, _AM, .4), (2.5, (59, 62, 64, 67), .4), (3, (60, 64, 67, 72), .9),
          (4, _FA, .4), (4.75, _FA, .4), (5.5, _FA, .4), (6.5, (60, 64, 67, 72), .4), (7, (59, 62, 67, 71), .9)], .8)
ACID_NOTES = [45, 45, 57, 45, 48, None, 45, 55, 57, 45, 60, 45, 43, 45, 52, 55,
              45, 45, 57, 45, 48, 45, 55, 57, 45, 60, 58, 57, 45, 57, 45, 52]
ACID_ACC, ACID_SLIDE = {0, 6, 10, 16, 20, 25, 29}, {2, 14, 22, 26}
ACID = [(k*.25, m, .3 if k in ACID_SLIDE else .18, .95 if k in ACID_ACC else .7) for k, m in enumerate(ACID_NOTES) if m]
DX7 = (tn([(0, (53, 57, 60, 64), 2.4), (2.5, (53, 57, 60, 64), 1.4), (4, (55, 59, 62, 67), 2.4), (6.5, (55, 59, 62, 67), 1.4)], .55) +
       tn([(0, 81, 1.5), (1.5, 79, .5), (2, 77, 1), (3, 76, 1), (4, 74, 1.5), (5.5, 76, .5), (6, 79, 2)], .86))
EURO_SEQ = [57, 69, 64, 69, 57, 67, 69, 72, 57, 69, 64, 76, 57, 67, 72, 69,
            57, 69, 64, 69, 57, 67, 69, 72, 60, 72, 67, 76, 59, 71, 67, 74]
EURO = [(k*.25, m, .2, .9 if k % 4 == 0 else .66) for k, m in enumerate(EURO_SEQ)]
LAST = arp_bar(0, "Am") + arp_bar(4, "F") + arp_bar(8, "G") + [(12, (57, 60, 64, 69), 5.0, .85)]

SCENES = [
 dict(key="modern", theme="intro", name="KORG MINILOGUE XD", credit="MODERN ELECTRO", size=3.4, notes=SHOW),
 dict(key="tel", theme="tel", year=1906, ys="1906", name="TELHARMONIUM", credit="EDWARDIAN PARLOUR WALTZ", size=.62,
      fact=("MUSIC SENT DOWN", "PHONE LINES"), notes=TEL),
 dict(key="theremin", theme="theremin", year=1920, ys="1920", name="THEREMIN", credit="SALON ROMANCE", size=.95,
      fact=("PLAYED WITHOUT", "EVER TOUCHING IT"), notes=THER),
 dict(key="ondes", theme="ondes", year=1928, ys="1928", name="ONDES MARTENOT", credit="FRENCH IMPRESSIONISM", size=.95,
      fact=("A RING ON A WIRE", "SLIDES THE PITCH"), notes=ONDES),
 dict(key="nova", theme="nova", year=1939, ys="1939", name="HAMMOND NOVACHORD", credit="1930s HOLLYWOOD SCORE", size=1.12,
      fact=("FIRST POLYPHONIC", "SYNTH FOR SALE"), notes=NOVA),
 dict(key="rca", theme="rca", year=1957, ys="1957", name="RCA MARK II", credit="1950s AVANT-GARDE", size=.5,
      fact=("1,700 VACUUM TUBES", "READ PUNCHED PAPER"), notes=RCA),
 dict(key="moog", theme="moog", year=1964, ys="1964", name="MOOG MODULAR", credit="SWITCHED-ON BAROQUE", size=1.4,
      fact=("ITS 1968 BACH LP", "WENT PLATINUM"), notes=MOOG),
 dict(key="mini", theme="mini", year=1970, ys="1970", name="MINIMOOG MODEL D", credit="70s PROG ROCK", size=2.3,
      fact=("FIRST SYNTH SOLD", "IN MUSIC STORES"), notes=MINI),
 dict(key="prophet", theme="prophet", year=1978, ys="1978", name="PROPHET-5", credit="NEW WAVE", size=1.8,
      fact=("SAVES ITS SOUNDS", "AT A BUTTON PRESS"), notes=PRO),
 dict(key="tb303", theme="tb303", year=1981, ys="1981", name="ROLAND TB-303", credit="CHICAGO ACID HOUSE", size=5.0,
      fact=("FLOPPED AS A BASS,", "BIRTHED ACID HOUSE"), notes=ACID),
 dict(key="dx7", theme="dx7", year=1983, ys="1983", name="YAMAHA DX7", credit="80s POWER BALLAD", size=1.7,
      fact=("ITS E.PIANO WAS ON", "40% OF 1986 NO.1S"), notes=DX7),
 dict(key="euro", theme="euro", year=1995, ys="1995", name="EURORACK MODULAR", credit="BERLIN TECHNO", size=2.35,
      fact=("15,000+ MODULES", "FROM 600+ MAKERS"), notes=EURO),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="KORG MINILOGUE XD", credit="MODERN ELECTRO", size=3.4,
      fact=("200 TONS THEN,", "2.8 KG TODAY"), notes=LAST),
]
TRIM = {}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- geometry helpers (metres; x right, y up, z toward the viewer)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
class Pan:
    """a flat panel: origin o, u across it (left -> right), v up it; n = outward normal (u x v)"""
    def __init__(s, o, u, v):
        s.o = np.asarray(o, float); s.u = unit(u); s.v = unit(v); s.n = unit(np.cross(s.u, s.v))
    def P(s, a, b, h=0.): return s.o + a*s.u + b*s.v + h*s.n
    def rect(s, g, a0, a1, b0, b1, col, h=.0015, emit=False, spec=0.):
        g.face([s.P(a0, b0, h), s.P(a1, b0, h), s.P(a1, b1, h), s.P(a0, b1, h)], col,
               ref=s.P((a0 + a1)/2, (b0 + b1)/2, h - 1), emit=emit, spec=spec)
    def obox(s, g, a0, a1, b0, b1, h0, h1, col, front=None, emit=False, spec=0.):
        V = [s.P(a, b, h) for h in (h0, h1) for b in (b0, b1) for a in (a0, a1)]
        ref = s.P((a0 + a1)/2, (b0 + b1)/2, (h0 + h1)/2)
        for idx, c_ in (((4, 5, 7, 6), front or col), ((0, 1, 3, 2), col), ((0, 1, 5, 4), col), ((2, 3, 7, 6), col),
                        ((0, 2, 6, 4), col), ((1, 3, 7, 5), col)):
            g.face([V[i] for i in idx], c_, ref=ref, emit=emit, spec=spec)
    def knob(s, g, a, b, r, h, col, top=None, n=6):
        S.prism(g, s.P(a, b, h/2), s.n, r, h, n, top or col, col)
    def line(s, g, a0, b0, a1, b1, col, h=.002): g.line(s.P(a0, b0, h), s.P(a1, b1, h), col)

def slab(g, x0, x1, prof, top, side, end=None, spec=0., emit=False):
    """extrude a convex (z, y) profile along x; faces turned upward get the top colour"""
    pr = np.asarray(prof, float); cz, cy = pr.mean(0); ref = ((x0 + x1)/2, cy, cz)
    for i in range(len(pr)):
        (z0, y0), (z1, y1) = pr[i], pr[(i + 1) % len(pr)]
        nz, ny = (y1 - y0), -(z1 - z0)
        if nz*((z0 + z1)/2 - cz) + ny*((y0 + y1)/2 - cy) < 0: nz, ny = -nz, -ny
        c_ = top if ny/(math.hypot(nz, ny) or 1) > .6 else side
        g.face([(x0, y0, z0), (x1, y0, z0), (x1, y1, z1), (x0, y1, z1)], c_, ref=ref, spec=spec, emit=emit)
    for x in (x0, x1): g.face([(x, y, z) for z, y in pr], end or side, ref=ref, spec=spec, emit=emit)

def fan(g, pts, center, col, ref, emit=False, two=True):
    c = np.asarray(center, float)
    for k in range(len(pts)):
        g.face([c, pts[k], pts[(k + 1) % len(pts)]], col, ref=ref, emit=emit, two=two)

def wheel(g, x, y, z, r, col=(20, 20, 22)):
    T.tube(g, (x - .006, y, z), (x + .006, y, z), r, col=col, n=10, spec=.2)

def active_of(ev): return {e[1] for e in ev if e[0] < e[3]}

def key_fx(fx, ev, km, kw, x0, ytop, zf, depth, up=.1):
    for age, m, v, dur, tp, cs in ev:
        if m not in km: continue
        x = P.key_x(km, kw, x0, m)
        if tp and v >= .6 and age < .9: fx.append(("note", (x, ytop + up, zf - depth*.5), age))
        if age < .07 and v >= .5: fx.append(("spark", (x, ytop + .012, zf - .02), age))

def last_note(s, t, ev):
    on = [e for e in ev if e[0] >= 0]
    return min(on)[1] if on else s["ev"][0][1]

def step16(t): return int(t/(BEAT_S/4)) % 16

# ---------------------------------------------------------------- 0 / 12: Korg minilogue xd (500 x 300 x 85 mm, 37 slim keys)
def minilogue(g, t, ev, fx, s):
    blk, edge, alu, wood = (24, 24, 28), (14, 14, 16), (60, 62, 68), (176, 104, 52)
    W, D = .25, .15
    slab(g, -W, W, [(D, 0), (D, .048), (-D, .048), (-D, 0)], blk, blk)
    slab(g, -W, W, [(.05, .048), (.05, .068), (-D, .086), (-D, .048)], alu, edge, edge, spec=.3)
    g.box(-W + .004, W - .004, .006, .08, -D - .012, -D, wood)                          # real-wood back panel
    for x0, x1 in ((-W, -.226), (.226, W)): g.box(x0, x1, .048, .068, .05, D, alu, spec=.3)
    act = active_of(ev)
    km, kw = P.build_keys(g, -.224, .224, .146, .07, 48, 22, act, nat=(240, 238, 232), sharp=(22, 22, 26), depth=.094)
    key_fx(fx, ev, km, kw, -.224, .07, .146, .094)
    pn = Pan((-W, .068, .05), (1, 0, 0), (0, .018, -.2))
    knob_c, knob_t = (30, 30, 34), (150, 152, 160)
    for r, b in enumerate((.072, .112, .152)):                                          # three rows of knobs
        for k in range(9):
            if r == 0 and k in (3, 4): continue
            pn.knob(g, .05 + .037*k, b, .0085, .013, knob_c, knob_t)
    for a in (.21, .225, .32, .335): pn.obox(g, a - .002, a + .002, .045, .062, 0, .012, (190, 190, 196))   # toggles
    st = step16(t)
    for k in range(16):                                                                # 16-step sequencer buttons
        a = .155 + .0165*k; on = k == st
        pn.obox(g, a, a + .011, .016, .026, 0, .004, (100, 130, 255) if on else (44, 48, 64), emit=on)
    pn.rect(g, .085, .145, .02, .036, (14, 18, 26), emit=True)                         # program display
    for k in range(6): pn.rect(g, .09 + .009*k, .096 + .009*k, .024, .032, (130, 190, 255), h=.002, emit=True)
    pn.rect(g, .405, .47, .118, .168, (6, 9, 11), emit=True)                           # OLED oscilloscope
    m = last_note(s, t, ev); ph = t*9; pts = []
    for k in range(13):
        u = k/12; w = (u*3 + ph) % 1
        pts.append((.41 + .055*u, .143 + .018*(w*2 - 1)*(.6 + .4*math.sin(m)), ))
    for (a0, b0), (a1, b1) in zip(pts, pts[1:]): pn.line(g, a0, b0, a1, b1, (220, 244, 255))
    for k in range(4): pn.obox(g, .405 + .017*k, .418 + .017*k, .03, .045, 0, .005, (70, 72, 80))
    pn.knob(g, .025, .04, .016, .004, (20, 20, 22))                                    # joystick
    T.tube(g, pn.P(.025, .04, .004), pn.P(.025, .04, .03), .0035, col=(20, 20, 22), n=6)
    S.prism(g, pn.P(.025, .04, .033), pn.n, .007, .008, 6, (40, 40, 44), (26, 26, 30))
    pn.knob(g, .06, .145, .012, .014, knob_c, knob_t)                                  # master volume

# ---------------------------------------------------------------- 1906: Telharmonium console (Telharmonic Hall photos)
def horn(g, p0, d, L, r0, R, col, segs=7):
    p0, d = np.asarray(p0, float), unit(d)
    def r(u): return r0*(R/r0)**(u**1.7)
    for k in range(segs):
        u0, u1 = k/segs, (k + 1)/segs
        T.tube(g, p0 + d*L*u0, p0 + d*L*u1, r(u0), r(u1), col=col, n=12, spec=.6, two=u1 > .55)
    g.face(S.ring(p0 + d*L*.93, d, R*.82, 12), (12, 8, 6), ref=p0, emit=True, two=True)
    return p0 + d*L

def telharmonium(g, t, ev, fx, s):
    wood, dark, slat, metal, ivory = (108, 60, 34), (66, 36, 22), (132, 82, 48), (176, 176, 184), (236, 226, 198)
    g.box(-.62, .62, 0, .66, -.2, .26, wood)                                 # keyboard desk
    g.box(-.6, .6, .66, .7, -.2, .33, dark)
    act = active_of(ev); lo = {m for m in act if m < 67}; hi = act - lo
    km, kw = P.build_keys(g, -.52, .52, .33, .725, 48, 29, lo, nat=ivory, depth=.13)
    g.box(-.55, .55, .7, .773, .08, .2, dark)                                # step up to the upper manual
    km2, kw2 = P.build_keys(g, -.52, .52, .2, .795, 48, 29, hi, nat=ivory, depth=.11)
    for age, m, v, dur, tp, cs in ev:
        if m >= 67 and tp and age < .9: fx.append(("note", (P.key_x(km2, kw2, -.52, m), .95, .15), age))
    slab(g, -.28, .28, [(.075, .8), (.09, .8), (.03, 1.04), (.015, 1.04)], dark, dark)        # music rack
    g.face([(-.2, .86, .084), (.2, .86, .084), (.2, 1.0, .049), (-.2, 1.0, .049)], (226, 220, 200), ref=(0, .9, 0))
    for x in (-.52, .52): g.box(x - .045, x + .045, .66, 2.0, -.16, -.06, dark)          # back frame posts
    for k in range(5):                                                                # horizontal expression slats
        y = 1.06 + .2*k; g.box(-.48, .48, y, y + .09, -.14, -.08, slat)
    g.box(-.6, .6, 2.0, 2.07, -.18, -.04, dark)                                       # cornice
    g.box(-.48, .48, .7, 1.0, -.16, -.1, wood)                                        # stop board
    for r_ in range(2):
        for k in range(10):
            x = -.4 + .089*k; on = (k + r_*3 + int(t/BEAT_S)) % 7 == 0
            S.prism(g, (x, .79 + .12*r_, -.085), (0, 0, 1), .013, .03, 6, HOT if on else (230, 220, 190), (120, 80, 50), emit=on)
    for sx in (-1, 1):                                                                # stop pegs on the posts
        for k in range(12):
            y = 1.02 + .08*k; T.tube(g, (sx*.565, y, -.11), (sx*.66, y, -.11), .009, col=(222, 210, 180), n=4, spec=.2)
    for sx in (-1, 1):                                                                # side cabinets with switch banks
        x0, x1 = sorted((sx*.68, sx*1.1))
        g.box(x0, x1, 0, .62, -.22, .22, wood)
        for r_ in range(3):
            for k in range(7):
                x = x0 + .05 + .053*k; z = -.15 + .13*r_
                g.box(x - .012, x + .012, .62, .66, z - .02, z + .02, metal, spec=.6)
            g.box(x0 + .02, x1 - .02, .62, .63, -.17 + .13*r_, -.13 + .13*r_, (60, 60, 66))
    T.tube(g, (.89, .62, -.05), (.89, .98, -.05), .014, col=(40, 36, 34), n=6)          # horn receiver on a stand
    mouth = horn(g, (.89, .98, -.05), (-.38, .18, 1), .56, .022, .19, (34, 30, 28))
    for age, m, v, dur, tp, cs in ev:
        if tp and v >= .6 and age < .6: fx.append(("wave", tuple(mouth + np.array([.02, 0, 0])), age, 0))

# ---------------------------------------------------------------- 1920: theremin (RCA AR-1264 style, Henry Ford / Science Museum / Met photos)
def theremin(g, t, ev, fx, s):
    mah, mah_d, mah_l, chrome, blk = (124, 60, 32), (84, 40, 22), (150, 78, 42), (214, 216, 226), (18, 14, 12)
    ox, W, D = -.16, .265, .165
    playing = any(e[0] < e[3] + .1 for e in ev)
    for x in (ox - W + .03, ox + W - .03):                                            # four tall slender legs
        for z in (-D + .03, D - .03): T.tube(g, (x, 0, z), (x, .52, z), .008, .015, col=mah_d, n=4, spec=.2)
    slab(g, ox - W, ox + W, [(D, .5), (D, .8), (D - .03, .815), (-D, .95), (-D, .5)], mah_l, mah, mah_d, spec=.25)
    g.line((ox - W, .803, D + .001), (ox + W, .803, D + .001), mah_d)
    g.box(ox - .09, ox + .09, .63, .71, D, D + .006, blk)                             # front escutcheon + two dials
    for dx in (-.045, .045): S.prism(g, (ox + dx, .67, D + .016), (0, 0, 1), .014, .02, 8, (226, 210, 170), (70, 46, 30))
    # pitch antenna: vertical rod at the RIGHT rear corner
    ax, az = ox + .2, -.12; ay = .815 + (.135 - az)/.3*.135
    S.prism(g, (ax, ay + .02, az), (0, 1, 0), .02, .04, 8, (40, 36, 34), (26, 22, 20))
    rod = mix(chrome, HOT, .7) if playing else chrome
    T.tube(g, (ax, ay + .04, az), (ax, 1.4, az), .009, col=rod, n=6, spec=.9); g.line((ax, ay + .04, az + .01), (ax, 1.4, az + .01), rod)
    # volume loop: a horizontal hairpin loop out of the LEFT side
    ly, lz = .86, -.05
    hp = [(ox - W, ly, lz - .035), (ox - W - .17, ly, lz - .035)]
    hp += [(ox - W - .17 - .035*math.sin(a), ly, lz - .035*math.cos(a)) for a in np.linspace(0, math.pi, 7)[1:-1]]
    hp += [(ox - W - .17, ly, lz + .035), (ox - W, ly, lz + .035)]
    for a, b in zip(hp, hp[1:]): T.tube(g, a, b, .009, col=chrome, n=5, spec=.9); g.line(a, b, chrome)
    # RCA diamond loudspeaker on its V stand, behind on the right
    sx, sz, sy, e = .52, -.3, 1.0, .25
    g.box(sx - .13, sx + .13, 0, .03, sz - .1, sz + .1, mah_d)
    for dx in (-.07, .07): T.tube(g, (sx + dx*.5, .03, sz), (sx + dx*3.4, sy, sz), .008, col=(30, 28, 28), n=4)
    dia = [(sx, sy - e, sz), (sx + e, sy, sz), (sx, sy + e, sz), (sx - e, sy, sz)]
    g.face(dia, (92, 70, 48), ref=(sx, sy, sz - 1), two=True)
    inner = [(sx + (p[0] - sx)*.86, sy + (p[1] - sy)*.86, sz + .006) for p in dia]
    g.face(inner, (70, 54, 40) if not playing else (110, 84, 56), ref=(sx, sy, sz - 1))
    for a, b in zip(dia, dia[1:] + dia[:1]): T.tube(g, a, b, .012, col=(60, 42, 28), n=4)
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", (sx + .1, sy + .3, sz), age))
        if tp and age < .6: fx.append(("wavel", (ax - .03, 1.2, az), age, 0))

# ---------------------------------------------------------------- 1928: ondes Martenot (keyboard + ring wire, palme diffuser)
def ondes(g, t, ev, fx, s):
    wood, wood_d, pale, metal, grey = (178, 112, 58), (118, 70, 36), (228, 206, 160), (214, 216, 224), (150, 152, 158)
    ox = -.14
    for x in (ox - .48, ox + .42):                                                    # two side legs on long feet
        g.box(x - .025, x + .025, 0, .035, -.2, .2, wood_d)
        g.box(x - .022, x + .022, .035, .68, -.05, .05, wood_d)
    g.box(ox - .45, ox + .39, .2, .24, -.03, .03, wood_d)                             # stretcher
    g.box(ox - .54, ox + .5, .68, .8, -.22, .19, wood)                                # keyboard cabinet
    g.box(ox - .54, ox + .5, .8, .87, -.22, .06, wood)
    for x0, x1 in ((ox - .54, ox - .47), (ox + .45, ox + .5)): g.box(x0, x1, .8, .85, .06, .2, wood)
    act = active_of(ev)
    km, kw = P.build_keys(g, ox - .465, ox + .445, .205, .822, 36, 42, act, nat=(238, 232, 214), depth=.14)
    # the ring on its wire, in front of the keys: slides to the pitch being played
    on = sorted([e for e in ev if e[0] >= 0])
    def kx(m): return P.key_x(km, kw, ox - .465, m)
    if on:
        cur = on[0]; prev = on[1] if len(on) > 1 else cur
        xr = lerp(kx(prev[1]), kx(cur[1]), ease(cur[0]/.12)); sounding = cur[0] < cur[3] + .15
    else: xr = kx(s["ev"][0][1]); sounding = False
    wy, wz = .79, .232
    g.line((ox - .47, wy, wz), (ox + .45, wy, wz), metal)
    for x in (ox - .475, ox + .455): g.box(x - .006, x + .006, .77, .8, wz - .008, wz + .008, metal)
    T.tube(g, (xr - .005, wy, wz), (xr + .005, wy, wz), .013, col=HOT if sounding else (236, 236, 240), n=8, two=True)
    T.tube(g, (xr, wy - .012, wz), (xr, wy - .03, wz + .012), .004, col=metal, n=4)
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", (xr, .95, wz), age))
        if age < .07: fx.append(("spark", (xr, wy, wz + .01), age))
    # expression drawer on the left: the "touche" key that shapes each note
    dx0 = ox - .72
    g.box(dx0, ox - .54, .7, .79, -.06, .23, wood_d)
    press = any(e[0] < e[3] for e in ev)
    g.box(dx0 + .04, dx0 + .12, .79, .8 - (.006 if press else 0), .07, .19, HOT if press else pale, emit=press)
    for k in range(4): g.box(dx0 + .02 + .035*k, dx0 + .045 + .035*k, .79, .8, -.03, .02, (60, 40, 26))
    # principal diffuser (tall box) with the palme (string lyre) on top
    px, pz = ox + .84, -.16
    g.box(px - .13, px + .13, 0, .62, pz - .1, pz + .1, grey)
    g.box(px - .11, px + .11, .05, .58, pz + .1, pz + .104, (110, 112, 118))
    g.box(px - .05, px + .05, .62, .66, pz - .04, pz + .04, wood_d)
    shape = [(-.035, 0), (.035, 0), (.07, .09), (.14, .22), (.162, .34), (.135, .48), (.065, .6), (0, .67), (-.065, .6),
             (-.135, .48), (-.162, .34), (-.14, .22), (-.07, .09)]
    base = .66
    fan(g, [(px + x, base + y, pz) for x, y in shape], (px, base + .3, pz), pale, ref=(px, base + .3, pz - 1))
    hole = [(px + .04*math.sin(a), base + .32 + .085*math.cos(a)*(1.25 if math.cos(a) > 0 else 1), pz + .004) for a in np.linspace(0, 2*math.pi, 9)[:-1]]
    fan(g, hole, (px, base + .32, pz + .004), (40, 28, 18), ref=(px, base + .3, pz - 1), two=False)
    rim = [(x*.9, y*.94) for x, y in shape[3:10]]
    hot = min([e[0] for e in ev if e[4]], default=None)
    for k, (x, y) in enumerate(rim):
        bx = -.03 + .06*k/(len(rim) - 1)
        P.vstring(g, (px + bx, base + .06, pz + .008), (px + x, base + y, pz + .008), metal, hot if hot is not None and hot < .5 else None, amp=.004)

# ---------------------------------------------------------------- 1939: Hammond Novachord (NMAH / National Music Centre photos)
def novachord(g, t, ev, fx, s):
    mah, mah_d, mah_l, blk, gold = (116, 52, 30), (80, 34, 20), (146, 70, 40), (22, 18, 18), (214, 176, 96)
    W = .575
    for x in (-.53, .53):                                                             # turned legs
        for z in (-.3, .27):
            for y0, y1, r0, r1 in ((0, .05, .03, .03), (.05, .11, .02, .024), (.11, .3, .03, .037), (.3, .4, .026, .03)):
                T.tube(g, (x, y0, z), (x, y1, z), r0, r1, col=mah, n=8, spec=.25)
    for x in (-.53, .53): g.box(x - .015, x + .015, .06, .09, -.3, .27, mah_d)          # stretchers
    g.box(-.53, .53, .06, .09, -.315, -.285, mah_d)
    g.box(-.13, .13, .02, .4, -.3, 0, mah_d)                                           # pedal box + expression pedal
    slab(g, -.05, .05, [(.02, .03), (.2, .05), (.2, .08), (.02, .08)], blk, blk)
    g.box(-W, W, .4, .95, -.37, .17, mah, spec=.2)                                    # main cabinet
    g.box(-.52, .52, .62, .703, .17, .37, mah, spec=.2)                               # key shelf
    act = active_of(ev)
    km, kw = P.build_keys(g, -.5, .5, .37, .725, 29, 42, act, nat=(236, 228, 210), depth=.15)
    key_fx(fx, ev, km, kw, -.5, .725, .37, .15, .14)
    for sx in (-1, 1):                                                                # rounded cheek blocks
        g.box(min(sx*.5, sx*.575), max(sx*.5, sx*.575), .4, .86, .17, .32, mah, spec=.2)
        T.tube(g, (sx*.5375, .4, .32), (sx*.5375, .86, .32), .0375, col=mah, n=8, spec=.25)
    pn = Pan((-.5, .735, .21), (1, 0, 0), (0, .12, -.05))                              # black control strip
    pn.obox(g, 0, 1., 0, .13, -.01, 0, blk)
    for r_ in range(2):
        for k in range(14):
            pn.knob(g, .06 + .068*k, .035 + .06*r_, .011, .014, (30, 24, 22), gold, n=6)
    g.box(-W, W, .95, .975, -.38, .2, mah_l, spec=.2)                                 # lid
    for sx in (-1, 1): g.box(min(sx*.33, sx*.55), max(sx*.33, sx*.55), .975, .995, -.32, .06, mah_d)
    slab(g, -.26, .26, [(-.02, .975), (0, .975), (-.06, 1.2), (-.08, 1.2)], mah_l, mah_d, mah_d)   # music rack

# ---------------------------------------------------------------- 1957: RCA Mark II (Columbia-Princeton studio photos): a room of racks
RCA_LAYOUT = ["tape", "panel", "scope", "panel", "tape"]
def rack_face(g, pn, w, kind, t, ev, idx):
    pa, pb, pc, wht = (118, 122, 128), (92, 96, 104), (140, 140, 136), (232, 232, 226)
    m = last_note_any(ev); bits = [(m >> k) & 1 for k in range(8)]
    pn.rect(g, .03, w - .03, 2.02, 2.07, wht)                                         # white caps along the top
    bands = (("knobs", .2, .45), ("jacks", .47, .8), ("lamps", .82, 1.02), ("knobs", 1.04, 1.32), ("buttons", 1.34, 1.66), ("meter", 1.68, 1.98))
    if kind == "scope": bands = (("knobs", .2, .5), ("jacks", .52, .78), ("buttons", .8, 1.2), ("crt", 1.22, 1.72), ("meter", 1.74, 1.98))
    if kind == "tape": bands = (("jacks", .2, .7), ("blank", .72, 1.0), ("knobs", 1.02, 1.3), ("buttons", 1.32, 1.62), ("lamps", 1.64, 1.98))
    for bi, (b, y0, y1) in enumerate(bands):
        pn.rect(g, .03, w - .03, y0, y1, pa if (bi + idx) % 2 else pb)
        if b == "knobs":
            for r_ in range(2):
                for k in range(6): pn.rect(g, .07 + .08*k, .1 + .08*k, y0 + .06 + .12*r_, y0 + .09 + .12*r_, (24, 24, 28), h=.004)
        elif b == "jacks":
            for r_ in range(3):
                for k in range(8): pn.rect(g, .06 + .06*k, .08 + .06*k, y0 + .06 + .09*r_, y0 + .08 + .09*r_, (16, 16, 18), h=.004)
        elif b == "buttons":
            for r_ in range(2):
                for k in range(7): pn.rect(g, .07 + .067*k, .11 + .067*k, y0 + .06 + .12*r_, y0 + .12 + .12*r_, wht, h=.004)
        elif b == "lamps":
            for k in range(8):
                on = bits[k] and any(e[0] < e[3] + .05 for e in ev)
                pn.rect(g, .06 + .06*k, .09 + .06*k, (y0 + y1)/2 - .015, (y0 + y1)/2 + .015, (255, 196, 110) if on else (90, 60, 40), h=.004, emit=on)
        elif b == "meter":
            c = pn.P(w/2, (y0 + y1)/2, .004); g.face(S.ring(c, pn.n, .085, 12), (234, 230, 214), ref=c - pn.n, emit=True)
            lv = max([max(0, 1 - e[0]/.6)*e[2] for e in ev] + [0]); ang = math.radians(150 - 120*lv)
            pn.line(g, w/2, (y0 + y1)/2 - .05, w/2 + .08*math.cos(ang), (y0 + y1)/2 - .05 + .08*math.sin(ang), (20, 16, 14), h=.006)
        elif b == "crt":
            c = pn.P(w/2, (y0 + y1)/2, .004); g.face(S.ring(c, pn.n, .2, 14), (20, 60, 30), ref=c - pn.n, emit=True)
            f = midi_hz(m)/200; pts = [(w/2 - .16 + .32*k/16, (y0 + y1)/2 + .09*math.sin(2*math.pi*f*k/16 + t*8)) for k in range(17)]
            for (a0, b0), (a1, b1) in zip(pts, pts[1:]): pn.line(g, a0, b0, a1, b1, (140, 255, 150), h=.006)
def last_note_any(ev):
    on = [e for e in ev if e[0] >= 0]
    return min(on)[1] if on else 69
def rca(g, t, ev, fx, s):
    frame, steel, paper = (44, 46, 52), (150, 150, 154), (238, 234, 222)
    H, RWD, RD, n = 2.1, .58, .5, len(RCA_LAYOUT)
    xb = n*RWD/2; zf = -.7
    for i, kind in enumerate(RCA_LAYOUT):                                             # back wall: five racks
        x0 = -xb + i*RWD
        g.box(x0, x0 + RWD, 0, H, zf - RD, zf, frame)
        rack_face(g, Pan((x0, 0, zf), (1, 0, 0), (0, 1, 0)), RWD, kind, t, ev, i)
        if kind == "tape":                                                            # paper-tape punch + reader on a shelf
            cx = x0 + RWD/2
            g.box(x0 + .02, x0 + RWD - .02, .76, .8, zf, zf + .34, frame)
            g.box(cx - .2, cx + .2, .8, .95, zf + .02, zf + .22, steel, spec=.4)
            kp = Pan((cx - .19, .95, zf + .22), (1, 0, 0), (0, .05, -.18))
            for r_ in range(3):
                for k in range(9): kp.rect(g, .02 + .04*k, .045 + .04*k, .02 + .05*r_, .04 + .05*r_, (226, 226, 220), h=.005)
            T.tube(g, (cx - .19, 1.04, zf + .08), (cx + .19, 1.04, zf + .08), .055, col=paper, n=10, spec=.2)
            g.face([(cx - .18, .99, zf + .1), (cx + .18, .99, zf + .1), (cx + .18, .55, zf + .4), (cx - .18, .55, zf + .4)], paper,
                   ref=(cx, .7, zf), two=True)
            notes = [e[1] for e in s["ev"]]
            tp_ = Pan((cx - .18, .99, zf + .1), (1, 0, 0), (0, -.44, .3))             # the hanging tape, top -> bottom
            if tp_.n[2] < 0: tp_ = Pan((cx + .18, .99, zf + .1), (-1, 0, 0), (0, -.44, .3))
            Ltape = math.hypot(.44, .3)
            for r_ in range(9):                                                       # punched holes scroll out as it plays
                u = (r_*.11 + t*.32) % 1.0; code = notes[(r_ + int(t*.32/.11)) % len(notes)]
                for k in range(7):
                    if (code >> k) & 1:
                        a = .04 + .045*k; b = u*Ltape
                        tp_.rect(g, a, a + .022, b - .012, b + .012, (30, 26, 22), h=.004)
    for sx in (-1, 1):                                                                # side walls: two racks each
        for j in range(2):
            z0 = zf + j*RWD
            x0, x1 = (-xb - RD, -xb) if sx < 0 else (xb, xb + RD)
            g.box(x0, x1, 0, H, z0, z0 + RWD, frame)
            pn = Pan((-xb, 0, z0 + RWD), (0, 0, -1), (0, 1, 0)) if sx < 0 else Pan((xb, 0, z0), (0, 0, 1), (0, 1, 0))
            rack_face(g, pn, RWD, "panel", t, ev, j + 1 + (sx > 0))
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", (0, 1.9, zf + .1), age))

# ---------------------------------------------------------------- 1964: Moog modular (tiered walnut cabinets, IIIc style)
CORD_COLS = [(220, 60, 50), (240, 200, 60), (60, 120, 220), (80, 190, 90), (232, 232, 232), (230, 120, 40), (180, 80, 200)]
def moog(g, t, ev, fx, s):
    wal, wal_d, blk, alu = (132, 80, 44), (94, 56, 30), (24, 24, 26), (196, 198, 204)
    W = .6
    g.box(-.58, .58, 0, .075, .14, .4, wal)                                           # keyboard case (walnut)
    for x0, x1 in ((-.58, -.555), (.555, .58)): g.box(x0, x1, .075, .11, .14, .4, wal)
    g.box(-.58, .58, .075, .11, .14, .25, wal_d)
    act = active_of(ev)
    km, kw = P.build_keys(g, -.44, .53, .39, .097, 36, 29, act, nat=(238, 236, 228), depth=.14)
    key_fx(fx, ev, km, kw, -.44, .097, .39, .14)
    g.box(-.55, -.45, .075, .09, .25, .39, blk)
    for z in (.29, .35): S.prism(g, (-.5, .1, z), (0, 1, 0), .012, .014, 6, alu, blk)
    lower = [(.12, 0), (.02, .44), (-.2, .44), (-.2, 0)]
    upper = [(.02, .44), (-.06, .86), (-.24, .86), (-.2, .44)]
    for prof in (lower, upper):
        slab(g, -W, W, prof, wal_d, blk, wal)
        for x0, x1 in ((-W - .03, -W), (W, W + .03)):
            cz = np.mean([p[0] for p in prof]); cy = np.mean([p[1] for p in prof])
            slab(g, x0, x1, [(cz + (z - cz)*1.05, cy + (y - cy)*1.04) for z, y in prof], wal, wal, wal, spec=.2)
    jacks = []
    for (z0, y0), (z1, y1) in (((.12, 0), (.02, .44)), ((.02, .44), (-.06, .86))):
        pn = Pan((-W, y0, z0), (1, 0, 0), (0, y1 - y0, z1 - z0)); L = math.hypot(y1 - y0, z1 - z0)
        pn.rect(g, -.005, 2*W + .005, 0, .028, wal, h=.004); pn.rect(g, -.005, 2*W + .005, L - .028, L, wal, h=.004)   # walnut rails
        xs = np.cumsum([0, .12, .1, .14, .1, .12, .1, .12, .14, .1, .1])
        for k in range(len(xs) - 1):
            a0, a1 = xs[k] + .015, xs[k + 1] - .005
            pn.line(g, xs[k + 1], .03, xs[k + 1], L - .03, (70, 70, 76))
            ac = (a0 + a1)/2
            for r_ in range(3):
                pn.knob(g, ac, L - .075 - .085*r_, .017, .016, (30, 30, 32), alu, n=6)
                pn.rect(g, ac - .018, ac + .018, L - .102 - .085*r_, L - .097 - .085*r_, (214, 214, 206), h=.002)
            for r_ in range(2):
                for dx in (-.025, .025):
                    p = (ac + dx, .05 + .045*r_); pn.rect(g, p[0] - .007, p[0] + .007, p[1] - .007, p[1] + .007, (150, 150, 156), h=.002)
                    jacks.append(pn.P(p[0], p[1], .004))
    rng = np.random.default_rng(64)
    for k in range(13):                                                               # drooping patch cords
        a, b = jacks[int(rng.integers(len(jacks)))], jacks[int(rng.integers(len(jacks)))]
        if np.linalg.norm(a - b) < .08: continue
        col = CORD_COLS[k % len(CORD_COLS)]; sag = .05 + .12*np.linalg.norm(a - b)
        pts = [a*(1 - u) + b*u + np.array([0, -sag*math.sin(math.pi*u), .06*math.sin(math.pi*u)]) for u in np.linspace(0, 1, 8)]
        for p0, p1 in zip(pts, pts[1:]): g.line(p0, p1, col)

# ---------------------------------------------------------------- 1970: Minimoog Model D (44 keys, panel tilted up behind the keys, wheels left)
def minimoog(g, t, ev, fx, s):
    wal, wal_d, blk, alu = (122, 68, 34), (86, 48, 24), (22, 22, 24), (190, 190, 196)
    W = .365
    g.box(-W, W, 0, .045, -.1, .21, wal)
    for x0, x1 in ((-W, -W + .02), (W - .02, W)):
        slab(g, x0, x1, [(.21, 0), (.21, .07), (.13, .095), (-.1, .095), (-.1, 0)], wal, wal, wal_d, spec=.2)
    g.box(-W + .02, W - .02, .045, .07, -.1, .085, wal_d)
    act = active_of(ev)
    km, kw = P.build_keys(g, -.27, .345, .205, .07, 41, 26, act, nat=(238, 236, 228), depth=.12)
    key_fx(fx, ev, km, kw, -.27, .07, .205, .12, .12)
    g.box(-.345, -.275, .045, .058, .085, .205, blk)                                   # pitch + mod wheels, LEFT of the keys
    on = [e for e in ev if e[0] < e[3]]
    for x in (-.325, -.297): wheel(g, x, .052, .145, .03)
    ang = math.radians(62)
    pn = Pan((-W + .02, .08, -.05), (1, 0, 0), (0, math.sin(ang), -math.cos(ang)))
    Wp = 2*W - .04
    pn.obox(g, 0, Wp, 0, .25, -.05, 0, wal_d, front=blk)
    for a0 in (-.022, Wp - .003): pn.obox(g, a0, a0 + .025, -.005, .26, -.056, .012, wal, spec=.2)
    kn = [(.04, .19), (.04, .1), (.13, .2), (.18, .2), (.23, .2), (.13, .13), (.18, .13), (.23, .13), (.13, .06), (.18, .06), (.23, .06),
          (.34, .2), (.34, .14), (.34, .08), (.38, .11), (.44, .2), (.49, .2), (.54, .2), (.44, .13), (.49, .13), (.54, .13),
          (.44, .06), (.49, .06), (.54, .06), (.62, .19), (.62, .1)]
    for a, b in kn: pn.knob(g, a, b, .0115, .013, blk, alu)
    rk = [(.29, .21, (60, 130, 210)), (.29, .16, (60, 130, 210)), (.29, .11, (60, 130, 210)), (.29, .06, (60, 130, 210)),
          (.39, .2, (60, 130, 210)), (.4, .05, (232, 122, 40)), (.59, .06, (232, 122, 40)), (.66, .06, (232, 232, 226)), (.08, .05, (232, 232, 226))]
    for a, b, c_ in rk: pn.obox(g, a - .009, a + .009, b - .013, b + .013, 0, .007, c_)
    pn.rect(g, .005, Wp - .005, .235, .24, (150, 150, 156))
    pn.rect(g, .005, Wp - .005, .015, .02, (150, 150, 156))

# ---------------------------------------------------------------- 1978: Prophet-5 (61 keys, wooden case, flat panel of knobs + program buttons)
def prophet(g, t, ev, fx, s):
    wal, wal_d, blk, alu = (136, 78, 40), (98, 56, 28), (20, 20, 22), (150, 150, 156)
    W = .475
    g.box(-W + .02, W - .02, 0, .05, -.2, .2, blk)
    for x0, x1 in ((-W, -W + .022), (W - .022, W)):
        slab(g, x0, x1, [(.205, 0), (.205, .07), (.15, .09), (-.205, .138), (-.205, 0)], wal, wal, wal_d, spec=.2)
    g.box(-W + .022, W - .022, 0, .047, .19, .207, wal, spec=.2)
    slab(g, -W + .022, W - .022, [(-.01, .05), (-.01, .086), (-.205, .13), (-.205, .05)], blk, blk, blk)
    g.box(-W + .022, W - .022, .05, .074, -.01, .06, blk)
    act = active_of(ev)
    km, kw = P.build_keys(g, -.38, .44, .19, .074, 36, 36, act, nat=(238, 236, 228), depth=.13)
    key_fx(fx, ev, km, kw, -.38, .074, .19, .13, .12)
    g.box(-.452, -.39, .05, .062, .07, .19, blk)
    for x in (-.433, -.408): wheel(g, x, .056, .13, .026)
    pn = Pan((-W + .022, .086, -.01), (1, 0, 0), (0, .044, -.195))
    groups = [(.03, 2), (.12, 2), (.21, 3), (.33, 3), (.45, 3), (.57, 4), (.73, 4)]
    for a0, nk in groups:
        for r_ in range(3):
            if r_ == 2 and nk == 2: continue
            for k in range(nk): pn.knob(g, a0 + .035*k, .17 - .045*r_, .0095, .012, (26, 26, 28), (128, 128, 134))
    prog = 1 + int(t/BAR_S) % 8                                                        # program recall: a button press per bar
    for k in range(8):
        a = .36 + .03*k; on = k + 1 == prog
        pn.obox(g, a, a + .02, .02, .038, 0, .006, (44, 44, 48))
        pn.rect(g, a + .007, a + .013, .041, .046, (255, 70, 50) if on else (70, 22, 20), emit=on)
    pn.rect(g, .64, .7, .02, .044, (36, 6, 6), emit=True)
    for k in range(2): pn.rect(g, .646 + .026*k, .668 + .026*k, .025, .039, (255, 60, 40), h=.002, emit=True)
    pn.rect(g, .005, .945, .195, .199, alu)

# ---------------------------------------------------------------- 1981: Roland TB-303 (silver box; note buttons laid out like a piano octave)
PC_KEY = {0: ("w", 0), 2: ("w", 1), 4: ("w", 2), 5: ("w", 3), 7: ("w", 4), 9: ("w", 5), 11: ("w", 6),
          1: ("b", 0), 3: ("b", 1), 6: ("b", 3), 8: ("b", 4), 10: ("b", 5)}
def tb303(g, t, ev, fx, s):
    sil, sil_d, blk = (200, 202, 208), (150, 152, 160), (22, 22, 24)
    W, D = .153, .0865
    slab(g, -W, W, [(D, 0), (D, .034), (-D, .046), (-D, 0)], sil, sil_d, sil_d, spec=.35)
    pn = Pan((-W, .034, D), (1, 0, 0), (0, .012, -2*D))
    pn.rect(g, .006, .3, .008, .072, blk)
    cur = [e for e in ev if e[0] < e[3] + .02]; m = min(cur)[1] if cur else None
    lit = PC_KEY.get(m % 12) if m is not None else None
    if m is not None and m % 12 == 0 and m >= 57: lit = ("w", 7)                      # the upper C button
    wa = [.09 + .018*k for k in range(8)]
    for k, a in enumerate(wa):
        on = lit == ("w", k)
        pn.obox(g, a, a + .014, .014, .034, 0, .004, HOT if on else (226, 226, 222), emit=on)
        pn.rect(g, a + .005, a + .009, .062, .066, (255, 50, 40) if on else (80, 20, 18), emit=on)
    for k in (0, 1, 3, 4, 5):
        a = wa[k] + .009; on = lit == ("b", k)
        pn.obox(g, a, a + .012, .04, .058, 0, .004, HOT if on else (96, 98, 104), emit=on)
    for k in range(4): pn.obox(g, .015 + .03*(k % 2), .038 + .03*(k % 2), .014 + .024*(k//2), .032 + .024*(k//2), 0, .004, (170, 172, 178))
    for k in range(3): pn.obox(g, .24 + .02*k, .255 + .02*k, .02, .05, 0, .004, (170, 172, 178))
    pn.rect(g, .006, .3, .12, .125, blk)
    chrome, top = (66, 68, 74), (214, 216, 222)
    for a, r in ((.03, .014), (.085, .011), (.13, .011), (.275, .014)): pn.knob(g, a, .098, r, .012, chrome, top, n=8)
    k_sweep = t/s["len"]
    for k in range(6):
        a = .03 + .045*k; pn.knob(g, a, .148, .0085, .01, chrome, top, n=8)
        ang = math.radians(225 - 270*(clamp(k_sweep*1.1) if k == 1 else (.5 + .3*clamp(k_sweep) if k == 3 else .45)))
        c = pn.P(a, .148, .0105); tip = pn.P(a + .007*math.cos(ang), .148 + .007*math.sin(ang), .0105)
        g.line(c, tip, (20, 20, 22))
    pn.obox(g, .285, .292, .14, .156, 0, .006, blk)
    for age, mm, v, dur, tp, cs in ev:
        if v > .9 and age < .9: fx.append(("note", (0, .1, 0), age))

# ---------------------------------------------------------------- 1983: Yamaha DX7 (61 keys, LCD, rows of colour-coded membrane buttons)
def dx7(g, t, ev, fx, s):
    body, body_l, blk = (44, 34, 30), (60, 48, 42), (16, 14, 14)
    W, D = .509, .1645
    g.box(-W, W, 0, .05, -D, D, body)
    slab(g, -W, W, [(-.03, .05), (-.03, .09), (-D, .102), (-D, .05)], body_l, body, body, spec=.2)
    g.box(-W, -.42, .05, .066, -.03, D, body); g.box(.475, W, .05, .066, -.03, D, body)
    g.box(-.42, .475, .05, .07, -.03, .025, body)
    act = active_of(ev)
    km, kw = P.build_keys(g, -.415, .47, .16, .072, 36, 36, act, nat=(236, 234, 226), depth=.135)
    key_fx(fx, ev, km, kw, -.415, .072, .16, .135, .12)
    for x in (-.485, -.455): wheel(g, x, .062, .1, .024, (24, 22, 22))
    pn = Pan((-W, .09, -.03), (1, 0, 0), (0, .012, -.1345))
    for a, pos in ((.035, .7), (.065, .45)):                                          # volume + data entry sliders
        pn.rect(g, a - .003, a + .003, .02, .115, (10, 10, 10))
        pn.obox(g, a - .009, a + .009, .02 + .085*pos, .035 + .085*pos, 0, .012, (70, 64, 60))
    fcols = [(150, 110, 70), (150, 110, 70), (170, 66, 58), (170, 66, 58), (66, 150, 124), (66, 150, 124), (90, 90, 96)]
    for r_ in range(2):
        for k in range(7): pn.rect(g, .11 + .03*k, .13 + .03*k, .045 + .045*r_, .07 + .045*r_, fcols[k], h=.002)
    pn.rect(g, .34, .45, .05, .1, (150, 176, 92), emit=True)                          # LCD
    for r_ in range(2):
        for k in range(12): pn.rect(g, .345 + .0085*k, .351 + .0085*k, .058 + .022*r_, .072 + .022*r_, (70, 86, 44), h=.002, emit=True)
    for r_ in range(2):                                                                # 32 voice-select buttons
        for k in range(16):
            sel = r_ == 0 and k == 10
            pn.rect(g, .485 + .0255*k, .503 + .0255*k, .045 + .045*r_, .07 + .045*r_, (120, 236, 212) if sel else (58, 176, 156), h=.002)
    for k in range(4):                                                                 # printed algorithm chart
        a = .925 + .018*(k % 2); b = .05 + .025*(k//2)
        pn.line(g, a, b, a + .012, b, (220, 220, 214)); pn.line(g, a, b, a, b + .015, (220, 220, 214))

# ---------------------------------------------------------------- 1995: Eurorack (Doepfer A-100 style suitcase, rows of 3U modules)
def _rows():
    rng = np.random.default_rng(1995); rows = []
    for r_ in range(3):
        hp, mods = 0, []
        while True:
            w = int(rng.choice([4, 6, 8, 8, 10, 12, 12, 14, 16, 20]))
            if hp + w > 84: break
            kind = rng.choice(["grey"]*6 + ["black"]*3 + ["white"])
            mods.append((hp, w, kind, int(rng.integers(1 << 12)))); hp += w
        if r_ == 2 and hp < 84: mods.append((hp, 84 - hp, "grey", 0))
        rows.append(mods)
    return rows
EURO_ROWS = _rows()
def eurorack(g, t, ev, fx, s):
    tol, alu, alu_d = (30, 30, 32), (200, 202, 208), (140, 142, 150)
    W, H, D = .25, .47, .2
    g.box(-W, -W + .02, 0, H, -D, 0, tol); g.box(W - .02, W, 0, H, -D, 0, tol)
    g.box(-W, W, 0, .02, -D, 0, tol); g.box(-W, W, H - .02, H, -D, 0, tol); g.box(-W, W, 0, H, -D, -D + .02, tol)
    for x0, x1, y0, y1 in ((-W - .006, -W + .006, 0, H), (W - .006, W + .006, 0, H), (-W, W, -.004, .008), (-W, W, H - .008, H + .004)):
        g.box(x0, x1, y0, y1, -.012, .004, alu, spec=.6)
    for x in (-W, W):
        for y in (0, H): g.box(x - .015, x + .015, y - .015, y + .015, -.02, .01, alu_d, spec=.6)
    for x in (-.09, .09): T.tube(g, (x, H, -.1), (x, H + .035, -.1), .008, col=(20, 20, 22), n=6)
    T.tube(g, (-.1, H + .035, -.1), (.1, H + .035, -.1), .012, col=(20, 20, 22), n=6)
    pn = Pan((-.215, 0, -.004), (1, 0, 0), (0, 1, 0))
    hpw = .00508; st = step16(t); hits = [e for e in ev if e[0] < .12]
    jacks = []; plates = {"grey": (176, 178, 184), "black": (40, 40, 44), "white": (222, 222, 216)}
    for r_, mods in enumerate(EURO_ROWS):
        y0 = .028 + .143*r_; y1 = y0 + .1334
        for mi, (hp, w, kind, seed) in enumerate(mods):
            a0, a1 = hp*hpw + .0006, (hp + w)*hpw - .0006
            plate = plates[kind]; ink = (24, 24, 26) if kind != "black" else (200, 200, 204)
            pn.rect(g, a0, a1, y0, y1, plate, h=.0005)
            if seed == 0: continue
            ac = (a0 + a1)/2; nk = 1 + min(3, w//5)
            for k in range(nk):
                cx = ac if w < 10 else (a0 + .012 + (a1 - a0 - .024)*(k % 2))
                cy = y1 - .022 - .03*(k if w < 10 else k//2)
                if cy < y0 + .05: break
                pn.knob(g, cx, cy, .0075 if w < 8 else .0095, .011, (30, 30, 34), (206, 206, 212))
            for k in range(min(4, max(2, w//4))):
                jx = a0 + (a1 - a0)*(k + .5)/min(4, max(2, w//4)); jy = y0 + .016
                pn.rect(g, jx - .004, jx + .004, jy - .004, jy + .004, (14, 14, 16), h=.002); jacks.append(pn.P(jx, jy, .004))
            if r_ == 2 and mi == 1:                                                   # step sequencer: 8 LEDs follow the clock
                for k in range(8):
                    on = k == st % 8; lx = a0 + .006 + (a1 - a0 - .012)*k/7
                    pn.rect(g, lx - .002, lx + .002, y0 + .04, y0 + .046, (255, 40, 30) if on else (70, 14, 12), h=.002, emit=on)
            elif seed % 3 == 0:
                on = bool(hits) and (seed + len(hits)) % 2 == 0
                c_on = (255, 50, 40) if seed % 2 else (80, 255, 90)
                pn.rect(g, ac - .003, ac + .003, y0 + .04, y0 + .046, c_on if on else dim(c_on, .3), h=.002, emit=on)
    rng = np.random.default_rng(600)
    for k in range(15):
        a, b = jacks[int(rng.integers(len(jacks)))], jacks[int(rng.integers(len(jacks)))]
        if np.linalg.norm(a - b) < .05: continue
        col = CORD_COLS[k % len(CORD_COLS)]; sag = .02 + .2*np.linalg.norm(a - b)
        pts = [a*(1 - u) + b*u + np.array([0, -sag*math.sin(math.pi*u), .03*math.sin(math.pi*u)]) for u in np.linspace(0, 1, 8)]
        for p0, p1 in zip(pts, pts[1:]): g.line(p0, p1, col)
    for age, m, v, dur, tp, cs in ev:
        if v > .85 and age < .9: fx.append(("note", (.16, .3, .06), age))

BUILD = {"modern": minilogue, "tel": telharmonium, "theremin": theremin, "ondes": ondes, "nova": novachord, "rca": rca,
         "moog": moog, "mini": minimoog, "prophet": prophet, "tb303": tb303, "dx7": dx7, "euro": eurorack}
def build(g, t, ev, fx, s): BUILD[s["key"]](g, t, ev, fx, s)

# ---------------------------------------------------------------- sound: each era in its own instrument's voice
def _t(n): return np.arange(n)/SR
def phase(fr): return 2*np.pi*np.cumsum(fr)/SR
def bl(ph, fmax, kind="saw", kmax=48):
    """band-limited oscillator from harmonics (works with gliding / vibrato phase)"""
    K = int(max(1, min(kmax, (SR*.42)//max(fmax, 20.)))); x = np.zeros_like(ph)
    if kind == "saw":
        for k in range(1, K + 1): x += np.sin(k*ph)/k
        return x*.6
    if kind == "sq":
        for k in range(1, K + 1, 2): x += np.sin(k*ph)/k
        return x*.8
    for k in range(1, K + 1, 2): x += ((-1)**((k - 1)//2))*np.sin(k*ph)/(k*k)
    return x*.85
def pulse(ph, fmax, w=.35): return bl(ph, fmax) - bl(ph + 2*np.pi*w, fmax)
def svf_lp(x, fc, q=.707):
    """2-pole state-variable low-pass (Simper TPT) with a time-varying cutoff"""
    n = len(x); fc = np.broadcast_to(np.asarray(fc, float), (n,))
    gg = np.tan(np.pi*np.clip(fc, 20, SR*.45)/SR); k = 1/q
    a1 = 1/(1 + gg*(gg + k)); a2 = gg*a1; a3 = gg*a2
    xs, A1, A2, A3 = x.tolist(), a1.tolist(), a2.tolist(), a3.tolist()
    ic1 = ic2 = 0.; out = [0.]*n
    for i in range(n):
        v3 = xs[i] - ic2; v1 = A1[i]*ic1 + A2[i]*v3; v2 = ic2 + A2[i]*ic1 + A3[i]*v3
        ic1 = 2*v1 - ic1; ic2 = 2*v2 - ic2; out[i] = v2
    return np.array(out)
def adsr(t, dur, a=.005, d=.2, s=.7, r=.1):
    e = np.where(t < a, t/a, s + (1 - s)*np.exp(-(t - a)/d))
    return e*np.where(t < dur, 1., np.exp(-(t - dur)/r))
def glide(f, fprev, t, tau):
    if not fprev or abs(fprev - f) < 1e-6: return np.full(len(t), f)
    return f*np.exp(np.log(fprev/f)*np.exp(-t/tau))
def vib(t, rate=5.5, depth=.004, delay=.2): return 1 + depth*np.sin(2*np.pi*rate*t)*np.clip((t - delay)/.25, 0, 1)
def echo(x, sec, fb=.3, n=2):
    d = int(sec*SR); out = np.pad(x, (0, d*n))
    for k in range(1, n + 1): out[d*k:d*k + len(x)] += x*(fb**k)
    return out
def prev_of(s, idx):
    nt = s["ev"][idx][0]
    for j in range(idx - 1, -1, -1):
        if s["ev"][j][0] < nt - 1e-6: return s["ev"][j]
    return None

_TEL_BP, _HORN = butter(2, [220/(SR/2), 3800/(SR/2)], "band"), butter(2, [800/(SR/2), 1600/(SR/2)], "band")
_NLP = butter(2, 2100/(SR/2))
def v_modern(f, dur, v):
    n = int((dur + .35)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = .5*bl(ph*1.004, f) + .5*bl(ph*.996, f) + .3*bl(ph*.5, f*.5, "sq")
    y = svf_lp(x, 600 + 4200*v*np.exp(-t/.13), 1.3)*adsr(t, dur, .003, .16, .5, .1)
    return echo(y, .75*BEAT_S, .3, 2)*v
def v_tel(f, dur, v):                                    # tonewheel sines heard through a telephone horn
    n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = sum(a*np.sin(k*ph) for k, a in ((1, 1.), (2, .5), (3, .32), (4, .14), (5, .1), (6, .05)) if k*f < 9000)
    y = lfilter(*_TEL_BP, x*adsr(t, dur, .03, .3, .85, .09)); y = y + .5*lfilter(*_HORN, y)
    return y*v
def v_ther(f, dur, v, fprev):                            # pure sine, portamento and a wide vibrato
    n = int((dur + .35)*SR); t = _t(n)
    ph = phase(glide(f, fprev, t, .07)*vib(t, 6.2, .008, .12))
    return (np.sin(ph) + .12*np.sin(2*ph) + .05*np.sin(3*ph))*adsr(t, dur, .07, .4, .9, .16)*v
def v_ondes(f, dur, v, fprev):                           # ring glide + the palme's sympathetic string shimmer
    n = int((dur + .5)*SR); t = _t(n)
    ph = phase(glide(f, fprev, t, .05)*vib(t, 5.6, .006, .15))
    x = (np.sin(ph) + .3*np.sin(2*ph) + .16*np.sin(3*ph) + .07*np.sin(4*ph))*adsr(t, dur, .05, .5, .85, .22)
    N = max(2, int(round(SR/f))); a = np.zeros(N + 1); a[0] = 1; a[N] = -.86
    return (x + .22*(1 - .86)*lfilter([1.], a, x))*v
def v_nova(f, dur, v):                                   # divide-down saws, deep vibrato, slow attack
    n = int((dur + .45)*SR); t = _t(n); ph = phase(f*vib(t, 5.4, .005, .08))
    x = lfilter(*_NLP, bl(ph, f) + bl(ph*1.005, f) + bl(ph*.995, f))
    return x*adsr(t, dur, .16, .5, .8, .35)*v*.6
def v_rca(f, dur, v, idx):                               # tube oscillators, resonators and noise
    n = int((dur + .3)*SR); t = _t(n); ph = 2*np.pi*f*t; kind = idx % 3
    if kind == 0: x = np.sin(ph) + .25*np.sin(2*ph)
    elif kind == 1:
        b, a = butter(2, [min(2.2*f, 9000)/(SR/2), min(3.6*f, 15000)/(SR/2)], "band"); x = bl(ph, f, "sq"); x = .6*x + 1.4*lfilter(b, a, x)
    else: x = bl(ph, f)*.8 + np.random.default_rng(idx).normal(0, .12, n)*np.exp(-t*30)
    k = int(.003*SR); x[:k] += np.random.default_rng(idx + 9).normal(0, .3, k)
    return np.tanh(1.3*x*adsr(t, dur, .002, .12, .35, .04))*v
def v_moog(f, dur, v):                                   # pulse + saw through the ladder filter
    n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    y = svf_lp(.7*pulse(ph, f, .38) + .4*bl(ph, f), 450 + 3200*v*np.exp(-t/.09), 2.2)
    return np.tanh(1.2*y)*adsr(t, dur, .004, .12, .65, .06)*v
def v_mini(f, dur, v, fprev):                            # three fat oscillators, glide, filter sweep
    n = int((dur + .3)*SR); t = _t(n); fr = glide(f, fprev, t, .045)*vib(t, 5.2, .004, .3); ph = phase(fr)
    fm = max(f, fprev or f)*1.01
    y = svf_lp(bl(ph, fm) + bl(ph*1.0035, fm) + .45*bl(ph*.5, fm*.5, "sq"), 420 + 2600*np.exp(-t/.26), 1.9)
    return np.tanh(1.4*y)*adsr(t, dur, .008, .3, .85, .1)*v*.7
def v_pro(f, dur, v):                                    # brassy poly: detuned saws + pulse, filter envelope
    n = int((dur + .3)*SR); t = _t(n); ph = 2*np.pi*f*t
    y = svf_lp(bl(ph*1.0045, f) + bl(ph*.9955, f) + .35*pulse(ph, f, .3), 650 + 3600*v*np.exp(-t/.2), 1.35)
    return y*adsr(t, dur, .006, .3, .55, .16)*v*.55
def v_303(s, idx):                                       # saw into a screaming resonant filter; accents, slides, the cutoff knob turning up
    nt, m, v, dur, tp, cs = s["ev"][idx]; f = midi_hz(m)
    pe = prev_of(s, idx); slide = pe is not None and pe[0] + pe[3] > nt + 1e-4
    n = int((dur + .12)*SR); t = _t(n); fp = midi_hz(pe[1]) if slide else None
    ph = phase(glide(f, fp, t, .035)); x = bl(ph, max(f, fp or f)*1.05)
    acc = v > .9; k = nt/s["len"]
    cut = 260 + 1500*k**1.3 + (1800 if acc else 900)*np.exp(-t/(.07 if acc else .14))*(.5 if slide else 1)
    y = np.tanh(2.2*svf_lp(x, cut, 7.5 if acc else 5.))
    return y*np.where(t < dur, 1., np.exp(-(t - dur)/.015))*np.minimum(1, t/.002)*(1.25 if acc else .9)
def v_dx7(f, dur, v):                                    # FM electric piano: 1:1 pair + a 14:1 tine on the attack
    n = int((dur + 1.1)*SR); t = _t(n); w = 2*np.pi*f*t; w2 = 2*np.pi*(f + 1.3)*t
    I = (.9 + 1.7*v)*np.exp(-t*2.6) + .22
    x = np.sin(w + I*np.sin(w)) + .55*np.sin(w2 + I*np.sin(w2))
    x += .35*np.sin(w + 2.4*v*np.exp(-t*28)*np.sin(14*w))*np.exp(-t*7)
    return x*np.exp(-t*1.3)*np.minimum(1, t/.002)*np.where(t < dur, 1., np.exp(-(t - dur)/.28))*v*.6
def v_euro(f, dur, v):                                   # complex oscillator: FM + wavefolder into a low-pass gate, delay
    n = int((dur + .3)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph + 1.6*v*np.exp(-t/.07)*np.sin(2*ph)); x = np.sin(2.1*x + .4*np.sin(3*ph)*np.exp(-t/.05))
    env = np.exp(-t/.16)*np.minimum(1, t/.002)
    return echo(svf_lp(x, 300 + 5200*env, 1.1)*env, .75*BEAT_S, .32, 2)*v

_NOTE = {}
def synth(s, idx):
    key = (round(s["t0"], 3), idx)
    if key in _NOTE: return _NOTE[key]
    nt, m, v, dur, tp, cs = s["ev"][idx]; f = midi_hz(m); k = s["key"]
    pe = prev_of(s, idx); fprev = midi_hz(pe[1]) if pe is not None else None
    legato = fprev if pe is not None and nt - (pe[0] + pe[3]) < .12 else None
    if k == "modern": x = v_modern(f, dur, v)
    elif k == "tel": x = v_tel(f, dur, v)
    elif k == "theremin": x = v_ther(f, dur, v, fprev)
    elif k == "ondes": x = v_ondes(f, dur, v, fprev)
    elif k == "nova": x = v_nova(f, dur, v)
    elif k == "rca": x = v_rca(f, dur, v, idx)
    elif k == "moog": x = v_moog(f, dur, v)
    elif k == "mini": x = v_mini(f, dur, v, legato)
    elif k == "prophet": x = v_pro(f, dur, v)
    elif k == "tb303": x = v_303(s, idx)
    elif k == "dx7": x = v_dx7(f, dur, v)
    else: x = v_euro(f, dur, v)
    _NOTE[key] = x; return x

# ---------------------------------------------------------------- backing: 808 electro (broken kick, claps on 2 + 4, 16th hats, cowbell, toms, synth bass)
HAT808 = [205.3, 304.4, 369.6, 522.7, 540., 800.]
def _clap(v):
    a, b = C.clap(1.), D.snare_s(185, 22, 38, 1200, 8000, .45, .25)*.35
    n = max(len(a), len(b)); return (np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))))*v
KIT = {"kick": lambda v: D.kick_s(47, 140, 3.6, .05, 1.0, 1.25, .3)*v, "clap": _clap,
       "hat": lambda v: D.metal_s(HAT808, 55, .1, .1, 7000)*v*.8, "ohat": lambda v: D.metal_s(HAT808, 8, .1, .45, 7000)*v*.7,
       "cow": lambda v: D.bell_s()*v*.8, "crash": P.crash,
       "tom0": lambda v: D.tom_s(210, 6, .25, .6)*v, "tom1": lambda v: D.tom_s(170, 6, .25, .6)*v,
       "tom2": lambda v: D.tom_s(135, 6, .25, .6)*v, "tom3": lambda v: D.tom_s(105, 6, .25, .7)*v}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("clap",)
KA, KB = (0, 6, 10), (0, 3, 6, 10, 13)
def drum_events(c):
    ev = []; s16 = c.BEAT/4; nb = len(BARS)
    for b in range(nb):
        T0 = b*c.BAR
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "clap", .9)]; continue
        ev += [(T0 + p*s16, "kick", 1. if p == 0 else .85) for p in (KA if b % 2 == 0 else KB)]
        ev += [(T0 + p*s16, "hat", (.55, .22, .38, .22)[p % 4]) for p in range(16) if p != 14]
        if b == 0: continue
        ev += [(T0 + 4*s16, "clap", .95), (T0 + 12*s16, "clap", .95), (T0 + 14*s16, "ohat", .5)]
        ev += [(T0 + p*s16, "cow", .4) for p in ((3, 11) if b % 2 else (6,))]
        if b % 4 == 3: ev += [(T0 + (12 + k)*s16, f"tom{k}", .8 - .04*k) for k in range(4)]
        if b in (4, 12, 20, 28): ev.append((T0, "crash", .75))
    return ev
BASS_PAT = ((0, 0, 2), (6, 0, 2), (10, 0, 1), (12, 12, 1), (14, 0, 2))
NO_BASS = (20, 21)        # the TB-303 scene is its own bass line
def bass_events(c):
    out = []; nb = len(BARS); s16 = c.BEAT/4
    for b in range(1, nb):
        T0, r = b*c.BAR, CH[BARS[b]][0]
        if b == nb - 1: out.append((T0, r, 2.4, 1.)); continue
        if b in NO_BASS: continue
        out += [(T0 + p*s16, r + o, d*s16*.9, .95 if p == 0 else .8) for p, o, d in BASS_PAT]
    return out
_BLP = butter(2, 700/(SR/2))
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .05)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = lfilter(*_BLP, .8*np.sin(ph) + .45*bl(ph, f, "sq", 24))
    env = np.minimum(1, t/.004)*np.exp(-t*2.2)*np.where(t < dur, 1., np.exp(-(t - dur)/.02))
    return np.tanh(1.5*x*env)*v
pad_voice = None
MIX = dict(drums=-19, bass=-24, verb=-25, verb_len=1.2)

# ---------------------------------------------------------------- background: inside a circuit board
THEMES = {   # board top, board bottom, glow
 "intro":    ((4, 30, 22), (10, 78, 52), (70, 236, 226)),
 "tel":      ((34, 6, 10), (98, 24, 30), (238, 148, 76)),
 "theremin": ((12, 9, 7), (52, 40, 30), (255, 178, 60)),
 "ondes":    ((22, 10, 40), (80, 42, 122), (222, 228, 244)),
 "nova":     ((36, 14, 10), (116, 56, 38), (250, 206, 92)),
 "rca":      ((20, 24, 28), (82, 90, 98), (110, 255, 130)),
 "moog":     ((28, 20, 10), (100, 76, 44), (255, 142, 40)),
 "mini":     ((26, 13, 8), (86, 46, 26), (232, 188, 44)),
 "prophet":  ((5, 8, 32), (24, 38, 104), (255, 62, 52)),
 "tb303":    ((58, 62, 70), (168, 174, 184), (236, 255, 40)),
 "dx7":      ((32, 5, 13), (98, 20, 40), (40, 222, 192)),
 "euro":     ((9, 9, 11), (38, 38, 44), (255, 42, 42)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(mix(T_[1], (34, 34, 40), .35), T_[2], .08))
def mv(a, b, k): return np.asarray(a, float)*(1 - k) + np.asarray(b, float)*k
BOARD_Y = 262
GRAD = {k: F.make_grad(v[0], v[1], K=8, y0=0, y1=BOARD_Y) for k, v in THEMES.items()}
FLOOR = {k: F.make_grad(dim(v[1], .55), dim(v[0], .7), K=6, y0=BOARD_Y, y1=RH) for k, v in THEMES.items()}
M_FLOOR = YY >= BOARD_Y
SX0, SY0, SX1, SY1 = 26, 106, 244, 164                 # oscilloscope bezel
IX0, IY0, IX1, IY1 = 29, 109, 241, 161                 # its screen

def _seg(x0, y0, x1, y1):
    n = max(abs(x1 - x0), abs(y1 - y0))
    return [(x0 + round((x1 - x0)*i/n), y0 + round((y1 - y0)*i/n)) for i in range(n)] if n else [(x0, y0)]
def _poly(pts):
    out = []
    for a, b in zip(pts, pts[1:]): out += _seg(*a, *b)
    return np.array(out + [pts[-1]])
def _offset(pts, o):
    P_ = np.array(pts, float); res = []
    for i in range(len(P_)):
        ds = []
        if i > 0: d = P_[i] - P_[i - 1]; ds.append(d/np.linalg.norm(d))
        if i < len(P_) - 1: d = P_[i + 1] - P_[i]; ds.append(d/np.linalg.norm(d))
        ns = [np.array([-d[1], d[0]]) for d in ds]
        m = (ns[0] + ns[1])/(1 + ns[0] @ ns[1]) if len(ns) == 2 else ns[0]
        res.append(tuple(int(v) for v in np.round(P_[i] + o*m)))
    return res
def _bus(pts, n, sp=3): return [_offset(pts, (i - (n - 1)/2)*sp) for i in range(n)]
TR = []
TR += _bus([(64, 166), (64, 180), (40, 204), (40, 236)], 5)
TR += _bus([(206, 166), (206, 180), (230, 204), (230, 236)], 5)
TR += _bus([(135, 166), (135, 186), (112, 209), (112, 232)], 4)
TR += _bus([(100, 166), (100, 176), (84, 192), (84, 232)], 3)
TR += _bus([(170, 166), (170, 176), (186, 192), (186, 232)], 3)
for k in range(4):
    TR.append([(24, 116 + 10*k), (6 + 4*k, 116 + 10*k), (6 + 4*k, 172 + 6*k)])
    TR.append([(246, 116 + 10*k), (264 - 4*k, 116 + 10*k), (264 - 4*k, 172 + 6*k)])
for x, j in ((40, -12), (54, -12), (216, 12), (230, 12)):
    TR.append([(x, 104), (x, 84), (x + j, 72), (x + j, 0)])
for k in range(5):
    y = 400 + 15*k
    TR.append([(0, y), (40 + 24*k, y), (48 + 24*k, y + 8), (118 + 18*k, y + 8)])
    TR.append([(269, y + 5), (226 - 20*k, y + 5), (218 - 20*k, y + 13), (170 - 6*k, y + 13)])
TRP = [_poly(tr) for tr in TR]
TRP = [p[(p[:, 0] >= 0) & (p[:, 0] < RW) & (p[:, 1] >= 0) & (p[:, 1] < RH)] for p in TRP]
M_TRACE = np.zeros((RH, RW), bool)
for p in TRP: M_TRACE[p[:, 1], p[:, 0]] = True
PADS = []
for tr in TR:
    for (x, y) in (tr[0], tr[-1]):
        edge = x <= 1 or x >= RW - 2 or y <= 1 or y in (104, 166) or x in (24, 246)
        if not edge: PADS.append((x, y))
PADS += [(30, 176), (240, 176), (120, 196), (150, 196), (12, 440), (258, 436), (80, 460), (190, 466)]
M_PAD = np.zeros((RH, RW), bool); M_HOLE = np.zeros((RH, RW), bool)
for x, y in PADS: M_PAD[max(0, y - 1):y + 2, max(0, x - 1):x + 2] = True; M_HOLE[y, x] = True
LEDS = [(14, 170), (252, 170), (34, 186), (236, 186), (20, 96), (250, 96)]
M_LED = np.zeros((RH, RW), bool)
for x, y in LEDS: M_LED[y:y + 2, x:x + 3] = True
_sil = Image.new("1", (RW, RH), 0); _sd = ImageDraw.Draw(_sil)
for box in ((2, 168, 30, 192), (240, 168, 268, 192), (74, 196, 96, 212), (174, 196, 196, 212), (2, 410, 34, 432), (236, 414, 268, 436)):
    _sd.rectangle(box, outline=1)
for x, y in ((50, 192), (222, 192), (26, 220), (244, 220)): _sd.ellipse((x - 4, y - 4, x + 4, y + 4), outline=1)
for x, y in ((58, 188), (214, 188)): _sd.line((x - 2, y, x + 2, y), fill=1); _sd.line((x, y - 2, x, y + 2), fill=1)
M_SILK = np.array(_sil) & ~M_TRACE & ~M_PAD
M_FING = (YY >= 252) & (YY < BOARD_Y) & ((XX % 6) < 4); M_FING_HI = M_FING & (YY == 252)
M_TRACE &= ~M_FING

# components standing on the floor like buildings (material ids, coloured per era)
_mat = Image.new("L", (RW, RH), 0); _md = ImageDraw.Draw(_mat)
DX_, DY_ = 5, -4
def _chip(x, yb, w, h):
    _md.polygon([(x, yb - h), (x + w, yb - h), (x + w + DX_, yb - h + DY_), (x + DX_, yb - h + DY_)], fill=3)
    _md.polygon([(x + w, yb - h), (x + w + DX_, yb - h + DY_), (x + w + DX_, yb + DY_), (x + w, yb)], fill=2)
    _md.rectangle((x, yb - h, x + w - 1, yb - 1), fill=1)
    for px in range(x + 2, x + w - 1, 3): _md.line((px, yb - 2, px, yb), fill=4)
    _md.point((x + 3, yb - h - 1), fill=2); _md.rectangle((x + w - 6, yb - h - 2, x + w - 4, yb - h - 2), fill=16)
def _cap(x, yb, w, h):
    ry = max(2, w//4)
    _md.ellipse((x, yb - ry, x + w - 1, yb + ry), fill=5)
    _md.rectangle((x, yb - h, x + w - 1, yb), fill=6)
    _md.rectangle((x, yb - h, x + int(w*.18), yb), fill=5); _md.rectangle((x + int(w*.8), yb - h, x + w - 1, yb), fill=5)
    _md.rectangle((x + int(w*.55), yb - h, x + int(w*.68), yb), fill=7)
    _md.rectangle((x + int(w*.2), yb - h, x + int(w*.32), yb), fill=9)
    _md.ellipse((x, yb - h - ry, x + w - 1, yb - h + ry), fill=8)
    _md.line((x + w//2 - 2, yb - h, x + w//2 + 2, yb - h), fill=5)
def _res(cx, yb, h):
    _md.line((cx, yb, cx, yb - h), fill=17)
    _md.rectangle((cx - 3, yb - h + 3, cx + 3, yb - h + 16), fill=10); _md.line((cx + 3, yb - h + 3, cx + 3, yb - h + 16), fill=11)
    for k, m_ in enumerate((12, 13, 14, 15)): _md.line((cx - 3, yb - h + 5 + 3*k, cx + 2, yb - h + 5 + 3*k), fill=m_)
def _coil(x, yb, w, h):
    for y in range(yb - h, yb): _md.line((x, y, x + w - 1, y), fill=19 if (y // 2) % 2 else 20)
    _md.polygon([(x, yb - h), (x + w, yb - h), (x + w + DX_, yb - h + DY_), (x + DX_, yb - h + DY_)], fill=21)
_cap(1, 268, 21, 70); _chip(24, 272, 26, 12); _cap(36, 268, 13, 44); _res(56, 272, 30); _cap(62, 273, 9, 20); _coil(76, 274, 12, 16)
_cap(248, 268, 21, 62); _chip(218, 272, 26, 13); _cap(222, 268, 14, 48); _res(210, 272, 26); _cap(196, 273, 9, 18); _coil(178, 274, 12, 14)
_chip(96, 268, 76, 18)
MAT = np.array(_mat); M_COMP = MAT > 0
def comp_pal(th):
    top, bot, glow = THEMES[th]
    chip = mix((22, 22, 26), bot, .15); sleeve = mix(dim(glow, .55), (20, 20, 30), .45)
    return np.array([(0, 0, 0), chip, dim(chip, .7), mix(chip, (255, 255, 255), .14), (196, 198, 206),
                     dim(sleeve, .7), sleeve, mix(sleeve, (255, 255, 255), .25), mix(sleeve, (220, 220, 228), .55),
                     mix(sleeve, (255, 255, 255), .6), (214, 190, 150), (170, 148, 112), (190, 40, 30), (40, 40, 40),
                     (230, 160, 40), (160, 120, 60), dim(glow, .35), (150, 150, 158), mix(bot, (255, 255, 255), .5),
                     (52, 52, 60), (36, 36, 42), (80, 80, 90)], float)
PALS = {k: comp_pal(k) for k in THEMES}

M_BEZ = (YY >= SY0) & (YY < SY1) & (XX >= SX0) & (XX < SX1)
M_BEZ &= ~(((YY == SY0) | (YY == SY1 - 1)) & ((XX == SX0) | (XX == SX1 - 1)))
M_SCR = (YY >= IY0) & (YY < IY1) & (XX >= IX0) & (XX < IX1)
M_BEZ_HI = M_BEZ & ~M_SCR & (YY < IY0); M_BEZ_LO = M_BEZ & ~M_SCR & (YY >= IY1)
M_GRAT = M_SCR & ((((XX - IX0 - 6) % 12 == 0) & ((YY - IY0 - 2) % 12 == 0)) | ((YY == (IY0 + IY1)//2) & (XX % 3 == 0)))
M_TXT = ((YY >= 38) & (YY < 102)) | ((YY >= 330) & (YY < 382))
M_TXTE = (((YY >= 34) & (YY < 38)) | ((YY >= 102) & (YY < 106)) | ((YY >= 326) & (YY < 330)) | ((YY >= 382) & (YY < 386))) & (BAY < .5)
M_TRACE &= ~M_BEZ; M_PAD &= ~M_BEZ

def scope_wave(c, si, T):
    """the current instrument's real waveform: its live notes summed, triggered on a rising zero crossing"""
    s = c.SCENES[si]; t = T - s["t0"]; live = []
    for idx, e in enumerate(s["ev"]):
        if e[0] <= t < e[0] + e[3] + 1.0: live.append(idx)
    if not live: return None
    main = max(live, key=lambda i: (s["ev"][i][0], s["ev"][i][1]))
    f = midi_hz(s["ev"][main][1]); N = int(clamp(3.2*SR/f, 400, 2400))
    buf = np.zeros(2*N); pk = 1e-6
    for idx in live:
        x = synth(s, idx); a = int((t - s["ev"][idx][0])*SR)
        if a >= len(x): continue
        seg = x[a:a + 2*N]; buf[:len(seg)] += seg; pk = max(pk, np.abs(x[:int(.3*SR)]).max())
    z = np.nonzero((buf[:N - 1] < 0) & (buf[1:N] >= 0))[0]; st = int(z[0]) if len(z) else 0
    w = buf[st:st + N]
    if np.abs(w).max() < 1e-5: return None
    lv = clamp(np.abs(w).max()/pk, .08, 1.)
    y = w[np.linspace(0, N - 1, IX1 - IX0).astype(int)]
    return -y/np.abs(w).max()*21*lv

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return mv(A_[i], B_[i], kb)
    bot, glow = col(1), col(2)
    bg = GRAD[th0]*(1 - kb) + GRAD[th1]*kb
    fl = FLOOR[th0]*(1 - kb) + FLOOR[th1]*kb; bg[M_FLOOR] = fl[M_FLOOR]
    bg[M_SILK] = bg[M_SILK]*.62 + mv(bot, (236, 236, 226), .5)*.38
    copper = mv(mv((178, 112, 50), glow, .3), bot, .3)
    bg[M_TRACE] = copper; bg[M_PAD] = mv(copper, (255, 255, 255), .18); bg[M_HOLE] = bot*.35
    gold = mv((214, 170, 70), glow, .15); bg[M_FING] = gold; bg[M_FING_HI] = np.minimum(255, gold*1.25)
    for o in c.ONSETS:                                                                # every note: light pulses race down the traces
        age = T - o
        if not 0 <= age < 1.5: continue
        key = int(round(o*1000))
        for j in range(3):
            tr = TRP[(key*7 + j*11) % len(TRP)]
            if (key + j) % 2: tr = tr[::-1]
            L = len(tr); p = age*250
            if p - 18 > L + 30: continue
            if p < L + 18:
                ks = np.arange(max(0, int(p) - 18), min(L, int(p) + 1))
                if len(ks):
                    a = np.clip(1 - (p - ks)/18, 0, 1)[:, None]; pts = tr[ks]
                    bg[pts[:, 1], pts[:, 0]] = bg[pts[:, 1], pts[:, 0]]*(1 - a) + mv(glow, (255, 255, 255), .25)*a
            if p >= L and L:
                ex, ey = tr[-1]; a = clamp(1 - (p - L)/40)
                bg[max(0, ey - 1):ey + 2, max(0, ex - 1):ex + 2] = bg[max(0, ey - 1):ey + 2, max(0, ex - 1):ex + 2]*(1 - a) + glow*a
    pal = PALS[th0]*(1 - kb) + PALS[th1]*kb
    pal[16] = mv(pal[16], glow, x["pulse"])
    bg[M_COMP] = pal[MAT[M_COMP]]
    bg[M_LED] = mv(glow*.3, glow, x["pulse"])
    bg[M_BEZ] = (64, 66, 74); bg[M_BEZ_HI] = (112, 114, 124); bg[M_BEZ_LO] = (34, 34, 40)    # the oscilloscope
    bg[M_SCR] = mv((3, 5, 6), glow, .07); bg[M_GRAT] = mv((3, 5, 6), glow, .3)
    wv = scope_wave(c, x["si"], T)
    cy = (IY0 + IY1)/2
    ys = np.full(IX1 - IX0, cy) if wv is None else cy + wv
    ys = np.clip(np.round(ys).astype(int), IY0 + 1, IY1 - 2)
    core = mv(glow, (255, 255, 255), .4); halo = glow
    for i in range(len(ys)):
        xx = IX0 + i; ya = ys[i]; yb = ys[i - 1] if i else ya
        lo, hi = min(ya, yb), max(ya, yb)
        for yy in (lo - 2, hi + 2):
            if IY0 <= yy < IY1: bg[yy, xx] = bg[yy, xx]*.8 + halo*.2
        for yy in (lo - 1, hi + 1):
            if IY0 <= yy < IY1: bg[yy, xx] = bg[yy, xx]*.5 + halo*.5
        bg[lo:hi + 1, xx] = core
    bg[M_TXT] *= .5; bg[M_TXTE] *= .72
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

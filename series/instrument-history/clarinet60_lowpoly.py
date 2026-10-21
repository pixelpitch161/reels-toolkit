"""Pixel Pitch: history of the clarinet, 60-second cut.  python3 clarinet60_lowpoly.py [out.mp4] | --still t ..."""
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
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "clarinet60_lowpoly.mp4", "cl"
BPM = 120
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 2.1, -6, 10, .25
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE CLARINET", "335 YEARS", "TODAY", 1690
INTRO_LINES = (("ONE REED, 17 KEYS,", "FOUR OCTAVES"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("335 YEARS FROM", "THE CHALUMEAU")
OUTRO_CREDIT = "ELECTRO-SWING CLARINET"
T3 = 1/3

# ---------------------------------------------------------------- music (electro-swing groove)
CH = {"Gm": (43, (55, 58, 62)), "Cm": (36, (55, 60, 63)), "D7": (38, (54, 60, 62)), "F": (41, (57, 60, 65)), "C7": (36, (55, 58, 64)),
      "C": (36, (55, 60, 64)), "G7": (43, (53, 59, 62)), "A": (45, (57, 61, 64)), "E7": (40, (56, 59, 62)), "Eb": (39, (55, 58, 63)),
      "Bb7": (46, (56, 62, 65)), "Dm": (38, (57, 62, 65)), "Bm": (47, (54, 59, 62)), "Em": (40, (55, 59, 64)), "F#7": (42, (54, 58, 64)),
      "D": (38, (57, 62, 66)), "Eb7": (39, (55, 61, 63)), "Fm": (41, (56, 60, 65))}
BARS = ["Gm", "Cm", "D7", "Gm",
        "F", ("C7", "C7", "F", "F"), "C", ("F", "F", "G7", "C"), "A", ("E7", "E7", "A", "A"), "Eb", ("Bb7", "Bb7", "Eb", "Eb"),
        "Dm", ("C", "C", "Dm", "Dm"), "Bm", ("Em", "Em", "F#7", "F#7"), "D", ("Gm", "Gm", "D", "D"), "Bb7", ("Eb7", "Eb7", "Bb7", "Bb7"),
        "Bb7", ("Eb", "Eb", "Bb7", "Bb7"), "Cm", ("Fm", "Fm", "G7", "G7"), "F", ("Dm", "Dm", "F", "F"),
        "Gm", "Cm", "D7", "Gm"]
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
RIFF = [(0, 67, 2*T3), (2*T3, 70, T3), (1, 74, 2*T3), (1 + 2*T3, 72, T3), (2, 70, 2*T3), (2 + 2*T3, 69, T3), (3, 67, 1),
        (4, 72, 2*T3), (4 + 2*T3, 75, T3), (5, 79, 1), (6, 77, 2*T3), (6 + 2*T3, 75, T3), (7, 74, 1)]
RIFF2 = [(0, 66, 2*T3), (2*T3, 69, T3), (1, 72, 2*T3), (1 + 2*T3, 75, T3), (2, 74, 1), (3, 72, 2*T3), (3 + 2*T3, 69, T3)]
TRILL = [(k/8, 53 if k % 2 == 0 else 55, 1/8) for k in range(12)]

SCENES = [
 dict(key="modern", theme="intro", name="MODERN CLARINET", credit="ELECTRO-SWING CLARINET",
      notes=tn(RIFF) + tn(RIFF2, b0=8) + tn([(12, 67, 2), (14, 74, 2*T3), (14 + 2*T3, 72, T3), (15, 70, 2*T3), (15 + 2*T3, 69, T3)])),
 dict(key="chal", theme="chal", year=1690, ys="1690s", name="CHALUMEAU", credit="IN THE STYLE OF TELEMANN",
      fact=("THE CLARINET'S", "SINGLE-REED ANCESTOR"),
      notes=seq(0, [(65, .5), (67, .5), (69, 1), (70, .5), (69, .5), (67, 1), (64, .5), (65, .5), (67, .5), (64, .5), (65, 2)], .85)),
 dict(key="denner", theme="denner", year=1700, ys="1700", name="DENNER CLARINET", credit="BAROQUE CLARINO FANFARE",
      fact=("ITS NAME MEANS", "'LITTLE TRUMPET'"),
      notes=seq(0, [(72, .5), (72, .25), (72, .25), (76, .5), (79, .5), (84, 1), (79, 1), (81, .5), (79, .5), (77, .5), (76, .5), (74, 1), (72, 1)], .85)),
 dict(key="classic", theme="classic", year=1791, ys="1791", name="CLASSICAL CLARINET", credit="IN THE STYLE OF MOZART",
      fact=("MOZART'S LAST", "CONCERTO WAS FOR IT"),
      notes=seq(0, [(73, 1.5), (71, .5), (69, .5), (73, .5), (76, 1), (74, .5), (73, .5), (71, .5), (74, .5), (73, 1), (69, 1)], .85)),
 dict(key="muller", theme="muller", year=1812, ys="1812", name="13-KEY CLARINET", credit="IN THE STYLE OF WEBER",
      fact=("MULLER'S 13 KEYS", "PLAYED IN ANY KEY"),
      notes=seq(0, [(63, 1), (70, .5), (75, .5), (79, 1.5), (77, .25), (75, .25), (74, .5), (75, .5), (77, .5), (70, .5), (75, 2)], .85)),
 dict(key="bass", theme="bass", year=1838, ys="1838", name="BASS CLARINET", credit="ROMANTIC OPERA SOLO", size=.62,
      fact=("ADOLPHE SAX REBUILT", "THE BASS CLARINET"),
      notes=seq(0, [(50, 1), (53, .5), (55, .5), (57, 1.5), (55, .5), (52, 1), (50, .5), (48, .5), (50, 2)], .9)),
 dict(key="boehm", theme="boehm", year=1844, ys="1844", name="BOEHM CLARINET", credit="IN THE STYLE OF BRAHMS",
      fact=("THE KEY SYSTEM MOST", "PLAYERS USE TODAY"),
      notes=seq(0, [(71, 1), (74, .5), (73, .5), (71, 1), (66, 1), (67, .5), (69, .5), (71, .5), (67, .5), (66, 2)], .85)),
 dict(key="klezmer", theme="klezmer", year=1915, ys="1910s", name="KLEZMER CLARINET", credit="KLEZMER FREYLEKHS",
      fact=("KLEZMER CLARINET", "LAUGHS AND CRIES"), gl={4: (-1.5, .12), 6: (-2, .16), 2: (-1, .1)},
      notes=seq(0, [(62, .5), (63, .5), (66, .5), (67, .5), (69, 1), (70, .5), (69, .5), (67, .5), (66, .5), (63, .5), (66, .5), (62, 2)], .88)),
 dict(key="nola", theme="nola", year=1923, ys="1920s", name="NEW ORLEANS CLARINET", credit="IN THE STYLE OF JOHNNY DODDS",
      fact=("CLARINETS WOVE THE", "NEW ORLEANS SOUND"), gl={2: (-2, .12)},
      notes=tn([(0, 70, 2*T3), (2*T3, 73, T3), (1, 74, 2*T3), (1 + 2*T3, 77, T3), (2, 79, 1), (3, 77, 1),
                (4, 79, 2*T3), (4 + 2*T3, 77, T3), (5, 75, 2*T3), (5 + 2*T3, 73, T3), (6, 74, 1), (7, 70, 1)], .88)),
 dict(key="gershwin", theme="gershwin", year=1924, ys="1924", name="JAZZ AGE CLARINET", credit="GERSHWIN: RHAPSODY IN BLUE",
      fact=("RHAPSODY IN BLUE", "OPENS WITH A GLISS"), gl={1.5: (-29, .62)},
      notes=tn(TRILL, .8) + tn([(1.5, 82, 1.5), (3, 80, .5), (3.5, 77, .5), (4, 79, 1), (5, 77, .5), (5.5, 75, .5), (6, 77, 2)], .9)),
 dict(key="goodman", theme="goodman", year=1935, ys="1930s", name="SWING CLARINET", credit="IN THE STYLE OF BENNY GOODMAN",
      fact=("BENNY GOODMAN: THE", "KING OF SWING"),
      notes=tn([(0, 72, 2*T3), (2*T3, 75, T3), (1, 77, 2*T3), (1 + 2*T3, 78, T3), (2, 79, 1), (3, 84, 1),
                (4, 80, 2*T3), (4 + 2*T3, 79, T3), (5, 77, 2*T3), (5 + 2*T3, 80, T3), (6, 79, 1), (7, 74, 1)], .88)),
 dict(key="bilk", theme="bilk", year=1962, ys="1962", name="POP CLARINET", credit="IN THE STYLE OF ACKER BILK",
      fact=("FIRST BRITISH NO. 1", "ON THE US HOT 100"),
      notes=seq(0, [(69, 1.5), (67, .5), (65, 1), (60, 1), (62, 1.5), (64, .5), (65, 2)], .85)),
 dict(key="modern", theme="intro", year=2020, ys="2020s", name="MODERN CLARINET", credit="ELECTRO-SWING CLARINET",
      fact=("ELECTRO-SWING PUTS", "IT ON DANCE FLOORS"),
      notes=tn(RIFF) + tn(RIFF2, b0=8) + [(12, 79, 4.3, .92)]),
]

TRIM = {"modern": .3, "chal": .6, "denner": -.5, "classic": -.4, "muller": -.3, "bass": .7, "boehm": -.4, "nola": -.2, "gershwin": .3, "goodman": -.2}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)

# ---------------------------------------------------------------- instruments (real proportions, metres)
# Bb clarinet ~66 cm: mouthpiece 7.5, barrel 6.5, upper joint 23, lower joint 20, bell 9.5 cm; body ~2.9 cm across,
# bell ~6.6 cm. Boehm system: six ringed tone holes on the front, long rods and pinky-key clusters down the side.
BLACK, NICKEL, SKIN, GREN, BOX = (24, 22, 26), (214, 216, 226), (230, 186, 150), (44, 30, 28), (222, 184, 116)
CL = {
 "modern":   dict(L=.66, r=.0145, rb=.033, wood=(40, 28, 26), keys=(222, 224, 234), rings=True, sys="boehm", tenon=(222, 224, 234)),
 "boehm":    dict(L=.66, r=.0145, rb=.032, wood=(52, 34, 28), keys=(206, 208, 216), rings=True, sys="boehm", tenon=(206, 208, 216)),
 "gershwin": dict(L=.66, r=.0145, rb=.033, wood=(36, 26, 26), keys=(214, 216, 226), rings=True, sys="boehm", tenon=(214, 216, 226)),
 "goodman":  dict(L=.66, r=.0145, rb=.033, wood=(30, 24, 24), keys=(222, 224, 234), rings=True, sys="boehm", tenon=(222, 224, 234), mic=True),
 "klezmer":  dict(L=.66, r=.0145, rb=.032, wood=(60, 38, 30), keys=(200, 196, 180), rings=True, sys="albert", tenon=(200, 196, 180)),
 "nola":     dict(L=.66, r=.0145, rb=.032, wood=(48, 32, 28), keys=(208, 204, 190), rings=True, sys="albert", tenon=(208, 204, 190)),
 "bilk":     dict(L=.66, r=.0145, rb=.032, wood=(44, 30, 28), keys=(214, 214, 220), rings=True, sys="albert", tenon=(214, 214, 220), hat=True),
 "muller":   dict(L=.66, r=.015, rb=.03, wood=(150, 96, 52), keys=(226, 196, 110), rings=False, sys="muller", tenon=(236, 226, 200)),
 "classic":  dict(L=.66, r=.015, rb=.03, wood=BOX, keys=(220, 186, 100), rings=False, sys="five", tenon=(240, 232, 212)),
 "denner":   dict(L=.55, r=.015, rb=.024, wood=(228, 196, 130), keys=(220, 186, 100), rings=False, sys="two", tenon=(210, 170, 110)),
 "chal":     dict(L=.38, r=.016, rb=0, wood=(214, 176, 112), keys=(220, 186, 100), rings=False, sys="chal", tenon=(196, 156, 96)),
 "bass":     dict(),
}
def rot_y(a): c, s_ = math.cos(a), math.sin(a); return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])
def rot_z(a): c, s_ = math.cos(a), math.sin(a); return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])
POSE = rot_y(math.radians(-16)) @ rot_z(math.radians(-63))
def xform(g, f0, l0, R=None, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]
        if R is not None: P_ = P_ @ R.T; n = R @ n
        g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]
        if R is not None: a, b = R @ a, R @ b
        g.L[i] = (a + t, b + t, col, gid)
def box_at(g, p, s, col, emit=False, spec=0.):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit, spec=spec)

def fingering(m):
    """(front holes closed 0..6, register key, throat key, pinky) for concert pitch m on a Bb clarinet"""
    w = m + 2; reg = w >= 71
    b = w - 19 if reg else w
    closed = 6 if b <= 56 else {57: 5, 58: 5, 59: 4, 60: 3, 61: 3, 62: 2, 63: 2, 64: 1}.get(b, 0)
    return closed, reg, 68 <= b <= 70, b < 55

def clarinet(g, c, m):
    """local frame: x = bore axis from the mouthpiece tip (x=0) to the bell rim, keys face +z"""
    L, r, wood, kc, tc = c["L"], c["r"], c["wood"], c["keys"], c["tenon"]; s = L/.66; Fz = np.array([0, 0, 1.])
    old = c["sys"] in ("five", "two", "chal", "muller")
    mp = .075 if c["sys"] != "chal" else .06
    T.tube(g, (0, 0, 0), (.014, 0, 0), r*.32, r*.62, col=BLACK, n=8, spec=.6)                 # mouthpiece beak
    T.tube(g, (.014, 0, 0), (mp, 0, 0), r*.62, r*.92, col=BLACK, n=10, spec=.6)
    T.tube(g, (.024, 0, 0), (.042, 0, 0), r*.86, r*.9, col=NICKEL if not old else (200, 180, 140), n=10, spec=.8)
    g.box(.003, .055, -r*.72 - .002, -r*.72 + .0015, -.0055, .0055, (226, 202, 144))       # reed
    if c["sys"] == "chal":
        T.tube(g, (mp, 0, 0), (L, 0, 0), r, col=wood, n=10, spec=.3)
        for x in (mp, L - .012): T.tube(g, (x, 0, 0), (x + .012, 0, 0), r*1.12, col=tc, n=10)
        mouth = np.array([L, 0, 0])
    else:
        xb, xu, xl, xbell = mp + .065*s, mp + .065*s + .23*s, mp + .065*s + .43*s, L
        T.tube(g, (mp, 0, 0), (xb, 0, 0), r*1.16, r*1.1, col=wood, n=10, spec=.4)            # barrel
        T.tube(g, (xb, 0, 0), (xl, 0, 0), r, col=wood, n=10, spec=.4)                         # upper + lower joints
        for x in (mp, xb - .006, xb, xu - .007, xu, xl - .008):                               # tenon rings / ferrules
            T.tube(g, (x, 0, 0), (x + .007, 0, 0), r*1.13, col=tc, n=10, spec=.8)
        if c["rb"]:
            prev = (xl, r)
            for k in range(1, 9):
                u = k/8; x = lerp(xl, xbell, u); rr = r + (c["rb"] - r)*u**2.4
                T.tube(g, (prev[0], 0, 0), (x, 0, 0), prev[1], rr, col=wood, n=12, spec=.4, two=True); prev = (x, rr)
            T.tube(g, (xbell - .008, 0, 0), (xbell, 0, 0), c["rb"]*1.06, col=tc, n=12, spec=.8, two=True)
            g.face(S.ring((xbell - .03, 0, 0), (1, 0, 0), r*1.1, 10), (16, 12, 10), ref=(0, 0, 0), emit=True, two=True)
        mouth = np.array([xbell, 0, 0])
    closed, reg, throat, pinky = fingering(m)
    if c["sys"] == "chal": holes = [mp + (.08 + .035*k)*1.0 for k in range(6)]
    else: holes = [lerp(xb, xu, f) for f in (.42, .56, .70)] + [lerp(xu, xl, f) for f in (.25, .42, .6)]
    for i, x in enumerate(holes):
        top = np.array([x, 0, r]); shut = i < closed
        if c["rings"]:
            T.tube(g, top + Fz*(.0 if shut else .0025), top + Fz*(.0025 if shut else .005), .0085, col=kc, n=10, spec=.9)
        if shut: box_at(g, top + Fz*.009, .0075, SKIN)
        else: g.face(S.ring(top + Fz*(.0052 if c["rings"] else .0004), Fz, .0042, 8), (14, 10, 10), ref=top - Fz, emit=True)
    side = np.array([0, .72, .69]); side /= np.linalg.norm(side)
    if c["sys"] in ("boehm", "albert"):
        for x0, x1 in ((xb + .02, xu - .01), (xu + .012, xl - .01)):                          # key rods
            g.line((x0, 0, 0) + side*(r + .004), (x1, 0, 0) + side*(r + .004), kc)
        for k in range(4 if c["sys"] == "boehm" else 2):                                      # side trill keys
            S.prism(g, np.array([lerp(xb, xu, .2 + .07*k), 0, 0]) + side*(r + .002), side, .0065, .004, 6, kc, dim(kc, .8))
        tk = np.array([lerp(xb, xu, .3), 0, 0]) - np.array([0, -.35, .94])*0 + np.array([0, .25, .97])*(r + .004)
        box_at(g, tk, .006, HOT if throat else kc, emit=throat, spec=.8)                      # throat A key
        for k, x in enumerate((xu - .028, xu - .018, xl - .03, xl - .02, xl - .01)):           # pinky keys
            p = np.array([x, 0, 0]) - side*0 + np.array([0, -.6, .8])*(r + .004)
            box_at(g, p, .005, HOT if (pinky and k % 2 == 0) else kc, emit=pinky, spec=.8)
    else:
        nk = {"chal": 2, "two": 2, "five": 5, "muller": 13}[c["sys"]]
        span = (mp + .02, L - .03) if c["sys"] == "chal" else (xb + .02, xl - .01)
        for k in range(nk):
            x = lerp(*span, (k + .5)/nk); sd = side if k % 2 == 0 else np.array([0, -.72, .69])/np.linalg.norm([0, -.72, .69])
            p = np.array([x, 0, 0]) + sd*(r + .003); on = (k == 0 and reg) or (k == nk - 1 and pinky)
            g.line(p - [.018, 0, 0], p + [.006, 0, 0], kc)
            if c["sys"] == "muller": S.prism(g, p + [.008, 0, 0], sd, .006, .005, 8, HOT if on else kc, dim(kc, .8), emit=on)
            else: g.box(p[0], p[0] + .012, p[1] - .005, p[1] + .005, p[2] - .005, p[2] + .005, HOT if on else kc, emit=on, spec=.7)
    if c["sys"] in ("boehm", "albert"):
        rk = np.array([lerp(xb, xu, .3), 0, 0]) + np.array([0, .9, .44])*(r + .004)
        box_at(g, rk, .005, HOT if reg else kc, emit=reg, spec=.8)                            # register key
    return mouth

def bass_clarinet(g, m):
    """vertical body, silver crook neck at the top, upturned silver bell at the bottom"""
    wood, met, r = (30, 26, 28), (214, 216, 226), .021; Fz = np.array([0, 0, 1.])
    T.tube(g, (0, .26, 0), (0, 1.0, 0), r, col=wood, n=10, spec=.4)
    for y in (.26, .62, .64, .99): T.tube(g, (0, y, 0), (0, y + .012, 0), r*1.12, col=met, n=10, spec=.8)
    neck = [np.array([0, 1.0, 0]), np.array([0, 1.06, 0]), np.array([-.02, 1.11, 0]), np.array([-.06, 1.14, 0]), np.array([-.11, 1.14, 0]), np.array([-.15, 1.12, 0])]
    for a, b in zip(neck, neck[1:]): T.tube(g, a, b, .0085, col=met, n=8, spec=.9)
    T.tube(g, neck[-1], neck[-1] + [-.05, -.045, 0], .012, .007, col=BLACK, n=8)
    bend = [np.array([0, .26, 0]) + np.array([.07*(1 - math.cos(a)), -.07*math.sin(a), 0]) for a in np.linspace(0, math.pi, 7)]
    for a, b in zip(bend, bend[1:]): T.tube(g, a, b, r*1.05, col=met, n=10, spec=.9)
    mouth = S.bell_dir(g, np.array([.14, .26, 0]), (.15, 1, 0), .16, r*1.05, .055, met)
    closed, reg, throat, pinky = fingering(m + 12)
    for i, y in enumerate((.92, .87, .82, .56, .51, .46)):
        top = np.array([0, y, r]); shut = i < closed
        T.tube(g, top + Fz*(0 if shut else .003), top + Fz*(.003 if shut else .006), .012, col=met, n=10, spec=.9)
        if shut: box_at(g, top + Fz*.012, .01, SKIN)
        else: g.face(S.ring(top + Fz*.0062, Fz, .006, 8), (14, 10, 10), ref=top - Fz, emit=True)
    g.line((.016, .95, .014), (.016, .3, .014), met)
    S.rod(g, (0, 0, -.03), (0, .12, -.01), .01); g.ngon(0, .004, -.03, .1, (40, 38, 46), n=8)
    return np.asarray(mouth, float)

def bowler(g, p, s=.06):
    crown, band = (34, 30, 34), (90, 20, 30)
    g.ngon(p[0], p[1] + .004, p[2], s*1.6, crown, n=12)
    T.tube(g, (p[0], p[1] + .004, p[2]), (p[0], p[1] + s*1.3, p[2]), s, s*.92, col=crown, n=12, spec=.3)
    T.tube(g, (p[0], p[1] + .006, p[2]), (p[0], p[1] + s*.3, p[2]), s*1.01, col=band, n=12)
    g.face(S.ring((p[0], p[1] + s*1.3, p[2]), (0, 1, 0), s*.92, 12), crown, ref=(p[0], 0, p[2]))

def mic(g, p):
    S.rod(g, (p[0], 0, p[2]), (p[0], p[1] - .03, p[2]), .006); g.ngon(p[0], .004, p[2], .07, (40, 38, 46), n=8)
    g.box(p[0] - .028, p[0] + .028, p[1] - .04, p[1] + .04, p[2] - .016, p[2] + .016, (190, 194, 204), spec=.8)
    for k in range(5): y = p[1] - .03 + .015*k; g.line((p[0] - .026, y, p[2] + .017), (p[0] + .026, y, p[2] + .017), (60, 60, 70))

def cur_note(s, t, ev):
    live = [e for e in ev if e[0] < e[3] + .03]
    return min(live)[1] if live else (min(ev)[1] if ev else s["ev"][0][1])

def build(g, t, ev, fx, s):
    k = s["key"]; m = cur_note(s, t, ev); f0, l0 = len(g.F), len(g.L)
    if k == "bass":
        mouth = bass_clarinet(g, m); up = True
    else:
        c = CL[k]; mouth = clarinet(g, c, m); up = False
        cen = np.array([c["L"]/2, 0, 0]); o = np.array([0, .39, 0]) - POSE @ cen
        xform(g, f0, l0, R=POSE, t=o); mouth = POSE @ mouth + o
        if c.get("hat"): bowler(g, (.27, 0, .2), .045)
        if c.get("mic"): mic(g, (.28, .3, .14))
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9: fx.append(("note", tuple(mouth + ([.05, .16, 0] if up else [.08, .12, 0])), age))
        if tp and age < .6: fx.append(("wave", tuple(mouth + [.02, 0, 0]), age, 0))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def reed(f, dur, v, bright=.6, vib=.004, vrate=5.5, odd=.8, scoop=.02, att=.03, formant=1500., fboost=.8, gl=None):
    n = int((dur + .14)*SR); t = _t(n)
    env = np.minimum(1, t/att)**1.3*np.where(t < dur, 1, np.exp(-(t - dur)/.04))*(.9 + .1*np.exp(-t*4))
    fr = f*(1 - scoop*np.exp(-t/.04))*(1 + vib*np.sin(2*np.pi*vrate*t)*np.clip((t - .15)/.25, 0, 1))
    if gl:
        semis, secs = gl; u = np.clip(t/secs, 0, 1); fr = fr*2**(semis*(1 - u*u*(3 - 2*u))/12)
    ph = 2*np.pi*np.cumsum(fr)/SR
    dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
    for k in range(1, 40):
        fk = f*k
        if fk > 8000: break
        base = (1/k**1.1)*((1 - odd*.9) if k % 2 == 0 else 1)*(1 + fboost*math.exp(-((fk - formant)/900)**2))
        a1 = base*math.exp(-k/2.5); a2 = base*math.exp(-k/(4 + 8*bright))
        s_ = np.sin(k*ph); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
    mb = np.clip(env*v, 0, 1)**1.2
    return (dark/sd*(1 - mb) + brt/sb*mb)*env*v
TONES = {"modern": dict(bright=.6, vib=.003), "chal": dict(bright=.25, vib=0, odd=.95, att=.04, formant=900, fboost=.3),
         "denner": dict(bright=.75, vib=0, odd=.6, formant=1800, fboost=1.0, att=.025), "classic": dict(bright=.4, vib=0, odd=.85, att=.035),
         "muller": dict(bright=.45, vib=.002, odd=.85), "bass": dict(bright=.35, vib=.002, odd=.95, formant=700, fboost=.6, att=.04),
         "boehm": dict(bright=.45, vib=.002, odd=.85), "klezmer": dict(bright=.75, vib=.006, vrate=6.2, scoop=.04, odd=.7),
         "nola": dict(bright=.7, vib=.009, vrate=6.0, scoop=.04, odd=.7), "gershwin": dict(bright=.75, vib=.004, odd=.7, att=.015),
         "goodman": dict(bright=.8, vib=.004, vrate=5.8, odd=.65), "bilk": dict(bright=.4, vib=.012, vrate=5.6, odd=.9, scoop=.03)}
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]; tone = dict(TONES[key])
    if m + 2 >= 71 and key not in ("chal", "bass"): tone["odd"] = tone.get("odd", .8)*.65          # clarion register: more even harmonics
    beat = round(nt/BEAT_S, 3); gl = next((v_ for b, v_ in s.get("gl", {}).items() if abs(b - beat) < 1e-3), None)
    x = reed(midi_hz(m), dur, v, gl=gl, **tone)
    if key == "modern":
        dl = int(.75*BEAT_S*SR); y = np.zeros(len(x) + 2*dl); y[:len(x)] += x; y[dl:dl + len(x)] += .25*x; y[2*dl:2*dl + len(x)] += .1*x; x = y
    return x

# ---------------------------------------------------------------- backing: electro-swing (four-on-the-floor, swing ride, walking bass, charleston piano)
def _clap(v):
    a, b = C.clap(1.)*.7, P.snare(1.)*.4; n = max(len(a), len(b)); return (np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))))*v
KIT = {"kick": lambda v: D.kick_s(50, 120, 7, .15, .45, 1.6, .3)*v, "clap": lambda v: _clap(v),
       "ride": lambda v: D.ride_s()*.8*v, "hat": lambda v: P.hat(v), "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("clap",)
RIDE = [(0, .7), (1, .8), (1 + 2*T3, .5), (2, .7), (3, .8), (3 + 2*T3, .5)]
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "clap", .8)]; continue
        ev += [(T0 + o*Q, "ride", v*(.6 if b == 0 else 1.)) for o, v in RIDE]
        if b == 0: ev += [(T0 + k*Q + 2*T3*Q, "hat", .5) for k in range(4)]; continue
        ev += [(T0 + k*Q, "kick", .95 if k % 2 == 0 else .85) for k in range(4)] + [(T0 + Q, "clap", .85), (T0 + 3*Q, "clap", .85)]
        ev += [(T0 + k*Q + .5*Q, "hat", .55) for k in range(4)]
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
        for k in range(4):
            r = _root(_chord_at(b, k)); nxt = _root(_chord_at(b + 1, 0))
            m = (r, r + 7, r + 12, nxt - 1 if k == 3 else r + 7)[k] if k < 3 else (nxt + 1 if nxt < r else nxt - 1)
            out.append((T0 + k*c.BEAT, m, .8*c.BEAT, .9 if k % 2 == 0 else .8))
    return out
def pluck(f, dur, v, bright=.5, pos=.2, sus=1.):
    ring = dur + .25; n = int(ring*SR); t = _t(n); x = np.zeros(n)
    for k in range(1, 18):
        if f*k > 7000: break
        a = (abs(math.sin(math.pi*k*pos)) + .04)/k**(1.5 - .8*bright)
        x += a*np.sin(2*np.pi*f*k*t + k)*np.exp(-t*(1.5 + .7*(k - 1))*(.6 + f/500)/sus)
    return x*np.minimum(1, t/.002)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))*v
def bass_voice(m, dur, v):
    f = midi_hz(m); x = pluck(f, dur, v, bright=.3, pos=.25, sus=2.2); t = _t(len(x))
    return x + .7*np.sin(2*np.pi*f*t)*np.exp(-t*3)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))*v
def _pno(notes, dur, v):
    n = int((dur + .3)*SR); out = np.zeros(n)
    for m in notes:
        x = P.additive(midi_hz(m), dur + .25, v, harm=10, tilt=1.2, d1=1.4); out[:len(x)] += x[:n]
    t = _t(n); return out*np.where(t < dur, 1, np.exp(-(t - dur)/.05))
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); out = np.zeros(n); hi = [m + 12 for m in notes]
    if abs(dur - (BAR_S + .05)) < .1: hits = [(0, .35), (1.5, .3), (2 + 2*T3, .25)]
    elif dur < BEAT_S + .2: hits = [(2*T3, .25)]
    else: hits = [(0, 4.)]
    for b, d in hits:
        x = _pno(hi, d*BEAT_S, .6*v); a = int(b*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
MIX = dict(drums=-21, bass=-23, pad=-28, verb=-23, verb_len=1.5)

# ---------------------------------------------------------------- background: night carnival - Ferris wheel, striped tents, fireworks on every note
THEMES = {   # sky top, sky bottom, wheel, bulbs, tent A, tent B, ground
 "intro":    ((10, 6, 40), (90, 30, 110), (230, 220, 255), (255, 210, 90), (220, 40, 70), (250, 240, 230), (40, 20, 50)),
 "chal":     ((20, 14, 10), (130, 90, 50), (240, 220, 180), (255, 200, 120), (150, 60, 30), (240, 220, 180), (50, 34, 22)),
 "denner":   ((10, 30, 50), (200, 150, 80), (250, 240, 210), (255, 220, 140), (40, 90, 150), (250, 236, 200), (40, 40, 40)),
 "classic":  ((30, 20, 50), (180, 130, 170), (250, 240, 250), (255, 230, 180), (130, 60, 120), (250, 240, 240), (50, 34, 56)),
 "muller":   ((8, 24, 30), (60, 130, 130), (220, 255, 240), (255, 240, 160), (30, 120, 110), (236, 240, 230), (20, 44, 44)),
 "bass":     ((6, 6, 16), (40, 40, 80), (200, 210, 255), (160, 190, 255), (40, 50, 110), (210, 214, 236), (16, 18, 36)),
 "boehm":    ((24, 8, 20), (140, 50, 70), (255, 220, 230), (255, 190, 150), (160, 30, 50), (250, 232, 230), (50, 16, 30)),
 "klezmer":  ((14, 20, 50), (110, 120, 170), (250, 250, 255), (255, 230, 120), (40, 70, 160), (246, 246, 250), (30, 34, 60)),
 "nola":     ((20, 8, 30), (120, 60, 140), (255, 230, 160), (255, 200, 80), (110, 40, 150), (60, 160, 80), (40, 20, 50)),
 "gershwin": ((4, 16, 60), (40, 90, 190), (210, 230, 255), (140, 200, 255), (30, 60, 170), (230, 240, 255), (10, 24, 60)),
 "goodman":  ((30, 10, 4), (170, 80, 30), (255, 220, 160), (255, 180, 60), (200, 70, 30), (250, 230, 200), (60, 24, 10)),
 "bilk":     ((10, 30, 30), (90, 160, 150), (240, 255, 250), (255, 240, 180), (40, 130, 120), (240, 250, 246), (20, 50, 48)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[6], T_[4], .45))
GY = 300
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=GY) for k, v in THEMES.items()}
_r = np.random.default_rng(9)
STARS = np.zeros((RH, RW), bool); STAR_PH = _r.random((RH, RW))*6.28
for _ in range(110): STARS[_r.integers(0, 200), _r.integers(0, RW)] = True
WC, WR = (CX, 192), 102
def _tent(ax, ay, x0, x1):
    m = (YY >= ay) & (YY < GY) & (XX >= x0 + (ay - YY)*0 + (YY - ay)*0)
    w = (YY - ay)/(GY - ay)
    inside = (YY >= ay) & (YY < GY) & (XX >= ax - (ax - x0)*w) & (XX <= ax + (x1 - ax)*w)
    stripe = (np.floor((XX - ax)/np.maximum(YY - ay + 4, 1)*5) % 2 == 0)
    return inside, inside & stripe
TA, TAS = _tent(30, 222, -40, 96)
TB_, TBS = _tent(242, 232, 178, 310)
FLAGS = [(30, 214), (242, 224)]
GROUND = YY >= GY
GLINES = GROUND & ((((YY - GY) % 9) == 0) | (((XX - CX)*12 % np.maximum((YY - GY + 6)*14, 1)) < 12))
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)

def background(c, x):
    th0, th1, kb, T_ = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return tuple(int(v) for v in mix(A_[i], B_[i], kb))
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    tw = np.sin(STAR_PH + T_*2.7) > -.2; bg[STARS & tw] = bg[STARS & tw]*.3 + 230*.7
    im = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8)); d = ImageDraw.Draw(im)
    eq, ages = x["eq"], x["ages"]
    pal = [col(3), col(4), col(5), col(2)]
    for a in ages:                                                                     # fireworks, one per note
        h = ((int(round((T_ - a)*100))*2654435761) % 997)/997.
        fx_, fy = 30 + h*210, 112 + ((h*7.31) % 1)*60; cl = pal[int(h*40) % 4]; k = clamp(1 - a/1.4)
        if a < .12:
            d.line([(fx_, fy + 60*(1 - a/.12)), (fx_, fy + 50*(1 - a/.12))], fill=mix((60, 60, 70), cl, .7)); continue
        rad = 26*(1 - math.exp(-(a - .12)*4))
        for j in range(14):
            an = j*2*math.pi/14 + h*3; px, py = fx_ + rad*math.cos(an), fy + rad*math.sin(an)*.9 + 14*(a - .12)**2
            d.rectangle([px - 1, py - 1, px, py], fill=mix(bg[int(clamp(py/RH)*(RH - 1)), int(clamp(px/RW)*(RW - 1))].astype(int), cl, k))
    wc = col(2); hub = WC; rot = T_*.22
    for sx in (-1, 1): d.line([hub, (hub[0] + sx*62, GY)], fill=dim(wc, .55), width=3)
    d.ellipse([hub[0] - WR, hub[1] - WR, hub[0] + WR, hub[1] + WR], outline=dim(wc, .75), width=2)
    d.ellipse([hub[0] - WR*.82, hub[1] - WR*.82, hub[0] + WR*.82, hub[1] + WR*.82], outline=dim(wc, .6), width=1)
    for k in range(12):
        an = rot + k*math.pi/6; ex, ey = hub[0] + WR*math.cos(an), hub[1] + WR*math.sin(an)
        d.line([hub, (ex, ey)], fill=dim(wc, .5), width=1)
        gc = pal[k % 3]
        d.rectangle([ex - 5, ey + 2, ex + 5, ey + 9], fill=dim(gc, .8), outline=dim(gc, .5)); d.line([(ex, ey), (ex, ey + 2)], fill=dim(wc, .6))
    for k in range(36):
        an = rot + k*math.pi/18; bx, by = hub[0] + WR*math.cos(an), hub[1] + WR*math.sin(an)
        lv = eq[k % 14]; bc = mix(dim(col(3), .3), col(3), .3 + .7*lv)
        d.rectangle([bx - 1, by - 1, bx + 1, by + 1], fill=bc)
    d.ellipse([hub[0] - 6, hub[1] - 6, hub[0] + 6, hub[1] + 6], fill=dim(wc, .8))
    bg = np.asarray(im).astype(float)
    ta, tb_ = np.array(col(4), float), np.array(col(5), float)
    for m_, ms in ((TA, TAS), (TB_, TBS)):
        bg[m_] = tb_*.75; bg[ms] = ta*.85
    bg[GROUND] = np.array(col(6), float); bg[GLINES] = np.array(col(6), float)*.7
    for k, (fx_, fy) in enumerate(FLAGS):
        wv = int(2*math.sin(T_*5 + k)); bg[fy:fy + 8, fx_] = (60, 50, 50)
        bg[fy:fy + 4, fx_ + 1:fx_ + 7 + wv] = np.array(col(3), float)
    for k in range(18):
        bx = 6 + k*15; by = int(GY - 3); lv = x["pulse"] if k % 2 else eq[k % 14]
        bg[by - 1:by + 1, bx:bx + 2] = np.array(mix(dim(col(3), .4), col(3), lv), float)
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

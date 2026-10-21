"""Pixel Pitch: history of the drum kit, 60-second cut.  python3 drums60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import piano_lowpoly as P
import cello_lowpoly as C
import electric_lowpoly as E
from pp_framework import mix, dim, lerp, clamp, midi_hz, SR, RW, RH, CX, YY, XX, BAY

OUT, TAG = "drums60_lowpoly.mp4", "dr"
BPM = 96
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.12, -8, 18, .6
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE DRUM KIT", "400 YEARS", "TODAY", 1500
INTRO_LINES = (("ONE PLAYER, FOUR", "LIMBS, A WHOLE BAND"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("400 YEARS FROM", "THE OTTOMAN DAVUL")
OUTRO_CREDIT = "MODERN POP-FUNK GROOVE"
T3 = 2/3

# ---------------------------------------------------------------- music (drum ids follow General MIDI)
CH = {"Em7": (40, (55, 59, 62)), "A7": (45, (55, 57, 61)), "Em": (40, (55, 59, 64)), "G": (43, (55, 59, 62)),
      "D": (38, (57, 62, 66)), "G7": (43, (53, 59, 62)), "C7": (36, (55, 58, 64)), "C": (36, (55, 60, 64)),
      "F": (41, (57, 60, 65)), "Dm7": (38, (57, 60, 65)), "E": (40, (56, 59, 64)), "A": (45, (57, 61, 64)), "Am": (45, (57, 60, 64))}
BARS = ["Em7", "A7", "Em7", "A7", "Em", "Em", "G", "D", "G7", "C7", "C", "G7", "F", "C7", "Dm7", "G7",
        "E", "A", "Am", "F", "Em7", "A7", "Em7", "Em7"]
def hits(b0, items): return [(b0 + o, m, .25, v) for o, m, v in items]
def popfunk(b0, fill=False, crash=False):
    it = [(0, 36, .95), (1.75, 36, .6), (2.5, 36, .85), (1, 38, 1.), (3, 38, 1.), (2.25, 38, .3), (3.75, 38, .35), (3.5, 46, .6)]
    it += [(k*.5, 42, .7 if k % 2 == 0 else .45) for k in range(7)]
    if fill:
        it = [x for x in it if x[0] < 2] + [(2, 48, .85), (2.25, 48, .8), (2.5, 45, .85), (2.75, 45, .8),
                                            (3, 41, .9), (3.25, 41, .85), (3.5, 38, .9), (3.75, 38, .95)]
    if crash: it.append((0, 49, .9))
    return hits(b0, it)
def mehter(b0):
    return hits(b0, [(0, 36, 1.), (0, 52, .85), (.5, 37, .6), (1, 37, .75), (2, 36, .95), (2, 52, .75), (2.5, 37, .6), (3, 37, .75), (3.5, 36, .8)])
def rudiments(b0, end_roll=False):
    it = [(0, 36, .9), (2, 36, .8)]
    def flam(t, v=1.):
        if b0 + t >= .06: it.append((t - .06, 38, .35))
        it.append((t, 38, v))
    flam(0); it += [(.5, 38, .6), (.75, 38, .6)]; flam(1); it += [(1.5, 38, .6), (1.75, 38, .6)]
    if end_roll: it += [(2 + k*.125, 38, .45 + .03*k) for k in range(16)]
    else:
        it += [(2 + k*.125, 38, .5) for k in range(4)] + [(2.5, 38, .9)]; flam(3); it.append((3.5, 38, .7))
    return hits(b0, it)
def second_line(b0):
    return hits(b0, [(0, 36, .95), (0, 49, .55), (1.5, 36, .75), (2, 36, .9), (2, 49, .5), (3, 36, .8), (.5, 38, .6), (1, 38, .9),
                     (1.75, 38, .5), (2.5, 38, .6), (3.25, 38, .5)] + [(3.5 + k*.125, 38, .4) for k in range(4)])
def ragtime(b0, choke=False):
    it = [(k, 36, .85) for k in range(4)] + [(k + .5, 38, .8) for k in range(4)] + [(1.75, 38, .5), (3.75, 38, .5), (2.75, 76, .7), (3.25, 76, .7)]
    return hits(b0, it + ([(3.5, 49, .6)] if choke else []))
def charleston(b0):
    return hits(b0, [(0, 36, .9), (2, 36, .85), (0, 76, .8), (1.5, 76, .8), (2.5, 56, .65), (3, 45, .7), (3.5, 45, .75), (1, 38, .5), (3, 38, .5)])
def swing(b0):
    return hits(b0, [(0, 51, .7), (1, 51, .8), (1 + T3, 51, .5), (2, 51, .7), (3, 51, .8), (3 + T3, 51, .5), (1, 44, .6), (3, 44, .6),
                     (2 + T3, 38, .5)] + [(k, 36, .4) for k in range(4)])
def krupa(b0):
    return hits(b0, [(0, 41, .85), (.5, 41, .6), (1, 41, .85), (1 + T3, 41, .6), (2, 41, .9), (2.5, 41, .6), (3, 41, .9), (3.5, 45, .8),
                     (3.75, 45, .85), (1, 44, .6), (3, 44, .6)] + [(k, 36, .7) for k in range(4)])
def rock(b0, fill=False, crash=False):
    it = [(k*.5, 42, .7 if k % 2 == 0 else .5) for k in range(8) if not (fill and k >= 6)] + [(0, 36, .95), (2, 36, .9), (2.5, 36, .8), (1, 38, 1.)]
    it += [(3, 48, .9), (3.25, 45, .9), (3.5, 41, .95), (3.75, 41, .95)] if fill else [(3, 38, 1.)]
    return hits(b0, it + ([(0, 49, .85)] if crash else []))
def eighties(b0, fill=False, crash=False):
    it = [(0, 36, .95), (1.5, 36, .75), (2, 36, .9), (1, 38, 1.)] + [(k*.5, 42, .6 if k % 2 == 0 else .4) for k in range(8) if not (fill and k >= 5)]
    it += [(2.5, 48, .9), (2.75, 48, .85), (3, 45, .9), (3.25, 45, .85), (3.5, 41, .95), (3.75, 41, .95)] if fill else [(3, 38, 1.)]
    return hits(b0, it + ([(0, 49, .85)] if crash else []))
def trapbar(b0):
    it = [(0, 36, 1.), (.75, 36, .8), (2.5, 36, .9), (1, 38, .9), (1, 39, .9), (3, 38, .9), (3, 39, .9), (1.5, 46, .5)]
    it += [(k*.25, 42, .55 if k % 2 == 0 else .35) for k in range(14) if k != 6] + [(3.5 + k/12, 42, .4) for k in range(6)]
    return hits(b0, it)

SCENES = [
 dict(key="modern", theme="intro", name="MODERN DRUM KIT", credit="MODERN POP-FUNK GROOVE",
      notes=popfunk(0, crash=True) + popfunk(4) + popfunk(8) + popfunk(12, fill=True)),
 dict(key="davul", theme="davul", year=1623, ys="1623", name="DAVUL & ZILS", credit="OTTOMAN MEHTER MARCH",
      fact=("ZILDJIAN STARTED", "MAKING CYMBALS: 1623"), notes=mehter(0) + mehter(4)),
 dict(key="military", theme="military", year=1775, ys="1770s", name="MILITARY SNARE", credit="FIFE & DRUM RUDIMENTS",
      fact=("ROPE DRUMS KEPT", "ARMIES IN STEP"), notes=rudiments(0) + rudiments(4, True)),
 dict(key="double", theme="double", year=1895, ys="1890s", name="DOUBLE DRUMMING", credit="NEW ORLEANS BRASS BAND",
      fact=("ONE DRUMMER PLAYS", "BASS AND SNARE"), notes=second_line(0) + second_line(4)),
 dict(key="pedal", theme="pedal", year=1909, ys="1909", name="BASS DRUM PEDAL", credit="RAGTIME",
      fact=("LUDWIG'S PEDAL", "FREES THE HANDS"), notes=ragtime(0) + ragtime(4, True)),
 dict(key="traps", theme="traps", year=1925, ys="1920s", name="TRAP KIT", credit="1920S JAZZ-AGE CHARLESTON",
      fact=("'TRAPS' WAS SHORT FOR", "CONTRAPTION"), notes=charleston(0) + charleston(4) + hits(0, [(0, 49, .6)])),
 dict(key="swing", theme="swing", year=1937, ys="1930s", name="SWING KIT", credit="IN THE STYLE OF GENE KRUPA",
      fact=("THE HI-HAT AND", "TOM-TOMS ARRIVE"), notes=swing(0) + krupa(4)),
 dict(key="rock", theme="rock", year=1964, ys="1960s", name="ROCK KIT", credit="1960S ROCK BACKBEAT",
      fact=("ROCK'S BACKBEAT", "CONQUERS THE WORLD"), notes=rock(0, crash=True) + rock(4, fill=True)),
 dict(key="simmons", theme="simmons", year=1983, ys="1980s", name="ELECTRONIC KIT", credit="1980S SYNTH-POP",
      fact=("HEXAGON PADS: THE", "80S ELECTRONIC KIT"), notes=eighties(0, crash=True) + eighties(4, fill=True)),
 dict(key="hybrid", theme="intro", year=2015, ys="2010s", name="HYBRID KIT", credit="MODERN HIP-HOP / POP",
      fact=("ACOUSTIC DRUMS MEET", "SAMPLE PADS"),
      notes=trapbar(0) + trapbar(4) + popfunk(8, fill=True, crash=True) + [(12, [36, 49], 4.3, 1.0)]),
]

# ---------------------------------------------------------------- kits
HEADW, HEADC, BR, BR2 = (236, 232, 220), (226, 210, 172), (226, 186, 90), (210, 170, 80)
def modern_kit(shell, logo, hoop=S.METAL):
    return [("bass", dict(c=(0, .32, -.05), r=.3, d=.34, shell=shell, head=HEADW, logo=logo, hoop=hoop)),
            ("tom", dict(c=(-.17, .76, -.02), ax=(0, .86, .5), r=.13, d=.15, shell=shell, head=HEADW, id=48, mount=True, hoop=hoop)),
            ("tom", dict(c=(.17, .76, -.02), ax=(0, .86, .5), r=.145, d=.16, shell=shell, head=HEADW, id=45, mount=True, hoop=hoop)),
            ("tom", dict(c=(.52, .4, .14), ax=(0, 1, .12), r=.19, d=.3, shell=shell, head=HEADW, id=41, legs=True, hoop=hoop)),
            ("snare", dict(c=(-.44, .55, .22), ax=(0, .95, .3), r=.17, d=.12, shell=(200, 200, 212), head=HEADW)),
            ("hat", dict(c=(-.72, .82, .14), r=.17)),
            ("cym", dict(c=(-.42, 1.1, -.2), nrm=(.25, .9, .4), r=.22, ids=(49,), col=BR)),
            ("cym", dict(c=(.62, 1.0, -.12), nrm=(-.2, .95, .3), r=.26, ids=(51,), col=BR2))]
VINT = (120, 70, 36)
KITS = {
 "modern": modern_kit((176, 100, 46), (210, 60, 50)),
 "hybrid": modern_kit((176, 100, 46), (210, 60, 50)) + [("pad", dict(c=(.36, .82, .38)))],
 "davul": [("davul", dict(c=(.05, .62, 0), ax=(.8, 0, .6), r=.36, d=.34, shell=(176, 40, 30), head=HEADC)),
           ("zil", dict(c=(-.66, .98, .26)))],
 "military": [("snare", dict(c=(-.2, .62, .14), ax=(.35, .9, .3), r=.2, d=.34, shell=(40, 60, 130), head=HEADC, ropes=True, hoop=(170, 40, 40))),
              ("davul", dict(c=(.5, .5, -.06), ax=(1, 0, .35), r=.3, d=.26, shell=(170, 30, 30), head=HEADC, ids=(36,)))],
 "double": [("bass", dict(c=(0, .35, -.05), r=.33, d=.3, shell=VINT, head=HEADC, logo=None)),
            ("cym", dict(c=(.12, .8, -.05), nrm=(0, 1, .25), r=.15, ids=(49,), col=BR, mount=(.12, .68, -.05))),
            ("snare", dict(c=(-.46, .55, .2), ax=(0, .95, .3), r=.17, d=.14, shell=(150, 90, 46), head=HEADC))],
 "pedal": [("bass", dict(c=(0, .35, -.05), r=.33, d=.3, shell=(90, 40, 24), head=HEADC, logo=(170, 40, 40), pedal=True)),
           ("cym", dict(c=(-.14, .8, -.05), nrm=(0, 1, .25), r=.15, ids=(49,), col=BR, mount=(-.14, .68, -.05))),
           ("block", dict(c=(.22, .74, .02), ids=(76,), col=(150, 90, 40))),
           ("snare", dict(c=(-.46, .55, .2), ax=(0, .95, .3), r=.17, d=.14, shell=(150, 90, 46), head=HEADC))],
 "traps": [("bass", dict(c=(0, .35, -.05), r=.33, d=.3, shell=(40, 34, 30), head=HEADC, logo=(50, 140, 150), pedal=True)),
           ("tom", dict(c=(-.14, .8, -.02), ax=(0, .85, .5), r=.15, d=.12, shell=(200, 40, 30), head=(232, 210, 160), id=45, mount=True, tacks=True)),
           ("block", dict(c=(.24, .72, .03), ids=(76,), col=(150, 90, 40))),
           ("bell", dict(c=(.34, .86, -.02), ids=(56,))),
           ("cym", dict(c=(.12, .98, -.1), nrm=(0, 1, .3), r=.14, ids=(49,), col=BR, mount=(.12, .68, -.08))),
           ("snare", dict(c=(-.46, .55, .2), ax=(0, .95, .3), r=.17, d=.14, shell=(200, 200, 210), head=HEADC))],
 "swing": [("bass", dict(c=(0, .36, -.05), r=.34, d=.32, shell=(236, 232, 222), head=HEADW, logo=(30, 30, 60))),
           ("tom", dict(c=(-.05, .84, -.02), ax=(0, .85, .5), r=.15, d=.18, shell=(236, 232, 222), head=HEADW, id=45, mount=True)),
           ("tom", dict(c=(.52, .42, .14), ax=(0, 1, .12), r=.2, d=.32, shell=(236, 232, 222), head=HEADW, id=41, legs=True)),
           ("snare", dict(c=(-.44, .55, .22), ax=(0, .95, .3), r=.17, d=.13, shell=(236, 232, 222), head=HEADW)),
           ("hat", dict(c=(-.72, .82, .14), r=.17)),
           ("cym", dict(c=(-.4, 1.1, -.2), nrm=(.25, .9, .4), r=.2, ids=(49,), col=BR)),
           ("cym", dict(c=(.6, 1.02, -.12), nrm=(-.2, .95, .3), r=.25, ids=(51,), col=BR2))],
 "rock": modern_kit((34, 34, 42), (30, 30, 36)),
 "simmons": [("hex", dict(c=(0, .38, -.05), nrm=(0, .15, 1), r=.32, h=.07, ids=(36,), top=(200, 24, 36), side=(236, 236, 240))),
             ("hex", dict(c=(-.44, .62, .22), nrm=(0, .95, .35), r=.17, h=.05, ids=(38,), top=(200, 24, 36), side=(236, 236, 240))),
             ("hex", dict(c=(-.18, .88, -.02), nrm=(0, .8, .6), r=.14, h=.05, ids=(48,), top=(200, 24, 36), side=(236, 236, 240))),
             ("hex", dict(c=(.18, .88, -.02), nrm=(0, .8, .6), r=.15, h=.05, ids=(45,), top=(200, 24, 36), side=(236, 236, 240))),
             ("hex", dict(c=(.52, .62, .14), nrm=(0, .9, .4), r=.18, h=.05, ids=(41,), top=(200, 24, 36), side=(236, 236, 240))),
             ("hat", dict(c=(-.74, .84, .14), r=.17)),
             ("cym", dict(c=(-.42, 1.14, -.2), nrm=(.25, .9, .4), r=.22, ids=(49,), col=BR))],
}
HANDS = {36: None, 44: None, 52: None, 42: "R", 46: "R", 51: "R", 49: "R", 56: "R", 76: "R", 38: "L", 37: "L", 39: "R",
         41: "alt", 45: "alt", 48: "alt"}
HANDS_BY = {"military": {38: "alt", 36: None}, "davul": {36: "R", 37: "L", 52: None}}
ANCH = {"L": np.array([-.4, 1.62, .8]), "R": np.array([.4, 1.62, .8])}

def hit_of(ev, ids):
    h, a = 0., 9.
    for age, m, v, *_ in ev:
        if m in ids and age < .6:
            x = v*math.exp(-age*13)
            if x > h: h, a = x, age
    return h, a

def draw_comp(g, kind, p, ev):
    pts = {}
    if kind == "bass":
        h, _ = hit_of(ev, (36,)); c = np.array(p["c"]); r, d = p["r"]*(1 + .025*h), p["d"]
        p0, p1 = S.drum(g, c, (0, 0, 1), r, d, p["shell"], p["head"], h, hoop=p.get("hoop", S.METAL))
        if p.get("logo"): g.face(S.ring(p1 + [0, 0, .004], (0, 0, 1), r*.42, 12), p["logo"], ref=c, emit=True)
        for sg in (-1, 1): g.line(c + [sg*r*.8, -r*.3, .06], (sg*(r + .1), 0, .14), S.METAL)
        if p.get("pedal"): g.box(-.05, .05, 0, .025, p0[2] - .16, p0[2] - .02, (60, 60, 70)); g.line((0, .02, p0[2] - .04), (0, .2, p0[2] - .01), S.METAL)
        pts[36] = p1 + [0, 0, .01]
    elif kind in ("snare", "tom"):
        ids = (38, 37) if kind == "snare" else (p["id"],)
        h, _ = hit_of(ev, ids); c = np.array(p["c"]); ax = S.unit(p["ax"])
        p0, p1 = S.drum(g, c, ax, p["r"], p["d"], p["shell"], p["head"], h, hoop=p.get("hoop", S.METAL))
        if p.get("ropes"): S.ropes(g, p0, p1, ax, p["r"], (226, 214, 180))
        if p.get("tacks"):
            for q in S.ring(p1 + ax*.004, ax, p["r"]*.9, 10): g.box(q[0] - .006, q[0] + .006, q[1] - .006, q[1] + .006, q[2] - .006, q[2] + .006, (60, 50, 40))
        if p.get("legs"):
            for q in S.ring(p0 + ax*.04, ax, p["r"]*1.06, 3, .5): g.line(q, (q[0] + (q[0] - c[0])*.3, 0, q[2] + (q[2] - c[2])*.3), S.METAL)
        elif p.get("mount"): S.rod(g, (c[0]*.5, .6, -.05), p0 + ax*.02)
        elif kind == "snare": S.tripod(g, c[0], c[2], p0[1] - .02); S.rod(g, (c[0], .08, c[2]), p0)
        for i in ids: pts[i] = p1 + ax*.006
    elif kind == "cym":
        h, a = hit_of(ev, p["ids"]); c = np.array(p["c"])
        if p.get("mount"): S.rod(g, p["mount"], c - [0, .02, 0])
        else: S.rod(g, (c[0], 0, c[2]), c - [0, .02, 0]); S.tripod(g, c[0], c[2], .14)
        hp = S.cymbal(g, c, p["nrm"], p["r"], p["col"], h, a)
        for i in p["ids"]: pts[i] = hp
    elif kind == "hat":
        c = np.array(p["c"]); h, a = hit_of(ev, (42, 46)); hp_, ap = hit_of(ev, (44,))
        opn = [age for age, m, *_ in ev if m == 46 and age < .45]
        gap = .05 if opn else (.006 if hp_ > .2 else .014)
        S.rod(g, (c[0], 0, c[2]), c + [0, gap + .03, 0]); S.tripod(g, c[0], c[2], .14)
        S.cymbal(g, c, (0, 1, .15), p["r"], BR, 0., 9., wob=0)
        hp = S.cymbal(g, c + [0, gap, 0], (0, 1, .15), p["r"], BR, max(h, hp_*.6), a, wob=.12)
        for i in (42, 46, 44): pts[i] = hp
    elif kind == "block":
        h, _ = hit_of(ev, p["ids"]); c = np.array(p["c"]); col = mix(p["col"], (255, 240, 200), h*.7)
        g.box(c[0] - .06, c[0] + .06, c[1] - .025, c[1] + .025, c[2] - .03, c[2] + .03, col, emit=h > .3)
        S.rod(g, (c[0], .62, c[2] - .03), c - [0, .025, 0])
        for i in p["ids"]: pts[i] = c + [0, .03, .01]
    elif kind == "bell":
        h, _ = hit_of(ev, p["ids"]); c = np.array(p["c"]); col = mix((150, 150, 160), (255, 250, 220), h*.7)
        g.box(c[0] - .03, c[0] + .03, c[1] - .05, c[1] + .05, c[2] - .025, c[2] + .025, col, spec=.8, emit=h > .3)
        S.rod(g, (c[0] - .1, .6, c[2]), c - [0, .05, 0])
        for i in p["ids"]: pts[i] = c + [0, .05, .01]
    elif kind == "hex":
        h, _ = hit_of(ev, p["ids"]); c = np.array(p["c"])
        top = S.prism(g, c, p["nrm"], p["r"], p["h"], 6, mix(p["top"], (255, 220, 220), h*.8), p["side"], emit=h > .2, phase=math.pi/6)
        S.rod(g, (c[0], 0, c[2] - .06), c - [0, .02, .06])
        for i in p["ids"]: pts[i] = top + S.unit(p["nrm"])*.006
    elif kind == "davul":
        ids = p.get("ids", (36, 37)); h1, _ = hit_of(ev, (36,)); h0, _ = hit_of(ev, (37,)) if 37 in ids else (0., 9.)
        c = np.array(p["c"]); ax = S.unit(p["ax"])
        p0, p1 = S.drum(g, c, ax, p["r"], p["d"], p["shell"], p["head"], h1, h0)
        S.ropes(g, p0, p1, ax, p["r"], (232, 220, 190), 10)
        for sg in (-1, 1): g.line(c + [sg*.12, -p["r"]*.6, 0], (c[0] + sg*.3, 0, c[2]), S.METAL)
        pts[36] = p1 + ax*.01
        if 37 in ids: pts[37] = p0 - ax*.01
    elif kind == "zil":
        h, a = hit_of(ev, (52,)); c = np.array(p["c"]); gap = .03 + .14*clamp(a*3) if a < 9 else .17
        for sg in (-1, 1):
            q = c + [sg*gap/2, 0, 0]
            S.cymbal(g, q, (-sg, .1, 0), .12, BR, h, a, wob=.1)
            g.line(q + [sg*.03, 0, 0], q + [sg*.09, .06, .02], (120, 70, 40))
        pts[52] = c
    elif kind == "pad":
        h, _ = hit_of(ev, (39,)); c = np.array(p["c"])
        g.box(c[0] - .13, c[0] + .13, c[1] - .03, c[1], c[2] - .09, c[2] + .09, (30, 30, 36), spec=.4)
        for i, (dx, dz) in enumerate(((-.06, -.04), (.06, -.04), (-.06, .04), (.06, .04))):
            col = mix((60, 60, 80), (80, 220, 255) if i % 2 else (255, 80, 200), .25 + .75*h)
            g.box(c[0] + dx - .045, c[0] + dx + .045, c[1], c[1] + .008, c[2] + dz - .03, c[2] + dz + .03, col, emit=True)
        S.rod(g, (c[0], 0, c[2]), c - [0, .03, 0])
        pts[39] = c + [0, .01, 0]
    return pts

def make_tracks(s, pts):
    rule = dict(HANDS); rule.update(HANDS_BY.get(s["key"], {}))
    tr = {"L": [], "R": []}; alt = True
    for e in sorted(s["ev"]):
        nt, m = e[0], e[1]; h = rule.get(m)
        if h is None or m not in pts: continue
        if h == "alt": h = "R" if alt else "L"; alt = not alt
        if tr[h] and abs(tr[h][-1][0] - nt) < .02: continue
        tr[h].append((nt, pts[m]))
    return tr

def build(g, t, ev, fx, s):
    key = s["key"]; pts = {}
    for kind, p in KITS[key]: pts.update(draw_comp(g, kind, p, ev))
    if "_tracks" not in s: s["_tracks"] = make_tracks(s, pts)
    for hand in ("L", "R"):
        pos, lift = S.stick_pos(s["_tracks"][hand], t)
        if pos is None: continue
        tip = pos + np.array([0, lift + .012, lift*.15])
        S.draw_stick(g, tip, ANCH[hand] + np.array([0, .05*math.sin(t*3 + (hand == "R")), 0]), .5, mallet=(key == "davul" and hand == "R"))
    for age, m, v, dur, tp, cs in ev:
        if m in pts and age < .07: fx.append(("spark", tuple(pts[m] + np.array([0, .04, 0])), age))
        if m in (36, 38, 39, 49, 52) and v >= .9 and tp and age < .9: fx.append(("note", tuple(pts[m] + np.array([.15, .3, 0])), age))

# ---------------------------------------------------------------- drum sounds
_rng = np.random.default_rng(77)
def _t(n): return np.arange(n)/SR
def nz(n): return _rng.normal(0, 1, n)
def bp(x, lo, hi): b, a = butter(2, [lo/(SR/2), hi/(SR/2)], "band"); return lfilter(b, a, x)
def hp(x, f): b, a = butter(2, f/(SR/2), "high"); return lfilter(b, a, x)
def lp(x, f): b, a = butter(2, f/(SR/2), "low"); return lfilter(b, a, x)
def kick_s(f0, f1, dec, click=.2, dur=.5, drive=1.5, sub=0.):
    n = int(dur*SR); t = _t(n); f = f0 + (f1 - f0)*np.exp(-t*35)
    x = np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*dec)
    k = int(.004*SR); x[:k] += nz(k)*click
    if sub: x = x + sub*np.sin(2*np.pi*f0*t)*np.exp(-t*dec*.6)
    return np.tanh(drive*x)
def snare_s(tone=190, ndec=16, tdec=30, lo=900, hi=7000, nlev=.6, dur=.4):
    n = int(dur*SR); t = _t(n)
    return bp(nz(n), lo, hi)*np.exp(-t*ndec)*nlev + np.sin(2*np.pi*tone*t)*np.exp(-t*tdec)*.6
def metal_s(freqs, dec, nlev=.2, dur=1.5, hpf=2500):
    n = int(dur*SR); t = _t(n)
    x = sum(np.sign(np.sin(2*np.pi*f*t + k)) for k, f in enumerate(freqs))/len(freqs) + nz(n)*nlev
    return hp(x, hpf)*np.exp(-t*dec)
def tom_s(f0, dec=7, bend=.35, dur=.7):
    n = int(dur*SR); t = _t(n); f = f0*(1 + bend*np.exp(-t*18))
    return np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*dec) + bp(nz(n), 200, 2000)*np.exp(-t*40)*.15
def wood_s(f=950, dec=38):
    t = _t(int(.25*SR)); return np.sin(2*np.pi*f*t)*np.exp(-t*dec) + .5*np.sin(2*np.pi*2.3*f*t)*np.exp(-t*dec*1.4)
def bell_s():
    t = _t(int(.6*SR)); x = np.sign(np.sin(2*np.pi*545*t)) + np.sign(np.sin(2*np.pi*815*t)); return bp(x, 400, 3000)*np.exp(-t*9)*.6
def ride_s():
    t = _t(int(1.6*SR)); return np.sin(2*np.pi*3150*t)*np.exp(-t*14)*.25 + metal_s([620, 1040, 1590, 2430, 3600, 5100], 1.8, .25, 1.6, 2500)*.6
def gated(x, sec):
    n = int(sec*SR); x = x[:n].copy(); f = int(.006*SR); x[-f:] *= np.linspace(1, 0, f); return x
VINTAGE = ("double", "pedal", "traps")
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; k = s["key"]
    if k == "davul":
        x = {36: lambda: kick_s(48, 95, 3., .05, 1.2, 1.3) + np.pad(bp(nz(int(.4*SR)), 100, 400)*np.exp(-_t(int(.4*SR))*10)*.4, (0, int(.8*SR))),
             37: lambda: bp(nz(int(.2*SR)), 1200, 4000)*np.exp(-_t(int(.2*SR))*50)*.7 + np.sin(2*np.pi*380*_t(int(.2*SR)))*np.exp(-_t(int(.2*SR))*40)*.5,
             52: lambda: metal_s([430, 690, 1150, 1720, 2650, 3900, 5300], 2.2, .3, 1.6, 1500)}[m]()
    elif k == "military":
        x = snare_s(220, 9, 28, 1500, 8000, .8, .5) if m == 38 else kick_s(55, 85, 4.5, .02, .9, 1.2)
    elif k in VINTAGE:
        x = {36: lambda: kick_s(56, 95, 5.5, .02, .8, 1.3), 38: lambda: snare_s(185, 14, 26, 700, 4500, .55),
             49: lambda: metal_s([520, 830, 1310, 1980, 2870, 4100], 4.5, .35, 1.0, 1800), 76: wood_s, 56: bell_s,
             45: lambda: tom_s(185, 9, .25)}[m]()
    elif k == "swing":
        x = {36: lambda: kick_s(55, 95, 6, .02, .7, 1.3), 38: lambda: snare_s(190, 15, 26, 800, 5000, .55), 51: ride_s,
             44: lambda: hp(nz(int(.12*SR)), 5000)*np.exp(-_t(int(.12*SR))*55)*.5, 41: lambda: tom_s(95, 5, .3),
             45: lambda: tom_s(140, 6.5, .3), 42: lambda: P.hat(1.), 46: lambda: P.hat(1., True),
             49: lambda: metal_s([480, 760, 1210, 1890, 2770, 4000, 5600], 2.0, .3, 1.8, 1800)}[m]()
    elif k == "simmons":
        if m == 36: x = kick_s(50, 160, 7, .4, .5, 2.0)
        elif m == 38:
            x = snare_s(180, 6, 20, 600, 6000, .9, .6); x = gated(x + lp(nz(len(x)), 5000)*np.exp(-_t(len(x))*3)*.5, .28)
        elif m in (48, 45, 41):
            f0 = {48: 300, 45: 220, 41: 150}[m]; n = int(.6*SR); t = _t(n); f = f0*(1 + 1.2*np.exp(-t*12))
            x = np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*6) + bp(nz(n), 300, 3000)*np.exp(-t*30)*.2
        elif m in (42, 46): x = P.hat(1., m == 46)
        else: x = P.crash(1.)
    else:   # rock / modern / hybrid
        modern = k != "rock"
        x = {36: lambda: kick_s(46, 140, 8, .35, .5, 1.8, .4) if modern else P.kick(1.),
             38: lambda: P.snare(1.)*1.1 + (snare_s(240, 22, 35, 2000, 9000, .4, len(P.snare(1.))/SR)*.5 if modern else 0),
             39: lambda: C.clap(1.), 42: lambda: P.hat(1.), 46: lambda: P.hat(1., True), 49: lambda: P.crash(1.), 51: ride_s,
             48: lambda: C.tom(210 if modern else 200, 1.), 45: lambda: C.tom(165 if modern else 160, 1.), 41: lambda: C.tom(115, 1.)}[m]()
    return np.asarray(x, float)*v

# ---------------------------------------------------------------- backing: funk bass + Rhodes (the drums are the beat)
def _rhodes(f, dur, v):
    n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph)*np.exp(-t*1.2) + .3*np.sin(2*ph)*np.exp(-t*5) + .08*np.sin(7*ph)*np.exp(-t*14)
    env = np.minimum(1, t/.004)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))
    return x*env*(1 + .15*np.sin(2*np.pi*5*t))*v
def pad_voice(notes, dur, v):
    n = int((dur + .4)*SR); out = np.zeros(n)
    stabs = [(0, .3), (1.5, .3), (2.75, .25), (3.5, .4)] if abs(dur - (BAR_S + .05)) < .1 else [(0, dur/BEAT_S)]
    for b, d in stabs:
        for m in notes:
            x = _rhodes(midi_hz(m), d*BEAT_S, .8*v); a = int(b*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
RIFF = [(0, 0, .7, .95), (.75, 0, .25, .7), (1.5, 12, .4, .8), (2, 7, .45, .85), (2.75, 10, .25, .7), (3, 12, .4, .8), (3.5, 7, .45, .8)]
def _root(name):
    r = CH[name][0]
    while r > 40: r -= 12
    while r < 28: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(nb):
        T0, r = b*c.BAR, _root(BARS[b])
        if b == nb - 1: out.append((T0, r, 2.6, 1.)); continue
        out += [(T0 + o*c.BEAT, r + iv, d*c.BEAT, v) for o, iv, d, v in RIFF]
    return out
bass_voice = E.bass_voice
KIT = {"ghost": lambda v: np.zeros(8), "ghostacc": lambda v: np.zeros(8)}
KICK_KINDS, ACCENT_KINDS = ("ghost",), ("ghostacc",)
def drum_events(c):   # silent markers so the background pulses with the featured kit
    out = []
    for s in c.SCENES:
        for e in s["ev"]:
            if e[1] == 36: out.append((s["t0"] + e[0], "ghost", 1.))
            elif e[1] in (38, 39, 49, 52): out.append((s["t0"] + e[0], "ghostacc", 1.))
    return out
MIX = dict(drums=-22, bass=-22, pad=-27, verb=-25, verb_len=1.2)

# ---------------------------------------------------------------- background: night city skyline
THEMES = {   # sky top, sky bottom, buildings, window light, searchlight
 "intro":    ((8, 6, 28), (60, 30, 90), (22, 18, 40), (255, 214, 120), (180, 140, 255)),
 "davul":    ((30, 10, 8), (190, 90, 40), (50, 24, 20), (255, 200, 110), (255, 170, 90)),
 "military": ((10, 16, 40), (70, 90, 140), (24, 28, 48), (255, 236, 170), (220, 230, 255)),
 "double":   ((26, 18, 10), (130, 100, 60), (44, 32, 22), (255, 220, 140), (255, 230, 180)),
 "pedal":    ((10, 24, 20), (70, 120, 100), (20, 40, 34), (240, 255, 200), (200, 255, 220)),
 "traps":    ((6, 6, 6), (60, 50, 30), (20, 18, 14), (255, 210, 90), (255, 220, 120)),
 "swing":    ((6, 10, 40), (40, 70, 150), (16, 20, 48), (255, 236, 170), (170, 200, 255)),
 "rock":     ((40, 8, 30), (240, 110, 60), (50, 16, 36), (255, 240, 120), (255, 120, 200)),
 "simmons":  ((10, 0, 30), (120, 0, 120), (20, 6, 40), (0, 255, 240), (255, 60, 220)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[2], T_[1], .5))
HOR = 300
_rc = np.random.default_rng(21)
BUILD = []; _x = -4
while _x < RW:
    w = int(_rc.integers(16, 32)); h = int(_rc.integers(62, 165)); BUILD.append((_x, min(RW, _x + w), HOR - h)); _x += w + int(_rc.integers(0, 3))
BID = np.full((RH, RW), -1, int)
for i, (x0, x1, top) in enumerate(BUILD): BID[top:HOR, max(0, x0):x1] = i
FACT = .78 + .3*_rc.random(len(BUILD))
WY, WX, WF, WB, WS, WV = [], [], [], [], [], []
for i, (x0, x1, top) in enumerate(BUILD):
    band = min(13, int(abs((x0 + x1)/2 - CX)/CX*13))
    for y in range(top + 5, HOR - 5, 5):
        for x in range(max(1, x0 + 3), min(RW - 2, x1 - 3), 4):
            WY.append(y); WX.append(x); WF.append((HOR - y)/(HOR - top)); WB.append(band); WS.append(_rc.random() < .12); WV.append(.7 + .35*_rc.random())
WY, WX, WF, WB, WS, WV = map(np.array, (WY, WX, WF, WB, WS, WV))
PY = np.concatenate([WY, WY, WY + 1, WY + 1]); PX = np.concatenate([WX, WX + 1, WX, WX + 1]); PI = np.tile(np.arange(len(WY)), 4)
_tall = sorted(range(len(BUILD)), key=lambda i: BUILD[i][2])[:3]
ANT = [((BUILD[i][0] + BUILD[i][1])//2, BUILD[i][2]) for i in _tall]
WT = np.zeros((RH, RW), bool)
_cand = [i for i, b in enumerate(BUILD) if 200 < (b[0] + b[1])/2 < 250 and b[2] > 170]
if _cand:
    x0, x1, top = BUILD[_cand[0]]; cx_ = (x0 + x1)//2
    WT[top - 22:top - 8, cx_ - 6:cx_ + 6] = True
    for k in range(7): WT[top - 26 + k, cx_ - k:cx_ + k + 1] = True
    for dx in (-5, 5): WT[top - 8:top, cx_ + dx] = True
for ax_, ay in ANT: WT[ay - 16:ay, ax_] = True
SKYM = (BID < 0) & (YY < HOR) & ~WT
ANG1 = np.arctan2(XX - 34, HOR - YY); D1 = np.hypot(XX - 34, HOR - YY)
ANG2 = np.arctan2(XX - (RW - 34), HOR - YY); D2 = np.hypot(XX - (RW - 34), HOR - YY)
MOON = SKYM & (np.hypot(XX - 214, YY - 128) < 12)
STARS = [(int(x), int(y), ph) for x, y, ph in zip(_rc.integers(0, RW, 70), _rc.integers(2, 160, 70), _rc.random(70)*6.28) if SKYM[int(y), int(x)]]
def _base(th):
    st, sb, bc, wl, sl = THEMES[th]
    bg = F.make_grad(st, sb, y0=0, y1=HOR)
    m = BID >= 0; bg[m] = np.array(bc, float)*FACT[BID[m]][:, None]
    bg[WT] = np.array(bc, float)*.8
    bg[MOON] = np.array(mix(wl, (255, 255, 255), .6), float)
    bg[YY >= HOR] = np.array(mix(bc, (0, 0, 0), .35), float); bg[(YY >= HOR) & (YY < HOR + 2)] = np.array(mix(bc, (255, 255, 255), .25), float)
    bg[(YY == HOR + 7) | ((YY == HOR + 8) & (XX % 12 > 5))] = np.array(mix(bc, (0, 0, 0), .6), float)
    bg[PY, PX] = np.array(bc, float)*.5
    return bg
BASE = {k: _base(k) for k in THEMES}

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    bg = BASE[th0]*(1 - kb) + BASE[th1]*kb
    A_, B_ = THEMES[th0], THEMES[th1]
    light, beam = np.array(mix(A_[3], B_[3], kb), float), np.array(mix(A_[4], B_[4], kb), float)
    for sx, sy, ph in STARS:
        b = .5 + .5*math.sin(T*2.3 + ph)
        if b > .45: bg[sy, sx] = bg[sy, sx]*(1 - b) + np.array([240, 240, 255])*b
    acc = x["accent"]
    for ang, dd, th in ((ANG1, D1, -.32 + .22*math.sin(T*.55)), (ANG2, D2, .32 + .22*math.sin(T*.5 + 2))):
        m = SKYM & (np.abs(ang - th) < .05) & (BAY < (.16 + .45*acc)*np.clip(1 - dd/430, 0, 1))
        bg[m] = bg[m]*.5 + beam*.5
    lit = (WF < x["eq"][WB]*1.08) | WS
    sel = lit[PI]
    bg[PY[sel], PX[sel]] = light*np.clip(WV[PI[sel]], 0, 1.1)[:, None]
    blink = (255, 60, 50) if x["pulse"] > .45 else (90, 20, 20)
    for ax_, ay in ANT: bg[ay - 17, ax_] = blink
    cols = [light, beam, np.array([255., 90, 90])]
    for k, bx in enumerate(range(6, RW, 12)):
        bg[HOR + 8:HOR + 10, bx:bx + 2] = cols[(k + x["beat_i"]) % 3]*(.45 + .55*x["pulse"])
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

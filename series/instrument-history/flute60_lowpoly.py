"""Pixel Pitch: history of the flute, 60-second cut.  python3 flute60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_shapes as S
import trumpet_lowpoly as T
import cello_lowpoly as C
import acoustic_lowpoly as A
import piano_lowpoly as P
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY

OUT, TAG = "flute60_lowpoly.mp4", "fl"
BPM = 112
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.1, -10, 22, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE FLUTE", "40,000 YEARS", "TODAY", -40000
INTRO_LINES = (("HUMANITY'S OLDEST", "KNOWN INSTRUMENT"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("40,000 YEARS FROM", "A BONE FLUTE")
OUTRO_CREDIT = "TRAP FLUTE LOOP"

# ---------------------------------------------------------------- music
CH = {"Cm": (36, (55, 60, 63)), "Ab": (44, (56, 60, 63)), "Fm": (41, (56, 60, 65)), "G": (43, (55, 59, 62)), "Em": (40, (55, 59, 64)),
      "D5": (38, (57, 62, 69)), "Dm": (38, (57, 62, 65)), "C": (36, (55, 60, 64)), "Am": (45, (57, 60, 64)), "D": (38, (57, 62, 66)),
      "A": (45, (57, 61, 64)), "G7": (43, (53, 59, 62)), "Fmaj7": (41, (57, 60, 64)), "Gm7": (43, (53, 58, 62)), "C7": (36, (55, 58, 64))}
BARS = ["Cm", "Ab", "Fm", "G",
        "Em", "Em", "D5", "D5", "Dm", ("C", "C", "Dm", "Dm"), "G", "Em", "C", ("Am", "Am", "G", "C"),
        ("G", "G", "Em", "C"), ("D", "D", "G", "G"), "G", ("G", "G", "G", "D"), "D", ("G", "G", "A", "A"),
        "C", "G7", "Fmaj7", ("Gm7", "Gm7", "C7", "C7"), "Cm", "Ab", "Fm", "Cm"]
TRAP = [[(0, 72, .5), (.5, 75, .5), (1, 79, .75), (1.75, 77, .25), (2, 75, .5), (2.5, 72, .5), (3, 70, .5), (3.5, 72, .5)],
        [(0, 72, .5), (.5, 75, .5), (1, 80, .75), (1.75, 79, .25), (2, 77, .5), (2.5, 75, .5), (3, 72, 1)],
        [(0, 77, .5), (.5, 80, .5), (1, 84, .75), (1.75, 82, .25), (2, 80, .5), (2.5, 77, .5), (3, 75, .5), (3.5, 77, .5)],
        [(0, 74, .5), (.5, 77, .5), (1, 79, 1), (2, 83, .5), (2.5, 79, .5), (3, 74, 1)]]
def trap(b0, bars, v=.85): return [(b0 + 4*i + o, m, d, v) for i, bar in enumerate(bars) for o, m, d in bar]

SCENES = [
 dict(key="modern", theme="intro", name="MODERN FLUTE", credit="MODERN TRAP FLUTE LOOP", notes=trap(0, TRAP)),
 dict(key="bone", theme="bone", year=-38000, ys="38,000 BC", name="BONE FLUTE", credit="PREHISTORIC PENTATONIC CALL",
      fact=("CARVED FROM A", "VULTURE'S WING BONE"),
      notes=seq(0, [(64, 1), (67, .5), (69, .5), (71, 1.5), (69, .5), (67, 1), (64, .5), (67, .5), (64, 2)], .85)),
 dict(key="ney", theme="ney", year=-2500, ys="2500 BC", name="NEY", credit="MIDDLE EASTERN MAQAM", size=.88,
      fact=("STILL PLAYED AFTER", "4,500 YEARS"),
      notes=seq(0, [(62, 1.5), (63, .25), (62, .25), (66, 1), (67, .5), (69, .5), (70, .5), (69, .5), (67, .5), (66, .5), (63, .5), (62, 1.5)], .85)),
 dict(key="pan", theme="pan", year=-400, ys="400 BC", name="PAN FLUTE", credit="ANCIENT GREEK MODE", size=.82,
      fact=("NAMED FOR PAN, GREEK", "GOD OF THE WILD"),
      notes=seq(0, [(69, 1), (67, .5), (65, .5), (64, 1), (62, 1), (64, .5), (65, .5), (67, 1), (65, .5), (64, .5), (62, 1)], .85)),
 dict(key="dizi", theme="dizi", year=1100, ys="1100s", name="DIZI", credit="CHINESE FOLK MELODY",
      fact=("A PAPER MEMBRANE", "GIVES IT A BUZZ"),
      notes=seq(0, [(74, .5), (76, .5), (79, 1), (81, .5), (79, .5), (76, 1), (74, .5), (71, .5), (74, 1), (76, .5), (74, .5), (71, 1)], .85)),
 dict(key="recorder", theme="recorder", year=1550, ys="1500s", name="RECORDER", credit="IN THE STYLE OF SUSATO",
      fact=("RENAISSANCE HOMES", "RANG WITH RECORDERS"),
      notes=seq(0, [(72, .5), (74, .5), (76, 1), (77, .5), (76, .5), (74, 1), (72, .5), (71, .5), (69, 1), (71, .5), (72, .5), (74, .5), (72, .5)], .85)),
 dict(key="traverso", theme="traverso", year=1735, ys="1730s", name="BAROQUE TRAVERSO", credit="IN THE STYLE OF QUANTZ",
      fact=("FREDERICK THE GREAT", "PLAYED THE TRAVERSO"),
      notes=seq(0, [(74, .5), (79, .5), (78, .25), (76, .25), (78, .5), (74, .5), (71, .5), (72, .25), (74, .25), (76, .5),
                    (72, .5), (69, .5), (71, .25), (72, .25), (74, 1), (71, .5), (67, 1)], .85)),
 dict(key="fife", theme="fife", year=1776, ys="1770s", name="FIFE", credit="YANKEE DOODLE (TRADITIONAL)",
      fact=("FIFE AND DRUM LED", "ARMIES INTO BATTLE"),
      notes=seq(0, [(79, .5), (79, .5), (81, .5), (83, .5), (79, .5), (83, .5), (81, .5), (74, .5), (79, .5), (79, .5), (81, .5), (83, .5), (79, 1), (78, 1)], .9)),
 dict(key="whistle", theme="whistle", year=1843, ys="1843", name="TIN WHISTLE", credit="IRISH REEL",
      fact=("THE PENNY WHISTLE:", "FOLK MUSIC FOR ALL"),
      notes=seq(0, [(m, .5) for m in [74, 78, 81, 78, 74, 78, 81, 83, 83, 81, 79, 78, 76, 74, 73, 71]], .85) + [(1.92, 83, .08, .45), (5.92, 81, .08, .45)]),
 dict(key="boehm", theme="boehm", year=1847, ys="1847", name="BOEHM FLUTE", credit="ROMANTIC VIRTUOSO SHOWPIECE",
      fact=("BOEHM'S METAL KEYS", "MAKE THE MODERN FLUTE"),
      notes=seq(0, [(m, .25) for m in [72, 76, 79, 84, 79, 76, 72, 76, 79, 84, 88, 84, 79, 76, 79, 84]], .8) +
            seq(4, [(m, .5) for m in [86, 83, 79, 77, 74, 77, 79, 83]], .85)),
 dict(key="jazz", theme="jazz", year=1962, ys="1960s", name="JAZZ FLUTE", credit="1960S BOSSA NOVA",
      fact=("JAZZ AND BOSSA NOVA", "PUT FLUTE IN FRONT"),
      notes=seq(0, [(77, 1.5), (76, .5), (74, .5), (72, 1), (69, .5), (70, 1.5), (69, .5), (67, .5), (65, 1), (67, .5)], .85)),
 dict(key="modern", theme="intro", year=2017, ys="2010s", name="HIP-HOP FLUTE", credit="2010S TRAP FLUTE LOOP",
      fact=("HIP-HOP FLUTE LOOPS", "TOP THE CHARTS"),
      notes=trap(0, TRAP[:2]) + trap(8, TRAP[2:3]) + [(12, 84, 4.3, .95)]),
]

# ---------------------------------------------------------------- instruments
SILVER, SKIN = (212, 216, 228), (230, 186, 150)
FL = {
 "modern":   dict(kind="trans", half=.82, r=.026, body=SILVER, spec=.9, holes=7, keys=True, lip=True, base=60),
 "boehm":    dict(kind="trans", half=.82, r=.026, body=(196, 198, 206), spec=.8, holes=7, keys=True, lip=True, base=60),
 "jazz":     dict(kind="trans", half=.82, r=.026, body=SILVER, spec=.9, holes=7, keys=True, lip=True, base=60, mic=True),
 "traverso": dict(kind="trans", half=.78, r=.027, body=(70, 40, 28), spec=.3, holes=6, base=62,
                  rings=[(0, (240, 232, 210), .04), (.3, (240, 232, 210), .03), (.56, (240, 232, 210), .03), (1, (240, 232, 210), .04)]),
 "fife":     dict(kind="trans", half=.48, r=.02, body=(150, 90, 50), spec=.2, holes=6, base=74,
                  rings=[(0, (220, 180, 90), .05), (1, (220, 180, 90), .05)]),
 "dizi":     dict(kind="trans", half=.74, r=.024, body=(222, 206, 140), spec=.2, holes=6, base=69, nodes=True, membrane=True, tassel=True),
 "bone":     dict(kind="diag", top=(-.3, 1.08), bot=(.3, .52), r=.032, body=(232, 220, 190), spec=.1, holes=5, base=64, mouth="bone"),
 "ney":      dict(kind="diag", top=(-.42, 1.36), bot=(.44, .3), r=.022, body=(214, 190, 130), spec=.1, holes=6, base=62, mouth="ney", nodes=True),
 "recorder": dict(kind="diag", top=(-.3, 1.26), bot=(.3, .38), r=.034, body=(224, 180, 110), spec=.2, holes=7, base=72, mouth="beak",
                  rings=[.28, .62, .97]),
 "whistle":  dict(kind="diag", top=(-.24, 1.16), bot=(.26, .46), r=.02, body=(214, 200, 160), spec=.8, holes=6, base=74, mouth="fipple"),
 "pan":      dict(kind="pan"),
}
STEPS = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6]
def covered(m, base, nh):
    if m is None: return nh//2
    return max(0, nh - STEPS[(m - base) % 12])
def box_at(g, p, s, col, emit=False):
    g.box(p[0] - s, p[0] + s, p[1] - s, p[1] + s, p[2] - s, p[2] + s, col, emit=emit)

def transverse(g, ev, fx, c, cur):
    half, r, Y = c["half"], c["r"], .8; x0, x1 = -half, half
    T.trumpet_stand(g, Y - r, 0.)
    T.tube(g, (x0, Y, 0), (x1, Y, 0), r, col=c["body"], n=10, spec=c["spec"])
    g.face(S.ring((x0, Y, 0), (1, 0, 0), r, 10), dim(c["body"], .8), ref=(0, Y, 0))
    g.face(S.ring((x1 + .001, Y, 0), (1, 0, 0), r*.8, 10), (24, 18, 14), ref=(0, Y, 0))
    for fr, col, w in c.get("rings", []):
        x = lerp(x0, x1, fr); T.tube(g, (max(x0, x - w/2), Y, 0), (min(x1, x + w/2), Y, 0), r*1.18, col=col, n=10, spec=.5)
    if c.get("nodes"):
        for fr in (.08, .35, .62, .9):
            x = lerp(x0, x1, fr); T.tube(g, (x - .008, Y, 0), (x + .008, Y, 0), r*1.07, col=dim(c["body"], .75), n=10, spec=.2)
    NH = S.unit((0, .75, .66))
    def surf(x, off=0.): return np.array([x, Y, 0.]) + NH*(r + off)
    xe = x0 + .14*(x1 - x0)
    if c.get("lip"): g.box(xe - .035, xe + .035, Y + r*.55, Y + r*.95, r*.4, r*.95, c["body"], spec=.9)
    box_at(g, surf(xe, .004), .011, (16, 12, 10))
    fracs = np.linspace(.48, .86, c["holes"]); k = covered(cur, c["base"], c["holes"])
    if c.get("membrane"): g.box(lerp(xe, lerp(x0, x1, fracs[0]), .5) - .018, lerp(xe, lerp(x0, x1, fracs[0]), .5) + .018,
                                Y + r*.6, Y + r*.95, r*.5, r*.95, (246, 242, 230), emit=True)
    if c.get("keys"):
        g.line(surf(lerp(x0, x1, fracs[0]) - .04, .012), surf(lerp(x0, x1, fracs[-1]) + .04, .012), (230, 232, 240))
    for i, fr in enumerate(fracs):
        x = lerp(x0, x1, fr); p = surf(x); cov = i < k
        if c.get("keys"): S.prism(g, p + NH*(.002 if cov else .006), NH, .02, .008, 8, (236, 238, 246), (180, 182, 192))
        elif not cov: box_at(g, p, .009, (16, 12, 10))
        if cov: box_at(g, p + NH*.024, .017, SKIN)
    if c.get("tassel"):
        for k2 in range(3): g.line((x1 - .02 + .01*k2, Y - r, 0), (x1 + .02 + .015*k2, Y - .28, .02), (200, 30, 30))
        g.box(x1 - .03, x1 + .01, Y - r - .03, Y - r, -.015, .015, (220, 180, 60))
    if c.get("mic"):
        S.rod(g, (-.55, 0, .3), (-.55, .78, .3), .01); g.ngon(-.55, .004, .3, .12, (40, 38, 46), n=8)
        g.box(-.6, -.5, .78, .96, .26, .34, (190, 194, 204), spec=.8); g.box(-.6, -.5, .85, .88, .25, .35, (40, 40, 48))
    for age, m, v, dur, tp, cs in ev:
        if age < .06: fx.append(("spark", tuple(surf(xe, .03)), age))
        if tp and age < .9: fx.append(("note", (x1 - .05, Y + .28, 0), age))
        if tp and age < .6: fx.append(("wave", (x1 + .03, Y, 0), age, 0))

def diagonal(g, ev, fx, c, cur):
    P0 = np.array([*c["top"], 0.]); P1 = np.array([*c["bot"], 0.]); d = S.unit(P1 - P0); Lb = np.linalg.norm(P1 - P0); r = c["r"]
    mid = (P0 + P1)/2; front = np.array([0, 0, 1.])
    S.rod(g, (mid[0], 0, -.05), mid + [0, -r, -.04], .012); g.ngon(mid[0], .004, -.05, .14, (40, 38, 46), n=8)
    T.tube(g, P0, P1, r, col=c["body"], n=10, spec=c["spec"])
    g.face(S.ring(P1 + d*.001, d, r*.8, 10), (24, 18, 14), ref=mid)
    mt = c["mouth"]
    if mt == "beak":
        T.tube(g, P0 - d*.08, P0, r*.7, r, col=c["body"], n=10, spec=.2); box_at(g, P0 + d*.07 + front*r, .012, (20, 14, 10))
    elif mt == "fipple":
        T.tube(g, P0, P0 + d*.14, r*1.2, col=(30, 30, 34), n=10, spec=.4); box_at(g, P0 + d*.16 + front*r, .009, (16, 12, 10))
    elif mt == "ney":
        T.tube(g, P0 - d*.03, P0 + d*.02, r*1.3, col=(236, 226, 200), n=10, spec=.4)
    elif mt == "bone":
        for p in (P0, P1): box_at(g, p, r*1.5, dim(c["body"], .88))
        box_at(g, P0 + front*r*1.2, .012, (40, 30, 20))
    if c.get("nodes"):
        for fr in np.linspace(.12, .92, 6): T.tube(g, P0 + d*Lb*(fr - .01), P0 + d*Lb*(fr + .01), r*1.08, col=dim(c["body"], .78), n=10, spec=.2)
    for fr in c.get("rings", []): T.tube(g, P0 + d*Lb*(fr - .015), P0 + d*Lb*(fr + .015), r*1.15, col=dim(c["body"], .7), n=10, spec=.3)
    fracs = np.linspace(.38, .85, c["holes"]); k = covered(cur, c["base"], c["holes"])
    for i, fr in enumerate(fracs):
        p = P0 + d*Lb*fr + front*r
        if i < k: box_at(g, p + front*.022, .017, SKIN)
        else: box_at(g, p, .009, (16, 12, 10))
    for age, m, v, dur, tp, cs in ev:
        if age < .06: fx.append(("spark", tuple(P0 + [0, .04, .02]), age))
        if tp and age < .9: fx.append(("note", tuple(P1 + [.15, .25, 0]), age))
        if tp and age < .6: fx.append(("wave", tuple(P1 + [.03, 0, 0]), age, 0))

def panflute(g, ev, fx, s, cur, live):
    notes = sorted({e[1] for e in s["ev"]}); n = max(8, len(notes)); Y = 1.12; r = .036; body = (214, 190, 120)
    xs = np.linspace(-.44, .44, n); lens = np.linspace(.66, .24, n)
    act = notes.index(cur) if (cur in notes and live) else -1
    S.rod(g, (0, 0, -.06), (0, Y - .3, -.05), .012); g.ngon(0, .004, -.06, .14, (40, 38, 46), n=8)
    for i, (x, L) in enumerate(zip(xs, lens)):
        col = mix(body, (255, 250, 220), .6) if i == act else body
        T.tube(g, (x, Y - L, 0), (x, Y, 0), r, col=col, n=8, spec=.2)
        T.tube(g, (x, Y - .03, 0), (x, Y, 0), r*1.08, col=dim(body, .8), n=8, spec=.2)
        g.face(S.ring((x, Y + .001, 0), (0, 1, 0), r*.75, 8), (30, 20, 10), ref=(x, Y - .1, 0))
    for yy in (Y - .14, Y - .23): g.box(-.48, .48, yy - .015, yy + .015, -.045, .045, (120, 70, 30))
    for age, m, v, dur, tp, cs in ev:
        if m in notes:
            x = xs[notes.index(m)]
            if age < .06: fx.append(("spark", (x, Y + .05, .02), age))
            if tp and age < .9: fx.append(("note", (x + .1, Y + .25, 0), age))

def build(g, t, ev, fx, s):
    c = FL[s["key"]]; live = [e for e in ev if e[0] < e[3] + .03]
    cur = min(live)[1] if live else (min(ev)[1] if ev else None)
    if c["kind"] == "trans": transverse(g, ev, fx, c, cur)
    elif c["kind"] == "diag": diagonal(g, ev, fx, c, cur)
    else: panflute(g, ev, fx, s, cur, bool(live))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
TONES = {"bone": dict(bright=.15, vib=0, chiff=.5, wobble=.004, attack=.05), "ney": dict(bright=.25, vib=.007, chiff=.3, scoop=.6, attack=.08),
         "pan": dict(bright=.1, vib=.002, chiff=.9, attack=.02), "dizi": dict(bright=.45, vib=.005, chiff=.4, buzz=.6),
         "recorder": dict(bright=.55, vib=.0015, chiff=.6, attack=.02), "traverso": dict(bright=.22, vib=.003, chiff=.35),
         "fife": dict(bright=.5, vib=0, chiff=.4, attack=.015), "whistle": dict(bright=.45, vib=.002, chiff=.45, attack=.015),
         "boehm": dict(bright=.35, vib=.005, chiff=.3), "jazz": dict(bright=.42, vib=.006, chiff=.35, scoop=.2),
         "modern": dict(bright=.35, vib=.0055, chiff=.3)}
def flute_tone(f, dur, v, bright=.3, vib=.004, chiff=.4, buzz=0., scoop=0., attack=.04, wobble=0., seed=0):
    n = int((dur + .18)*SR); t = _t(n)
    fr = f*(1 + vib*np.sin(2*np.pi*5.2*t)*np.clip((t - .12)/.2, 0, 1))
    if scoop: fr = fr*2 ** (-scoop*np.exp(-t/.06)/12)
    if wobble: fr = fr*(1 + wobble*np.sin(2*np.pi*.7*t + seed))
    ph = 2*np.pi*np.cumsum(fr)/SR
    x = np.sin(ph) + bright*(.45*np.sin(2*ph) + .22*np.sin(3*ph) + .1*np.sin(4*ph))
    if buzz: x = x + buzz*(.25*np.sin(5*ph) + .18*np.sin(7*ph) + .12*np.sin(9*ph))*(.6 + .4*np.sin(2*np.pi*31*t))
    env = np.minimum(1, t/attack)*np.where(t < dur, 1, np.exp(-(t - dur)/.05))
    ch = D.lp(D.nz(n), 2500)*np.exp(-t/.018)*chiff*.5
    return (x*env + ch)*v*.5
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    x = flute_tone(midi_hz(m), dur, v, seed=idx, **TONES[key])
    if key == "modern":
        dl = int(.75*BEAT_S*SR); y = np.zeros(len(x) + 2*dl); y[:len(x)] += x; y[dl:dl + len(x)] += .3*x; y[2*dl:2*dl + len(x)] += .12*x; x = y
    return x

# ---------------------------------------------------------------- backing: Afrobeats groove, log-drum bass, kalimba
def conga(f0):
    def fn(v):
        n = int(.3*SR); t = _t(n); fr = f0*(1 + .1*np.exp(-t*30))
        return (np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*14) + D.bp(D.nz(n), 800, 3000)*np.exp(-t*60)*.3)*v
    return fn
KIT = {"kick": P.kick, "clap": lambda v: C.clap(v)*.8, "rim": C.rim, "shaker": A.shaker, "conga_lo": conga(190), "conga_hi": conga(270), "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("clap",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "clap", .9)]; continue
        ev += [(T0 + k*.25*Q, "shaker", .45 if k % 2 else .25) for k in range(16)]
        if b == 0: ev += [(T0, "kick", .7), (T0 + 2*Q, "kick", .7)]; continue
        ev += [(T0, "kick", 1.), (T0 + 1.5*Q, "kick", .7), (T0 + 2*Q, "kick", .9), (T0 + 3.5*Q, "kick", .7), (T0 + Q, "clap", .85), (T0 + 3*Q, "clap", .85),
               (T0 + .75*Q, "rim", .6), (T0 + 2.25*Q, "rim", .6), (T0 + 3.25*Q, "rim", .6), (T0 + 2.5*Q, "conga_lo", .7),
               (T0 + 1.25*Q, "conga_hi", .6), (T0 + 3.75*Q, "conga_hi", .6)]
        if b in (4, nb - 4): ev.append((T0, "crash", .6))
    return ev
def _root(name):
    r = CH[name][0]
    while r > 44: r -= 12
    while r < 33: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0, ch = b*c.BAR, BARS[b]; sq = list(ch) if isinstance(ch, tuple) else [ch]*4
        if b == nb - 1: out.append((T0, _root(sq[0]), 2.4, 1.)); continue
        out += [(T0 + o*c.BEAT, _root(sq[int(o)]) + iv, .5*c.BEAT, v) for o, iv, v in ((0, 0, .95), (.75, 0, .75), (1.5, 7, .8), (2.5, 12, .85), (3.25, 10, .75))]
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .3)*SR); t = _t(n); fr = f*(1 + .5*np.exp(-t*30)); ph = 2*np.pi*np.cumsum(fr)/SR
    return np.tanh(1.6*(np.sin(ph) + .25*np.sin(2*ph))*np.exp(-t*4.5)*np.minimum(1, t/.002))*v
def _kal(f, v):
    t = _t(int(.9*SR)); return (np.sin(2*np.pi*f*t)*np.exp(-t*3.5) + .25*np.sin(2*np.pi*4.95*f*t)*np.exp(-t*14))*np.minimum(1, t/.002)*v
def pad_voice(notes, dur, v):
    n = int((dur + .9)*SR); out = np.zeros(n); hi = [m + 12 for m in notes]
    if abs(dur - (BAR_S + .05)) < .1: seq_ = [(k*.5, hi[i]) for k, i in enumerate((0, 1, 2, 1, 2, 0, 1, 2))]
    elif dur < BEAT_S + .2: seq_ = [(0, hi[0]), (.5, hi[2])]
    else: seq_ = [(k*.05, m) for k, m in enumerate(hi)]
    for b, m in seq_:
        x = _kal(midi_hz(m), .7*v); a = int(b*BEAT_S*SR); e = min(n, a + len(x)); out[a:e] += x[:e - a]
    return out
MIX = dict(drums=-22, bass=-24, pad=-30, verb=-22, verb_len=1.8)

# ---------------------------------------------------------------- background: windy sky above a sea of clouds
THEMES = {   # sky top, sky bottom, sun, cloud light, cloud shade, wind ribbons
 "intro":    ((60, 140, 230), (200, 230, 255), (255, 250, 220), (255, 255, 255), (170, 200, 235), (255, 255, 255)),
 "bone":     ((20, 20, 40), (140, 110, 140), (255, 230, 200), (220, 210, 220), (120, 110, 140), (255, 240, 220)),
 "ney":      ((210, 140, 60), (255, 220, 150), (255, 250, 220), (255, 240, 210), (220, 170, 110), (255, 250, 230)),
 "pan":      ((30, 90, 200), (140, 200, 255), (255, 255, 230), (255, 255, 255), (160, 190, 230), (255, 255, 255)),
 "dizi":     ((90, 130, 110), (200, 220, 200), (255, 240, 210), (240, 250, 240), (150, 180, 160), (255, 255, 240)),
 "recorder": ((120, 90, 40), (240, 210, 140), (255, 245, 200), (255, 248, 230), (210, 180, 130), (255, 250, 230)),
 "traverso": ((150, 90, 130), (250, 200, 210), (255, 240, 230), (255, 245, 250), (210, 160, 190), (255, 250, 250)),
 "fife":     ((60, 30, 40), (240, 120, 80), (255, 220, 160), (255, 220, 200), (170, 90, 90), (255, 230, 210)),
 "whistle":  ((70, 100, 90), (180, 210, 180), (250, 250, 230), (240, 248, 240), (140, 170, 150), (240, 255, 240)),
 "boehm":    ((50, 30, 90), (200, 150, 210), (255, 230, 240), (240, 230, 250), (150, 120, 180), (255, 240, 255)),
 "jazz":     ((10, 14, 40), (40, 60, 120), (240, 240, 220), (120, 140, 180), (60, 70, 110), (200, 210, 255)),
}
def ped_cols(th): T_ = THEMES[th]; return F.ped_cols_from(mix(T_[4], T_[0], .35))
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=300) for k, v in THEMES.items()}
SUN_D = np.hypot(XX - CX, YY - 168)
CLOUDS = [(20, 112, 1.0, 1.0), (150, 128, .8, .7), (240, 104, 1.2, .9), (80, 150, .7, 1.3), (200, 156, .9, 1.1), (320, 138, 1.0, .8)]
PUFFS = [(0, 0, 10), (10, -4, 8), (-10, -2, 8), (18, 2, 6), (-17, 3, 6)]
XS = np.arange(RW)
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    A_, B_ = THEMES[th0], THEMES[th1]
    sun, cl, cs, rib = (np.array(mix(A_[i], B_[i], kb), float) for i in (2, 3, 4, 5))
    loud, eq = x["loud"], x["eq"]
    r = 26 + 6*loud
    bg[SUN_D < r] = sun
    gm = (SUN_D >= r) & (SUN_D < r + 14) & (BAY < .4*(1 - (SUN_D - r)/14)); bg[gm] = bg[gm]*.55 + sun*.45
    for x0, cy, sc, spd in CLOUDS:
        cx = (x0 + T*4*spd + x["cum"]*12*spd) % (RW + 80) - 40
        ya, yb, xa, xb = int(cy - 16*sc), int(cy + 12*sc), int(cx - 30*sc), int(cx + 30*sc)
        ya, yb, xa, xb = max(0, ya), min(RH, yb), max(0, xa), min(RW, xb)
        if xa >= xb or ya >= yb: continue
        sy, sx = YY[ya:yb, xa:xb], XX[ya:yb, xa:xb]
        inside = np.zeros(sy.shape, bool)
        for dx, dy, rr in PUFFS: inside |= np.hypot(sx - (cx + dx*sc), sy - (cy + dy*sc)) < rr*sc
        sl = bg[ya:yb, xa:xb]; low = inside & (sy > cy + 2*sc)
        sl[inside] = cl; sl[low] = cs
    for i, (base, band) in enumerate(((118, 2), (146, 5), (214, 8), (250, 11))):
        y = (base + (3 + 12*eq[band])*np.sin(XS*.045 + T*1.8 + i*1.7) + 2*np.sin(XS*.11 - T*1.1)).astype(int)
        m = (((XS + int(T*60) + i*13) % 26) < 17) & (y >= 0) & (y < RH)
        bg[y[m], XS[m]] = bg[y[m], XS[m]]*.25 + rib*.75
    for a in x["ages"]:
        h = ((int(round((T - a)*100))*2654435761) % 997)/997.
        bx, by, fl = int(-6 + a*120 + h*40), int(108 + h*60 - a*18), int(a*10) % 2
        for dx, dy in ((-2, -1 if fl else 1), (-1, 0), (0, 1), (1, 0), (2, -1 if fl else 1)):
            if 0 <= bx + dx < RW and 0 <= by + dy < RH: bg[by + dy, bx + dx] = (40, 40, 60)
    top = 292 + 4*np.sin(XS*.07 + T*.6) + 3*np.sin(XS*.19 + T*1.1)
    sea = YY >= top[None, :]
    band = ((YY - top[None, :])//9) % 2
    bg[sea] = cl
    sh = sea & (band == 1); bg[sh] = cs
    bg[TEXTB] *= .5; bg[TEXTE] *= .72
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

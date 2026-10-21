"""Pixel Pitch: history of the trumpet.  python3 trumpet_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import violin_lowpoly as V
import piano_lowpoly as P
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT

OUT, TAG = "trumpet_lowpoly.mp4", "tr"
BPM = 104
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.12, -10, 18, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE TRUMPET", "3,300 YEARS", "TODAY", -2000
INTRO_LINES = (("THREE VALVES, BRIGHT", "BRASS, HUGE SOUND"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("3,300 YEARS SINCE", "KING TUT'S TOMB")
OUTRO_CREDIT = "R. STRAUSS: ZARATHUSTRA (1896)"

CH = {"C": (36, (55, 60, 64)), "Bb": (46, (53, 58, 62)), "F": (41, (53, 57, 60)), "D": (38, (57, 62, 66)),
      "A": (45, (57, 61, 64)), "Eb": (39, (55, 58, 63)), "Bb7": (46, (56, 58, 62)), "C7": (36, (55, 58, 64)),
      "Eb7": (39, (55, 61, 63)), "G7": (43, (53, 59, 62))}
BARS = ["C", "C", "Bb", "F", "C", "C", "D", "A", "Eb", "Bb7", "F", "C7", "Bb7", "Eb7", "G7", "C"]
T3 = 2/3
SCENES = [
 dict(key="modern", theme="intro", name="MODERN TRUMPET", credit="R. STRAUSS: ZARATHUSTRA (1896)",
      notes=[(0, 60, 2, .9), (2, 67, 2, .9), (4, 72, 2.5, 1.0), (6.5, 76, .5, .95), (7, 75, 1, 1.0)]),
 dict(key="tut", theme="tut", year=-1323, ys="1323 BC", name="TUTANKHAMUN'S TRUMPET", credit="ANCIENT EGYPTIAN FANFARE",
      fact=("KING TUT WAS BURIED", "WITH TWO TRUMPETS"),
      notes=[(0, 58, 1.5, .9), (1.5, 58, .5, .8), (2, 65, 2, .9), (4, 58, 1, .85), (5, 65, 1, .9), (6, 58, 2, .9)]),
 dict(key="busine", theme="busine", year=1250, ys="1200s", name="BUSINE", credit="MEDIEVAL HERALD FANFARE",
      fact=("LONG STRAIGHT HORNS", "FOR KNIGHTS & KINGS"), size=.8,
      notes=seq(0, [(67, .5), (67, .25), (67, .25), (72, 1), (67, .5), (72, .5), (76, 1), (79, 1.5), (76, .5), (72, .5), (76, .5), (72, 1)], .9)),
 dict(key="natural", theme="natural", year=1690, ys="1690s", name="NATURAL TRUMPET", credit="IN THE STYLE OF PURCELL",
      fact=("NO VALVES: JUST LIPS", "AND A FOLDED TUBE"),
      notes=seq(0, [(69, .5), (74, .5), (78, .5), (81, .5), (78, .75), (76, .25), (74, 1), (76, .5), (78, .5), (76, .5), (74, .5), (69, 1), (74, 1)], .85)),
 dict(key="keyed", theme="keyed", year=1796, ys="1796", name="KEYED TRUMPET", credit="IN THE STYLE OF HAYDN",
      fact=("HAYDN'S CONCERTO WAS", "WRITTEN FOR KEYS"),
      notes=seq(0, [(70, .5), (75, .5), (74, .25), (75, .25), (77, .5), (79, 1), (78, .5), (79, .5),
                    (80, .5), (79, .5), (77, .5), (75, .5), (74, .5), (70, .5), (75, 1)], .8)),
 dict(key="valve", theme="valve", year=1818, ys="1818", name="VALVE TRUMPET", credit="IN THE STYLE OF ARBAN",
      fact=("VALVES FINALLY UNLOCK", "EVERY NOTE"),
      notes=seq(0, [(72, .5), (77, .25), (76, .25), (77, .5), (81, .5), (84, 1), (81, .5), (77, .5),
                    (79, .25), (81, .25), (79, .25), (77, .25), (76, .5), (74, .5), (72, 1), (77, 1)], .85)),
 dict(key="modern", theme="intro", year=1925, ys="1920s", name="MODERN TRUMPET", credit="IN THE STYLE OF LOUIS ARMSTRONG",
      fact=("JAZZ MAKES THE TRUMPET", "A SOLO SUPERSTAR"),
      notes=seq(0, [(70, T3), (73, 1 - T3), (74, T3), (77, 1 - T3), (74, 1), (70, 1),
                    (75, T3), (78, 1 - T3), (79, T3), (82, 1 - T3), (79, 1), (75, 1)], .85) +
            seq(8, [(67, 1), (72, 1), (76, 1), (79, 1)], .95) + [(12, [60, 67, 72, 76, 79, 84], 4.3, 1.0)]),
]

# ---------------------------------------------------------------- brass geometry
BRASS, SILVER, GOLDB = (228, 178, 72), (212, 214, 224), (240, 200, 96)
def _basis(d):
    d = d/np.linalg.norm(d); a = np.array([0, 1., 0]) if abs(d[1]) < .9 else np.array([1., 0, 0])
    u = np.cross(d, a); u /= np.linalg.norm(u); v = np.cross(d, u); return d, u, v
def tube(g, p0, p1, r0, r1=None, col=BRASS, n=8, spec=.9, two=False):
    p0, p1 = np.array(p0, float), np.array(p1, float); r1 = r0 if r1 is None else r1
    d, u, v = _basis(p1 - p0); mid = (p0 + p1)/2
    ring = lambda p, r: [p + r*(math.cos(2*math.pi*k/n)*u + math.sin(2*math.pi*k/n)*v) for k in range(n)]
    A, B = ring(p0, r0), ring(p1, r1)
    for k in range(n):
        g.face([A[k], A[(k+1) % n], B[(k+1) % n], B[k]], col, ref=mid, spec=spec, two=two)
def bend(g, c, R, r, a0, a1, col=BRASS, segs=6):
    pts = [np.array([c[0] + R*math.cos(math.radians(a)), c[1] + R*math.sin(math.radians(a)), c[2]])
           for a in np.linspace(a0, a1, segs + 1)]
    for p, q in zip(pts, pts[1:]): tube(g, p, q, r, col=col)
def bell(g, start, length, r0, r1, col=BRASS, segs=6):
    x0, y, z = start
    prev = (x0, r0)
    for k in range(1, segs + 1):
        s = k/segs; x = x0 + length*s; r = r0 + (r1 - r0)*s**3
        tube(g, (prev[0], y, z), (x, y, z), prev[1], r, col=col, two=True); prev = (x, r)
    tube(g, (x0 + length - .012, y, z), (x0 + length + .004, y, z), r1*1.07, r1*1.07, col=col, two=True)
    ring = [(x0 + length*.55, y + r1*.35*math.cos(a), z + r1*.35*math.sin(a)) for a in np.linspace(0, 2*math.pi, 11)[:-1]]
    g.face(ring, (30, 20, 10), ref=(x0, y, z), emit=True, two=True)
    return (x0 + length, y, z)
def mouthpiece(g, x, y, col=SILVER):
    tube(g, (x - .07, y, 0), (x, y, 0), .03, .012, col=col); tube(g, (x - .085, y, 0), (x - .07, y, 0), .034, .034, col=col)
def trumpet_stand(g, y_top, x=0.):
    metal = (40, 38, 46)
    g.box(x - .2, x + .2, 0, .03, -.14, .14, metal, spec=.5)
    g.box(x - .014, x + .014, .03, y_top, -.014, .014, metal, spec=.5)
    for sg in (-1, 1): g.box(x + sg*.06 - .01, x + sg*.06 + .01, y_top - .04, y_top + .01, -.03, .03, metal)
    g.box(x - .07, x + .07, y_top - .04, y_top - .02, -.03, .03, metal)

FING = {54: "123", 55: "13", 56: "23", 57: "12", 58: "1", 59: "2", 60: "", 61: "123", 62: "13", 63: "23", 64: "12", 65: "1",
        66: "2", 67: "", 68: "23", 69: "12", 70: "1", 71: "2", 72: "", 73: "12", 74: "1", 75: "2", 76: "", 77: "1", 78: "2",
        79: "", 80: "23", 81: "12", 82: "1", 83: "2", 84: "", 85: "12", 86: "1", 87: "2", 88: ""}
def fingering(m):
    w = m + 2
    while w > 88: w -= 12
    while w < 54: w += 12
    return FING[w]

def sounding(ev):
    live = [(e[0], e[1]) for e in ev if e[0] < e[3] + .05]
    return min(live) if live else None

def brass_fx(fx, ev, mouth):
    for age, m, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("wave", mouth, age, 0)); fx.append(("note", (mouth[0] - .1, mouth[1] + .28, mouth[2]), age))

def build_valved(g, t, ev, fx, nval, col, cap):
    trumpet_stand(g, .36)
    mouthpiece(g, -.82, .74)
    tube(g, (-.82, .74, 0), (.5, .74, 0), .018, col=col)
    bend(g, (.5, .69, 0), .05, .018, 90, -90, col=col)
    tube(g, (.5, .64, 0), (.1, .64, 0), .018, col=col)
    tube(g, (-.1, .44, 0), (-.55, .44, 0), .02, col=col)
    bend(g, (-.55, .49, 0), .05, .02, 270, 90, col=col)
    tube(g, (-.55, .54, 0), (.25, .54, 0), .022, col=col)
    mouth = bell(g, (.25, .54, 0), .62, .022, .17, col=col)
    cur = sounding(ev); fing = fingering(cur[1]) if cur else ""
    xs = [-.08, 0, .08] if nval == 3 else [-.045, .045]
    for i, x in enumerate(xs):
        tube(g, (x, .4, 0), (x, .8, 0), .034, col=col)
        tube(g, (x, .78, 0), (x, .81, 0), .04, col=cap)
        tube(g, (x, .4, 0), (x, .37, 0), .038, col=cap)
        down = str(i + 1) in fing
        top = .86 - (.035 if down else 0)
        tube(g, (x, .81, 0), (x, top, 0), .008, col=SILVER)
        tube(g, (x, top, 0), (x, top + .018, 0), .028, col=HOT if down else (236, 230, 214), spec=.4)
        tube(g, (x, .4, 0), (x, .33, 0), .014, col=col)
    tube(g, (xs[0], .33, 0), (xs[-1], .33, 0), .014, col=col)
    brass_fx(fx, ev, mouth)

def build_natural(g, t, ev, fx, keyed=False):
    col = BRASS
    trumpet_stand(g, .4, -.05)
    mouthpiece(g, -.9, .72)
    tube(g, (-.9, .72, 0), (.45, .72, 0), .016, col=col)
    bend(g, (.45, .65, 0), .07, .016, 90, -90, col=col)
    tube(g, (.45, .58, 0), (-.6, .58, 0), .016, col=col)
    bend(g, (-.6, .51, 0), .07, .017, 90, 270, col=col)
    tube(g, (-.6, .44, 0), (.2, .44, 0), .018, col=col)
    tube(g, (-.16, .44, 0), (-.06, .44, 0), .042, .042, col=GOLDB)
    tube(g, (.16, .44, 0), (.22, .44, 0), .03, .03, col=GOLDB)
    mouth = bell(g, (.2, .44, 0), .68, .018, .16, col=col)
    cord = [(-.4 + .7*k/8, .72 - .1*math.sin(math.pi*k/8), .02) for k in range(9)]
    for a, b in zip(cord, cord[1:]): g.line(a, b, (200, 40, 50))
    g.box(.28, .32, .52, .62, .0, .04, (200, 40, 50))
    if keyed:
        cur = sounding(ev); opened = set()
        if cur and cur[1] not in (58, 65, 70, 74, 77, 79, 82):
            opened = {cur[1] % 4, (cur[1] // 2) % 4}
        for k, x in enumerate((-.42, -.28, -.14, 0.0)):
            up = k in opened
            g.line((x - .06, .62, .02), (x, .6 + (.035 if up else .015), .02), SILVER)
            g.box(x - .02, x + .02, .596 + (.03 if up else 0), .61 + (.03 if up else 0), -.02, .02, HOT if up else SILVER, emit=up, spec=.6)
    brass_fx(fx, ev, mouth)

def build_busine(g, t, ev, fx):
    col = BRASS
    trumpet_stand(g, .58, -.1)
    mouthpiece(g, -1.05, .62)
    tube(g, (-1.05, .62, 0), (.75, .62, 0), .02, col=col)
    for x in (-.6, -.1, .35): tube(g, (x - .035, .62, 0), (x + .035, .62, 0), .042, .042, col=GOLDB)
    mouth = bell(g, (.75, .62, 0), .4, .02, .2, col=col)
    sway = .02*math.sin(t*2.5)
    red, gold = (186, 30, 40), (240, 200, 96)
    q = [(-.45, .6, .025), (.25, .6, .025), (.25 + sway, .3, .025), (-.45 + sway, .3, .025)]
    g.face(q, red, two=True)
    for (a, b) in (((-.43, .58), (.23, .58)), ((-.43 + sway, .32), (.23 + sway, .32))):
        g.face([(a[0], a[1], .027), (b[0], b[1], .027), (b[0], b[1] - .015, .027), (a[0], a[1] - .015, .027)], gold, two=True, emit=True)
    cx, cy = -.1 + sway/2, .45
    g.face([(cx, cy + .08, .028), (cx + .06, cy, .028), (cx, cy - .08, .028), (cx - .06, cy, .028)], gold, two=True, emit=True)
    for k in range(8):
        x = -.43 + k*.095 + sway
        g.face([(x, .3, .026), (x + .03, .3, .026), (x + .03, .27, .026), (x, .27, .026)], gold, two=True, emit=True)
    brass_fx(fx, ev, mouth)

def build_tut(g, t, ev, fx):
    trumpet_stand(g, .44, -.1)
    tube(g, (-.78, .6, 0), (-.72, .6, 0), .03, .03, col=GOLDB)
    tube(g, (-.72, .6, 0), (.35, .6, 0), .018, .032, col=SILVER)
    for x in (-.72, -.2, .33): tube(g, (x - .015, .6, 0), (x + .015, .6, 0), .036, .036, col=GOLDB)
    mouth = bell(g, (.35, .6, 0), .44, .032, .18, col=SILVER)
    tube(g, (.77, .6, 0), (.8, .6, 0), .19, .19, col=GOLDB, two=True)
    brass_fx(fx, ev, mouth)

def build(g, t, ev, fx, s):
    k = s["key"]
    if k == "modern": build_valved(g, t, ev, fx, 3, BRASS, SILVER)
    elif k == "valve": build_valved(g, t, ev, fx, 2, (220, 168, 76), (200, 150, 70))
    elif k == "natural": build_natural(g, t, ev, fx)
    elif k == "keyed": build_natural(g, t, ev, fx, keyed=True)
    elif k == "busine": build_busine(g, t, ev, fx)
    else: build_tut(g, t, ev, fx)

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
TONE = {"tut": dict(bright=.8, vib=0, scoop=.06, rough=.25), "busine": dict(bright=1.2, vib=0, scoop=.04, rough=0),
        "natural": dict(bright=1.3, vib=.002, scoop=.03, rough=0), "keyed": dict(bright=.5, vib=.003, scoop=.02, rough=0),
        "valve": dict(bright=.8, vib=.004, scoop=.025, rough=0), "modern": dict(bright=1.1, vib=.006, scoop=.03, rough=0)}
def brass(f, dur, vel, bright=1., vib=.004, scoop=.03, rough=0., tilt=.75):
    n = int((dur + .12)*SR); t = _t(n)
    env = np.minimum(1, t/.035)**1.5*np.where(t < dur, 1, np.exp(-(t - dur)/.03))*(.88 + .12*np.exp(-t*3))
    fr = f*(1 - scoop*np.exp(-t/.025))*(1 + vib*np.sin(2*np.pi*5.2*t)*np.clip((t - .18)/.2, 0, 1))
    ph = 2*np.pi*np.cumsum(fr)/SR
    dark = np.zeros(n); brt = np.zeros(n); sd = sb = 0
    for k in range(1, 30):
        if f*k > 7000: break
        a1 = math.exp(-k/2.5)/k**tilt; a2 = math.exp(-k/(5 + 6*bright))/k**tilt
        s_ = np.sin(k*ph); dark += a1*s_; brt += a2*s_; sd += a1; sb += a2
    mb = np.clip(env*vel, 0, 1)**1.3
    x = (dark/sd*(1 - mb) + brt/sb*mb)*env
    if rough: x = x + rough*np.sin(ph*.5)*env*.3
    return x*vel
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    x = brass(midi_hz(m), dur, v, **TONE[s["key"]])
    return x*(.7 if cs > 1 else 1.0)

_r = np.random.default_rng(41)
_SN = butter(2, [700/(SR/2), 6000/(SR/2)], "band")
def timp(f0):
    def fn(v):
        n = int(1.4*SR); t = _t(n); fr = f0*(1 + .03*np.exp(-t*20)); ph = 2*np.pi*np.cumsum(fr)/SR
        return (np.sin(ph) + .5*np.sin(1.5*ph) + .3*np.sin(1.98*ph))*np.exp(-t*2.4)*np.minimum(1, t/.004)*.7*v
    return fn
def bd(v):
    n = int(.5*SR); t = _t(n); fr = 50 + 50*np.exp(-t*25)
    return np.tanh(1.3*np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*6))*v
def sn(v):
    n = int(.2*SR); t = _t(n)
    return (lfilter(*_SN, _r.normal(0, 1, n))*np.exp(-t*22)*.7 + np.sin(2*np.pi*200*t)*np.exp(-t*35)*.4)*v
def roll(v):
    n = int(.55*SR); out = np.zeros(n)
    for k in range(16):
        a = int(k*.033*SR); h = sn(v*(.25 + .5*k/15))[:n - a]; out[a:a + len(h)] += h*.6
    return out
def cym(v):
    n = int(1.0*SR); t = _t(n); nz = np.diff(_r.normal(0, 1, n + 1))
    x = sum(np.sin(2*np.pi*fq*t + k) for k, fq in enumerate((3200, 4100, 5300, 6700)))*.05
    return (nz*.12 + x)*np.exp(-t*3.5)*v
KIT = {"bd": bd, "sn": sn, "sng": lambda v: sn(v)*.3, "roll": roll, "cym": cym, "timpC": timp(65.4), "timpG": timp(98.0),
       "crash": P.crash}
KICK_KINDS, ACCENT_KINDS = ("bd", "timpC"), ("sn",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT
    ev.append((0, "timpC", 1.))
    ev += [(2*Q + k*.25*Q, "timpG" if k % 2 == 0 else "timpC", .55 + .04*k) for k in range(8)]
    ev += [(B, "timpC", .9), (B + 2*Q, "timpG", .8), (B + 3*Q, "roll", .7), (B, "bd", .6), (B + 2*Q, "bd", .6)]
    for b in range(2, 16):
        T0 = b*B
        if b == 15:
            ev += [(T0, "bd", 1.), (T0, "crash", 1.), (T0, "timpC", 1.), (T0, "cym", 1.)]; continue
        ev += [(T0, "bd", 1.), (T0 + 1.5*Q, "bd", .8), (T0 + 2.5*Q, "bd", .9), (T0, "cym", .6), (T0 + 2.5*Q, "cym", .45),
               (T0 + Q, "sn", .9), (T0 + 3*Q, "sn", .9)]
        ev += [(T0 + x*Q, "sng", 1.) for x in (.25, .75, 1.75, 2.25, 2.75, 3.25)]
        if b % 2 == 1: ev.append((T0 + 3*Q, "roll", .8))
        if b % 2 == 0: ev.append((T0, "crash", .5))
        if b == 14: ev += [(T0 + 3*Q, "timpG", .8), (T0 + 3.5*Q, "timpC", .9)]
    return ev
def bass_voice(m, dur, v):
    return brass(midi_hz(m), dur, v, bright=.3, vib=0, scoop=.02, tilt=1.1)
def bass_events(c):
    out = []
    for b in range(2, 16):
        r = CH[BARS[b]][0]; T0 = b*c.BAR
        if b == 15: out.append((T0, r, 2.4, 1.)); continue
        nxt = CH[BARS[b + 1]][0]
        out += [(T0, r, .8*c.BEAT, .9), (T0 + c.BEAT, r + 7, .7*c.BEAT, .75), (T0 + 2*c.BEAT, r, .8*c.BEAT, .85),
                (T0 + 3*c.BEAT, r + 7, .4*c.BEAT, .7), (T0 + 3.5*c.BEAT, nxt - 1, .4*c.BEAT, .7)]
    return out
pad_voice = None
MIX = dict(drums=-22, bass=-23, verb=-22, verb_len=1.6)

# ---------------------------------------------------------------- background: art-deco sunburst stage
THEMES = {   # top, bottom, ray light, ray dark, bulbs
 "intro":   ((8, 10, 30), (34, 44, 96), (70, 84, 160), (28, 34, 74), (255, 214, 120)),
 "tut":     ((14, 18, 48), (60, 90, 168), (206, 172, 90), (40, 60, 120), (255, 224, 140)),
 "busine":  ((30, 6, 10), (124, 26, 32), (190, 64, 52), (80, 16, 22), (255, 210, 120)),
 "natural": ((6, 26, 28), (30, 100, 100), (64, 156, 144), (18, 62, 62), (255, 224, 150)),
 "keyed":   ((20, 10, 34), (84, 52, 124), (150, 118, 196), (52, 32, 84), (240, 220, 255)),
 "valve":   ((6, 24, 14), (30, 92, 58), (76, 166, 104), (18, 58, 36), (255, 232, 150)),
}
GR = {k: F.make_grad(v[0], v[1], y0=0, y1=300) for k, v in THEMES.items()}
def ped_cols(th): return F.ped_cols_from(mix(THEMES[th][1], THEMES[th][3], .4))
ANG = np.arctan2(YY - 190, XX - CX)
RAD = np.hypot(XX - CX, YY - 190)
ARCR = np.hypot(XX - CX, (YY - 300)*1.0)
BULBS = [(6, y) for y in range(104, 330, 12)] + [(RW - 7, y) for y in range(104, 330, 12)] + [(x, 98) for x in range(6, RW, 12)]

def background(c, x):
    th0, th1, kb = x["th0"], x["th1"], x["kb"]; T = x["T"]
    A, B = THEMES[th0], THEMES[th1]
    top, bot, rl, rd, bulb = (np.array(mix(A[i], B[i], kb), float) for i in range(5))
    bg = GR[th0]*(1 - kb) + GR[th1]*kb
    rot = T*.12 + .05*x["pulse"]
    ray = (np.floor((ANG + rot)/(2*math.pi/24)) % 2 == 0)
    strength = .35 + .4*x["loud"]
    lit = ray & (BAY < strength*np.clip(1.4 - RAD/260, 0, 1)*np.clip((YY - 100)/30, 0, 1)) & (YY < 300)
    bg[lit] = bg[lit]*.45 + rl*.55
    dk = ~ray & (YY < 300) & (BAY < .25)
    bg[dk] = bg[dk]*.6 + rd*.4
    for i in range(6):
        r0 = 118 + i*15; lev = x["eq"][min(13, 2*i + 1)]
        ring = (np.abs(ARCR - r0) < 1.3) & (YY < 300)
        col = rd*(1 - lev) + np.minimum(255, rl*1.6)*lev
        bg[ring] = col
    floor = YY >= 300
    bg[floor] = bot*.45
    refl = floor & (BAY < .18) & (YY < 340)
    bg[refl] = bg[refl]*.5 + rl*.5
    for k, (bx, by) in enumerate(BULBS):
        on = ((k + x["beat_i"]) % 3 == 0) or x["pulse"] > .8 and k % 2 == 0
        col = bulb if on else bulb*.3
        bg[by - 1:by + 2, bx - 1:bx + 2] = col*.6
        bg[by, bx] = np.minimum(255, col*1.2)
        if on:
            for dx, dy in ((2, 0), (-2, 0), (0, 2), (0, -2)):
                if 0 <= bx + dx < RW and 0 <= by + dy < RH: bg[by + dy, bx + dx] = bg[by + dy, bx + dx]*.6 + bulb*.4
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

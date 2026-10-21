"""Pixel Pitch: history of the acoustic guitar.  python3 acoustic_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_guitar as G
import piano_lowpoly as P
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY

OUT, TAG = "acoustic_lowpoly.mp4", "ac"
BPM = 100
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .72, -12, 20, .34
HEAD_INTRO, HEAD, TODAY, START_YEAR = "ACOUSTIC GUITAR", "1,200 YEARS", "TODAY", 600
INTRO_LINES = (("6 STRINGS, A HOLLOW", "BODY, ENDLESS SONGS"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("1,200 YEARS FROM", "THE OUD")
OUTRO_CREDIT = "ROMANCE (ANON., 1800s)"

# ---------------------------------------------------------------- music
CH = {"Em": (40, (55, 59, 64)), "Dm": (38, (57, 62, 65)), "Cm": (36, (55, 60, 63)), "G": (43, (55, 59, 62)),
      "D": (38, (57, 62, 66)), "A": (45, (57, 61, 64)), "C": (36, (55, 60, 64)), "G7": (43, (53, 59, 62)),
      "Am": (45, (57, 60, 64)), "E": (40, (56, 59, 64)), "B7": (47, (54, 57, 63))}
BARS = ["Em", "Em", "Dm", "Cm", "G", "D", "D", "A", "C", "G7", "Am", "E", "G", "C", "B7", "E"]

def romance(start, mel, bass=40, inner=(59, 55)):
    out = []
    for i, m in enumerate(mel):
        b = start + i*.5
        out.append((b, [bass, m] if i % 3 == 0 else m, .5, .8))
        out += [(b + 1/6, inner[0], .34, .4), (b + 2/6, inner[1], .2, .4)]
    return out

def tremolo(start, mels, basses):
    out = []
    for k, (m, bs) in enumerate(zip(mels, basses)):
        b = start + k
        out += [(b, bs, .25, .45), (b + .25, m, .25, .8), (b + .5, m, .25, .6), (b + .75, m, .25, .65)]
    return out

SCENES = [
 dict(key="steel", theme="intro", name="MODERN ACOUSTIC", credit="ROMANCE (ANON., 1800s)",
      notes=romance(0, [71, 71, 71, 71, 69, 67, 67, 66, 64, 64, 67, 71, 76, 76, 76, 76])),
 dict(key="oud", theme="oud", year=800, ys="800s", name="OUD", credit="IN THE STYLE OF ZIRYAB",
      fact=("ZIRYAB ADDS A FIFTH", "PAIR OF STRINGS"), size=1.05,
      notes=seq(0, [(62, .5), (63, .5), (65, .5), (67, .5), (69, 1), (67, .5), (65, .5),
                    (63, .5), (65, .5), (63, .5), (62, .5), (60, 1), (62, 1)])),
 dict(key="vihuela", theme="vihuela", year=1536, ys="1530s", name="VIHUELA", credit="IN THE STYLE OF LUIS DE MILAN",
      fact=("SPAIN'S VIHUELA", "HAS A GUITAR SHAPE"),
      notes=seq(0, [(67, 1), (71, .5), (72, .5), (74, 1), (72, .5), (71, .5), (69, 1), (66, .5), (67, .5), (69, 1.5), (62, .5)])
            + [(0, 43, 2, .45), (2, 50, 2, .45), (4, 50, 2, .45), (6, 45, 2, .45)]),
 dict(key="baroque", theme="baroque", year=1674, ys="1670s", name="BAROQUE GUITAR", credit="IN THE STYLE OF GASPAR SANZ",
      fact=("FIVE PAIRS OF STRINGS", "AND STRUMMED CHORDS"),
      notes=[(0, [62, 66, 69, 74], .5, .9), (.5, 74, .25, .7), (.75, 76, .25, .7), (1, 78, .5, .8), (1.5, 76, .5, .7),
             (2, [62, 66, 69, 74], .5, .9), (2.5, [62, 66, 69, 74], .5, .6), (3, 74, .5, .8), (3.5, 73, .5, .7),
             (4, [57, 61, 64, 69], .5, .9), (4.5, 69, .25, .7), (4.75, 71, .25, .7), (5, 73, .5, .8), (5.5, 71, .5, .7),
             (6, [57, 61, 64, 69], .5, .9), (6.5, [57, 61, 64, 69], .5, .6), (7, 69, 1, .8)]),
 dict(key="early", theme="early", year=1800, ys="1800s", name="EARLY GUITAR", credit="IN THE STYLE OF FERNANDO SOR",
      fact=("SIX SINGLE STRINGS", "BECOME THE STANDARD"),
      notes=[(0, 48, 4, .45), (4, 43, 4, .45)] +
            seq(0, [(72, 1), (76, 1), (74, 1), (72, 1), (71, 1), (74, 1), (77, 1), (74, 1)], .8) +
            [(b + .5, m, .5, .4) for b, m in zip(range(8), [64, 67, 64, 67, 65, 67, 65, 67])]),
 dict(key="torres", theme="torres", year=1860, ys="1860s", name="TORRES GUITAR", credit="IN THE STYLE OF TARREGA",
      fact=("TORRES DESIGNS THE", "MODERN CLASSICAL"),
      notes=tremolo(0, [76, 77, 76, 72, 71, 74, 71, 68], [45, 52, 57, 52, 40, 47, 52, 47])),
 dict(key="steel", theme="intro", year=1931, ys="1930s", name="STEEL-STRING GUITAR", credit="TRAVIS-STYLE FINGERPICKING",
      fact=("STEEL STRINGS AND A", "BIG DREADNOUGHT BODY"),
      notes=[(0, 43, .5, .45), (0, 71, .5, .8), (.5, 59, .5, .4), (1, 50, .5, .45), (1.5, 74, .5, .75), (2, 43, .5, .45),
             (2.5, 59, .5, .4), (3, 50, .5, .45), (3.5, 71, .5, .75),
             (4, 48, .5, .45), (4, 72, .5, .8), (4.5, 64, .5, .4), (5, 55, .5, .45), (5.5, 76, .5, .75), (6, 48, .5, .45),
             (6.5, 64, .5, .4), (7, 55, .5, .45), (7.5, 72, .5, .75)] +
            romance(8, [75, 75, 75, 75, 73, 71, 71, 69], bass=47, inner=(59, 57)) +
            [(12, [40, 47, 52, 56, 59, 64], 4.2, 1.0)]),
]

# ---------------------------------------------------------------- instruments
GUIT = [(0, .12), (.04, .28), (.12, .35), (.24, .37), (.36, .33), (.46, .26), (.52, .245), (.58, .26), (.68, .29), (.78, .28), (.88, .23), (.96, .14), (1, .07)]
DREAD = [(0, .14), (.04, .32), (.14, .38), (.28, .385), (.4, .35), (.5, .31), (.58, .31), (.7, .32), (.82, .3), (.92, .22), (.98, .12), (1, .08)]
VIH = [(0, .1), (.05, .26), (.18, .3), (.34, .28), (.46, .22), (.54, .21), (.64, .24), (.78, .25), (.9, .2), (1, .08)]
BARQ = [(0, .1), (.05, .25), (.16, .3), (.3, .28), (.44, .2), (.52, .19), (.6, .22), (.74, .25), (.88, .2), (1, .08)]
OUDP = [(0, .08), (.05, .28), (.16, .38), (.3, .4), (.45, .37), (.6, .3), (.75, .2), (.88, .12), (1, .07)]
EARLY = [(0, .1), (.04, .25), (.13, .31), (.25, .32), (.37, .28), (.47, .21), (.53, .2), (.6, .22), (.7, .255), (.8, .25), (.9, .2), (.97, .12), (1, .06)]
GUT, SILK, BRONZE, STEEL = (226, 206, 158), (206, 206, 216), (212, 172, 112), (222, 224, 232)
SIX = [40, 45, 50, 55, 59, 64]
GC = {
 "oud":     dict(L=.95, d=.34, prof=OUDP, top=(214, 172, 112), side=(116, 62, 30), neck=.32, fb0=.72, fbw=(.085, .065), fbcol=(40, 26, 20),
                 frets=0, head="oud", headcol=(96, 54, 26), strings=[36, 41, 45, 50, 55, 60], courses=True, scol=[GUT]*6,
                 sb=.075, sn=.04, hole=dict(type="oud3", u=.62, r=.075), bridge="oud", bridge_u=.16, pluck_u=.3, inlays=False),
 "vihuela": dict(L=.95, d=.14, prof=VIH, top=(222, 190, 130), side=(150, 90, 44), neck=.66, fb0=.7, fbcol=(70, 44, 26), frets=10,
                 head="steel", headcol=(150, 90, 44), strings=[43, 48, 53, 57, 62, 67], courses=True, scol=[GUT]*6, sb=.07, sn=.036,
                 hole=dict(type="rose", u=.62, r=.075), bridge="classical", bridge_u=.2, pluck_u=.36, inlays=False, fret_col=(120, 90, 60)),
 "baroque": dict(L=.85, d=.12, prof=BARQ, top=(228, 198, 142), side=(70, 36, 24), neck=.6, fb0=.7, fbcol=(30, 22, 20), frets=9,
                 head="slot", headcol=(70, 36, 24), strings=[45, 50, 55, 59, 64], courses=True, scol=[GUT]*5, sb=.065, sn=.034,
                 hole=dict(type="rose", u=.62, r=.07, ring=(236, 226, 200), ring2=(30, 22, 20)), bridge="classical", bridge_u=.2,
                 pluck_u=.36, inlays=False, fret_col=(120, 90, 60)),
 "early":   dict(L=.82, d=.12, prof=EARLY, top=(224, 192, 130), side=(176, 120, 60), neck=.62, fb0=.72, fbcol=(36, 26, 22), frets=17,
                 head="steel", headcol=(36, 26, 22), strings=SIX, scol=[SILK]*3 + [GUT]*3, sb=.065, sn=.032,
                 hole=dict(type="round", u=.62, r=.06, ring=(232, 212, 162)), bridge="classical", bridge_u=.2, pluck_u=.36, inlays=False),
 "torres":  dict(L=.92, d=.14, prof=GUIT, top=(228, 198, 140), side=(122, 58, 32), neck=.64, fb0=.7, fbcol=(32, 24, 22), frets=19,
                 head="slot", headcol=(122, 58, 32), strings=SIX, scol=[SILK]*3 + [GUT]*3, sb=.07, sn=.035,
                 hole=dict(type="round", u=.63, r=.07, ring=(196, 156, 90), ring2=(44, 90, 66)), bridge="classical", bridge_u=.2,
                 pluck_u=.36, inlays=False),
 "steel":   dict(L=.98, d=.2, prof=DREAD, top=(226, 190, 122), side=(112, 52, 30), neck=.6, fb0=.72, fbcol=(30, 24, 22), frets=20,
                 head="steel", headcol=(28, 22, 22), strings=SIX, scol=[BRONZE]*4 + [STEEL]*2, sb=.075, sn=.036,
                 hole=dict(type="round", u=.64, r=.08, ring=(232, 222, 202), ring2=(40, 30, 26)), bridge="acoustic", bridge_u=.2,
                 pluck_u=.38, guards=[([(.07, .48), (.24, .47), (.3, .55), (.27, .64), (.15, .66), (.1, .6)], (72, 36, 24))]),
}
def build(g, t, ev, fx, s): G.build_guitar(g, t, ev, fx, s, GC[s["key"]])

# ---------------------------------------------------------------- sound
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]; f = midi_hz(m); ring = 1.8
    if key == "oud":
        x = P.ks_pluck(f, ring, .8, .9955, idx)*.7 + P.ks_pluck(f*1.004, ring, .8, .9955, idx + 50)*.5
    elif key in ("vihuela", "baroque"):
        x = P.ks_pluck(f, ring, .6, .997, idx)*.6 + P.ks_pluck(f*1.003, ring, .6, .997, idx + 50)*.5
    elif key in ("early", "torres"):
        x = lfilter([.5], [1, -.5], P.ks_pluck(f, ring, .35, .9978, idx))*1.4
    else:
        x = P.ks_pluck(f, ring, .9, .9988, idx)*.7 + P.ks_pluck(f*1.0025, ring, .9, .9988, idx + 50)*.3
    if cs >= 3:
        same = sorted(e[1] for e in s["ev"] if abs(e[0] - nt) < 1e-6)
        x = np.concatenate([np.zeros(int(.014*same.index(m)*SR)), x])
    return x*v

_r = np.random.default_rng(31)
def _t(n): return np.arange(n)/SR
_BP = butter(2, [1000/(SR/2), 4200/(SR/2)], "band"); _CL = butter(2, [900/(SR/2), 2500/(SR/2)], "band")
def cbass(v):
    n = int(.3*SR); t = _t(n); f = 52 + 40*np.exp(-t*30)
    return np.tanh(1.4*np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*12))*v
def cslap(v):
    n = int(.14*SR); t = _t(n)
    return (lfilter(*_BP, _r.normal(0, 1, n))*np.exp(-t*30)*.8 + np.sin(2*np.pi*230*t)*np.exp(-t*40)*.4)*v
def shaker(v):
    n = int(.09*SR); t = _t(n); nz = np.diff(np.diff(_r.normal(0, 1, n + 2)))
    return nz*np.minimum(1, t/.015)*np.exp(-t*45)*.05*v
def palmas(v):
    n = int(.25*SR); t = _t(n); nz = lfilter(*_CL, _r.normal(0, 1, n))
    env = sum(np.exp(-(t - k*.008)/.004)*(t >= k*.008) for k in range(3)) + .7*np.exp(-(t - .018)/.07)*(t >= .018)
    return nz*env*.45*v
def tamb(v):
    n = int(.35*SR); t = _t(n)
    x = sum(np.sin(2*np.pi*fq*t + k) for k, fq in enumerate((5100, 6300, 7150, 8200)))*.12
    return (x + np.diff(_r.normal(0, 1, n + 1))*.08)*np.exp(-t*9)*v
KIT = {"cbass": cbass, "cslap": cslap, "cghost": lambda v: cslap(v)*.35, "shaker": shaker, "palmas": palmas, "tamb": tamb}
KICK_KINDS, ACCENT_KINDS = ("cbass",), ("cslap", "palmas")

def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT
    for b in range(16):
        T0 = b*B
        if b == 15: ev += [(T0, "cbass", 1.), (T0, "tamb", 1.), (T0, "palmas", .9)]; continue
        if b == 0:
            ev += [(T0 + k*.5*Q, "shaker", .4 if k % 2 else .25) for k in range(8)]
            ev += [(T0 + x*Q, "cbass", .6) for x in (0, 1.5, 3)]
            continue
        ev += [(T0, "cbass", 1.), (T0 + 1.5*Q, "cbass", .8), (T0 + 3*Q, "cbass", .85), (T0 + Q, "cslap", .9), (T0 + 3*Q, "cslap", .9),
               (T0 + .75*Q, "cghost", .8), (T0 + 2.5*Q, "cghost", 1.), (T0 + 3.75*Q, "cghost", .9)]
        ev += [(T0 + k*.5*Q, "shaker", .45 if k % 2 else .25) for k in range(8)]
        if b >= 2: ev += [(T0 + Q, "palmas", .55), (T0 + 3*Q, "palmas", .55)]
        if b % 2 == 0 and b >= 2: ev.append((T0, "tamb", .7))
    return ev

def bass_voice(m, dur, v):
    f = midi_hz(m); n = int(max(dur, .3)*SR); t = _t(n); ph = 2*np.pi*np.cumsum(f*(1 + .08*np.exp(-t*50)))/SR
    x = np.sin(ph) + .25*np.sin(2*ph) + .1*np.sin(3*ph)
    env = np.minimum(1, t/.004)*np.exp(-t*3.0); r = int(.03*SR); env[-r:] *= np.linspace(1, 0, r)
    return x*env*v
def bass_events(c):
    out = []
    for b in range(1, 16):
        r = CH[BARS[b]][0]; r = r - 12 if r >= 43 else r; T0 = b*c.BAR
        if b == 15: out.append((T0, r, 2.6, 1.)); continue
        out += [(T0, r, 1.4*c.BEAT, .9), (T0 + 1.5*c.BEAT, r + 7, 1.4*c.BEAT, .75), (T0 + 3*c.BEAT, r + 12, .9*c.BEAT, .7)]
    return out
pad_voice = None
MIX = dict(drums=-22, bass=-25, verb=-24, verb_len=1.3)

# ---------------------------------------------------------------- background: dusk hills
THEMES = {   # sky top, sky bottom, sun, far hills, near hills
 "intro":   ((28, 14, 48), (236, 124, 72), (255, 214, 120), (110, 46, 60), (40, 16, 34)),
 "oud":     ((6, 10, 30), (40, 90, 120), (232, 236, 214), (32, 44, 74), (12, 16, 32)),
 "vihuela": ((20, 40, 90), (130, 180, 214), (255, 244, 206), (72, 112, 74), (26, 52, 32)),
 "baroque": ((40, 8, 24), (206, 64, 62), (255, 196, 124), (96, 26, 42), (32, 8, 18)),
 "early":   ((30, 24, 60), (192, 152, 204), (255, 232, 224), (92, 72, 124), (38, 28, 58)),
 "torres":  ((6, 24, 24), (64, 146, 124), (226, 255, 224), (26, 74, 62), (8, 28, 24)),
}
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=262) for k, v in THEMES.items()}
def ped_cols(th): return F.ped_cols_from(mix(THEMES[th][3], THEMES[th][4], .3))
_rr = np.random.default_rng(12)
FLIES = [(_rr.random()*RW, 140 + _rr.random()*150, _rr.random()*6.28, k) for k in range(32)]
STARS = [(int(_rr.integers(0, RW)), int(_rr.integers(4, 150)), _rr.random()*6.28) for _ in range(40)]
XS = np.arange(RW)
BANDX = np.minimum(13, (np.abs(XS - CX)/CX*13).astype(int))

def background(c, x):
    th0, th1, kb = x["th0"], x["th1"], x["kb"]
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    T0, T1 = THEMES[th0], THEMES[th1]
    sun, far, near = (np.array(mix(T0[i], T1[i], kb), float) for i in (2, 3, 4))
    for sx, sy, ph in STARS:
        b = .5 + .5*math.sin(x["T"]*2.2 + ph)
        if b > .4: bg[sy, sx] = bg[sy, sx]*(1 - b*.8) + np.array([240, 236, 255])*b*.8
    d = np.hypot(XX - CX, YY - 206)
    r = 40 + 12*x["loud"] + 4*x["pulse"]
    glow = (d < r + 16) & (BAY < .35*(1 - (d - r)/16))
    bg[glow] = bg[glow]*.55 + sun*.45
    bg[d < r] = sun
    eq = x["eq"][BANDX]
    for k, (base, amp, fq, ph, react, col) in enumerate((
            (236, 14, .031, .6, 10, far), (254, 10, .045, 2.0, 20, mix(tuple(far), tuple(near), .5)), (274, 8, .06, .3, 28, near))):
        ridge = (base - amp*np.sin(XS*fq + ph) - 6*np.sin(XS*fq*2.3 + ph*1.7) - react*eq).astype(int)
        col = np.array(col, float); rim = col*.5 + sun*.5
        m = YY >= ridge[None, :]
        bg[m] = col
        top = (YY == ridge[None, :]); bg[top] = rim
    for fx_, fy, ph, k in FLIES:
        xx = int(fx_ + 6*math.sin(x["T"]*.7 + ph)); yy = int(fy + 4*math.sin(x["T"]*1.1 + ph*2))
        flash = max([1 - a/.4 for a in x["ages"] if a < .4 and (int(a*1000) + k) % 3 == 0] + [0])
        b = .25 + .25*math.sin(x["T"]*3 + ph) + .75*flash
        if 0 <= xx < RW and 0 <= yy < RH:
            col = np.array([255, 240, 150])
            bg[yy, xx] = bg[yy, xx]*(1 - min(1, b)) + col*min(1, b)
            if flash > .5:
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if 0 <= xx + dx < RW and 0 <= yy + dy < RH: bg[yy + dy, xx + dx] = bg[yy + dy, xx + dx]*.5 + col*.5
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

"""Pixel Pitch: history of the turntable, 60-second cut.  python3 turntables60_lowpoly.py [out.mp4] | --still t ...

Modern Technics SL-1200MK7 showcase, then 1877 Edison tinfoil phonograph -> 2001 Final Scratch -> back to the MK7.
Backdrop: on a giant spinning record (the pedestal sits on its label, the grooves glint in rings on every kick, a giant
tonearm hovers overhead, a mirror ball throws light spots on the beat, the label art changes per era).
Beat: UK garage 2-step at 130 BPM (skippy kick, shuffled hats, a late snare, sub bass, chopped organ stabs) with a
scratch fill at every scene change. Each era's riff is coloured as if heard through that era's machine (tinfoil,
horn, 78, LP, sound-system boxes, hi-fi, CD), with real scratches (a sample pushed back and forth under the needle,
fader cuts included) from 1975 on. The records on screen follow the same scratch motion as the audio.
Models were built from reference photos (Smithsonian / NMAH, victor-victrola.com, Philco archive photos, Technics
product pages, Christie's Kool Herc lot, Smithsonian NMAAHC Grand Wizzard Theodore turntables, Pioneer archive,
Stanton Final Scratch photos).
"""
import math, sys
from functools import lru_cache
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

OUT, TAG = "turntables60_lowpoly.mp4", "tt"
BPM = 130
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, 1.0, -14, 10, .3
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE TURNTABLE", "149 YEARS", "TODAY", 1870
INTRO_LINES = (("OVER 3 MILLION", "SOLD SINCE 1972"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("149 YEARS FROM", "EDISON'S TINFOIL")
OUTRO_CREDIT = "UK GARAGE + SCRATCH"
LIGHTING = True

# ---------------------------------------------------------------- music (D minor / F major, UK garage 2-step)
CH = {"Dm": (38, (60, 64, 65, 69)), "Bb": (34, (60, 62, 65, 69)), "Gm": (43, (58, 62, 65, 69)), "A": (45, (61, 64, 67, 69)),
      "F": (41, (60, 64, 65, 69)), "C": (36, (60, 64, 67, 72)), "C7": (36, (58, 60, 64, 67)), "Am": (45, (60, 64, 67, 69))}
BARS = ["Dm", "Bb", "Gm", "A",                                   # showcase
        "F", "C", "Bb", "F", "F", "C7", "Gm", "C7", "F", "Bb",     # 1877 .. 1950s
        "Bb", "Am", "Dm", "Dm", "Dm", "C", "Gm", "A",              # 1972 .. 1976
        "Dm", "C", "Bb", "C", "Dm", "Dm", "Dm", "Bb",              # 1979 .. 2020s
        "A", "Dm"]                                               # outro, final hit
BED, BABY, CHIRP, TRANS, FWD, SCRIB, TEAR, FLARE, BACKSPIN = range(9)   # midi codes < 12: surface noise + scratches
def tn(items, v=.85, b0=0): return [(b0 + b, m, d, v) for b, m, d in items]
def stabs(b0, ch, pos, v=.85, d=.3): return [(b0 + p, CH[ch][1], d, v) for p in pos]

SHOW = (stabs(0, "Dm", (0, .75, 1.5)) + tn([(2, FWD, 1)], .9) + stabs(0, "Dm", (3.5,)) +
        stabs(4, "Bb", (0, .75, 1.5)) + tn([(6, BABY, 1)], .9) + stabs(4, "Bb", (3.5,)) +
        stabs(8, "Gm", (0, .75, 1.5, 2.5, 3.5)) +
        stabs(12, "A", (0,)) + tn([(12.5, TRANS, 1)], .9) + stabs(12, "A", (1.5, 2.25)))
MARY = tn([(0, 69, .5), (.5, 67, .5), (1, 65, .5), (1.5, 67, .5), (2, 69, .5), (2.5, 69, .5), (3, 69, 1),
           (4, 67, .5), (4.5, 67, .5), (5, 67, 1), (6, 69, .5), (6.5, 72, .5), (7, 72, 1)], .85)
MARY_VOW = "aiaaieaieaiea"
SOUSA = (tn([(0, 65, .75), (.75, 65, .25), (1, 70, .5), (1.5, 74, .5), (2, 77, 1), (3, 74, .75), (3.75, 70, .25),
             (4, 72, .5), (4.5, 69, .5), (5, 65, .5), (5.5, 69, .5), (6, 72, .75), (6.75, 75, .25), (7, 77, 1)], .86) +
         tn([(1, (62, 65), .3), (3, (62, 65), .3), (5, (63, 69), .3), (7, (63, 69), .3)], .42))
RAG = (tn([(0, 72, .5), (.5, 81, .25), (.75, 77, .5), (1.25, 81, .25), (1.5, 84, .5), (2, 81, .25), (2.25, 77, .5), (2.75, 72, .25),
           (3, 74, .25), (3.25, 77, .5), (3.75, 77, .25),
           (4, 76, .5), (4.5, 79, .25), (4.75, 82, .5), (5.25, 79, .25), (5.5, 76, .5), (6, 72, .25), (6.25, 76, .5), (6.75, 79, .25),
           (7, 82, .5), (7.5, 79, .5)], .84) +
       tn([(1, (57, 60, 65), .4), (3, (57, 60, 65), .4), (5, (58, 60, 64), .4), (7, (58, 60, 64), .4)], .5))
_GM7, _C9 = (65, 69, 70, 74), (64, 67, 70, 74)
BIGBAND = (tn([(0, _GM7, .45), (.75, _GM7, .25), (1.5, _GM7, .45), (4, _C9, .45), (4.75, _C9, .25), (5.5, _C9, .45)], .82) +
           tn([(2, 74, .33), (2.33, 77, .33), (2.67, 79, .33), (3, 82, .5), (3.5, 81, .5),
               (6, 79, .33), (6.33, 82, .33), (6.67, 84, .33), (7, 86, .9)], .86))
BOOGIE = (tn([(0, 41, .62), (.67, 45, .3), (1, 48, .62), (1.67, 50, .3), (2, 51, .62), (2.67, 50, .3), (3, 48, .62), (3.67, 45, .3),
              (4, 46, .62), (4.67, 50, .3), (5, 53, .62), (5.67, 55, .3), (6, 56, .62), (6.67, 55, .3), (7, 53, .62), (7.67, 50, .3)], .9) +
          tn([(1, (60, 65, 69), .3), (3, (60, 65, 69), .3), (5, (62, 65, 70), .3), (7, (62, 65, 70), .3)], .55))
PHILLY = (tn([(0, 77, 1.5), (1.5, 74, .5), (2, 77, .5), (2.5, 81, 1.5), (4, 79, 1.5), (5.5, 76, .5), (6, 72, .5), (6.5, 76, .5), (7, 79, 1)], .84) +
          tn([(0, (58, 62, 65, 69), 1.2), (2.5, (58, 62, 65, 69), .4), (4, (57, 60, 64, 67), 1.2), (6.5, (57, 60, 64, 67), .4)], .5))
_HS = (65, 69, 72, 74)
FUNK = (tn([(0, 62, .2), (.5, 62, .2), (.75, 65, .2), (1.25, 67, .2), (1.5, 69, .2), (2, 72, .2), (2.5, 74, .2), (2.75, 72, .2),
            (3, 69, .2), (3.5, 67, .2), (3.75, 65, .2), (4, 62, .2), (4.5, 62, .2), (4.75, 65, .2), (5.25, 67, .2), (5.5, 68, .2),
            (5.75, 69, .2)], .84) +
        tn([(6, _HS, .4), (6.75, _HS, .2), (7.25, _HS, .4)], .88))
BBOY = (tn([(0, (62, 65, 69), .25), (.75, (62, 65, 69), .25), (1.5, (62, 65, 69), .25), (3.5, (62, 65, 69), .25),
            (4, (60, 64, 67), .25), (4.75, (60, 64, 67), .25), (5.5, (60, 64, 67), .25)], .9) +
        tn([(2, BABY, 1.5), (6, FWD, 1)], .92))
DISCO = (tn([(0, (62, 67, 70), .5), (1, 67, .25), (1.25, 69, .25), (1.5, 70, .25), (1.75, 72, .25), (2, 74, 1.4),
             (4, (61, 64, 69), .5), (5, 64, .25), (5.25, 67, .25), (5.5, 69, .25), (5.75, 73, .25), (6, 76, 1)], .86) +
         tn([(3.5, FWD, .5)], .8))
ELECTRO = (tn([(0, 74, .5), (.75, 77, .25), (1, 81, .5), (1.5, 79, .25), (1.75, 77, .25), (2, 74, .75), (3, 72, .25),
               (4, 76, .5), (4.75, 79, .25), (5, 84, .5), (5.5, 82, .25), (5.75, 79, .25), (6, 76, .5)], .84) +
           tn([(3.25, CHIRP, .75), (6.5, TRANS, .5)], .9))
_ARP_BB, _ARP_C = (70, 74, 77, 74, 70, 74, 77, 82), (72, 76, 79, 76, 72, 76, 79, 84)
TRANCE = ([(k*.125, 70, .1, .8) for k in range(4)] +                                 # the CDJ cue stutter
          [(.5 + .25*k, _ARP_BB[(k + 2) % 8], .22, .9 if k % 4 == 2 else .66) for k in range(12)] +
          [(4 + .25*k, _ARP_C[k % 8], .22, .9 if k % 4 == 0 else .66) for k in range(12)] +
          tn([(3.5, SCRIB, .5)], .8) + tn([(7, (64, 67, 72, 76), .9)], .6))
MINIMAL = (tn([(0, 62, .12), (.75, 74, .12), (1.5, 69, .12), (2, 65, .12), (2.75, 72, .12), (3.5, 69, .12),
               (4, 62, .12), (4.75, 74, .12), (5.5, 69, .12), (6.75, 65, .12)], .88) +
           tn([(6, TRANS, .75)], .9))
LAST = (stabs(0, "Dm", (0, .75, 1.5)) + tn([(2, CHIRP, 1)], .9) + stabs(0, "Dm", (3.5,)) +
        stabs(4, "Bb", (0, .75, 1.5, 2.5)) +
        stabs(8, "A", (0, .75, 1.5)) + tn([(10, FLARE, 1), (11, BACKSPIN, 1)], .9) +
        [(12, (50, 62, 65, 69, 72, 76), 4.0, .85)])
def bed(dur=8): return [(0, BED, dur, .8)]

SCENES = [
 dict(key="modern", theme="modern", name="TECHNICS SL-1200MK7", credit="UK GARAGE + SCRATCH", size=3.5, notes=SHOW),
 dict(key="edison", theme="edison", year=1877, ys="1877", name="EDISON PHONOGRAPH", credit="MARY HAD A LITTLE LAMB", size=3.3,
      fact=("RECORDS AND PLAYS", "BACK ON TINFOIL"), notes=MARY + bed()),
 dict(key="berliner", theme="berliner", year=1887, ys="1887", name="BERLINER GRAMOPHONE", credit="SOUSA-STYLE MARCH", size=3.0,
      fact=("PATENTED: FLAT", "DISCS, NOT TUBES"), notes=SOUSA + bed()),
 dict(key="victrola", theme="victrola", year=1906, ys="1906", name="VICTROLA", credit="RAGTIME", size=1.15,
      fact=("THE HORN HIDES", "IN THE CABINET"), notes=RAG + bed()),
 dict(key="lp", theme="lp", year=1948, ys="1948", name="LP RECORD PLAYER", credit="LATE-40s BIG BAND", size=3.4,
      fact=("THE LP ARRIVES,", "SPINNING AT 33 1/3"), notes=BIGBAND + bed()),
 dict(key="jamaica", theme="jamaica", year=1955, ys="1950s", name="JAMAICAN SOUND SYSTEM", credit="JAMAICAN R&B SHUFFLE", size=1.0,
      fact=("WARDROBE-SIZED", "SPEAKER BOXES"), notes=BOOGIE + bed()),
 dict(key="sl1200", theme="sl1200", year=1972, ys="1972", name="TECHNICS SL-1200", credit="PHILLY SOUL", size=3.1,
      fact=("MAGNET UNDER THE", "PLATTER DRIVES IT"), notes=PHILLY + bed()),
 dict(key="herc", theme="herc", year=1973, ys="1973", name="KOOL HERC'S TWIN DECKS", credit="FUNK BREAKBEAT", size=1.05,
      fact=("TWO DECKS STRETCH", "THE BREAKBEAT"), notes=FUNK + bed()),
 dict(key="scratch", theme="scratch", year=1975, ys="1975", name="THE FIRST SCRATCH", credit="BRONX B-BOY BREAK", size=2.2,
      fact=("A 12-YEAR-OLD", "INVENTS SCRATCHING"), notes=BBOY + bed()),
 dict(key="disco", theme="disco", year=1976, ys="1976", name="12-INCH SINGLE", credit="DISCO", size=1.45,
      fact=("WIDER GROOVES,", "LOUDER DISCO"), notes=DISCO + bed()),
 dict(key="mk2", theme="mk2", year=1979, ys="1979", name="TECHNICS SL-1200MK2", credit="ELECTRO-FUNK", size=3.3,
      fact=("PITCH SLIDER:", "THE DJ STANDARD"), notes=ELECTRO + bed()),
 dict(key="cdj", theme="cdj", year=1994, ys="1994", name="PIONEER CDJ-500", credit="90s TRANCE", size=3.5,
      fact=("CDS GET A JOG", "DIAL FOR CUEING"), notes=TRANCE),
 dict(key="fs", theme="fs", year=2001, ys="2001", name="FINAL SCRATCH", credit="MINIMAL TECHNO", size=1.62,
      fact=("TIMECODE RECORDS", "PLAY DIGITAL FILES"), notes=MINIMAL),
 dict(key="modern", theme="modern", year=2020, ys="2020s", name="TECHNICS SL-1200MK7", credit="UK GARAGE + SCRATCH", size=3.5,
      fact=("DISCONTINUED 2010,", "BACK BY 2016"), notes=LAST),
]
TRIM = {"disco": 1.5, "edison": -.5}
for _s in SCENES: _s["trim"] = TRIM.get(_s["key"], 0.)
def scene_t0(si): return 0. if si == 0 else (SHOW_BARS + SCENE_BARS*(si - 1))*BAR_S

# ---------------------------------------------------------------- scratch motion (shared by the audio and the records on screen)
def np_ease(x): x = np.clip(x, 0, 1); return x*x*(3 - 2*x)
SCR_PAR = {BABY: (2, .13), CHIRP: (2, .13), TRANS: (1, .2), FWD: (2, .12), SCRIB: (8, .022), TEAR: (1, .18), FLARE: (1, .2)}
def qmotion(kind, t, d):
    """record position under the hand (seconds of audio from the hold point) and the crossfader gate"""
    t = np.asarray(t, float)
    if kind == BACKSPIN: return -1.7*np.clip(t/d, 0, 1)**1.6, np.ones_like(t)
    cpb, A = SCR_PAR[kind]
    n = max(1, round(d/BEAT_S*cpb)); fr = (t*n/d) % 1
    if kind == TEAR:
        q = A*np.where(fr < .5, np.where(fr < .2, .5*np_ease(fr/.2), np.where(fr < .3, .5, .5 + .5*np_ease((fr - .3)/.2))),
                       1 - np_ease((fr - .5)/.5))
    else: q = A*(1 - np.cos(2*np.pi*fr))/2
    hs = (2*fr) % 1
    if kind == CHIRP: g = hs < .5
    elif kind == TRANS: g = np.floor(t/(BEAT_S/8)) % 2 == 0
    elif kind == FWD: g = fr < .5
    elif kind == FLARE: g = ~((hs > .38) & (hs < .62))
    else: g = np.ones_like(t, bool)
    return q, g.astype(float)

FILL_KINDS = (BABY, CHIRP, TRANS, TEAR, FLARE, SCRIB, FWD)
FILL_BARS = list(range(4, 31, 2))                       # every scene change, and the hand-off into the outro
SCR = []                                                # (start, length, kind, scene index or -1 for a fill)
for _si, _s in enumerate(SCENES):
    for _b, _m, _d, _v in _s["notes"]:
        if isinstance(_m, int) and 1 <= _m <= 8: SCR.append((scene_t0(_si) + _b*BEAT_S, _d*BEAT_S, _m, _si))
for _k, _b in enumerate(FILL_BARS): SCR.append((_b*BAR_S - BEAT_S, BEAT_S, FILL_KINDS[_k % len(FILL_KINDS)], -1))
SCR.sort()
_CUM = np.cumsum([0.] + [(-1.7 if k == BACKSPIN else 0.) - d for a, d, k, si in SCR])
def scr_at(T):
    for i, (a, d, k, si) in enumerate(SCR):
        if a <= T < a + d: return i
        if a > T: return None
    return None
def playpos(T):
    """record time: normal play, except the record follows the hand during a scratch (continuous)"""
    off = 0.
    for i, (a, d, k, si) in enumerate(SCR):
        if T >= a + d: continue
        if T >= a:
            q, _ = qmotion(k, T - a, d); return a + float(q) + _CUM[i]
        return T + _CUM[i]
    return T + _CUM[-1]
def jog(T):
    """(record offset under the hand, gate, kind, scene) of the scratch at T, or None"""
    i = scr_at(T)
    if i is None: return None
    a, d, k, si = SCR[i]; q, g = qmotion(k, T - a, d)
    if k == BACKSPIN: q = -.9*math.sin(math.pi*(T - a)/d)
    return float(q), float(g), k, si, (T - a)/d
def hand_k(T, lead=.18):
    """0..1: how far a hand is in on the record (eases in before a scratch, out after)"""
    best = 0.
    for a, d, k, si in SCR:
        if a - lead < T < a + d + lead: best = max(best, ease(min((T - (a - lead))/lead, (a + d + lead - T)/lead)))
    return best

# ---------------------------------------------------------------- DSP helpers
def _t(n): return np.arange(n)/SR
_RNG = np.random.default_rng(130)
@lru_cache(None)
def _bw(kind, f1, f2=0, o=2):
    if kind == "band": return butter(o, [f1/(SR/2), f2/(SR/2)], "band")
    return butter(o, f1/(SR/2), kind)
def lp(x, f, o=2): return lfilter(*_bw("low", int(f), 0, o), x)
def hp(x, f, o=2): return lfilter(*_bw("high", int(f), 0, o), x)
def bp(x, f1, f2, o=2): return lfilter(*_bw("band", int(f1), int(f2), o), x)
def peak(x, f, q, g): bw = f/q; return x + g*bp(x, f - bw/2, f + bw/2)
def bl(ph, fmax, kind="saw", kmax=40):
    K = int(max(1, min(kmax, (SR*.42)//max(fmax, 20.)))); x = np.zeros_like(ph)
    if kind == "saw":
        for k in range(1, K + 1): x += np.sin(k*ph)/k
        return x*.6
    for k in range(1, K + 1, 2): x += np.sin(k*ph)/k
    return x*.8
def adsr(t, dur, a=.005, d=.2, s=.7, r=.1):
    e = np.where(t < a, t/a, s + (1 - s)*np.exp(-(t - a)/d))
    return e*np.where(t < dur, 1., np.exp(-(t - dur)/r))
def echo(x, sec, fb=.3, n=2):
    d = int(sec*SR); out = np.pad(x, (0, d*n))
    for k in range(1, n + 1): out[d*k:d*k + len(x)] += x*(fb**k)
    return out
def vib(t, rate=5.5, depth=.004, delay=.2): return 1 + depth*np.sin(2*np.pi*rate*t)*np.clip((t - delay)/.25, 0, 1)
WOW = {"edison": ((.018, .9), (.007, 3.1)), "berliner": ((.011, 1.17), (.003, 4.)), "victrola": ((.004, 1.3),), "lp": ((.0015, .55),)}
def wow(T0, t, key):
    m = np.ones_like(t)
    for a, fr in WOW.get(key, ()): m += a*np.sin(2*np.pi*fr*(T0 + t))
    return m
def phase(fr): return 2*np.pi*np.cumsum(fr)/SR
def addmix(*xs):
    n = max(len(x) for x in xs); out = np.zeros(n)
    for x in xs: out[:len(x)] += x
    return out

# ---------------------------------------------------------------- voices
FORMANTS = {"a": ((800, 1.), (1150, .55), (2900, .25)), "e": ((520, 1.), (1750, .45), (2500, .25)),
            "i": ((340, 1.), (2050, .35), (2850, .22)), "o": ((520, 1.), (860, .6), (2550, .2))}
def formant(x, vowel, sharp=.14):
    return sum(g*bp(x, f*(1 - sharp), f*(1 + sharp)) for f, g in FORMANTS[vowel])
def v_vocal(fr, dur, v, vowel):                         # a singing/reciting voice (Edison's test recording)
    n = len(fr); t = _t(n); ph = phase(fr*vib(t, 5.2, .012, .12))
    K = int(min(30, (SR*.42)//fr.max())); x = sum(np.sin(k*ph)/k**1.1 for k in range(1, K + 1))
    y = formant(x, vowel)*adsr(t, dur, .03, .25, .85, .07)
    k = int(.02*SR); y[:k] += bp(_RNG.normal(0, 1, k), 1500, 5000)*np.linspace(.5, 0, k)
    return y*v
def v_cornet(fr, dur, v):
    n = len(fr); t = _t(n); x = bl(phase(fr*vib(t, 5.4, .005, .25)), fr.max())
    lo = lp(x, 900); y = lo + (.25 + .75*v*np.exp(-t/.25))*(x - lo)
    return y*adsr(t, dur, .025, .2, .8, .06)*v
def v_piano(f, dur, v):
    x = P.additive(f, dur + .5, v, harm=10, tilt=1.25, d1=1.5, attack=.003)
    t = _t(len(x)); return x*np.where(t < dur, 1., np.exp(-(t - dur)/.12))*.5*(.4 + v)
def v_brass(fr, dur, v):                                # big-band brass section
    n = len(fr); t = _t(n); ph = phase(fr*vib(t, 5.8, .004, .3))
    x = bl(ph*1.004, fr.max()) + bl(ph*.996, fr.max()); lo = lp(x, 1100)
    return (lo + (.3 + .7*np.exp(-t/.18))*(x - lo))*adsr(t, dur, .02, .15, .75, .07)*v*.6
def v_upbass(f, dur, v):
    n = int((dur + .25)*SR); t = _t(n)
    x = P.ks_pluck(f, n/SR, .5, .996, int(f)) + .7*np.sin(2*np.pi*f*t)*np.exp(-t*3)
    return x*np.where(t < dur, 1., np.exp(-(t - dur)/.05))*v
def v_strings(f, dur, v, att=.14):
    n = int((dur + .35)*SR); t = _t(n); ph = phase(np.full(n, f)*vib(t, 5.5, .004, .1))
    x = lp(bl(ph, f) + bl(ph*1.005, f) + bl(ph*.995, f), 3000)
    return x*adsr(t, dur, att, .4, .9, .25)*v*.5
def v_clav(f, dur, v):
    n = int((dur + .15)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = hp(bl(ph, f) - bl(ph + 2*np.pi*.25, f), 250); lo = lp(x, 650)
    y = lo + (.2 + .8*np.exp(-t/.05))*(x - lo)
    return y*np.exp(-t*6)*np.where(t < dur, 1., np.exp(-(t - dur)/.02))*np.minimum(1, t/.002)*v
def v_hornstab(f, dur, v):
    n = int((dur + .2)*SR); t = _t(n); x = bl(2*np.pi*f*t, f) + bl(2*np.pi*f*1.006*t, f)
    lo = lp(x, 1400); return (lo + .9*np.exp(-t/.12)*(x - lo))*adsr(t, dur, .008, .12, .55, .05)*v*.6
def v_lead(f, dur, v, fprev):                           # electro-funk synth lead with glide
    n = int((dur + .2)*SR); t = _t(n)
    fr = f*np.exp(np.log((fprev or f)/f)*np.exp(-t/.04)) if fprev else np.full(n, f)
    ph = phase(fr*vib(t, 6, .006, .15)); x = .6*bl(ph, fr.max(), "sq") + .5*bl(ph*1.003, fr.max())
    lo = lp(x, 800); return np.tanh(1.5*(lo + (.3 + .7*np.exp(-t/.12))*(x - lo)))*adsr(t, dur, .005, .2, .7, .06)*v*.7
def v_supersaw(f, dur, v):
    n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = sum(bl(ph*(1 + dt), f) for dt in (0, .006, -.006, .012, -.012))*.4; lo = lp(x, 1300)
    y = (lo + np.exp(-t/.09)*(x - lo))*adsr(t, dur, .003, .16, .3, .08)
    return echo(y, .75*BEAT_S, .32, 2)*v
def v_blip(f, dur, v):
    n = int(.3*SR); t = _t(n); ph = 2*np.pi*f*t
    y = np.sin(ph + 1.6*np.exp(-t/.025)*np.sin(2*ph))*np.exp(-t/.06)*np.minimum(1, t/.001)
    return echo(y, .75*BEAT_S, .38, 3)*v
def v_organ(f, dur, v, rel=.06):                        # garage organ stab
    n = int((dur + rel*5)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = sum(a*np.sin(k*ph) for k, a in ((1, 1.), (2, .55), (3, .35), (4, .25), (6, .12)) if k*f < 15000)
    x[:int(.003*SR)] += .4*_RNG.normal(0, 1, int(.003*SR))
    return np.tanh(1.2*x*adsr(t, dur, .002, .22, .4, rel))*v*.6
def v_rhodes(f, dur, v): return D._rhodes(f, dur, v)

# ---------------------------------------------------------------- scratch sources (the sample under the needle)
def _src_fresh():                                      # "fresh": f + vowel + sh
    n = int(.75*SR); t = _t(n); x = np.zeros(n)
    k1, k2 = int(.05*SR), int(.38*SR)
    x[:k1] = bp(_RNG.normal(0, 1, k1), 1200, 6500)*np.linspace(.2, .6, k1)
    f0 = 200*np.exp(-t[:k2 - k1]*.9); ph = phase(f0)
    v_ = sum(np.sin(k*ph)/k for k in range(1, 28))
    x[k1:k2] = formant(v_, "e", .16)*adsr(t[:k2 - k1], .3, .01, .3, .8, .04)*1.6
    x[k2:] = bp(_RNG.normal(0, 1, n - k2), 2200, 7000)*np.exp(-t[:n - k2]*5)*.55
    return x/np.abs(x).max()
def _src_ahh():
    n = int(1.3*SR); t = _t(n); ph = phase(165*np.exp(-t*.08)*vib(t, 5, .01, .1))
    x = formant(sum(np.sin(k*ph)/k for k in range(1, 30)), "a", .15)*np.minimum(1, t/.015)*np.exp(-t*.4)
    return x/np.abs(x).max()
def _src_horn():
    n = int(.8*SR); t = _t(n); x = sum(bl(2*np.pi*midi_hz(m)*t, midi_hz(m)) for m in (62, 65, 69, 74))
    lo = lp(x, 1600); x = (lo + .8*np.exp(-t/.2)*(x - lo))*np.minimum(1, t/.006)*np.exp(-t*2.2)
    return x/np.abs(x).max()
def _src_digi():
    n = int(1.0*SR); t = _t(n); ph = 2*np.pi*220*t
    x = np.sin(ph + 2.2*np.sin(2*ph)*np.exp(-t*3))*np.minimum(1, t/.004)*np.exp(-t*1.5)
    return x/np.abs(x).max()
SRC = {"fresh": _src_fresh(), "ahh": _src_ahh(), "horn": _src_horn(), "digi": _src_digi()}
def _smooth_gate(g):
    a = math.exp(-1/(.0025*SR)); return lfilter([1 - a], [1, -a], g)
def scratch_audio(kind, d, src="fresh", s0=.0, friction=.05):
    sx = SRC[src]; n = int((d + .03)*SR); t = _t(n); q, g = qmotion(kind, np.minimum(t, d), d)
    if kind == TRANS or src in ("ahh", "digi"): s0 = max(s0, .12)
    vel = np.gradient(q, 1/SR); idx = (s0 + q)*SR
    y = np.interp(idx, np.arange(len(sx)), sx, left=0., right=0.)*np.clip(np.abs(vel), 0, 1.6)**.7
    y = y + bp(_RNG.normal(0, 1, n), 400, 3000)*np.clip(np.abs(vel), 0, 4)*friction
    y *= _smooth_gate(g); f = int(.02*SR); y[-f:] *= np.linspace(1, 0, f)
    return y

# ---------------------------------------------------------------- each era through its own machine
def machine(x, key):
    if key == "edison":                                 # tinfoil: thin, peaky and crunchy
        y = bp(bp(x, 430, 2700), 380, 3000); y = peak(peak(y, 1150, 3, 1.4), 2000, 4, .8); return np.tanh(2.6*y)*.6
    if key == "berliner":                               # small horn
        y = bp(x, 280, 3400); y = peak(peak(y, 750, 2.5, 1.), 1500, 3, .6); return np.tanh(1.8*y)*.7
    if key == "victrola":                               # 78 through the folded internal horn
        y = bp(x, 170, 4500); return np.tanh(1.4*peak(y, 650, 2, .8))*.8
    if key == "lp": return lp(hp(x, 55), 9500)
    if key == "jamaica":                                # homemade speaker boxes: huge bass, overdriven
        y = x + 2.4*lp(x, 160); return lp(np.tanh(1.8*y), 5200)*.7
    if key == "herc":                                   # two copies of the record, a few ms apart
        d = int(.011*SR); return x + .75*np.pad(x, (d, 0))[:len(x)]
    if key == "scratch": return np.tanh(1.3*x)
    if key == "disco": return x + .35*hp(x, 3000)
    return x

def surface(key, n):
    """surface noise for the machines that have it: hiss, swish once per turn, crackle"""
    t = _t(n); r = np.random.default_rng(len(key)*7 + n % 97)
    hiss, swish, rate, crk, hum = {"edison": (.05, (.6, 1.)), "berliner": (.04, (.5, 1.17)), "victrola": (.034, (.45, 1.3)),
                                   "lp": (.006, (0, 0)), "jamaica": (.01, (0, 0)), "sl1200": (.003, (0, 0)), "herc": (.004, (0, 0)),
                                   "scratch": (.004, (0, 0)), "disco": (.003, (0, 0)), "mk2": (.003, (0, 0))}[key] + \
                                  ({"edison": 30, "berliner": 26, "victrola": 22, "lp": 7, "jamaica": 10}.get(key, 4),
                                   {"edison": .45, "berliner": .4, "victrola": .35, "lp": .18, "jamaica": .22}.get(key, .1),
                                   .02 if key == "jamaica" else 0.)
    x = bp(r.normal(0, 1, n), 900, 6000)*hiss*(1 + swish[0]*np.sin(2*np.pi*swish[1]*t))
    if key == "edison": x += lp(r.normal(0, 1, n), 300)*.03*(1 + np.sin(2*np.pi*t))
    cl = np.zeros(n); k = r.poisson(rate*n/SR)
    pos = r.integers(0, n - 200, k); cl[pos] = r.choice([-1, 1], k)*r.random(k)**2*crk
    x += bp(lfilter([1.], [1, -.6], cl), 900, 9000)
    if hum: x += hum*(np.sin(2*np.pi*60*t) + .5*np.sin(2*np.pi*120*t))
    f = int(.08*SR); x[:f] *= np.linspace(0, 1, f); x[-f:] *= np.linspace(1, 0, f)
    return x

SCRATCH_SRC = {"modern": "fresh", "scratch": "horn", "disco": "ahh", "mk2": "ahh", "cdj": "digi", "fs": "digi"}
def prev_of(s, idx):
    nt = s["ev"][idx][0]
    for j in range(idx - 1, -1, -1):
        if s["ev"][j][0] < nt - 1e-6 and s["ev"][j][1] >= 12: return s["ev"][j]
    return None
_NOTE = {}
def synth(s, idx):
    key = (round(s["t0"], 3), idx)
    if key in _NOTE: return _NOTE[key]
    nt, m, v, dur, tp, cs = s["ev"][idx]; k = s["key"]; T0 = s["t0"] + nt
    if m == BED: x = surface(k, int((dur + .1)*SR))*v*{"edison": 2.5, "berliner": 2., "victrola": 2.2, "lp": 2., "jamaica": 1.6}.get(k, 4.)
    elif m < 12:
        src = SCRATCH_SRC.get(k, "fresh")
        x = machine(scratch_audio(m, dur, src, friction=0. if k in ("cdj", "fs") else .05), k)*v*(1.6 if k == "scratch" else 2.0)
    else:
        f = midi_hz(m)
        if k == "edison":
            n = int((dur + .3)*SR); fr = f*wow(T0, _t(n), k)
            notes = [e for e in s["ev"] if e[1] >= 12]; vi = notes.index(s["ev"][idx]) if s["ev"][idx] in notes else 0
            x = v_vocal(fr, dur, v, MARY_VOW[vi % len(MARY_VOW)])
        elif k == "berliner":
            n = int((dur + .25)*SR); x = v_cornet(f*wow(T0, _t(n), k), dur, v)
        elif k == "victrola": x = v_piano(f*float(wow(T0, np.zeros(1), k)[0]), dur, v)
        elif k == "lp":
            n = int((dur + .25)*SR); x = v_brass(f*wow(T0, _t(n), k), dur, v)
        elif k == "jamaica": x = v_upbass(f, dur, v)*1.3 if m < 57 else v_piano(f, dur, v)*.8
        elif k == "sl1200": x = v_strings(f, dur, v) if cs == 1 else v_rhodes(f, dur, v)*.55
        elif k == "herc": x = v_clav(f, dur, v) if cs == 1 else v_hornstab(f, dur, v)
        elif k == "scratch": x = v_hornstab(f, dur, v)
        elif k == "disco": x = addmix(v_strings(f, dur, v, .03)*1.4, v_piano(f/2, dur, v)*.35 if cs == 1 else np.zeros(1))
        elif k == "mk2":
            pe = prev_of(s, idx); x = v_lead(f, dur, v, midi_hz(pe[1]) if pe is not None and nt - (pe[0] + pe[3]) < .1 else None)
        elif k == "cdj": x = v_supersaw(f, dur, v) if cs == 1 else v_strings(f, dur, v, .2)*.8
        elif k == "fs": x = v_blip(f, dur, v)
        else: x = v_organ(f, dur, v, .9 if nt >= 12*BEAT_S - 1e-6 else .06)
        x = machine(x, k)
    _NOTE[key] = x; return x

# ---------------------------------------------------------------- backing: UK garage 2-step
HAT = [3150., 4400., 5600., 6900., 7700., 9100.]
def _snare(v):
    a, b = P.snare(1.)*1.1, C.clap(1.)*.8; n = max(len(a), len(b))
    return (np.pad(a, (0, n - len(a))) + np.pad(b, (0, n - len(b))))*v
KIT = {"kick": lambda v: D.kick_s(48, 150, 7.5, .3, .45, 1.7, .35)*v, "snare": _snare, "rim": lambda v: C.rim(v)*.8,
       "hat": lambda v: D.metal_s(HAT, 70, .35, .07, 7000)*v*.7, "ohat": lambda v: D.metal_s(HAT, 16, .35, .3, 6500)*v*.55,
       "shk": lambda v: hp(_RNG.normal(0, 1, int(.05*SR)), 6000)*np.exp(-_t(int(.05*SR))*90)*v*.25, "crash": P.crash}
def _fill(kind, src):
    x = scratch_audio(kind, BEAT_S, src); return x*.36/np.sqrt((x[np.abs(x) > 1e-4]**2).mean())
for _k in range(len(FILL_KINDS)):
    KIT[f"fill{_k}"] = (lambda v, kk=FILL_KINDS[_k], src=("fresh", "ahh")[_k % 2]: _fill(kk, src)*v)
KIT["spin"] = lambda v: scratch_audio(BACKSPIN, BEAT_S, "ahh")*v
KICK_KINDS, ACCENT_KINDS = ("kick",), ("snare",)
SWING, LATE = .2, .028
def drum_events(c):
    ev = []; s16 = c.BEAT/4; nb = len(BARS)
    def at(b, p, late=0.): return b*c.BAR + (p + (SWING if p % 2 else 0))*s16 + late
    for b in range(nb):
        T0 = b*c.BAR
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "snare", .8)]; continue
        fill = (b + 1) in FILL_BARS
        ev += [(at(b, p), "kick", 1. if p == 0 else .85) for p in ((0, 10) if b % 2 == 0 else (0, 7, 10)) if not (fill and p >= 12)]
        ev += [(at(b, p, LATE), "snare", .95) for p in (4, 12)]
        ev += [(at(b, p), "hat", (.5, .3)[p % 2]) for p in range(1, 16, 2) if not (fill and p >= 12)]
        ev += [(at(b, p), "ohat", .45) for p in (2, 6, 10, 14) if not (fill and p >= 12)]
        ev += [(at(b, p), "shk", .6) for p in range(0, 16, 2) if not (fill and p >= 12)]
        if b % 2: ev.append((at(b, 15), "rim", .45))
        if b == 0 or b in (4, 16, 28): ev.append((T0, "crash", .8))
        if fill: ev.append((T0 + 3*c.BEAT, f"fill{FILL_BARS.index(b + 1) % len(FILL_KINDS)}", 1.))
    return ev
BASS_PAT = ((0, 0, 2.6), (3, 0, 2), (7, 0, 1.6), (10, 0, 2), (13, 12, 1.2))
NO_BASS = (12, 13)                                     # the sound-system scene plays its own boogie bass
def bass_events(c):
    out = []; nb = len(BARS); s16 = c.BEAT/4
    for b in range(nb):
        T0, r = b*c.BAR, CH[BARS[b]][0]
        if b == nb - 1: out.append((T0, r, 2.6, 1.)); continue
        if b not in NO_BASS:
            out += [(T0 + (p + (SWING if p % 2 else 0))*s16, r + o, d*s16, .95 if p == 0 else .82) for p, o, d in BASS_PAT
                    if not ((b + 1) in FILL_BARS and p >= 12)]
        if 4 <= b < 28: out.append((T0 + 14*s16, 200 + b, .45*c.BEAT, .5))   # a chopped organ stab on the and-of-4
    return out
def bass_voice(m, dur, v):
    if m >= 200:                                        # chopped stab: the bar's chord on a short organ hit
        notes = CH[BARS[m - 200]][1]; xs = [v_organ(midi_hz(n_), dur, .7) for n_ in notes]
        return sum(xs)/len(xs)*v*.55
    f = midi_hz(m); n = int((dur + .04)*SR); t = _t(n)
    x = np.sin(2*np.pi*f*t*(1 + .06*np.exp(-t*40))) + .22*np.sin(4*np.pi*f*t)
    env = np.minimum(1, t/.004)*np.where(t < dur, 1., np.exp(-(t - dur)/.02))*(.75 + .25*np.exp(-t*6))
    return np.tanh(1.6*x*env)*v
pad_voice = None
MIX = dict(drums=-18, bass=-22, verb=-27, verb_len=.9)

# ---------------------------------------------------------------- geometry helpers (metres; x right, y up, z toward the viewer)
UP = (0, 1, 0)
def unit(v): v = np.asarray(v, float); return v/np.linalg.norm(v)
def cyl(g, c, r, h, col, top=None, n=12, axis=UP, emit=False, phase=0.):
    return S.prism(g, c, axis, r, h, n, top or col, col, emit, phase)
def cyl_dots(g, c, r, h, n, ang, top, side, dot, every=2, emit_dot=False):
    """vertical cylinder whose side faces alternate (strobe dots / ribbed rubber), rotating with ang"""
    c = np.asarray(c, float); y0, y1 = c[1] - h/2, c[1] + h/2
    pts = [(c[0] + r*math.cos(ang + 2*math.pi*k/n), c[2] + r*math.sin(ang + 2*math.pi*k/n)) for k in range(n)]
    g.face([(x, y1, z) for x, z in pts], top, ref=(c[0], y0 - 1, c[2]))
    for k in range(n):
        (xa, za), (xb, zb) = pts[k], pts[(k + 1) % n]; isdot = k % every == 0
        g.face([(xa, y0, za), (xb, y0, zb), (xb, y1, zb), (xa, y1, za)], dot if isdot else side, ref=tuple(c), emit=emit_dot and isdot)
def wedges(g, c, r, y, ang, c1, c2, n=8, r0=0.):
    for k in range(n):
        a0, a1 = ang + 2*math.pi*k/n, ang + 2*math.pi*(k + 1)/n
        P0 = (c[0] + r0*math.cos(a0), y, c[1] + r0*math.sin(a0))
        P1 = (c[0] + r*math.cos(a0), y, c[1] + r*math.sin(a0)); P2 = (c[0] + r*math.cos(a1), y, c[1] + r*math.sin(a1))
        pts = [P0, P1, P2] if r0 == 0 else [P0, P1, P2, (c[0] + r0*math.cos(a1), y, c[1] + r0*math.sin(a1))]
        g.face(pts, c1 if k % 2 else c2, ref=(c[0], y - 1, c[1]))
def label(g, c, r, y, ang, c1, c2, sc=1.):
    """plain record label: outer colour, inner disc, one tick that shows the spin"""
    ring = [(c[0] + r*math.cos(2*math.pi*k/14), y, c[1] + r*math.sin(2*math.pi*k/14)) for k in range(14)]
    g.face(ring, c1, ref=(c[0], y - 1, c[1]))
    dy = .0004*sc
    inner = [(c[0] + .42*r*math.cos(2*math.pi*k/10), y + dy, c[1] + .42*r*math.sin(2*math.pi*k/10)) for k in range(10)]
    g.face(inner, c2, ref=(c[0], y - 1, c[1]))
    a0, a1 = ang - .3, ang + .3
    tick = [(c[0] + .55*r*math.cos(a), y + dy, c[1] + .55*r*math.sin(a)) for a in (a0, a1)] + \
           [(c[0] + .92*r*math.cos(a), y + dy, c[1] + .92*r*math.sin(a)) for a in (a1, a0)]
    g.face(tick, c2, ref=(c[0], y - 1, c[1]))
def circle(g, c, r, y, col, n=20):
    pts = [(c[0] + r*math.cos(2*math.pi*k/n), y, c[1] + r*math.sin(2*math.pi*k/n)) for k in range(n + 1)]
    for a, b in zip(pts, pts[1:]): g.line(a, b, col)
def obox(g, c, d, L, W, y0, y1, col, spec=0., emit=False):
    """box centred at c (x, z), long axis along d (x, z)"""
    d = np.asarray(d, float); d = d/np.linalg.norm(d); n = np.array([-d[1], d[0]])
    cs = [np.asarray(c) + d*L/2*sa + n*W/2*sb for sa, sb in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    V = [(p[0], y, p[1]) for y in (y0, y1) for p in cs]; ref = (c[0], (y0 + y1)/2, c[1])
    for idx in ((0, 1, 2, 3), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)):
        g.face([V[i] for i in idx], col, ref=ref, spec=spec, emit=emit)
def rrect(W, Dd, r, k=3):
    pts = []
    for cx, cz, a0 in ((W - r, Dd - r, 0), (-W + r, Dd - r, 90), (-W + r, -Dd + r, 180), (W - r, -Dd + r, 270)):
        for j in range(k + 1):
            a = math.radians(a0 + 90*j/k); pts.append((cx + r*math.cos(a), cz + r*math.sin(a)))
    return pts
def notes_fx(fx, ev, p, sp=None, vmin=.55):
    for age, m, v, dur, tp, cs in ev:
        if m >= 12 and tp and v >= vmin and age < .9: fx.append(("note", p, age))
        if 1 <= m <= 8 and age < .5 and int(age*30) % 3 == 0: fx.append(("spark", sp or p, age))

def rec_angle(T, rpm): return rpm*2*math.pi/60*playpos(T)

def glove(g, c, ang, y, sc, sleeve=(60, 60, 70), out=1.):
    """a white cartoon glove resting on a record at c (x, z), fingers pointing along ang; the sleeve trails outward"""
    d = np.array([math.cos(ang), math.sin(ang)]); n = np.array([-d[1], d[0]]); W_ = (244, 242, 236)
    c = np.asarray(c, float)
    obox(g, c, d, .06*sc, .055*sc, y, y + .022*sc, W_)
    for k, o in enumerate((-.019, -.006, .007, .02)):
        L = (.03, .036, .034, .028)[k]*sc; p = c + d*(.03*sc + L/2) + n*o*sc
        obox(g, p, d, L, .0115*sc, y, y + .014*sc, W_)
    obox(g, c - n*.034*sc + d*.012*sc, d*.6 + n*-.4, .03*sc, .013*sc, y, y + .014*sc, W_)
    obox(g, c - d*.04*sc, d, .022*sc, .066*sc, y - .002*sc, y + .028*sc, (226, 224, 218))       # cuff
    a = (c[0] - d[0]*.05*sc, y + .014*sc, c[1] - d[1]*.05*sc)
    b = (c[0] - d[0]*.16*sc*out, y + .11*sc*out, c[1] - d[1]*.16*sc*out)
    T.tube(g, a, b, .02*sc, .024*sc, col=sleeve, n=8, spec=0.)

# ---------------------------------------------------------------- a Technics-style direct-drive deck (also the base of several scenes)
TT_W, TT_D, TT_TOP = .2265, .1765, .12
PCX, PCZ = -.065, -.005
def tonearm(g, Lp, piv, sty, base_y, ya, ry, col, col2, cw, sc, head=(40, 40, 44)):
    px, pz = piv; sx, sz = sty
    cyl(g, Lp(px, base_y + .006, pz), .036*sc, .012*sc, col2, n=14)
    cyl(g, Lp(px, (base_y + .012 + ya)/2, pz), .016*sc, (ya - base_y - .012)*sc, dim(col2, .8), n=8)
    gx, gy, gz = Lp(px, ya, pz); g.box(gx - .013*sc, gx + .013*sc, gy - .012*sc, gy + .01*sc, gz - .013*sc, gz + .013*sc, col, spec=.4)
    dx, dz = sx - px, sz - pz; L = math.hypot(dx, dz); ux, uz = dx/L, dz/L; nx, nz = -uz, ux
    ex, ez = sx - ux*.042, sz - uz*.042
    pts = [Lp(px + (ex - px)*u + nx*o, ya - .004*u, pz + (ez - pz)*u + nz*o) for u, o in ((0, 0), (.35, .02), (.72, -.012), (1, 0))]
    for a, b in zip(pts, pts[1:]): T.tube(g, a, b, .0045*sc, col=col, n=6, spec=.7)
    ca, sa = math.cos(.38), math.sin(.38)
    hd = (ux*ca - uz*sa, ux*sa + uz*ca)
    hc = Lp(sx - hd[0]*.012, 0, sz - hd[1]*.012)
    obox(g, (hc[0], hc[2]), hd, .052*sc, .02*sc, Lp(0, ya - .009, 0)[1], Lp(0, ya - .003, 0)[1], head, spec=.3)
    cc = Lp(sx, 0, sz)
    obox(g, (cc[0], cc[2]), hd, .022*sc, .015*sc, Lp(0, ry + .002, 0)[1], Lp(0, ya - .009, 0)[1], (24, 24, 26))
    fl = Lp(sx + hd[0]*.02 - hd[1]*.012, ya - .006, sz + hd[1]*.02 + hd[0]*.012)
    g.line(fl, Lp(sx + hd[0]*.03 - hd[1]*.03, ya - .002, sz + hd[1]*.03 + hd[0]*.03), dim(col, .9))
    T.tube(g, Lp(px - ux*.018, ya, pz - uz*.018), Lp(px - ux*.05, ya, pz - uz*.05), .009*sc, col=dim(col, .8), n=6)
    T.tube(g, Lp(px - ux*.05, ya, pz - uz*.05), Lp(px - ux*.088, ya, pz - uz*.088), .022*sc, col=cw, n=10, spec=.5)
    rp = Lp(px + .038, base_y, pz + .072)
    g.box(rp[0] - .005*sc, rp[0] + .005*sc, rp[1], rp[1] + (ya - base_y - .004)*sc, rp[2] - .005*sc, rp[2] + .005*sc, dim(col2, .7))
    return Lp(sx, ry + .002, sz)

DECK = {
 "mk7":  dict(plinth=(46, 46, 50), edge=(30, 30, 33), platter=(66, 66, 70), ptop=(40, 40, 42), dots=(150, 150, 156),
              mat="felt", mat_col=(20, 20, 22), rec=(18, 18, 20), lab=((212, 168, 70), (26, 24, 24)), arm=(44, 44, 48),
              arm2=(30, 30, 32), cw=(40, 40, 44), pitch="slider", strobe=False, target=True, led=(255, 50, 40), rpm=33.3),
 "sl1200": dict(plinth=(216, 200, 164), edge=(162, 148, 118), platter=(186, 184, 178), ptop=(40, 38, 36), dots=(90, 88, 84),
              mat="rib", mat_col=(30, 30, 30), rec=(20, 20, 22), lab=((236, 140, 44), (112, 62, 30)), arm=(206, 206, 210),
              arm2=(70, 70, 72), cw=(150, 150, 156), pitch="knob", strobe=False, target=False, led=None, rpm=33.3, lid=True),
 "mk2":  dict(plinth=(176, 182, 194), edge=(122, 128, 140), platter=(196, 198, 204), ptop=(40, 40, 42), dots=(96, 98, 104),
              mat="felt", mat_col=(52, 52, 56), rec=(18, 18, 20), lab=((226, 44, 40), (200, 204, 212)), arm=(214, 214, 218),
              arm2=(56, 56, 60), cw=(170, 170, 176), pitch="slider", strobe=True, target=True, led=None, rpm=33.3),
}
def deck(g, T, cfg, ox=0., oz=0., y0=0., sc=1., hand=False, sleeve=(70, 60, 90), fx=None, ev=None):
    def Lp(x, y, z): return (ox + x*sc, y0 + y*sc, oz + z*sc)
    W, Dd, top = TT_W, TT_D, TT_TOP
    for x in (-W + .045, W - .045):
        for z in (-Dd + .045, Dd - .045): cyl(g, Lp(x, .016, z), .03*sc, .032*sc, cfg.get("feet", (28, 28, 30)), n=10)
    pts = rrect(W, Dd, .05) if cfg.get("rounded") else [(-W, -Dd), (W, -Dd), (W, Dd), (-W, Dd)]
    P.prism(g, [(ox + x*sc, oz + z*sc) for x, z in pts], y0 + .03*sc, y0 + top*sc, cfg["edge"], cfg["plinth"])
    ang = rec_angle(T, cfg["rpm"])
    pc = Lp(PCX, top + .013, PCZ)
    cyl_dots(g, pc, .166*sc, .024*sc, 44, ang, cfg["ptop"], cfg["platter"], cfg["dots"])
    ytop = top + .025
    if cfg["mat"] != "none": cyl(g, Lp(PCX, ytop + .0015, PCZ), .161*sc, .003*sc, cfg["mat_col"], n=28)
    if cfg["mat"] == "rib":
        for r in (.154, .158): circle(g, (pc[0], pc[2]), r*sc, Lp(0, ytop + .0032, 0)[1], (64, 64, 62), 28)
    ry = ytop + .003 + .004
    if cfg.get("no_record"): ry = ytop + .003
    else:
        rec = cfg.get("rec", (18, 18, 20))
        S.prism(g, Lp(PCX, ry - .002, PCZ), UP, .151*sc, .004*sc, 28, rec, dim(rec, .8))
        yy = Lp(0, ry + .0004, 0)[1]
        for r in (.085, .118, .14): circle(g, (pc[0], pc[2]), r*sc, yy, mix(rec, (150, 150, 160), .35), 24)
        if cfg.get("timecode"):
            wedges(g, (pc[0], pc[2]), .148*sc, Lp(0, ry + .0006, 0)[1], ang, (190, 194, 200), rec, 36, .095*sc)
        l1, l2 = cfg["lab"]
        label(g, (pc[0], pc[2]), .05*sc, Lp(0, ry + .0008, 0)[1], ang, l1, l2, sc)
    cyl(g, Lp(PCX, ry + .006, PCZ), .0035*sc, .012*sc, (200, 200, 206), n=6)
    sty = (PCX + .07, PCZ + .095)
    tip = tonearm(g, Lp, (.155, -.095), sty, top, ry + .024, ry, cfg["arm"], cfg["arm2"], cfg["cw"], sc)
    yb = top
    if cfg["pitch"] == "slider":
        sl = Lp(.196, yb + .001, .085); g.box(sl[0] - .006*sc, sl[0] + .006*sc, sl[1], sl[1] + .002*sc, sl[2] - .062*sc, sl[2] + .062*sc, (14, 14, 16))
        kz = .085 + .03*math.sin(T*.7)
        kp = Lp(.196, yb, kz); g.box(kp[0] - .014*sc, kp[0] + .014*sc, kp[1], kp[1] + .012*sc, kp[2] - .008*sc, kp[2] + .008*sc,
                                     (30, 30, 32) if cfg["plinth"][0] > 100 else (60, 60, 64))
        for dz in (-.004, 0, .004): g.line(Lp(.184, yb + .0125, kz + dz), Lp(.208, yb + .0125, kz + dz), (150, 150, 156))
    elif cfg["pitch"] == "knob":
        cyl_dots(g, Lp(-.19, yb + .011, .118), .017*sc, .02*sc, 12, 0., (210, 210, 214), (170, 170, 176), (120, 120, 126))
        g.box(*[v for p in zip(Lp(-.215, yb, .152), Lp(-.15, yb + .0015, .158)) for v in p], (20, 20, 22))
        lv = Lp(-.2 + .03*(.5 + .5*math.sin(T*.5)), yb, .155); g.box(lv[0] - .005*sc, lv[0] + .005*sc, lv[1], lv[1] + .012*sc, lv[2] - .005*sc, lv[2] + .005*sc, (16, 16, 18))
        for x in (-.09, -.06): cyl(g, Lp(x, yb + .006, .152), .007*sc, .012*sc, (180, 180, 186), n=8)
    if cfg["pitch"] == "slider":
        bp_ = Lp(-.19, yb, .148); g.box(bp_[0] - .02*sc, bp_[0] + .02*sc, bp_[1], bp_[1] + .007*sc, bp_[2] - .014*sc, bp_[2] + .014*sc, dim(cfg["plinth"], .75))
        if cfg.get("led"):
            lp_ = Lp(-.19, yb + .0072, .158); g.box(lp_[0] - .004*sc, lp_[0] + .004*sc, lp_[1], lp_[1] + .001*sc, lp_[2] - .002*sc, lp_[2] + .002*sc, cfg["led"], emit=True)
        for x in (-.15, -.125):
            q = Lp(x, yb, .153); g.box(q[0] - .009*sc, q[0] + .009*sc, q[1], q[1] + .005*sc, q[2] - .006*sc, q[2] + .006*sc, dim(cfg["plinth"], .8))
    if cfg.get("strobe"):
        cyl(g, Lp(-.207, yb + .012, .1), .011*sc, .024*sc, (40, 40, 42), top=(255, 70, 50), n=8, emit=False)
        cyl(g, Lp(-.207, yb + .025, .1), .008*sc, .003*sc, (255, 60, 40), n=8, emit=True)
    if cfg.get("target"):
        cyl(g, Lp(.208, yb + .022, .04), .004*sc, .044*sc, (40, 40, 44), n=6)
        cyl(g, Lp(.208, yb + .046, .04), .006*sc, .006*sc, (255, 250, 236), n=6, emit=True)
    if cfg.get("lid"):                                     # open acrylic dust cover: just its edges, like clear plastic
        a = math.radians(58); hy, hz = top + .004, -Dd
        dv, hv = np.array([0, math.sin(a), math.cos(a)]), np.array([0, math.cos(a), -math.sin(a)])
        cs = [np.array([x, hy, hz]) + dv*v + hv*h for x in (-W + .003, W - .003) for v in (0, Dd*2 - .01) for h in (-.002, .055)]
        cs = [Lp(*p) for p in cs]
        for i, j in ((0, 1), (2, 3), (0, 2), (1, 3), (4, 5), (6, 7), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)):
            g.line(cs[i], cs[j], (214, 222, 232))
        for x in (-.14, .14): g.box(*[v for p in zip(Lp(x - .015, top, -Dd - .012), Lp(x + .015, top + .02, -Dd + .004)) for v in p], (60, 60, 64))
    if hand:
        k = hand if isinstance(hand, float) else 1.
        if k > .02:
            j = jog(T); q = j[0] if j else 0.
            ha = math.radians(150) + cfg["rpm"]*2*math.pi/60*q*1.0
            hp_ = (PCX + .118*math.cos(ha), PCZ + .118*math.sin(ha)); c = Lp(hp_[0], 0, hp_[1])
            glove(g, (c[0], c[2]), ha + math.pi + .25, Lp(0, ry + .001 + (1 - k)*.06, 0)[1], sc*1.3, sleeve, .6 + .4*k)
    if fx is not None and ev is not None:
        notes_fx(fx, ev, (Lp(sty[0], ry + .12, sty[1])), tip)
    return Lp, ry, tip

# ---------------------------------------------------------------- 0 / 13: Technics SL-1200MK7 (matte black)
def mk7(g, t, ev, fx, s):
    T_ = s["t0"] + t
    deck(g, T_, DECK["mk7"], hand=hand_k(T_), sleeve=(30, 30, 36), fx=fx, ev=ev)

# ---------------------------------------------------------------- 1877: Edison tinfoil phonograph (Smithsonian / Edison NHP photos)
def edison(g, t, ev, fx, s):
    T_ = s["t0"] + t
    wood, wood_l, iron, brass, foil, steel = (112, 64, 36), (140, 84, 48), (44, 46, 44), (196, 156, 72), (214, 216, 222), (160, 162, 168)
    g.box(-.25, .25, 0, .03, -.15, .15, wood)                                  # stepped wooden base
    g.box(-.23, .23, .03, .042, -.13, .13, wood_l)
    g.box(-.2, .2, .042, .058, -.095, .07, iron)                               # cast-iron bed
    yc = .17; ang = rec_angle(T_, 60.)
    for sx in (-1, 1):                                                         # A-frame bearing posts
        x = sx*.13
        T.tube(g, (x, .058, -.075), (x, yc, 0), .009, col=iron, n=6, spec=.2)
        T.tube(g, (x, .058, .055), (x, yc, 0), .009, col=iron, n=6, spec=.2)
        g.box(x - .014, x + .014, yc - .016, yc + .016, -.016, .016, iron)
    T.tube(g, (-.29, yc, 0), (.27, yc, 0), .0065, col=steel, n=8, spec=.8)     # the threaded main shaft
    for k in range(26):                                                        # its thread, turning
        x0 = .075 + k*.0072; a0 = ang*1. + k*1.9
        g.line((x0, yc + .007*math.cos(a0), .007*math.sin(a0)), (x0 + .0072, yc + .007*math.cos(a0 + 1.9), .007*math.sin(a0 + 1.9)), (90, 92, 96))
    shift = .004*math.sin(T_*.3)
    T.tube(g, (-.065 + shift, yc, 0), (.065 + shift, yc, 0), .055, col=foil, n=18, spec=.6)   # tinfoil-wrapped cylinder
    for sx in (-1, 1):
        ring = [(sx*.066 + shift, yc + .055*math.cos(a), .055*math.sin(a)) for a in np.linspace(0, 2*math.pi, 19)[:-1]]
        g.face(ring, brass, ref=(0, yc, 0))
        for k in range(4):
            a = ang + k*math.pi/2; g.line((sx*.067 + shift, yc, 0), (sx*.067 + shift, yc + .05*math.cos(a), .05*math.sin(a)), dim(brass, .55))
    for k in range(9):                                                         # recorded groove rings + the foil seam
        x = -.05 + k*.0125 + shift; pts = [(x, yc + .0555*math.cos(a), .0555*math.sin(a)) for a in np.linspace(0, 2*math.pi, 15)]
        for a_, b_ in zip(pts, pts[1:]): g.line(a_, b_, (150, 152, 160) if k < 6 else (190, 192, 198))
    for x0, x1 in ((-.06, .06),):
        g.line((x0 + shift, yc + .056*math.cos(ang), .056*math.sin(ang)), (x1 + shift, yc + .056*math.cos(ang), .056*math.sin(ang)), (120, 120, 128))
    ca = ang                                                                   # crank on the left end
    cp = np.array([-.29, yc, 0]); ce = cp + np.array([0, .065*math.cos(ca), .065*math.sin(ca)])
    T.tube(g, cp, ce, .006, col=iron, n=5, spec=.3)
    T.tube(g, ce, ce + np.array([-.055, 0, 0]), .011, .012, col=(150, 86, 44), n=8, spec=.2)
    # recorder / reproducer: diaphragm housing on its arm, mouthpiece toward the viewer, stylus on the foil
    hz = .085
    T.tube(g, (-.012, .062, .062), (-.012, yc - .03, hz), .006, col=iron, n=5); T.tube(g, (.012, .062, .062), (.012, yc - .03, hz), .006, col=iron, n=5)
    cyl(g, (0, yc, hz), .034, .018, iron, top=brass, n=16, axis=(0, 0, 1))
    T.tube(g, (0, yc, hz + .009), (0, yc, hz + .05), .011, .021, col=brass, n=10, spec=.8, two=True)
    g.face(S.ring((0, yc, hz + .049), (0, 0, 1), .016, 10), (30, 22, 12), ref=(0, yc, 0), emit=True)
    T.tube(g, (0, yc, hz - .009), (0, yc, .056), .0025, col=steel, n=4)
    cyl(g, (.07, .1, .1), .014, .02, (30, 28, 26), n=10, axis=(0, 0, 1))      # adjusting knob
    T.tube(g, (.07, .1, .09), (.02, yc - .02, hz - .01), .004, col=iron, n=4)
    for age, m, v, dur, tp, cs in ev:
        if m >= 12 and age < .8: fx.append(("wave", (0, yc, hz + .07), age, 0))
    notes_fx(fx, ev, (0, yc + .12, hz))

# ---------------------------------------------------------------- 1887: Berliner hand-wind gramophone (Smithsonian / LoC photos)
def berliner(g, t, ev, fx, s):
    T_ = s["t0"] + t
    oak, oak_d, rub, iron, nickel, blk = (166, 108, 56), (120, 74, 38), (46, 30, 22), (40, 40, 42), (190, 190, 196), (24, 22, 22)
    g.box(-.21, .21, 0, .035, -.13, .13, oak)
    ang = rec_angle(T_, 70.); cx, cz = -.075, -.01
    cyl(g, (cx, .052, cz), .016, .034, iron, n=8)
    cyl(g, (cx, .072, cz), .088, .008, (60, 60, 62), n=24)                     # turntable
    S.prism(g, (cx, .078, cz), UP, .086, .004, 24, rub, dim(rub, .8))         # 7-inch hard-rubber disc
    for r in (.05, .07): circle(g, (cx, cz), r, .0805, (90, 66, 50), 20)
    label(g, (cx, cz), .028, .081, ang, (232, 218, 186), (150, 110, 70))
    cyl(g, (cx, .086, cz), .006, .012, nickel, n=6)
    ca = ang*.35                                                               # the two belt pulleys, facing the viewer
    sp, bp_ = np.array([-.13, .068, .108]), np.array([.07, .085, .108])
    for c, r in ((sp, .026), (bp_, .05)):
        g.box(c[0] - .008, c[0] + .008, .035, c[1], c[2] - .02, c[2] - .01, oak_d)
        cyl(g, c, r, .012, (150, 108, 60), top=(176, 128, 72), n=16, axis=(0, 0, 1), phase=ca)
        g.line(c + np.array([0, 0, .007]), c + np.array([r*math.cos(ca), r*math.sin(ca), .007]), (70, 46, 24))
    for sgn in (1, -1):
        g.line(sp + np.array([0, sgn*.026, 0]), bp_ + np.array([0, sgn*.05, 0]), (150, 92, 50))
        g.line(sp + np.array([0, sgn*.026, .004]), bp_ + np.array([0, sgn*.05, .004]), (150, 92, 50))
    hb = bp_ + np.array([.036*math.cos(ca*2.85), .036*math.sin(ca*2.85), 0])     # crank handle turning with the big pulley
    T.tube(g, hb, hb + np.array([0, 0, .04]), .008, .009, col=oak_d, n=6, spec=.2)
    # horn on its wooden arm from the post at the back right
    g.box(.13, .16, .035, .17, -.1, -.07, oak_d)
    so = np.array([cx + .062, .115, cz + .03])
    T.tube(g, (.145, .17, -.085), so + np.array([.0, .0, -.0]), .008, col=oak_d, n=6, spec=.1)
    cyl(g, so, .024, .012, nickel, top=(214, 214, 220), n=12, axis=(1, 0, 0))
    T.tube(g, so - np.array([0, .01, 0]), so - np.array([0, .033, 0]), .0025, col=nickel, n=4)
    d = unit((.85, .3, -.42)); L = .3
    mouth = None
    for k in range(7):
        u0, u1 = k/7, (k + 1)/7
        r0, r1 = .011*(.085/.011)**(u0**1.8), .011*(.085/.011)**(u1**1.8)
        T.tube(g, so + d*L*u0 + np.array([.008, 0, 0]), so + d*L*u1 + np.array([.008, 0, 0]), r0, r1, col=blk, n=12, spec=.5, two=u1 > .5)
    mouth = so + d*L + np.array([.008, 0, 0])
    T.tube(g, mouth - d*.008, mouth + d*.002, .087, .088, col=(196, 156, 72), n=14, spec=.8, two=True)
    g.face(S.ring(mouth - d*.03, d, .07, 12), (14, 10, 8), ref=so, emit=True, two=True)
    for age, m, v, dur, tp, cs in ev:
        if m >= 12 and tp and age < .7: fx.append(("wave", tuple(mouth + np.array([.03, 0, 0])), age, 0))
    notes_fx(fx, ev, tuple(mouth + np.array([0, .1, 0])))

# ---------------------------------------------------------------- 1906: Victor Victrola VTLA (victor-victrola.com photos): no external horn
def victrola(g, t, ev, fx, s):
    T_ = s["t0"] + t
    mah, mah_d, mah_l, gold, felt = (112, 44, 28), (78, 28, 18), (140, 60, 38), (214, 172, 84), (40, 92, 52)
    W, Dd = .25, .26
    for x in (-W + .03, W - .03):
        for z in (-Dd + .03, Dd - .03): g.box(x - .025, x + .025, 0, .045, z - .025, z + .025, mah_d)
    g.box(-W, W, .045, .86, -Dd, Dd, mah)                                      # tall upright cabinet
    for x0, x1 in ((-W + .02, -.006), (.006, W - .02)):                        # lower record-storage doors
        g.box(x0, x1, .09, .5, Dd, Dd + .006, mah_l)
        g.box(x0 + .02, x1 - .02, .12, .47, Dd + .006, Dd + .01, mah)
        kx = x1 - .02 if x0 < 0 else x0 + .02; cyl(g, (kx, .3, Dd + .016), .007, .01, gold, n=6, axis=(0, 0, 1))
    g.box(-W + .03, W - .03, .55, .81, Dd - .004, Dd + .001, (34, 16, 10))     # behind the open doors: the horn's mouth
    for k in range(7):                                                         # its wooden louvres
        x = -.18 + k*.06; g.box(x - .012, x + .012, .56, .8, Dd + .001, Dd + .006, (176, 114, 62))
    for y0, y1 in ((.54, .555), (.805, .82)): g.box(-W + .02, W - .02, y0, y1, Dd, Dd + .008, gold, spec=.6)
    op = math.radians(78)                                                      # the two small horn doors, swung open
    for sx in (-1, 1):
        hx = sx*(W - .02); dw = W - .026
        e = (hx - sx*dw*math.cos(op), Dd + dw*math.sin(op))
        q = [(hx, .54, Dd), (e[0], .54, e[1]), (e[0], .82, e[1]), (hx, .82, Dd)]
        g.face(q, mah_l, ref=(0, .68, 0), two=True)
        cyl(g, (lerp(hx, e[0], .85), .68, lerp(Dd, e[1], .85) + .006), .006, .01, gold, n=6, axis=(0, 0, 1))
    g.box(-W - .02, W + .02, .86, .89, -Dd - .02, Dd + .02, mah_d)             # moulding
    g.box(-W - .01, W + .01, .89, .99, -Dd - .01, Dd + .01, mah)               # motor-board compartment
    g.box(-W, W, .99, 1.0, -Dd, Dd, mah_d)
    ang = rec_angle(T_, 78.); cx, cz = -.04, .01
    cyl(g, (cx, 1.008, cz), .165, .012, felt, n=24)                            # green felt turntable
    S.prism(g, (cx, 1.016, cz), UP, .152, .004, 24, (18, 16, 18), (12, 12, 12))
    for r in (.09, .125): circle(g, (cx, cz), r, 1.0185, (70, 66, 70), 22)
    label(g, (cx, cz), .048, 1.019, ang, (70, 30, 20), gold)
    cyl(g, (cx, 1.024, cz), .004, .012, (210, 200, 170), n=6)
    pv = np.array([.18, 1.0, -.17])                                           # gold tonearm + Exhibition soundbox
    cyl(g, pv + np.array([0, .012, 0]), .03, .024, gold, n=12)
    a1 = pv + np.array([0, .05, 0]); a2 = np.array([.05, 1.08, .02]); a3 = np.array([.035, 1.075, .085])
    T.tube(g, pv + np.array([0, .024, 0]), a1, .014, col=gold, n=8, spec=.9)
    T.tube(g, a1, a2, .014, .011, col=gold, n=8, spec=.9); T.tube(g, a2, a3, .011, col=gold, n=8, spec=.9)
    sb = np.array([.03, 1.05, .1])
    cyl(g, sb, .032, .012, (196, 196, 202), top=(226, 226, 230), n=14, axis=unit((.3, .1, .95)))
    T.tube(g, sb - np.array([0, .02, 0]), sb - np.array([0, .03, 0]), .002, col=(200, 200, 206), n=4)
    lh = math.radians(40); hy, hz = 1.0, -Dd                                   # the domed lid, hinged at the back, open
    dv, hv = np.array([0, math.sin(lh), math.cos(lh)]), np.array([0, math.cos(lh), -math.sin(lh)])
    prof = [(0, 0), (0, .03), (.05, .05), (.47, .05), (.52, .03), (.53, 0)]
    for x0, x1, col in ((-W - .01, W + .01, mah_l),):
        pts = [np.array([0, hy, hz]) + dv*v + hv*h for v, h in prof]
        for i in range(len(pts) - 1):
            a, b = pts[i], pts[i + 1]
            g.face([(x0, a[1], a[2]), (x1, a[1], a[2]), (x1, b[1], b[2]), (x0, b[1], b[2])], col, ref=(0, hy + .1, hz + .3), two=True)
        g.face([(x0, p[1], p[2]) for p in pts], mah, ref=(0, hy + .2, hz + .2), two=True)
        g.face([(x1, p[1], p[2]) for p in pts], mah, ref=(0, hy + .2, hz + .2), two=True)
        a, b = pts[0], pts[-1]
        g.face([(x0, a[1], a[2]), (x1, a[1], a[2]), (x1, b[1], b[2]), (x0, b[1], b[2])], mah_d, ref=(0, hy + .3, hz - .3), two=True)
    T.tube(g, (W + .01, .95, .06), (W + .06, .95, .06), .006, col=(200, 200, 206), n=5, spec=.8)   # crank
    T.tube(g, (W + .06, .95, .06), (W + .06, .95 + .05*math.cos(ang*.2), .06 + .05*math.sin(ang*.2)), .005, col=(200, 200, 206), n=5)
    for age, m, v, dur, tp, cs in ev:
        if m >= 12 and tp and age < .7: fx.append(("wave", (.12, .68, Dd + .05), age, 0))
    notes_fx(fx, ev, (0, 1.2, .05))

# ---------------------------------------------------------------- 1948: Columbia / Philco LP player (brown bakelite, ribbed, tan arm)
def lp_player(g, t, ev, fx, s):
    T_ = s["t0"] + t
    bak, bak_d, bak_l, tan, mat = (84, 46, 28), (58, 30, 18), (112, 64, 40), (206, 176, 128), (176, 136, 96)
    W, Dd, H = .16, .15, .115
    prof = [(Dd, .008), (Dd, H - .02), (Dd - .02, H), (-Dd, H), (-Dd, .008)]
    pr = np.asarray(prof, float)
    for i in range(len(pr)):
        (z0, y0), (z1, y1) = pr[i], pr[(i + 1) % len(pr)]
        g.face([(-W, y0, z0), (W, y0, z0), (W, y1, z1), (-W, y1, z1)], bak, ref=(0, H/2, 0), spec=.25)
    for x in (-W, W): g.face([(x, y, z) for z, y in prof], bak, ref=(0, H/2, 0), spec=.25)
    for k in range(7):                                                          # moulded ribs
        y = .018 + k*.012
        g.line((-W + .01, y, Dd + .001), (W - .01, y, Dd + .001), bak_l)
        for x in (-W - .001, W + .001): g.line((x, y, Dd - .01), (x, y, -Dd + .01), bak_l)
    for x in (-W + .03, W - .03):
        for z in (-Dd + .03, Dd - .03): g.box(x - .015, x + .015, 0, .008, z - .015, z + .015, (20, 14, 10))
    g.box(-W + .012, W - .012, H, H + .002, -Dd + .012, Dd - .025, bak_d)
    ang = rec_angle(T_, 33.3); cx, cz = -.03, .0
    cyl(g, (cx, H + .008, cz), .1, .012, mat, top=mat, n=24)
    S.prism(g, (cx, H + .016, cz), UP, .127, .004, 26, (20, 18, 20), (14, 14, 14))     # 10-inch LP, overhanging the mat
    for r in (.07, .1): circle(g, (cx, cz), r, H + .0185, (74, 70, 74), 22)
    label(g, (cx, cz), .045, H + .019, ang, (190, 36, 36), (236, 224, 196))
    cyl(g, (cx, H + .024, cz), .0035, .012, (210, 210, 214), n=6)
    pv = (.118, -.085); cyl(g, (pv[0], H + .014, pv[1]), .024, .026, tan, n=10)
    path = [(pv[0], pv[1]), (.122, -.02), (.1, .04), (.055, .075)]
    yv = H + .03
    for (x0, z0), (x1, z1) in zip(path, path[1:]):
        w0 = .02 if x0 == pv[0] else .014
        obox(g, ((x0 + x1)/2, (z0 + z1)/2), (x1 - x0, z1 - z0), math.hypot(x1 - x0, z1 - z0) + .008, w0, yv - .006, yv + .006, tan, spec=.35)
    obox(g, (.045, .082), (-.8, .4), .03, .018, H + .02, yv, tan, spec=.35)
    cyl(g, (.12, .06, Dd + .006), .014, .012, bak_d, top=tan, n=10, axis=(0, 0, 1))
    lh = math.radians(80); hy, hz = H, -Dd                                       # lid hinged up at the back
    dv, hv = np.array([0, math.sin(lh), math.cos(lh)]), np.array([0, math.cos(lh), -math.sin(lh)])
    L_ = 2*Dd - .01
    cs = [np.array([x, hy, hz]) + dv*v + hv*h for x in (-W, W) for v in (0, L_) for h in (0, .035)]
    o = lambda i: tuple(cs[i])
    for idx, col in (((0, 4, 5, 1), bak_d), ((2, 6, 7, 3), bak_d), ((0, 1, 3, 2), bak_l), ((4, 5, 7, 6), bak),
                     ((1, 5, 7, 3), bak), ((0, 4, 6, 2), bak)):
        g.face([o(i) for i in idx], col, ref=tuple(np.mean(cs, 0)), spec=.2)
    notes_fx(fx, ev, (cx + .08, H + .16, .05))

# ---------------------------------------------------------------- 1950s: Jamaican sound system ("House of Joy" boxes, a deck and a valve amp)
def jamaica(g, t, ev, fx, s, pulse=0.):
    T_ = s["t0"] + t
    ply, ply_d, gold, red, cone, blk = (36, 104, 58), (22, 70, 38), (226, 184, 52), (196, 48, 36), (40, 34, 30), (16, 14, 12)
    lo = [e for e in ev if e[1] < 57 and e[1] >= 12]
    thump = max([math.exp(-e[0]*9)*e[2] for e in lo] + [0.])
    for sx in (-1, 1):                                                          # two wardrobe-sized speaker boxes
        x0, x1 = sorted((sx*.36, sx*.8)); z0, z1 = -.32, .06
        g.box(x0, x1, 0, 1.36, z0, z1, ply)
        g.box(x0 - .01, x1 + .01, 1.28, 1.36, z0 - .01, z1 + .01, gold)          # painted bands
        g.box(x0 - .005, x1 + .005, .02, .07, z0 - .005, z1 + .005, red)
        g.box(x0, x1, 1.2, 1.24, z1, z1 + .004, red)
        cx_ = (x0 + x1)/2
        for cy_, r in ((.44, .17), (.9, .17)):
            g.face(S.ring((cx_, cy_, z1 + .003), (0, 0, 1), r + .02, 16), ply_d, ref=(cx_, cy_, z0))
            g.face(S.ring((cx_, cy_, z1 + .005), (0, 0, 1), r, 16), blk, ref=(cx_, cy_, z0))
            push = .03*thump
            T.tube(g, (cx_, cy_, z1 + .004), (cx_, cy_, z1 + .03 + push), r*.94, r*.32, col=cone, n=14, spec=.1, two=True)
            cyl(g, (cx_, cy_, z1 + .035 + push), r*.3, .01, (60, 52, 46), n=10, axis=(0, 0, 1))
        for k in range(3): g.box(cx_ - .14 + .1*k, cx_ - .1 + .1*k, 1.1, 1.16, z1, z1 + .004, (14, 30, 18))   # port slots
    tw = (122, 82, 48)                                                           # the selector's table
    g.box(-.32, .32, .5, .53, -.22, .2, tw)
    for x in (-.29, .29):
        for z in (-.19, .17): g.box(x - .015, x + .015, 0, .5, z - .015, z + .015, dim(tw, .8))
    ox, oz, sc = -.1, -.01, .7                                                  # 1950s deck: cream plinth, steel platter
    def Lp(x, y, z): return (ox + x*sc, .53 + y*sc, oz + z*sc)
    g.box(*[v for p in zip(Lp(-.2, 0, -.17), Lp(.2, .07, .17)) for v in p], (206, 200, 182))
    ang = rec_angle(T_, 45.); pc = Lp(-.04, .082, 0)
    cyl(g, pc, .15*sc, .024*sc, (168, 170, 176), n=24)
    S.prism(g, Lp(-.04, .097, 0), UP, .13*sc, .004*sc, 24, (18, 18, 20), (12, 12, 12))
    label(g, (pc[0], pc[2]), .045*sc, Lp(0, .0995, 0)[1], ang, gold, (30, 110, 60), sc)
    tonearm(g, Lp, (.14, -.1), (.03, .07), .07, .125, .097, (150, 150, 156), (110, 110, 114), (120, 120, 126), sc, head=(80, 80, 84))
    g.box(.1, .29, .53, .65, -.12, .12, (52, 50, 48))                             # valve amp
    for k in range(4):
        cyl(g, (.13 + .044*k, .68, -.04), .014, .06, (255, 150, 60), n=8, emit=True)
    for k in range(3): cyl(g, (.14 + .055*k, .59, .123), .012, .01, (226, 220, 200), n=8, axis=(0, 0, 1))
    g.line((-.06, .65, -.06), (.02, .9, -.3), (30, 30, 30))
    notes_fx(fx, ev, (0, 1.1, .1))

# ---------------------------------------------------------------- 1972: the original Technics SL-1200 (champagne silver, rotary pitch knob, dust cover)
def sl1200(g, t, ev, fx, s):
    T_ = s["t0"] + t; deck(g, T_, DECK["sl1200"], fx=fx, ev=ev)

# ---------------------------------------------------------------- 1973: Kool Herc's two decks on a folding table + Shure Vocal Master columns
def herc(g, t, ev, fx, s, pulse=0.):
    T_ = s["t0"] + t
    top, leg = (130, 86, 54), (176, 150, 96)
    g.box(-.64, .64, .7, .73, -.26, .26, top)
    rr = np.random.default_rng(73)
    for k in range(40):                                                          # the mottled table top
        x, z = rr.uniform(-.6, .6), rr.uniform(-.23, .23); g.box(x - .02, x + .02, .73, .7305, z - .012, z + .012, (168, 120, 80))
    for sx in (-1, 1):
        for sz in (-1, 1):
            T.tube(g, (sx*.58, .7, sz*.22), (sx*.6, 0, sz*.18), .012, col=leg, n=5, spec=.5)
        T.tube(g, (sx*.59, .3, -.2), (sx*.59, .3, .2), .008, col=leg, n=4, spec=.5)
    tbl = dict(plinth=(40, 38, 36), edge=(26, 24, 22), platter=(170, 170, 176), ptop=(34, 34, 34), dots=(110, 110, 116),
               mat="felt", rpm=33.3, arm=(190, 190, 196), arm2=(60, 60, 62), cw=(140, 140, 146), pitch="none", strobe=False, target=False)
    deck(g, T_, dict(tbl, mat_col=(40, 170, 160), lab=((240, 210, 60), (200, 50, 40))), ox=-.37, y0=.73, sc=1.)
    deck(g, T_ + .3, dict(tbl, mat_col=(24, 24, 26), lab=((60, 120, 220), (230, 230, 220))), ox=.37, y0=.73, sc=1.)
    g.box(-.11, .11, .73, .82, -.15, .15, (24, 24, 26))                          # mixer with wooden cheeks
    for x in (-.115, .115): g.box(x - .008, x + .008, .73, .84, -.16, .16, (130, 80, 40))
    for k in range(6):
        for r_ in range(2): cyl(g, (-.075 + .03*k, .825, -.09 + .07*r_), .008, .01, (210, 210, 214), n=6)
    for x in (-.04, .04): g.box(x - .006, x + .006, .82, .83, .03, .12, (200, 200, 204))
    g.box(-.13, .13, .02, .18, -.1, .1, (190, 190, 194))                         # amp under the table, blue meters
    for x in (-.06, .06): g.box(x - .045, x + .045, .09, .15, .1, .104, (60, 150, 255), emit=True)
    lo = [e for e in ev if e[1] >= 12]
    thump = max([math.exp(-e[0]*10)*e[2] for e in lo] + [0.])
    for sx in (-1, 1):                                                           # Vocal Master columns
        cx_, cz_ = sx*.82, -.12
        g.box(cx_ - .12, cx_ + .12, 0, 1.36, cz_ - .1, cz_ + .1, (24, 24, 26))
        g.box(cx_ - .12, cx_ + .12, 1.33, 1.36, cz_ - .1, cz_ + .102, (186, 188, 194), spec=.5)
        g.box(cx_ - .12, cx_ + .12, 0, .03, cz_ - .1, cz_ + .102, (186, 188, 194), spec=.5)
        for x in (cx_ - .04, cx_ + .04): g.box(x - .008, x + .008, .03, 1.33, cz_ + .1, cz_ + .106, (196, 198, 204), spec=.6)
        g.box(cx_ - .11, cx_ + .11, .05 + .02*thump, 1.31, cz_ + .101, cz_ + .102, mix((30, 30, 34), (60, 60, 70), thump))
    notes_fx(fx, ev, (0, 1.05, .1))

# ---------------------------------------------------------------- 1975: the first scratch (round-cornered silver deck, felt slipmat, a hand, a mixer)
def scratch(g, t, ev, fx, s):
    T_ = s["t0"] + t
    cfg = dict(plinth=(200, 200, 204), edge=(150, 150, 156), platter=(180, 180, 186), ptop=(40, 40, 42), dots=(100, 100, 106),
               mat="felt", mat_col=(238, 228, 196), rec=(18, 18, 20), lab=((252, 214, 40), (110, 40, 150)), arm=(210, 210, 214),
               arm2=(60, 60, 64), cw=(160, 160, 166), pitch="none", strobe=False, target=False, rpm=33.3, rounded=True)
    Lp, ry, tip = deck(g, T_, cfg, ox=-.11, hand=1., sleeve=(110, 40, 150), fx=fx, ev=ev)
    mx0 = .15                                                                    # two-channel mixer, crossfader follows the cuts
    g.box(mx0, mx0 + .19, 0, .075, -.15, .15, (30, 30, 32))
    slab_top = [(mx0, .075, -.15), (mx0 + .19, .075, -.15), (mx0 + .19, .09, .1), (mx0, .09, .1)]
    for k in range(2):
        for r_ in range(3): cyl(g, (mx0 + .055 + .08*k, .088 - .006*r_ + .004, -.1 + .045*r_), .009, .012, (220, 220, 224), n=6)
        g.box(mx0 + .05 + .08*k, mx0 + .06 + .08*k, .085, .09, .04, .09, (14, 14, 16))
    j = jog(T_); gate = j[1] if j else 1.
    g.box(mx0 + .02, mx0 + .17, .076, .078, .12, .135, (14, 14, 16))
    xf = mx0 + lerp(.05, .14, gate)
    g.box(xf - .012, xf + .012, .076, .09, .118, .137, (240, 230, 90), emit=False)
    for age, m, v, dur, tp, cs in ev:
        if 1 <= m <= 8 and age < dur*BEAT_S + .1: fx.append(("strum", tuple(np.array(tip) + np.array([0, .01, 0])), tuple(np.array(tip) + np.array([-.1, .05, .02])), (age*7) % .15))

# ---------------------------------------------------------------- 1976: disco booth deck playing a 12-inch, its plain sleeve leaning beside it
def disco(g, t, ev, fx, s, pulse=0.):
    T_ = s["t0"] + t
    wal, chrome, hot = (70, 42, 30), (150, 152, 160), (255, 108, 30)
    H = .5
    g.box(-.42, .42, 0, H, -.24, .22, wal)
    g.box(-.44, .44, H, H + .03, -.26, .24, chrome, spec=.4)
    beat = math.exp(-((T_ % BEAT_S))*8)
    for k in range(3):                                                           # booth light strips, chasing on the beat
        y = .08 + .13*k
        on = (int(T_/BEAT_S) + k) % 3 == 0
        g.box(-.38, .38, y, y + .05, .22, .226, mix(dim(hot, .4), hot, beat) if on else dim(hot, .35), emit=True)
        g.box(-.4, .4, y + .065, y + .075, .22, .225, chrome, spec=.5)
    cfg = dict(plinth=(30, 30, 32), edge=(20, 20, 22), platter=(186, 186, 192), ptop=(36, 36, 36), dots=(110, 110, 116),
               mat="felt", mat_col=(26, 26, 28), rec=(16, 16, 18), lab=(hot, (240, 236, 226)), arm=(200, 200, 206), arm2=(56, 56, 58),
               cw=(150, 150, 156), pitch="slider", strobe=False, target=False, rpm=45., led=None)
    deck(g, T_, cfg, ox=-.1, oz=-.02, y0=H + .03, sc=1.05, fx=fx, ev=ev)
    for x in (.24, .34): cyl(g, (x, H + .05, -.08), .03, .03, (40, 40, 44), n=8)   # a pair of cans on the console
    T.tube(g, (.24, H + .08, -.08), (.34, H + .08, -.08), .006, col=(40, 40, 44), n=5)
    # the 12-inch's plain die-cut sleeve, leaning against the booth front
    sw = .31; bx, bz = .26, .33; lean = math.asin((bz - .228)/sw)
    base = np.array([bx, 0, bz]); up_ = np.array([0, math.cos(lean), -math.sin(lean)])
    cs = [base + np.array([-sw/2, 0, 0]), base + np.array([sw/2, 0, 0]), base + np.array([sw/2, 0, 0]) + up_*sw, base + np.array([-sw/2, 0, 0]) + up_*sw]
    nrm = unit(np.cross(cs[1] - cs[0], cs[3] - cs[0]))
    if nrm[2] < 0: nrm = -nrm
    off = nrm*.006
    g.face([tuple(p + off) for p in cs], (226, 224, 214), ref=tuple(base + up_*.15 - nrm))
    g.face([tuple(p) for p in cs], (200, 198, 188), ref=tuple(base + up_*.15 + nrm))
    for i in range(4): g.face([tuple(cs[i]), tuple(cs[(i + 1) % 4]), tuple(cs[(i + 1) % 4] + off), tuple(cs[i] + off)], (190, 186, 176), ref=tuple(base + up_*.15))
    hc = base + up_*sw/2 + nrm*.0075
    g.face(S.ring(hc, nrm, .05, 14), (28, 26, 26), ref=tuple(hc - nrm), emit=True)

# ---------------------------------------------------------------- 1979: Technics SL-1200MK2 (pitch slider, strobe, target light)
def mk2(g, t, ev, fx, s):
    T_ = s["t0"] + t; deck(g, T_, DECK["mk2"], hand=hand_k(T_), sleeve=(54, 54, 64), fx=fx, ev=ev)

# ---------------------------------------------------------------- 1994: Pioneer CDJ-500 (top-loading CD, display, big jog dial)
def cdj(g, t, ev, fx, s):
    T_ = s["t0"] + t
    blk, blk_l, edge = (28, 28, 30), (44, 44, 48), (18, 18, 20)
    W, Dd, H = .16, .18, .1
    for x in (-W + .03, W - .03):
        for z in (-Dd + .03, Dd - .03): cyl(g, (x, .006, z), .015, .012, (14, 14, 14), n=8)
    P.prism(g, [(-W, -Dd), (W, -Dd), (W, Dd), (-W, Dd)], .012, H, edge, blk)
    g.box(-W, W, H, H + .012, -Dd, -.03, blk_l)                                 # raised rear deck with the CD well
    cc = (-.0, -.105)
    g.face(S.ring((cc[0], H + .0125, cc[1]), UP, .068, 20), (10, 10, 12), ref=(cc[0], 0, cc[1]))
    spin = playpos(T_)*2*math.pi*5.5                                             # the CD, spinning (rainbow sheen)
    rb = [(200, 200, 210), (170, 200, 230), (210, 180, 230), (230, 220, 160), (180, 230, 200), (220, 190, 200)]
    for k in range(12):
        a0, a1 = spin + 2*math.pi*k/12, spin + 2*math.pi*(k + 1)/12
        y = H + .013
        pts = [(cc[0] + .012*math.cos(a0), y, cc[1] + .012*math.sin(a0)), (cc[0] + .06*math.cos(a0), y, cc[1] + .06*math.sin(a0)),
               (cc[0] + .06*math.cos(a1), y, cc[1] + .06*math.sin(a1)), (cc[0] + .012*math.cos(a1), y, cc[1] + .012*math.sin(a1))]
        g.face(pts, rb[k % 6], ref=(cc[0], 0, cc[1]), spec=.6)
    g.face(S.ring((cc[0], H + .0135, cc[1]), UP, .011, 8), (60, 60, 64), ref=(cc[0], 0, cc[1]))
    lh = math.radians(70); hy, hz = H + .012, -Dd                               # the lid, flipped up
    dv, hv = np.array([0, math.sin(lh), math.cos(lh)]), np.array([0, math.cos(lh), -math.sin(lh)])
    cs = [np.array([x, hy, hz]) + dv*v + hv*h for x in (-.1, .1) for v in (0, .15) for h in (0, .01)]
    for idx, col in (((0, 1, 3, 2), (36, 36, 40)), ((4, 5, 7, 6), blk_l), ((0, 4, 5, 1), edge), ((2, 6, 7, 3), edge), ((1, 5, 7, 3), edge), ((0, 4, 6, 2), edge)):
        g.face([tuple(cs[i]) for i in idx], col, ref=tuple(np.mean(cs, 0)))
    g.box(-.12, .1, H, H + .001, -.025, .008, (20, 12, 6))                      # amber fluorescent display + digits
    tsec = int((T_ - s["t0"])*10)
    for k in range(5):
        on = (tsec // (1, 1, 10, 100, 600)[k]) % 10
        x = -.1 + k*.03 + (.01 if k > 2 else 0)
        for j in range(3):
            if (on + j) % 3 != 1: g.box(x + .002*j, x + .014, H + .001, H + .0016, -.017 + .009*j, -.013 + .009*j, (255, 160, 50), emit=True)
    j = jog(T_); playing = j is None
    g.box(-.11, .11, H, H + .0012, .016, .02, (60, 60, 64))
    for k, (z, col) in enumerate(((.075, (120, 230, 80)), (.135, (255, 150, 40)))):   # PLAY / CUE
        lit = (k == 0 and playing) or (k == 1 and not playing)
        cyl(g, (-.12, H + .006, z), .019, .012, (50, 50, 54), top=col if lit else dim(col, .45), n=12, emit=lit)
    for k in range(4): g.box(-.13 + .016*k, -.12 + .016*k, H, H + .005, .035, .045, (70, 70, 76))
    jg = playpos(T_)*2.4                                                         # jog dial
    cyl_dots(g, (.0, H + .009, .095), .075, .018, 28, jg, (20, 20, 22), (34, 34, 38), (60, 60, 66))
    cyl(g, (.0, H + .019, .095), .058, .003, (40, 40, 44), n=20)
    cyl(g, (.042*math.cos(jg), H + .021, .095 + .042*math.sin(jg)), .009, .003, (14, 14, 14), n=8)
    g.box(.125, .137, H, H + .002, .02, .16, (12, 12, 14))                       # pitch slider
    g.box(.118, .144, H, H + .012, .085, .1, (60, 60, 64))
    notes_fx(fx, ev, (cc[0], H + .14, cc[1] + .02), (cc[0], H + .03, cc[1]))

# ---------------------------------------------------------------- 2001: Final Scratch (two decks with timecode records, the ScratchAmp, a laptop)
def fs(g, t, ev, fx, s):
    T_ = s["t0"] + t
    cfg = dict(plinth=(176, 180, 188), edge=(120, 124, 132), platter=(190, 192, 198), ptop=(36, 36, 38), dots=(100, 100, 106),
               mat="felt", mat_col=(30, 30, 34), rec=(16, 16, 18), lab=((230, 232, 236), (60, 140, 255)), arm=(210, 210, 214),
               arm2=(56, 56, 60), cw=(160, 160, 166), pitch="slider", strobe=False, target=True, rpm=33.3, timecode=True, led=None)
    deck(g, T_, cfg, ox=-.34, sc=.98, fx=fx, ev=ev)
    deck(g, T_ + .4, cfg, ox=.34, sc=.98)
    alu = (186, 192, 202)                                                        # ScratchAmp: three rounded lobes
    lobes = []
    for k in range(3):
        a = math.radians(90 + 120*k)
        for j in range(5):
            b = a - math.radians(45) + math.radians(90)*j/4
            lobes.append((.065*math.cos(a) + .035*math.cos(b), .2 + .065*math.sin(a)*.0 + .065*math.sin(a) + .035*math.sin(b)))
    cxz = np.mean(lobes, 0); lobes = [(x - cxz[0], z - cxz[1] + .21) for x, z in lobes]
    P.prism(g, lobes, 0, .045, dim(alu, .8), alu, spec=.5)
    g.face(S.ring((0, .0455, .21), UP, .012, 8), (60, 150, 255), ref=(0, 0, .21), emit=True)
    for k in range(6):
        a = math.radians(30 + 60*k); x, z = .072*math.cos(a), .21 + .072*math.sin(a)
        cyl(g, (x, .022, z), .007, .012, (220, 50, 40) if k % 2 else (230, 230, 230), n=6, axis=(math.cos(a), 0, math.sin(a)))
    g.box(-.16, .16, 0, .02, -.36, -.14, (196, 200, 208), spec=.3)               # laptop
    la = math.radians(105); hz = -.36
    sv = np.array([0, math.sin(la), math.cos(la)])
    scr = [np.array([x, .02, hz]) + sv*v for x in (-.16, .16) for v in (0, .21)]
    g.face([tuple(scr[i]) for i in (0, 1, 3, 2)], (196, 200, 208), ref=(0, .1, hz - .2), spec=.3)
    inner = [np.array([x, .02, hz + .002]) + sv*v for x in (-.145, .145) for v in (.015, .195)]
    g.face([tuple(inner[i]) for i in (0, 1, 3, 2)], (10, 16, 40), ref=(0, .1, hz - .2), emit=True)
    nrm = np.array([0, -sv[2], sv[1]]) * -1
    for lane in range(2):                                                        # two scrolling waveforms
        v0 = .06 + .08*lane
        for k in range(24):
            x = -.13 + .011*k; amp = .02*abs(math.sin(k*1.7 + T_*(9 + 3*lane) + lane))
            p = np.array([x, .02, hz + .004]) + sv*v0
            g.line(tuple(p - sv*amp), tuple(p + sv*amp), (80, 170, 255) if lane == 0 else (120, 220, 255))
    g.line(tuple(np.array([-.003, .02, hz + .005]) + sv*.03), tuple(np.array([-.003, .02, hz + .005]) + sv*.18), (255, 255, 255))

BUILD = {"modern": mk7, "edison": edison, "berliner": berliner, "victrola": victrola, "lp": lp_player, "jamaica": jamaica,
         "sl1200": sl1200, "herc": herc, "scratch": scratch, "disco": disco, "mk2": mk2, "cdj": cdj, "fs": fs}
def build(g, t, ev, fx, s): BUILD[s["key"]](g, t, ev, fx, s)

# ---------------------------------------------------------------- background: on a giant spinning record
THEMES = {   # record surface, label, label 2, accent (glints, spots), sky top, sky bottom, label art
 "modern":   ((16, 16, 20), (24, 22, 26), (40, 36, 30), (236, 190, 80), (6, 6, 10), (40, 30, 16), "rings"),
 "edison":   ((150, 154, 162), (178, 138, 62), (116, 86, 40), (244, 210, 130), (22, 20, 22), (92, 82, 66), "sunburst"),
 "berliner": ((64, 40, 28), (232, 218, 186), (126, 84, 52), (246, 234, 204), (18, 12, 8), (84, 58, 40), "arcs"),
 "victrola": ((58, 20, 14), (206, 160, 72), (96, 36, 22), (250, 212, 120), (16, 6, 4), (88, 38, 24), "dots"),
 "lp":       ((20, 18, 20), (188, 34, 36), (238, 226, 198), (240, 228, 200), (14, 8, 10), (76, 32, 36), "half"),
 "jamaica":  ((14, 42, 24), (224, 180, 44), (24, 112, 58), (255, 140, 58), (46, 18, 56), (226, 100, 48), "quad"),
 "sl1200":   ((70, 38, 20), (228, 122, 40), (122, 66, 30), (255, 160, 70), (22, 10, 6), (116, 54, 22), "bands"),
 "herc":     ((86, 34, 26), (40, 192, 180), (152, 62, 44), (70, 224, 204), (16, 10, 10), (96, 42, 34), "tag"),
 "scratch":  ((52, 20, 78), (252, 216, 44), (100, 42, 146), (255, 228, 84), (14, 6, 26), (74, 30, 104), "zigzag"),
 "disco":    ((150, 154, 168), (255, 106, 30), (204, 208, 218), (255, 142, 60), (24, 14, 10), (118, 62, 40), "rays"),
 "mk2":      ((122, 126, 136), (224, 42, 40), (194, 198, 206), (255, 74, 62), (12, 12, 16), (74, 76, 86), "ringdot"),
 "cdj":      ((42, 72, 10), (176, 244, 44), (114, 42, 224), (176, 110, 255), (16, 4, 34), (64, 22, 118), "checker"),
 "fs":       ((118, 124, 134), (204, 210, 218), (60, 140, 255), (90, 166, 255), (6, 10, 22), (40, 56, 84), "ticks"),
}
def ped_cols(th):
    T_ = THEMES[th]; return F.ped_cols_from(mix(mix(T_[5], (34, 34, 40), .45), T_[3], .1))
def mv(a, b, k): return np.asarray(a, float)*(1 - k) + np.asarray(b, float)*k

H_ = .045*DIST; Y_REC = -2*H_
_CAM0 = P.Cam(DIST, 0.)
_dx = (XX + .5 - CX)/P.F_PX; _dy = (YY + .5 - P.CY)/P.F_PX
_DIR = _CAM0.fwd[None, None] + _dx[..., None]*_CAM0.right[None, None] - _dy[..., None]*_CAM0.up[None, None]
_down = _DIR[..., 1] < -1e-6
_tt = np.where(_down, (Y_REC - _CAM0.eye[1])/np.where(_down, _DIR[..., 1], -1.), 0.)
_PX = _CAM0.eye[0] + _tt*_DIR[..., 0]; _PZ = _CAM0.eye[2] + _tt*_DIR[..., 2]
R_MAP = np.where(_down, np.hypot(_PX, _PZ), np.inf); TH_MAP = np.arctan2(_PZ, _PX)
R_LAB, R_RUN, R_GRV, R_REC, R_PLT = 1.3, 1.42, 4.84, 5.0, 5.42
def _idx(m): return np.nonzero(m.ravel())[0]
I_LAB, I_RUN, I_GRV, I_EDGE, I_PLT = (_idx(R_MAP < R_LAB), _idx((R_MAP >= R_LAB) & (R_MAP < R_RUN)), _idx((R_MAP >= R_RUN) & (R_MAP < R_GRV)),
                                      _idx((R_MAP >= R_GRV) & (R_MAP < R_REC)), _idx((R_MAP >= R_REC) & (R_MAP < R_PLT)))
_R, _TH, _B = R_MAP.ravel(), TH_MAP.ravel(), BAY.ravel()
def _wrap(a): return (a + math.pi) % (2*math.pi) - math.pi
_rg = _R[I_GRV]
G_TEX = 1 + .055*np.where(((_rg/.075) + _B[I_GRV]*.35) % 1 < .5, 1, -1)
for _r0 in (2.42, 3.12, 3.86, 4.42): G_TEX = np.where(np.abs(_rg - _r0) < .045, 1.18, G_TEX)
SHEEN = (np.exp(-(_wrap(_TH[I_GRV] + math.pi/2)/.3)**2)*(1 - (_rg - R_RUN)/(R_GRV - R_RUN))*.9 +
         np.exp(-(_wrap(_TH[I_GRV] - math.pi/2)/.22)**2)*.55)
SHEEN = np.floor(SHEEN*4 + _B[I_GRV]*.999)/4
LAB_RN, LAB_TH = _R[I_LAB]/R_LAB, _TH[I_LAB]
SKY = {k: F.make_grad(v[4], v[5], K=8, y0=96, y1=200) for k, v in THEMES.items()}
M_TXT = ((YY >= 38) & (YY < 102)) | ((YY >= 330) & (YY < 382))
M_TXTE = (((YY >= 34) & (YY < 38)) | ((YY >= 102) & (YY < 106)) | ((YY >= 326) & (YY < 330)) | ((YY >= 382) & (YY < 386))) & (BAY < .5)
BALL_C, BALL_R = (24, 122), 11
_by, _bx = np.mgrid[-BALL_R:BALL_R + 1, -BALL_R:BALL_R + 1]
_bm = _bx**2 + _by**2 <= BALL_R**2
B_X, B_Y = _bx[_bm] + BALL_C[0], _by[_bm] + BALL_C[1]
_nx, _ny = _bx[_bm]/BALL_R, _by[_bm]/BALL_R; _nz = np.sqrt(np.clip(1 - _nx**2 - _ny**2, 0, 1))
B_LON, B_LAT, B_SH = np.arctan2(_nx, _nz), np.arcsin(np.clip(_ny, -1, 1)), .45 + .55*np.clip(_nz*.8 - _ny*.4 - _nx*.3, 0, 1)
_sr = np.random.default_rng(1977)
SPOTS = [(_sr.random(), _sr.random(), _sr.random()*6.28, _sr.random()) for _ in range(54)]
SPECK = [(_sr.uniform(1.9, 4.7), _sr.uniform(0, 6.28)) for _ in range(26)]
def proj0(pts):
    S_, d = _CAM0.proj(np.asarray(pts, float)); return S_, d

def label_art(kind, rn, th, c1, c2, acc):
    """label colours for label pixels (rn 0..1 across the label, th already rotated)"""
    n = len(rn); col = np.tile(np.asarray(c1, float), (n, 1))
    def put(m, c): col[m] = c
    a01 = (th % (2*math.pi))/(2*math.pi)
    if kind == "rings":
        put((np.abs(rn - .72) < .025) | (np.abs(rn - .93) < .02), acc); put(((a01*3) % 1 < .16) & (rn > .78) & (rn < .88), acc)
    elif kind == "sunburst": put(((a01*28) % 1 < .5) & (rn > .66), c2); put(np.abs(rn - .95) < .03, acc)
    elif kind == "arcs": put((rn > .74) & (rn < .86) & ((a01 % .5) < .3), c2); put(np.abs(rn - .95) < .02, c2)
    elif kind == "dots":
        put(np.abs(rn - .7) < .02, acc); put(np.abs(rn - .95) < .025, acc)
        put((np.abs(rn - .83) < .05) & (((a01*4) % 1) < .07), acc)
    elif kind == "half": put(a01 < .5, c2); put(np.abs(rn - .8) < .04, acc*.0 + np.asarray(c1)*.6)
    elif kind == "quad": put(((a01*4).astype(int) % 2) == 1, c2); put(np.abs(rn - .94) < .04, acc); put(np.abs(rn - .7) < .02, (20, 20, 20))
    elif kind == "bands": put(((rn*9).astype(int) % 2) == 1, c2)
    elif kind == "tag": put(np.abs(rn - (.8 + .08*np.sin(th*5))) < .045, c2); put(np.abs(rn - .96) < .02, acc)
    elif kind == "zigzag": put(np.abs(rn - (.82 + .07*(np.abs((a01*16) % 2 - 1) - .5)*2)) < .04, c2); put(np.abs(rn - .68) < .02, c2)
    elif kind == "rays": put(((a01*12) % 1 < .45) & (rn > .64), c2)
    elif kind == "ringdot": put(np.abs(rn - .78) < .05, c2); put((np.abs(rn - .9) < .05) & (a01 < .06), c2)
    elif kind == "checker": put(((rn*5).astype(int) + (a01*10).astype(int)) % 2 == 1, c2)
    elif kind == "ticks": put((np.abs(rn - .86) < .04) & ((a01*24) % 1 < .35), c2); put(np.abs(rn - .72) < .015, c2)
    col[rn < .05] = (20, 20, 22)
    return col

def tonearm_angle(T):
    return 2.62 + .1*math.sin(2*math.pi*T/22) - .05*clamp(T/4)

def draw_tonearm(d, T, acc, pulse):
    piv = np.array([3.2, Y_REC, -6.2]); top = np.array([3.2, .56, -6.2]); L = 5.3; a = tonearm_angle(T)
    u = np.array([math.cos(a), 0, math.sin(a)]); nrm = np.array([-u[2], 0, u[0]])
    S0, _ = proj0([piv + np.array([.55*math.cos(k*math.pi/8), 0, .55*math.sin(k*math.pi/8)]) for k in range(16)])
    d.polygon([tuple(p) for p in S0], fill=(54, 54, 60), outline=(20, 20, 24))
    def thick(a3, b3, w3, col, hi=None):
        Sa, da = proj0([a3, b3]); w0, w1 = w3*P.F_PX/da[0], w3*P.F_PX/da[1]
        v = Sa[1] - Sa[0]; ln = np.hypot(*v) + 1e-9; n2 = np.array([-v[1], v[0]])/ln
        poly = [Sa[0] + n2*w0, Sa[1] + n2*w1, Sa[1] - n2*w1, Sa[0] - n2*w0]
        d.polygon([tuple(p) for p in poly], fill=col, outline=(16, 16, 20))
        if hi: d.line([tuple(Sa[0] + n2*w0*.45), tuple(Sa[1] + n2*w1*.45)], fill=hi)
    thick(piv, top, .2, (70, 70, 78), (130, 130, 140))
    thick(top - u*.25, top - u*1.15, .26, (44, 44, 50), (96, 96, 104))          # counterweight
    pts = [top + u*L*k + nrm*o + np.array([0, -.08*k, 0]) for k, o in ((0, 0), (.4, .32), (.78, -.18), (1, 0))]
    for p0, p1 in zip(pts, pts[1:]): thick(p0, p1, .075, (176, 178, 186), (236, 238, 244))
    ha = a + .4; hu = np.array([math.cos(ha), 0, math.sin(ha)]); hn = np.array([-hu[2], 0, hu[0]])
    hc = pts[-1] + hu*.3
    sh = [hc - hu*.35 + hn*.16, hc + hu*.35 + hn*.2, hc + hu*.35 - hn*.2, hc - hu*.35 - hn*.16]
    S1, _ = proj0(sh); d.polygon([tuple(p) for p in S1], fill=(40, 40, 46), outline=(14, 14, 18))
    S2, _ = proj0([hc + hu*.1 + np.array([0, -.08, 0]), hc + hu*.1 + np.array([0, -.3, 0])])
    d.line([tuple(S2[0]), tuple(S2[1])], fill=mix((120, 120, 128), acc, pulse), width=2)
    S3, _ = proj0([hc + hu*.32 + hn*.2, hc + hu*.32 + hn*.5])
    d.line([tuple(S3[0]), tuple(S3[1])], fill=(150, 150, 158))

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    A_, B_ = THEMES[th0], THEMES[th1]
    def col(i): return mv(A_[i], B_[i], kb)
    rec, lab, lab2, acc = col(0), col(1), col(2), col(3)
    img = (SKY[th0]*(1 - kb) + SKY[th1]*kb).reshape(-1, 3).copy()
    pulse, accent = x["pulse"], x["accent"]
    j = jog(T); jq = j[0] if j else 0.
    rot = .32*T + 2.6*jq
    # grooves: base colour x groove texture + fixed sheen wedges + rings of light on every kick
    base = rec*G_TEX[:, None]
    shc = mv(mix(tuple(int(v) for v in rec), (255, 255, 255), .35), acc, .35)
    g_ = base + (SHEEN*(.55 + .25*accent))[:, None]*(shc - rec)[None]*.6
    for k in c.KICKS:
        age = T - k
        if 0 <= age < .75:
            rk = R_RUN + age*5.8; w = .05 + .05*age
            m = np.abs(_rg - rk) < w
            g_[m] = g_[m]*(1 - .6*(1 - age/.75)) + acc*.6*(1 - age/.75)
    img[I_GRV] = g_
    img[I_RUN] = mv(rec, shc, .18)
    img[I_EDGE] = mv(rec, (0, 0, 0), .35)
    pl = np.tile(np.array([170., 172, 178]), (len(I_PLT), 1))
    dots = ((((_TH[I_PLT] - rot*.6) % (2*math.pi))/(2*math.pi)*90) % 1 < .45) & (np.abs(_R[I_PLT] - 5.2) < .12)
    pl[dots] = (96, 98, 104); img[I_PLT] = pl
    # label art (rotating) with the era crossfade
    thr = LAB_TH - rot
    la = label_art(B_[6], LAB_RN, thr, B_[1], B_[2], np.asarray(B_[3], float))
    if kb < 1: la = la*kb + label_art(A_[6], LAB_RN, thr, A_[1], A_[2], np.asarray(A_[3], float))*(1 - kb)
    img[I_LAB] = la*(.92 + .08*pulse)
    img = img.reshape(RH, RW, 3)
    im = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)); d = ImageDraw.Draw(im)
    for r_, a_ in SPECK:                                                         # dust specks turning with the record
        a2 = a_ + rot
        Sx, dd = proj0([(r_*math.cos(a2), Y_REC, r_*math.sin(a2))])
        if dd[0] > .1 and 0 <= Sx[0][0] < RW and 0 <= Sx[0][1] < RH:
            d.point((int(Sx[0][0]), int(Sx[0][1])), fill=tuple(int(v) for v in mv(rec, (230, 230, 236), .55)))
    draw_tonearm(d, T, tuple(int(v) for v in acc), pulse)
    img = np.asarray(im).astype(float)
    # mirror ball + its light spots (new burst on every beat)
    spin = T*1.6
    fac = ((np.floor((B_LON + spin)/(math.pi/7)) + np.floor((B_LAT + 2)/(math.pi/9))) % 2)
    tw = ((np.floor((B_LON + spin)/(math.pi/7))*7 + np.floor((B_LAT + 2)/(math.pi/9))*3 + int(T/BEAT_S)) % 11 == 0)
    bc = mv((120, 124, 134), (210, 214, 224), fac[:, None])*B_SH[:, None]
    bc[tw] = mv((255, 255, 255), acc, .3)*(.6 + .4*pulse)
    img[B_Y, B_X] = bc
    img[0:BALL_C[1] - BALL_R, BALL_C[0]] = (70, 70, 76)
    bt = math.exp(-((T % BEAT_S))*5)
    for u, v, ph, sz in SPOTS:
        px = int(((u + T*.035*(1 + .4*v)) % 1)*RW); py = int(104 + v*(RH - 104))
        b = bt*(.45 + .55*abs(math.sin(ph + T*2.3)))
        if b < .08: continue
        w = 2 if sz < .6 else 3; h = 2 if py > 200 else w
        cc = mv((255, 255, 255), acc, .45)
        img[py:py + h, px:px + w] = img[py:py + h, px:px + w]*(1 - .7*b) + cc*.7*b
    img[M_TXT] *= .5; img[M_TXTE] *= .72
    return np.clip(img, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

"""Low-poly x pixel-art history of the cello -> 1080x1920 MP4.

Differences from the piano / violin videos:
  * opens on the modern cello (Bach Suite No. 1) before the history
  * new teal opening colour and a new audio-reactive background: halo, ripple rings on every
    note, equalizer bars driven by the real mix, motes that speed up with loudness
  * new beat: 100 BPM half-time hip-hop groove, swung 16th hats, claps, rims, 808 sub bass

usage:  python3 cello_lowpoly.py [out.mp4]        render the video
        python3 cello_lowpoly.py --still 3.1 7.5   write cstill_<t>.png frames
"""
import math, subprocess, sys, wave
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter, butter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import piano_lowpoly as P
import violin_lowpoly as V
from piano_lowpoly import (Geo, render_geo, outline, Cam, text_w, draw_text, center_text, mix, ease, ease_out_back,
                           clamp, dim, lerp, midi_hz, sprite, NOTE_SPR, SPARK, build_pedestal, kick, snare, hat,
                           crash, pad, B4, RW, RH, UP, FPS, SR, CREAM, GOLD, GOLD_D, INK, HOT, HOTSTR, CX)

OUT = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "cello_lowpoly.mp4"
BPM = 100; BEAT = 60 / BPM; BAR = 4 * BEAT
DIST, ISCALE = 3.0, .8
FLIP = V.FLIP

# ---------------------------------------------------------------- music + story
CH = {"G": (43, (55, 59, 62)), "C": (36, (55, 60, 64)), "Dm": (38, (57, 62, 65)), "D": (38, (57, 62, 66)),
      "A": (45, (57, 61, 64)), "Am": (45, (57, 60, 64)), "E": (40, (56, 59, 64)), "Bm": (47, (54, 59, 62)),
      "F#m": (42, (54, 57, 61))}
BARS = ["G", "C", "Dm", "C", "Dm", "A", "G", "D", "C", "G", "Am", "E",
        ("D", "A", "Bm", "F#m"), ("G", "D", "G", "A"), "D", "G"]
seq = V.seq
BACH = [43, 50, 59, 57, 59, 50, 59, 50]*2 + [43, 52, 60, 59, 60, 52, 60, 52]*2

SCENES = [
 dict(key="cmodern", theme="intro", intro=True, name="MODERN CELLO", credit="BACH: CELLO SUITE NO. 1",
      notes=seq(0, [(m, .25) for m in BACH], .8)),
 dict(key="cvielle", theme="cvielle", year=1200, ys="1200s", name="VIELLE", credit="IN THE STYLE OF A TROUBADOUR SONG",
      fact=("THE MEDIEVAL FIDDLE", "STARTS THE FAMILY"),
      notes=seq(0, [(62, .5), (65, .5), (67, .5), (69, 1), (67, .5), (65, .5), (64, .5), (67, 1), (64, .5), (62, .5), (64, 1), (60, 1)])),
 dict(key="gamba", theme="gamba", year=1490, ys="1490s", name="VIOLA DA GAMBA", credit="IN THE STYLE OF MARIN MARAIS",
      fact=("PLAYED BETWEEN THE", "LEGS: GAMBA = LEG"),
      notes=seq(0, [(62, 1.5), (60, .25), (58, .25), (57, 1), (53, .5), (55, .5), (57, .5), (58, .5), (61, 1), (64, .5), (61, .5), (57, 1)])),
 dict(key="bassv", theme="bassv", year=1550, ys="1550s", name="BASS VIOLIN", credit="IN THE STYLE OF PRAETORIUS",
      fact=("THE VIOLIN FAMILY", "GETS A BASS VOICE"),
      notes=seq(0, [(43, .5), (47, .5), (50, .5), (47, .5), (55, 1), (50, 1), (50, .5), (54, .5), (57, .5), (54, .5), (50, 1), (45, 1)])),
 dict(key="bologna", theme="bologna", year=1660, ys="1660s", name="VIOLONCELLO", credit="IN THE STYLE OF GABRIELLI",
      fact=("WIRE-WOUND STRINGS", "SHRINK THE CELLO"),
      notes=seq(0, [(m, .5) for m in [48, 52, 55, 60, 64, 60, 55, 52, 55, 59, 62, 67, 65, 62, 59, 55]])),
 dict(key="cstrad", theme="cstrad", year=1710, ys="1710s", name="STRADIVARI CELLO", credit="IN THE STYLE OF VIVALDI",
      fact=("STRADIVARI SETS", "THE CELLO'S SIZE"),
      notes=seq(0, [(57, .5), (64, .25), (64, .25), (64, .5), (60, .5), (69, 1), (67, .5), (65, .5),
                    (64, .5), (68, .25), (68, .25), (68, .5), (64, .5), (71, 1), (68, .5), (64, .5)], .85)),
 dict(key="cmodern", theme="intro", year=1850, ys="1850s", name="MODERN CELLO", credit="PACHELBEL: CANON IN D",
      fact=("THE ENDPIN LETS IT", "STAND ON THE FLOOR"),
      notes=seq(0, [(m, 1) for m in [62, 57, 59, 54, 55, 50, 55, 57]], .85) +
            seq(8, [(m, .25) for m in [50, 57, 66, 64, 66, 57, 66, 57]*2], .8) + [(12, [43, 50, 59, 67], 4.3, 1.0)]),
]
DRONES = {"cvielle": [50]}
for si, s in enumerate(SCENES):
    s["t0"] = 2*si*BAR
    s["ev"] = []
    for beat, ms, dur, v in s["notes"]:
        ml = list(ms) if isinstance(ms, (list, tuple)) else [ms]
        for m in ml: s["ev"].append((beat*BEAT, m, v, dur*BEAT, m == max(ml)))
_on = sorted({round(s["t0"] + e[0], 4) for s in SCENES for e in s["ev"] if e[4]})
ONSETS = []
for _o in _on:
    if not ONSETS or _o - ONSETS[-1] >= .28: ONSETS.append(_o)
OUTRO_T, FINAL_T = 14*BAR, 15*BAR
TOTAL = 15*BAR + 2.6

THEMES = {   # (sky top, sky bottom, accent)
 "intro":   ((4, 24, 30), (14, 92, 96), (100, 240, 220)),
 "cvielle": ((8, 12, 42), (34, 52, 124), (130, 160, 255)),
 "gamba":   ((26, 10, 34), (86, 34, 110), (210, 130, 240)),
 "bassv":   ((6, 26, 16), (24, 92, 56), (140, 230, 160)),
 "bologna": ((12, 16, 24), (58, 70, 90), (180, 210, 236)),
 "cstrad":  ((30, 6, 26), (110, 24, 90), (255, 120, 220)),
}
YY, XX = np.mgrid[0:RH, 0:RW]
BAY = B4[YY % 4, XX % 4]
HALO_Y = 186
DISTMAP = np.hypot(XX - CX, (YY - HALO_Y)*1.05)
def make_grad(top, bot):
    K = 8; pal = np.array([mix(top, bot, i/(K-1)) for i in range(K)], float)
    return pal[np.clip(np.floor((YY/RH)*(K-1) + BAY), 0, K-1).astype(int)]
GRADS = {k: make_grad(v[0], v[1]) for k, v in THEMES.items()}
def ped_cols(theme):
    b = THEMES[theme][1]
    return dict(low_side=dim(b, .55), low_top=dim(b, .85), side=dim(b, .8), top=mix(b, (255, 255, 255), .14), shadow=dim(b, .45))
_r = np.random.default_rng(8)
MOTES = [(int(_r.integers(0, RW)), _r.random()*240, .6 + _r.random()*.8) for _ in range(40)]

# ---------------------------------------------------------------- instruments
GPROF = [(0, .06), (.04, .19), (.14, .28), (.28, .29), (.4, .25), (.47, .2), (.55, .2), (.63, .215), (.72, .2),
         (.82, .15), (.92, .1), (1, .06)]
GUT, WOUND = (224, 204, 156), (206, 210, 224)
CC = {
 "cvielle": dict(L=1.0, d=.12, prof=V.VC["vielle"]["prof"], top=(178, 104, 52), side=(128, 70, 34), fb0=.62, neck=.2,
                 fbcol=(70, 44, 28), head="leaf", holes="c", strings=[55, 62, 67, 74], scol=[GUT]*4, bridge=.3,
                 tail=(.02, .15), sb=.05, sn=.024, bow=(1.0, .12, (110, 70, 40), (80, 50, 30))),
 "gamba":   dict(L=1.05, d=.15, prof=GPROF, top=(198, 130, 68), side=(140, 86, 40), fb0=.55, neck=.36, fbcol=(40, 30, 26),
                 head="scroll", holes="c", strings=[38, 43, 48, 52, 57, 62], scol=[GUT]*6, bridge=.3, tail=(.02, .16),
                 sb=.075, sn=.036, frets=7, bow=(1.05, .09, (120, 80, 44), (70, 46, 28))),
 "bassv":   dict(L=1.12, d=.18, prof=V.VPROF, top=(176, 100, 42), side=(124, 66, 26), fb0=.55, neck=.3, fbcol=(36, 28, 26),
                 head="scroll", holes="f", strings=[34, 41, 48, 55], scol=[GUT]*4, bridge=.33, tail=(.03, .2),
                 sb=.062, sn=.03, paint=True, bow=(1.05, .07, (110, 70, 40), (70, 46, 28))),
 "bologna": dict(L=1.0, d=.17, prof=V.VPROF, top=(190, 112, 50), side=(136, 76, 30), fb0=.55, neck=.32, fbcol=(32, 26, 26),
                 head="scroll", holes="f", strings=[36, 43, 50, 57], scol=[WOUND, WOUND, GUT, GUT], bridge=.33, tail=(.03, .2),
                 sb=.06, sn=.03, bow=(1.05, .05, (100, 62, 34), (60, 40, 26))),
 "cstrad":  dict(L=.98, d=.17, prof=V.VPROF, top=(206, 90, 36), side=(152, 60, 26), fb0=.55, neck=.33, fbcol=(30, 24, 26),
                 head="scroll", holes="f", strings=[36, 43, 50, 57], scol=[WOUND, WOUND, GUT, GUT], bridge=.33, tail=(.03, .2),
                 sb=.06, sn=.03, bow=(1.05, .025, (90, 52, 28), (50, 34, 24))),
 "cmodern": dict(L=.98, d=.17, prof=V.VPROF, top=(188, 84, 40), side=(130, 52, 26), fb0=.55, neck=.34, fbcol=(26, 22, 24),
                 head="scroll", holes="f", strings=[36, 43, 50, 57], scol=[WOUND]*4, bridge=.33, tail=(.03, .2),
                 sb=.06, sn=.03, endpin=True, tuners=True, bow=(1.08, -.012, (70, 40, 24), (26, 22, 26))),
}

def play2(g, t, ev, fx, s, strings, TBN, scol, contact, base_ang, bowcfg, drones_TBN=()):
    active = {}
    for age, m, v, dur, tp in ev:
        if age < dur + .05:
            i = V.string_idx(strings, m); active.setdefault(i, []).append((age, m))
    for i, (T_, B_, N_) in enumerate(TBN):
        g.line(T_, B_, dim(scol[i], .7))
        if i in active:
            B, N = np.array(B_), np.array(N_); pts = []
            for k in range(9):
                u = k/8; p = B*(1-u) + N*u; p[0] += .009*math.sin(math.pi*u)*FLIP[0]; pts.append(p)
            for k in range(8): g.line(pts[k], pts[k+1], HOTSTR)
            age, m = min(active[i]); uf = 1 - 2 ** (-(m - strings[i])/12)
            if uf > .01:
                fp = N + (B - N)*uf + [0, 0, .008]
                g.box(fp[0]-.02, fp[0]+.02, fp[1]-.02, fp[1]+.02, fp[2]-.004, fp[2]+.012, HOT, emit=True)
        else:
            g.line(B_, N_, scol[i])
    for T_, B_, N_ in drones_TBN:
        B, N = np.array(B_), np.array(N_); g.line(T_, B_, (150, 146, 140))
        mid = (B + N)/2 + [.005*FLIP[0], 0, 0]; g.line(B, mid, HOTSTR); g.line(mid, N, HOTSTR)
    segs = s.setdefault("_bow", V.bow_track(s["ev"]))
    cur = [i for i in active] or [len(strings)//2]
    ang = base_ang + math.radians((sum(cur)/len(cur) - (len(strings)-1)/2)*-4)
    V.draw_bow(g, contact, ang, V.bow_pos(segs, t), *bowcfg)
    if active: fx.append(("dust", contact, t))
    for age, m, v, dur, tp in ev:
        if tp and age < .9: fx.append(("note", (contact[0] + .38, contact[1] + .62, contact[2]), age))

def build_cello(g, t, ev, fx, s, key):
    c = CC[key]; L, d = c["L"], c["d"]; zf = d/2
    if c.get("endpin"):
        y0 = .3
        g.box(-.012, .012, .02, y0, -.012, .012, (200, 200, 212), spec=.8); g.ngon(0, .004, 0, .06, (30, 26, 30), n=6)
    else:
        y0 = .22; wood = (70, 44, 30)
        g.box(-.17, .17, .03, y0, -.16, .12, wood); g.box(-.21, .21, 0, .03, -.19, .15, dim(wood, .8))
    yt = y0 + L
    V.body(g, c["prof"], L, y0, d, c["top"], c["side"])
    yfb0, yn = y0 + c["fb0"]*L, yt + c["neck"]
    V.slab_xy(g, [(-.05, yt - .04), (.05, yt - .04), (.04, yn), (-.04, yn)], -.05, zf, c["side"])
    V.slab_xy(g, [(-.065, yfb0), (.065, yfb0), (.045, yn), (-.045, yn)], zf, zf + .045, c["fbcol"], spec=.3)
    st = c["strings"]; n = len(st); nd = len(DRONES.get(key, []))
    if c["head"] == "scroll":
        ph = .27 if n > 4 else .2
        V.slab_xy(g, [(-.045, yn), (.045, yn), (.04, yn + ph), (-.04, yn + ph)], -.06, zf, c["side"])
        V.xcyl(g, 0, yn + ph + .05, -.015, .068, .048, c["side"]); V.xcyl(g, 0, yn + ph + .05, -.015, .036, .052, dim(c["side"], .7))
        for k in range(n):
            yy = yn + .03 + (ph - .06)*k/max(1, n - 1); sg = -1 if k % 2 == 0 else 1
            g.box(min(sg*.04, sg*.11), max(sg*.04, sg*.11), yy - .013, yy + .013, -.03, .002, (30, 24, 24))
    else:
        V.slab_xy(g, V.octo(0, yn + .13, .1, .14), -.02, zf, c["side"])
        for k in range(n + nd):
            px = -.05 + .1*(k % 2); py = yn + .05 + .17*(k//2)/max(1, (n + nd - 1)//2)
            g.box(px - .014, px + .014, py - .014, py + .014, zf, zf + .05, (40, 30, 26))
    yb = y0 + c["bridge"]*L; ta, tb = y0 + c["tail"][0]*L, y0 + c["tail"][1]*L
    sb, sn = c["sb"], c["sn"]
    g.box(-sb - .03, sb + .03, yb - .012, yb + .012, zf, zf + .09, (228, 198, 148))
    V.slab_xy(g, [(-.065, ta), (.065, ta), (.04, tb), (-.04, tb)], zf, zf + .028, (30, 26, 28), spec=.3)
    hz = zf + .002; dark = (22, 14, 12)
    if c["holes"] == "f":
        for sg in (-1, 1):
            a, b = (sg*.105*L, y0 + .31*L), (sg*.075*L, y0 + .5*L)
            V.thin(g, a, b, .018, hz, dark)
            for (x, y) in (a, b): V.decal(g, V.octo(x, y, .02, .02, 6), hz, dark)
    else:
        for sg in (-1, 1):
            cx, cy, r = sg*.08*L, y0 + .44*L, .07*L
            angs = np.linspace(-1.0, 1.0, 5) + (0 if sg > 0 else math.pi)
            P_ = [(cx + r*math.cos(a), cy + r*math.sin(a)) for a in angs]
            for k in range(4): V.thin(g, P_[k], P_[k+1], .018, hz, dark)
    if c.get("frets"):
        for k in range(1, c["frets"] + 1):
            y = yn - (yn - yb)*(1 - 2 ** (-k/12))
            if y > yfb0:
                w = lerp(.065, .045, (y - yfb0)/(yn - yfb0))
                V.thin(g, (-w, y), (w, y), .008, zf + .0465, (206, 196, 176))
    if c.get("paint"):
        V.decal(g, V.octo(0, y0 + .14*L, .075, .06), hz, (214, 170, 66))
        V.decal(g, V.octo(0, y0 + .14*L, .045, .035), hz + .001, (40, 70, 140))
        for sg in (-1, 1):
            for u in (.2, .72):
                x = sg*.24*L*(1 if u < .5 else .82); y = y0 + u*L
                V.decal(g, [(x, y - .025), (x + .018, y), (x, y + .025), (x - .018, y)], hz, (214, 170, 66))
    if c.get("tuners"):
        for i in range(4):
            x = lerp(-.03, .03, i/3); g.box(x - .008, x + .008, tb - .03, tb - .005, zf + .02, zf + .045, (210, 210, 220), spec=.7)
    TBN = [((lerp(-.035, .035, i/(n-1)), tb, zf + .032), (lerp(-sb, sb, i/(n-1)), yb, zf + .092),
            (lerp(-sn, sn, i/(n-1)), yn, zf + .052)) for i in range(n)]
    drones = [((-.05 - .02*k, tb, zf + .032), (-sb - .035 - .02*k, yb, zf + .088), (-.055 - .02*k, yn + .04, zf + .055))
              for k in range(nd)]
    contact = (0, yb + .3*(yfb0 - yb), zf + .115)
    play2(g, t, ev, fx, s, st, TBN, c["scol"], contact, math.radians(-10), c["bow"], drones)
    if key == "cstrad" and int(t*6) % 2 == 0:
        fx.append(("spark", (.26, yt - .12, zf), 0)); fx.append(("spark", (-.28, y0 + .3, zf), 0))

# ---------------------------------------------------------------- audio
rng = np.random.default_rng(21)
def _t(n): return np.arange(n)/SR
_BP = butter(2, [900/(SR/2), 2600/(SR/2)], "band")
def clap(v):
    n = int(.28*SR); t = _t(n); nz = lfilter(*_BP, rng.normal(0, 1, n))
    env = sum(np.exp(-(t - k*.009)/.004)*(t >= k*.009) for k in range(3)) + .8*np.exp(-(t - .02)/.09)*(t >= .02)
    return nz*env*.5*v
def rim(v):
    n = int(.08*SR); t = _t(n)
    return (np.sin(2*np.pi*1750*t)*.5 + np.sin(2*np.pi*420*t))*np.exp(-t*60)*.5*v
def impact(v):
    n = int(1.1*SR); t = _t(n); f = 34 + 44*np.exp(-t*6)
    return np.tanh(1.5*np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*3.0))*v
def tom(f0, v):
    n = int(.35*SR); t = _t(n); f = f0*(1 + .4*np.exp(-t*20))
    return np.sin(2*np.pi*np.cumsum(f)/SR)*np.exp(-t*9)*v
def bass808(m, dur, v):
    f = midi_hz(m); n = int(dur*SR); t = _t(n); fm = f*(1 + .5*np.exp(-t*40))
    ph = 2*np.pi*np.cumsum(fm)/SR
    x = np.sin(ph) + .15*np.sin(2*ph)
    env = np.minimum(1, t/.005)*np.exp(-t*.9); r = int(.04*SR); env[-r:] *= np.linspace(1, 0, r)
    return np.tanh(1.5*x*env)*v
def sub(root): return root - 12 if root >= 42 else root

def drum_events():
    ev = []
    for b in range(16):
        T0 = b*BAR
        if b == 15: ev += [(T0, 'impact', 1.), (T0, 'crash', 1.), (T0, 'kick', 1.)]; continue
        if b == 0:
            ev += [(T0, 'kick', .7), (T0 + 2*BEAT, 'rim', .5)] + [(T0 + k*.5*BEAT, 'hat', .3) for k in range(8)]
            continue
        ev += [(T0, 'kick', 1.), (T0 + .75*BEAT, 'kick', .6), (T0 + 2.5*BEAT, 'kick', .85), (T0 + 2*BEAT, 'snare', .5),
               (T0 + 2*BEAT, 'clap', .9), (T0 + 1.5*BEAT, 'rim', .4), (T0 + 3.5*BEAT, 'rim', .35)]
        if b % 2 == 1: ev.append((T0 + 3.25*BEAT, 'kick', .5))
        ev += [(T0 + (k*.25 + (.06 if k % 2 else 0))*BEAT, 'hat', [.5, .18, .32, .18][k % 4]) for k in range(16)]
        if b % 2 == 0: ev.append((T0, 'impact', .55))
        if b in (2, 12): ev.append((T0, 'crash', .7))
        if b == 13: ev += [(T0 + (3 + k*.25)*BEAT, f'tom{k}', .75 - .05*k) for k in range(4)]
    return sorted(ev)
DRUMS = drum_events()
KICKS = [t for t, k, v in DRUMS if k == 'kick']

CTONE = {
 "cvielle": V.TONE["vielle"],
 "gamba":   dict(tilt=1.15, form=[(300, .8, .5), (1200, 1.0, .5)], vib=.001, vdel=.25, att=.06, noise=0),
 "bassv":   dict(tilt=1.1, form=[(180, 1.0, .5), (500, .6, .4), (1500, .5, .4)], vib=.002, vdel=.2, att=.06, noise=0),
 "bologna": dict(tilt=1.05, form=[(220, 1.0, .5), (600, .6, .4), (1800, .7, .45)], vib=.003, vdel=.15, att=.05, noise=0),
 "cstrad":  dict(tilt=1.0, form=[(220, 1.0, .5), (600, .6, .4), (2000, .9, .45)], vib=.0045, vdel=.12, att=.045, noise=0),
 "cmodern": dict(tilt=1.0, form=[(220, 1.0, .5), (600, .7, .4), (2000, 1.0, .45)], vib=.0055, vdel=.1, att=.04, noise=0),
}
V.TONE.update(CTONE)

def render_audio(path="audio_cello.wav"):
    n = int((TOTAL + 3)*SR)
    drums, bassb, padb, inst = (np.zeros(n) for _ in range(4))
    def add(buf, T0, x):
        a = int(T0*SR); e = min(n, a + len(x)); buf[a:e] += x[:e-a]
    toms = [196, 165, 131, 98]
    for T0, k, v in DRUMS:
        if k.startswith('tom'): x = tom(toms[int(k[3])], v)
        else: x = {'kick': kick, 'snare': snare, 'hat': lambda v: hat(v)*.7, 'crash': crash, 'clap': clap,
                   'rim': rim, 'impact': impact}[k](v)
        add(drums, T0, x)
    for b in range(1, 16):
        T0, ch = b*BAR, BARS[b]
        if b == 15: add(bassb, T0, bass808(sub(CH["G"][0]), 2.6, 1.)); continue
        if isinstance(ch, tuple):
            for k, cc in enumerate(ch): add(bassb, T0 + k*BEAT, bass808(sub(CH[cc][0]), BEAT*.95, .85))
        else:
            r = sub(CH[ch][0]); add(bassb, T0, bass808(r, 1.6*BEAT, .9)); add(bassb, T0 + 2.5*BEAT, bass808(r, 1.2*BEAT, .8))
    for b in range(16):
        ch = BARS[b]
        if isinstance(ch, tuple):
            for k, cc in enumerate(ch): add(padb, b*BAR + k*BEAT, pad(CH[cc][1], BEAT + .05, .5))
        else:
            add(padb, b*BAR, pad(CH[ch][1], 2.6 if b == 15 else BAR + .05, .5 if b else .35))
    def rms(x):
        x = x[np.abs(x) > 1e-6]; return np.sqrt((x**2).mean()) if len(x) else 1.
    def at(x, db): return x*(10**(db/20))/rms(x)
    for s in SCENES:
        sb_ = np.zeros(n)
        for nt, m, v, dur, tp in s["ev"]: add(sb_, s["t0"] + nt, V.bowed(midi_hz(m), dur, v, s["key"]))
        for dm in DRONES.get(s["key"], []): add(sb_, s["t0"], V.bowed(midi_hz(dm), 2*BAR - .1, .3, s["key"], drone=True))
        a = int(s["t0"]*SR); e = int((s["t0"] + 2*BAR)*SR)
        inst += sb_*(10**(-15/20))/(np.sqrt((sb_[a:e]**2).mean()) + 1e-9)
    drums, bassb, padb = at(drums, -23), at(bassb, -25), at(padb, -32)
    wet_in = inst + padb*.6
    r = np.random.default_rng(1); irn = int(1.8*SR)
    ir = r.normal(0, 1, irn)*np.exp(-np.arange(irn)/SR*3.0); ir[0] = 0; ir /= np.sqrt((ir**2).sum())
    Lf = 1 << (n + irn).bit_length()
    wet = np.fft.irfft(np.fft.rfft(wet_in, Lf)*np.fft.rfft(ir, Lf), Lf)[:n]
    out = drums + bassb + padb + inst + at(wet, -23)
    out = out[:int(TOTAL*SR)]
    fade = int(.8*SR); out[-fade:] *= np.linspace(1, 0, fade)
    out = np.tanh(1.6*out/np.abs(out).max())/np.tanh(1.6)*.94
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR); w.writeframes((out*32767).astype(np.int16).tobytes())
    return path, out

def analyze(x):
    """per-frame equalizer bands (bass in the middle) and loudness, 0..1"""
    nfr = int(TOTAL*FPS) + 2; W = 4096; win = np.hanning(W)
    edges = np.geomspace(45, 5000, 15); fr = np.fft.rfftfreq(W, 1/SR)
    masks = [(fr >= edges[b]) & (fr < edges[b+1]) for b in range(14)]
    E = np.zeros((nfr, 14)); Lo = np.zeros(nfr)
    for f in range(nfr):
        c = int(f/FPS*SR); seg = x[max(0, c - W//2): c + W//2]
        if len(seg) < W: seg = np.pad(seg, (0, W - len(seg)))
        sp = np.abs(np.fft.rfft(seg*win))**2
        E[f] = [sp[m].mean() for m in masks]; Lo[f] = np.sqrt((seg**2).mean())
    E = 10*np.log10(E + 1e-12)
    lo, hi = np.percentile(E, 20, axis=0), np.percentile(E, 98, axis=0)
    E = np.clip((E - lo)/(hi - lo), 0, 1)
    Lo = np.clip(Lo/np.percentile(Lo, 97), 0, 1)
    for f in range(1, nfr):
        E[f] = np.maximum(E[f], E[f-1]*.82); Lo[f] = max(Lo[f], Lo[f-1]*.9)
    return E, Lo, np.cumsum(Lo)/FPS

# ---------------------------------------------------------------- frame
def scene_at(T): return min(6, int(T/(2*BAR)))

def background(T, fi, theme0, theme1, kb, pulse):
    bg = GRADS[theme0]*(1 - kb) + GRADS[theme1]*kb
    acc = np.array(mix(THEMES[theme0][2], THEMES[theme1][2], kb), float)
    loud = LOUD[fi]
    bg[200:] += acc*.07*pulse
    # halo
    r = 62 + 22*loud
    m = (DISTMAP < r) & (BAY < .16 + .2*loud)
    bg[m] = bg[m]*.62 + acc*.38
    e = np.abs(DISTMAP - r) < 1.1
    bg[e] = bg[e]*.35 + acc*.65
    # ripple rings from each note
    for on in ONSETS:
        age = T - on
        if 0 <= age < 1.3:
            rr = 30 + age*150; a = 1 - age/1.3
            m = (np.abs(DISTMAP - rr) < 1.6) & (BAY < a*.85)
            bg[m] = bg[m]*.35 + np.minimum(255, acc*1.15)*.65
    # equalizer bars behind the turntable (bass in the middle)
    hor = 262
    for i in range(27):
        band = min(13, abs(i - 13)); h = int(4 + 86*EQ[fi, band])
        x0 = i*10 + 1
        col = np.linspace(.45, 1.0, h)[::-1, None, None]*acc
        bg[hor - h:hor, x0:x0 + 8] = bg[hor - h:hor, x0:x0 + 8]*.25 + col*.75
        bg[hor - h:hor - h + 1, x0:x0 + 8] = np.minimum(255, acc*1.3)
        rh = h//2
        refl = BAY[hor:hor + rh, x0:x0 + 8] < .3
        sl = bg[hor:hor + rh, x0:x0 + 8]; sl[refl] = sl[refl]*.6 + acc*.4
    # motes drift upward faster when the music is louder
    cum = CUM[fi]
    for x, y0, spd in MOTES:
        y = int(330 - ((y0 + T*10*spd + cum*40*spd) % 240))
        if 0 <= y < RH: bg[y, x] = np.minimum(255, acc*1.25)
    return np.clip(bg, 0, 255).astype(np.uint8)

def frame(T, fi=0):
    FLIP[0] = 1 if fi % 2 == 0 else -1
    si = scene_at(T); s = SCENES[si]
    prev_theme = SCENES[si-1]["theme"] if si > 0 else s["theme"]
    t = T - s["t0"]
    kb = ease(t/(.5*BEAT)) if si > 0 else 1.0
    last_kick = max([k for k in KICKS if k <= T], default=-9)
    pulse = math.exp(-(T - last_kick)*7)
    img = background(T, fi, prev_theme, s["theme"], kb, pulse)
    pc0, pc1 = ped_cols(prev_theme), ped_cols(s["theme"]); pc = {k: mix(pc0[k], pc1[k], kb) for k in pc1}
    rend = np.zeros((RH, RW, 3), np.uint8); zb = np.zeros((RH, RW)); gb = np.zeros((RH, RW), np.int8)
    pin = t/(.35*BEAT); scale = ease_out_back(pin); spin = -100*(1 - ease(pin))
    if si < 6:
        pout = (t - (2*BAR - .35*BEAT))/(.35*BEAT)
        if pout > 0: scale *= 1 - ease(pout); spin += 100*pout*pout
    yaw = -10 + 22*math.sin(2*math.pi*T/8) + spin
    cam = Cam(DIST, yaw, scale*ISCALE)
    gp = Geo(); build_pedestal(gp, DIST, pulse, .32*scale, pc)
    render_geo(gp, cam, rend, zb, gb, scale=1.0)
    fx = []
    if scale > .01:
        ev = [(t - nt, m, v, dur, tp) for nt, m, v, dur, tp in s["ev"] if t >= nt]
        gi = Geo(); build_cello(gi, t, ev, fx, s, s["key"]); render_geo(gi, cam, rend, zb, gb)
    outline(rend, zb, gb)
    m = gb > 0; img[m] = rend[m]
    im = Image.fromarray(img); d = ImageDraw.Draw(im)
    bgb = THEMES[s["theme"]][1]
    hl = "THE CELLO" if si == 0 else "800 YEARS"; hw = text_w(hl, 3)
    downbeat = math.exp(-((T % BAR))*6)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], fill=INK)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 36], outline=mix(GOLD_D, (255, 230, 150), downbeat))
    center_text(d, 13, hl, CREAM, 3)
    outro = T >= OUTRO_T
    if si == 0:
        ys = "TODAY"
    elif outro:
        ys = "TODAY"
    else:
        prev = SCENES[si-1].get("year", 900) if si > 1 else 900
        k = ease(t/BEAT)
        ys = (f"{int(lerp(prev, s['year'], k))}" if k < 1 else s["ys"])
        if k < 1 and int(lerp(prev, s['year'], k)) < 1000: ys += " AD"
    center_text(d, 45, ys, GOLD, 3)
    center_text(d, 74, s["name"], CREAM, 2)
    credit = "BACH: CELLO SUITE NO. 1" if outro else s["credit"]
    center_text(d, 92, credit, mix(bgb, (226, 196, 130), ease((t - .3)/.3)), 1)
    if si == 0:
        lines = ("THE CELLO: 4 STRINGS,", "ONE HUGE WARM VOICE") if t < 1.5*BAR else ("BUT HOW DID WE", "GET HERE?")
        cols = (CREAM, CREAM) if t < 1.5*BAR else (CREAM, GOLD)
        fk = ease((t - .2)/.3) if t < 1.5*BAR else ease((t - 1.5*BAR)/.25)
        for li, line in enumerate(lines): center_text(d, 336 + li*19, line, mix(bgb, cols[li], fk), 2)
    elif not outro:
        fk = ease((t - .2)/.3)
        for li, line in enumerate(s["fact"]): center_text(d, 336 + li*19, line, mix(bgb, CREAM, fk), 2)
    else:
        fk = ease((T - OUTRO_T)/.3)
        center_text(d, 336, "800 YEARS FROM A", mix(bgb, CREAM, fk), 2)
        center_text(d, 355, "MEDIEVAL FIDDLE", mix(bgb, GOLD, fk), 2)
    rr = np.random.default_rng(fi)
    for f in fx:
        if f[0] == "note":
            S = cam.screen(f[1]); a = f[2]/.9
            sprite(d, NOTE_SPR, int(S[0]) - 5 + int(4*math.sin(f[2]*8)), int(S[1]) - int(f[2]*40) - 14,
                   mix((120, 96, 140), (255, 246, 214), (1 - a)**.6), 2)
        elif f[0] == "spark":
            S = cam.screen(f[1]); sprite(d, SPARK, int(S[0]) - 2, int(S[1]) - 2, (255, 246, 190), 1, False)
        elif f[0] == "dust":
            S = cam.screen(f[1])
            for _ in range(3):
                d.point((int(S[0] + rr.normal(0, 6)), int(S[1] + rr.normal(0, 3) - rr.random()*6)), fill=(255, 250, 230))
    if T >= FINAL_T:
        tb = T - FINAL_T; r2 = np.random.default_rng(11)
        for k in range(26):
            ang = r2.random()*6.28; sp = 40 + r2.random()*80
            x = CX + math.cos(ang)*sp*min(1, tb*2.2); y = 190 + math.sin(ang)*sp*.8*min(1, tb*2.2) + tb*tb*20
            if tb < 2.2: sprite(d, SPARK, int(x), int(y), mix((255, 240, 180), (120, 96, 140), clamp(tb/2.2)), 1, False)
    if si > 0:
        n = 6; xs = [36 + i*(RW - 72)//(n - 1) for i in range(n)]; cur = si - 1
        d.line([(xs[0], 392), (xs[-1], 392)], fill=dim(bgb, 1.4))
        d.line([(xs[0], 392), (xs[-1] if outro else xs[cur], 392)], fill=GOLD_D)
        bob = int(2*math.exp(-((T % BEAT))*14))
        for i, x in enumerate(xs):
            if i < cur or outro: d.rectangle([x-2, 390, x+2, 394], fill=GOLD_D)
            elif i == cur:
                d.rectangle([x-3, 389-bob, x+3, 395-bob], fill=GOLD); d.rectangle([x-4, 388-bob, x+4, 396-bob], outline=(255, 230, 150))
            else: d.rectangle([x-2, 390, x+2, 394], fill=dim(bgb, 1.4))
    return im

if __name__ == "__main__":
    wav, mixed = render_audio()
    EQ, LOUD, CUM = analyze(mixed)
    if "--still" in sys.argv:
        for ts in sys.argv[sys.argv.index("--still") + 1:]:
            T = float(ts); frame(T, int(T*FPS)).resize((RW*2, RH*2), Image.NEAREST).save(f"cstill_{ts}.png")
        sys.exit()
    nframes = int(TOTAL*FPS)
    p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{RW}x{RH}", "-r", str(FPS), "-i", "-", "-i", wav,
        "-vf", f"scale={RW*UP}:{RH*UP}:flags=neighbor", "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-crf", "16", "-preset", "medium", "-c:a", "aac", "-b:a", "192k", "-shortest", OUT], stdin=subprocess.PIPE)
    for fi in range(nframes):
        p.stdin.write(frame(fi/FPS, fi).tobytes())
    p.stdin.close(); p.wait()
    print("wrote", OUT, f"{TOTAL:.1f}s")

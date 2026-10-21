"""Pixel Pitch: history of the cello, 60-second cut (shared-engine formula).
python3 cello60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import violin_lowpoly as V
import cello_lowpoly as C
import piano_lowpoly as P
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY, HOT, HOTSTR

OUT, TAG = "cello60_lowpoly.mp4", "c6"
BPM = 100
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 2
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .8, -10, 22, .32
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE CELLO", "1,100 YEARS", "TODAY", 600
INTRO_LINES = (("THE CELLO: 4 STRINGS,", "ONE HUGE WARM VOICE"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("1,100 YEARS FROM", "THE REBAB")
OUTRO_CREDIT = "BACH: CELLO SUITE NO. 1"

# ---------------------------------------------------------------- music
CH = dict(C.CH); CH.update({"Gm": (43, (55, 58, 62)), "D7": (38, (54, 60, 62))})
BARS = ["G", "C", "D7", "G",                    # showcase: Bach Prelude mm. 1-4
        "D", "Gm", "Dm", "C", "Dm", "A", "Gm", "D", "G", "D", "C", "G", "Am", "E", "D", "A",
        ("D", "A", "Bm", "F#m"), ("G", "D", "G", "A"),   # modern: Pachelbel
        "D", "C", "G"]                         # outro + final chord
P1, P2, P3 = [43, 50, 59, 57, 59, 50, 59, 50], [43, 52, 60, 59, 60, 52, 60, 52], [43, 54, 60, 59, 60, 54, 60, 54]
P4a, P4b = [43, 55, 59, 57, 59, 55, 59, 55], [43, 55, 59, 57, 59, 55, 59, 54]
BACH4 = P1*2 + P2*2 + P3*2 + P4a + P4b

SCENES = [
 dict(key="cmodern", theme="intro", name="MODERN CELLO", credit="BACH: CELLO SUITE NO. 1",
      notes=seq(0, [(m, .25) for m in BACH4], .8)),
 dict(key="rebab", theme="rebab", year=900, ys="900s", name="REBAB", credit="MELODY: MAQAM HIJAZ",
      fact=("BOWED STRINGS ARRIVE", "FROM THE ARAB WORLD"), size=1.1,
      notes=seq(.5, [(69, .5), (70, .5), (69, .5), (67, .5), (66, .5), (63, .5), (62, .5), (67, 1), (70, .5), (69, .5), (67, .5), (66, .5), (67, 1)])),
 dict(key="cvielle", theme="cvielle", year=1200, ys="1200s", name="VIELLE", credit="IN THE STYLE OF A TROUBADOUR SONG",
      fact=("THE MEDIEVAL FIDDLE", "STARTS THE FAMILY"),
      notes=seq(0, [(62, .5), (65, .5), (67, .5), (69, 1), (67, .5), (65, .5), (64, .5), (67, 1), (64, .5), (62, .5), (64, 1), (60, 1)])),
 dict(key="gamba", theme="gamba", year=1490, ys="1490s", name="VIOLA DA GAMBA", credit="IN THE STYLE OF MARIN MARAIS",
      fact=("PLAYED BETWEEN THE", "LEGS: GAMBA = LEG"),
      notes=seq(0, [(62, 1.5), (60, .25), (58, .25), (57, 1), (53, .5), (55, .5), (57, .5), (58, .5), (61, 1), (64, .5), (61, .5), (57, 1)])),
 dict(key="violone", theme="violone", year=1500, ys="1500s", name="VIOLONE", credit="IN THE STYLE OF DIEGO ORTIZ",
      fact=("A GIANT VIOL FOR", "THE LOWEST NOTES"), size=.85,
      notes=seq(0, [(43, .5), (45, .5), (46, .5), (48, .5), (50, 1), (46, 1), (45, .5), (46, .5), (48, .5), (45, .5), (42, 1), (38, 1)])),
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
 dict(key="spalla", theme="spalla", year=1720, ys="1720s", name="CELLO DA SPALLA", credit="IN THE STYLE OF BACH'S 6TH SUITE",
      fact=("A SMALL CELLO PLAYED", "ON THE SHOULDER"),
      notes=seq(0, [(m, .25) for m in [62, 74, 62, 74, 64, 74, 64, 74, 66, 74, 66, 74, 67, 74, 66, 74]], .75) +
            seq(4, [(m, .25) for m in [61, 73, 61, 73, 64, 73, 64, 73, 69, 73, 69, 73, 67, 73, 64, 73]], .75)),
 dict(key="cmodern", theme="intro", year=1850, ys="1850s", name="MODERN CELLO", credit="PACHELBEL: CANON IN D",
      fact=("THE ENDPIN LETS IT", "STAND ON THE FLOOR"),
      notes=seq(0, [(m, 1) for m in [62, 57, 59, 54, 55, 50, 55, 57]], .85) +
            seq(8, [(m, .25) for m in [50, 57, 66, 64, 66, 57, 66, 57]*2 + P2*2], .8) + [(16, [43, 50, 59, 67], 4.3, 1.0)]),
]
DRONES = {"cvielle": [50]}

# ---------------------------------------------------------------- instruments
CC = dict(C.CC)
CC["violone"] = dict(L=1.25, d=.18, prof=C.GPROF, top=(186, 118, 60), side=(128, 76, 36), fb0=.55, neck=.44, fbcol=(40, 30, 26),
                     head="scroll", holes="c", strings=[31, 36, 41, 45, 50, 55], scol=[C.GUT]*6, bridge=.3, tail=(.02, .16),
                     sb=.085, sn=.04, frets=7, bow=(1.15, .1, (120, 80, 44), (70, 46, 28)))
CC["spalla"] = dict(L=.72, d=.12, prof=V.VPROF, top=(198, 106, 44), side=(140, 70, 30), fb0=.55, neck=.26, fbcol=(30, 24, 26),
                    head="scroll", holes="f", strings=[36, 43, 50, 57, 64], scol=[C.WOUND, C.WOUND, C.GUT, C.GUT, C.GUT],
                    bridge=.33, tail=(.03, .2), sb=.065, sn=.032, strap=True, bow=(.9, .03, (96, 58, 32), (60, 40, 26)))

def bow_track(ev):
    groups = {}
    for e in ev: groups[e[0]] = max(groups.get(e[0], 0), e[3])
    segs, s, d = [], 0.0, 1
    for t0 in sorted(groups):
        dur = groups[t0]; trav = clamp(dur*.55, .12, .7); d = -d
        tgt = clamp(s + d*trav, -.42, .42)
        if abs(tgt - s) < .08: d = -d; tgt = clamp(s + d*trav, -.42, .42)
        segs.append((t0, t0 + dur, s, tgt)); s = tgt
    return segs

def play6(g, t, ev, fx, s, strings, TBN, scol, contact, base_ang, bowcfg, drones=()):
    FL = V.FLIP[0]; active = {}
    for age, m, v, dur, tp, cs in ev:
        if age < dur + .05: active.setdefault(V.string_idx(strings, m), []).append((age, m))
    for i, (T_, B_, N_) in enumerate(TBN):
        g.line(T_, B_, dim(scol[i], .7))
        if i in active:
            B, N = np.array(B_), np.array(N_); pts = []
            for k in range(9):
                u = k/8; p = B*(1 - u) + N*u; p[0] += .009*math.sin(math.pi*u)*FL; pts.append(p)
            for k in range(8): g.line(pts[k], pts[k+1], HOTSTR)
            age, m = min(active[i]); uf = 1 - 2 ** (-(m - strings[i])/12)
            if uf > .01:
                fp = N + (B - N)*uf + [0, 0, .008]
                g.box(fp[0] - .02, fp[0] + .02, fp[1] - .02, fp[1] + .02, fp[2] - .004, fp[2] + .012, HOT, emit=True)
        else:
            g.line(B_, N_, scol[i])
    for T_, B_, N_ in drones:
        B, N = np.array(B_), np.array(N_); g.line(T_, B_, (150, 146, 140))
        mid = (B + N)/2 + [.005*FL, 0, 0]; g.line(B, mid, HOTSTR); g.line(mid, N, HOTSTR)
    segs = s.setdefault("_bow6", bow_track(s["ev"]))
    cur = list(active) or [len(strings)//2]
    ang = base_ang + math.radians((sum(cur)/len(cur) - (len(strings) - 1)/2)*-4)
    V.draw_bow(g, contact, ang, V.bow_pos(segs, t), *bowcfg)
    if active: fx.append(("dust", contact, t))
    for age, m, v, dur, tp, cs in ev:
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
            pp = [(cx + r*math.cos(a), cy + r*math.sin(a)) for a in angs]
            for k in range(4): V.thin(g, pp[k], pp[k+1], .018, hz, dark)
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
    if c.get("strap"):
        pts = [(-.2, y0 + .12, -d/2 - .01), (-.36, y0 + .4, -.04), (-.36, y0 + .75, -.04), (-.18, yt - .05, -d/2 - .01)]
        for a, b in zip(pts, pts[1:]):
            g.line(a, b, (120, 70, 36)); g.line(np.array(a) + [.01, 0, 0], np.array(b) + [.01, 0, 0], (120, 70, 36))
    TBN = [((lerp(-.035, .035, i/(n - 1)), tb, zf + .032), (lerp(-sb, sb, i/(n - 1)), yb, zf + .092),
            (lerp(-sn, sn, i/(n - 1)), yn, zf + .052)) for i in range(n)]
    drones = [((-.05 - .02*k, tb, zf + .032), (-sb - .035 - .02*k, yb, zf + .088), (-.055 - .02*k, yn + .04, zf + .055)) for k in range(nd)]
    contact = (0, yb + .3*(yfb0 - yb), zf + .115)
    play6(g, t, ev, fx, s, st, TBN, c["scol"], contact, math.radians(-10), c["bow"], drones)
    if key == "cstrad" and int(t*6) % 2 == 0:
        fx.append(("spark", (.26, yt - .12, zf), 0)); fx.append(("spark", (-.28, y0 + .3, zf), 0))

def build_rebab(g, t, ev, fx, s):
    wood, light, skin = (118, 64, 30), (156, 90, 42), (234, 214, 172)
    g.box(-.016, .016, 0, .3, -.016, .016, (90, 54, 30))
    V.xcyl(g, 0, .03, 0, .03, .03, (200, 170, 90))
    g.box(-.2, .2, .3, .74, -.09, .06, wood)
    g.face([(-.17, .33, .061), (.17, .33, .061), (.17, .71, .061), (-.17, .71, .061)], skin, ref=(0, .5, 0))
    for k in range(7):
        for y in (.315, .725): V.decal(g, V.octo(-.15 + .05*k, y, .008, .008, 4), .0615, (240, 236, 220))
    g.box(-.026, .026, .74, 1.52, -.03, .03, light)
    g.box(-.045, .045, 1.52, 1.6, -.04, .04, wood)
    V.xcyl(g, 0, 1.64, 0, .035, .035, (200, 170, 90))
    for y in (1.38, 1.46): g.box(-.085, .085, y - .012, y + .012, -.012, .012, (70, 40, 22))
    g.box(-.055, .055, .39, .41, .061, .1, (226, 196, 146))
    TBN = [((x*.8, .315, .065), (x, .40, .102), (x*.6, 1.40, .042)) for x in (-.02, .02)]
    play6(g, t, ev, fx, s, [62, 69], TBN, [(214, 210, 200)]*2, (0, .56, .118), math.radians(-6), (.95, .16, (100, 62, 34), (70, 44, 26)))

def build(g, t, ev, fx, s):
    if s["key"] == "rebab": build_rebab(g, t, ev, fx, s)
    else: build_cello(g, t, ev, fx, s, s["key"])

# ---------------------------------------------------------------- sound
V.TONE.update({"violone": dict(tilt=1.15, form=[(150, 1.0, .5), (400, .7, .4), (1200, .5, .4)], vib=.001, vdel=.25, att=.07, noise=0),
               "spalla": dict(tilt=1.0, form=[(300, 1.0, .5), (800, .6, .4), (2400, .8, .45)], vib=.004, vdel=.12, att=.04, noise=0)})
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    x = V.bowed(midi_hz(m), dur, v, s["key"])
    if idx == 0 and s["key"] in DRONES:
        for dm in DRONES[s["key"]]:
            dr = V.bowed(midi_hz(dm), s["len"] - .1, .3, s["key"], drone=True)
            L = max(len(x), len(dr)); x = np.pad(x, (0, L - len(x))) + np.pad(dr, (0, L - len(dr)))
    return x

KIT = {"kick": P.kick, "snare": P.snare, "hat": lambda v: P.hat(v)*.7, "crash": P.crash, "clap": C.clap, "rim": C.rim,
       "impact": C.impact, "tom0": lambda v: C.tom(196, v), "tom1": lambda v: C.tom(165, v), "tom2": lambda v: C.tom(131, v),
       "tom3": lambda v: C.tom(98, v)}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("clap",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "impact", 1.), (T0, "crash", 1.), (T0, "kick", 1.)]; continue
        if b == 0: ev += [(T0, "kick", .7), (T0 + 2*Q, "rim", .5)] + [(T0 + k*.5*Q, "hat", .3) for k in range(8)]; continue
        ev += [(T0, "kick", 1.), (T0 + .75*Q, "kick", .6), (T0 + 2.5*Q, "kick", .85), (T0 + 2*Q, "snare", .5), (T0 + 2*Q, "clap", .9),
               (T0 + 1.5*Q, "rim", .4), (T0 + 3.5*Q, "rim", .35)]
        if b % 2 == 1: ev.append((T0 + 3.25*Q, "kick", .5))
        ev += [(T0 + (k*.25 + (.06 if k % 2 else 0))*Q, "hat", [.5, .18, .32, .18][k % 4]) for k in range(16)]
        if 4 <= b < 22 and b % 2 == 0: ev.append((T0, "impact", .55))
        if b in (4, 20, 22): ev.append((T0, "crash", .7))
        if b in (3, 21, 23): ev += [(T0 + (3 + k*.25)*Q, f"tom{k}", .75 - .05*k) for k in range(4)]
    return ev
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0, ch = b*c.BAR, BARS[b]
        if b == nb - 1: out.append((T0, C.sub(CH["G"][0]), 2.6, 1.)); continue
        if isinstance(ch, tuple): out += [(T0 + k*c.BEAT, C.sub(CH[cc][0]), c.BEAT*.95, .85) for k, cc in enumerate(ch)]
        else:
            r = C.sub(CH[ch][0]); out += [(T0, r, 1.6*c.BEAT, .9), (T0 + 2.5*c.BEAT, r, 1.2*c.BEAT, .8)]
    return out
bass_voice = C.bass808
pad_voice = P.pad
MIX = dict(drums=-23, bass=-25, pad=-32, verb=-23, verb_len=1.8)

# ---------------------------------------------------------------- background: halo, ripples, equalizer, motes
THEMES = {   # sky top, sky bottom, accent
 "intro":   ((4, 24, 30), (14, 92, 96), (100, 240, 220)),
 "rebab":   ((32, 6, 12), (108, 24, 36), (224, 108, 112)),
 "cvielle": ((8, 12, 42), (34, 52, 124), (130, 160, 255)),
 "gamba":   ((26, 10, 34), (86, 34, 110), (210, 130, 240)),
 "violone": ((12, 16, 24), (58, 70, 90), (180, 210, 236)),
 "bassv":   ((6, 26, 16), (24, 92, 56), (140, 230, 160)),
 "bologna": ((18, 24, 6), (78, 96, 24), (210, 240, 110)),
 "cstrad":  ((30, 6, 26), (110, 24, 90), (255, 120, 220)),
 "spalla":  ((8, 18, 30), (52, 92, 130), (170, 215, 255)),
}
GRADS = {k: F.make_grad(v[0], v[1]) for k, v in THEMES.items()}
def ped_cols(th): return F.ped_cols_from(THEMES[th][1])
HALO_Y = 186
DISTMAP = np.hypot(XX - CX, (YY - HALO_Y)*1.05)
_r = np.random.default_rng(8)
MOTES = [(int(_r.integers(0, RW)), _r.random()*240, .6 + _r.random()*.8) for _ in range(40)]

def background(c, x):
    th0, th1, kb, T = x["th0"], x["th1"], x["kb"], x["T"]
    bg = GRADS[th0]*(1 - kb) + GRADS[th1]*kb
    acc = np.array(mix(THEMES[th0][2], THEMES[th1][2], kb), float)
    loud = x["loud"]
    bg[200:] += acc*.07*x["pulse"]
    r = 62 + 22*loud
    m = (DISTMAP < r) & (BAY < .16 + .2*loud); bg[m] = bg[m]*.62 + acc*.38
    e = np.abs(DISTMAP - r) < 1.1; bg[e] = bg[e]*.35 + acc*.65
    for age in x["ages"]:
        if age < 1.3:
            rr = 30 + age*150; a = 1 - age/1.3
            m = (np.abs(DISTMAP - rr) < 1.6) & (BAY < a*.85)
            bg[m] = bg[m]*.35 + np.minimum(255, acc*1.15)*.65
    hor = 262
    for i in range(27):
        band = min(13, abs(i - 13)); h = int(4 + 86*x["eq"][band]); x0 = i*10 + 1
        col = np.linspace(.45, 1.0, h)[::-1, None, None]*acc
        bg[hor - h:hor, x0:x0 + 8] = bg[hor - h:hor, x0:x0 + 8]*.25 + col*.75
        bg[hor - h:hor - h + 1, x0:x0 + 8] = np.minimum(255, acc*1.3)
        rh = h//2; refl = BAY[hor:hor + rh, x0:x0 + 8] < .3
        sl = bg[hor:hor + rh, x0:x0 + 8]; sl[refl] = sl[refl]*.6 + acc*.4
    for mx, y0, spd in MOTES:
        y = int(330 - ((y0 + T*10*spd + x["cum"]*40*spd) % 240))
        if 0 <= y < RH: bg[y, mx] = np.minimum(255, acc*1.25)
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])

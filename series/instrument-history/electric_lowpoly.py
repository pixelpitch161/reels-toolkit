"""Pixel Pitch: history of the electric guitar.  python3 electric_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
from scipy.signal import lfilter
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_guitar as G
import piano_lowpoly as P
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR, RW, RH, CX, YY, XX, BAY

OUT, TAG = "electric_lowpoly.mp4", "el"
BPM = 132
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .72, -12, 20, .34
HEAD_INTRO, HEAD, TODAY, START_YEAR = "ELECTRIC GUITAR", "90 YEARS", "TODAY", 1900
INTRO_LINES = (("PICKUPS TURN STRINGS", "INTO ELECTRICITY"), ("BUT HOW DID WE", "GET HERE?"))
OUTRO_LINES = ("90 YEARS FROM", "A FRYING PAN")
OUTRO_CREDIT = "GRIEG: HALL OF THE MOUNTAIN KING"

CH = {"Bm": (47, (54, 59, 62)), "F#": (42, (54, 58, 61)), "A": (45, (57, 61, 64)), "E": (40, (56, 59, 64)),
      "F": (41, (53, 57, 60)), "C7": (36, (55, 58, 64)), "G": (43, (55, 59, 62)), "D": (38, (57, 62, 66)),
      "C": (36, (55, 60, 64)), "Am": (45, (57, 60, 64))}
BARS = ["Bm", "F#", "A", "E", "F", "C7", "G", "D", "C", "Am", "A", "D", "Am", "E", "F#", "Bm"]
GRIEG = seq(0, [(59, .5), (61, .5), (62, .5), (64, .5), (66, .5), (62, .5), (66, 1)], .85) + \
        seq(4, [(65, .5), (61, .5), (65, 1), (64, .5), (60, .5), (64, 1)], .85)
T3 = 2/3
SCENES = [
 dict(key="superstrat", theme="intro", name="MODERN ELECTRIC", credit="GRIEG: HALL OF THE MOUNTAIN KING", notes=GRIEG),
 dict(key="pan", theme="pan", year=1932, ys="1932", name="FRYING PAN", credit="IN THE STYLE OF HAWAIIAN STEEL",
      fact=("THE FRYING PAN: FIRST", "ELECTRIC GUITAR SOLD"), size=1.15,
      notes=seq(0, [(69, 1), (73, 1), (76, 1.5), (74, .5), (73, 1), (71, .5), (69, .5), (68, 1), (69, 1)], .8)),
 dict(key="es150", theme="es150", year=1936, ys="1936", name="GIBSON ES-150", credit="IN THE STYLE OF CHARLIE CHRISTIAN",
      fact=("CHARLIE CHRISTIAN", "PLUGS INTO JAZZ"),
      notes=seq(0, [(65, T3), (69, 1 - T3), (72, T3), (74, 1 - T3), (75, T3), (74, 1 - T3), (72, 1)], .8) +
            seq(4, [(70, T3), (69, 1 - T3), (67, T3), (64, 1 - T3), (65, T3), (67, 1 - T3), (60, 1)], .8)),
 dict(key="tele", theme="tele", year=1950, ys="1950", name="TELECASTER", credit="IN THE STYLE OF COUNTRY TWANG",
      fact=("THE FIRST MASS-MADE", "SOLID-BODY GUITAR"),
      notes=[(0, 67, .5, .8), (.5, 71, .5, .8), (1, 74, .5, .8), (1.5, [71, 74], .5, .8), (2, 76, .5, .8), (2.5, 74, .5, .8),
             (3, 71, 1, .8), (4, 69, .5, .8), (4.5, 66, .5, .8), (5, 69, .5, .8), (5.5, [66, 69], .5, .8), (6, 74, .5, .8),
             (6.5, 72, .5, .8), (7, 71, 1, .8)]),
 dict(key="lespaul", theme="lespaul", year=1952, ys="1952", name="LES PAUL", credit="IN THE STYLE OF LES PAUL",
      fact=("LES PAUL'S NAME ON A", "GOLD SOLID BODY"),
      notes=seq(0, [(72, .25), (74, .25), (76, .25), (79, .25), (81, .5), (79, .5), (76, .5), (74, .5), (72, 1)], .8) +
            seq(4, [(69, .25), (72, .25), (76, .25), (81, .25), (84, .5), (81, .5), (79, .5), (76, .5), (69, 1)], .8)),
 dict(key="strat", theme="strat", year=1954, ys="1954", name="STRATOCASTER", credit="IN THE STYLE OF 1950S ROCK AND ROLL",
      fact=("THREE PICKUPS AND A", "WHAMMY BAR"),
      notes=[(0, [69, 73], .5, .8), (.5, [69, 73], .5, .75), (1, [69, 73], .5, .8), (1.5, [69, 73], .5, .75), (2, 72, .25, .8),
             (2.25, 73, .25, .8), (2.5, 69, .5, .8), (3, [64, 69], 1, .85), (4, [66, 69], .5, .8), (4.5, [66, 69], .5, .75),
             (5, [66, 69], .5, .8), (5.5, [66, 69], .5, .75), (6, 65, .25, .8), (6.25, 66, .25, .8), (6.5, 62, .5, .8),
             (7, [57, 62], 1, .85)]),
 dict(key="superstrat", theme="intro", year=1985, ys="1980s", name="SUPERSTRAT", credit="RIMSKY-KORSAKOV: BUMBLEBEE",
      fact=("FAST NECKS AND FLOYD", "ROSE: THE SHRED ERA"),
      notes=seq(0, [(m, .25) for m in [76, 75, 74, 73, 74, 73, 72, 71, 72, 71, 70, 69, 68, 67, 66, 65]], .85) +
            seq(4, [(m, .25) for m in [64, 65, 64, 63, 64, 65, 64, 63, 64, 65, 66, 67, 68, 69, 70, 71]], .85) +
            seq(8, [(66, .5), (68, .5), (70, .5), (71, .5), (73, .5), (70, .5), (73, 1)], .9) + [(12, [47, 54, 59, 66], 4.3, 1.0)]),
]

# ---------------------------------------------------------------- instruments
PAN = [(u, math.sqrt(max(0, 1 - (2*u - 1)**2))*.5) for u in np.linspace(0, 1, 11)]
ARCH = [(0, .12), (.04, .3), (.14, .37), (.28, .38), (.42, .33), (.5, .29), (.58, .3), (.7, .33), (.82, .31), (.92, .24), (.98, .13), (1, .07)]
TELE = [(0, .15, .15), (.05, .32, .32), (.15, .37, .37), (.3, .36, .36), (.45, .32, .32), (.55, .31, .31), (.65, .33, .32),
        (.75, .32, .26), (.82, .3, .09), (.9, .25, .08), (.97, .15, .08), (1, .08, .08)]
LP = [(0, .14, .14), (.05, .32, .32), (.15, .38, .38), (.3, .37, .37), (.45, .32, .32), (.55, .3, .3), (.65, .33, .33),
      (.75, .32, .28), (.82, .29, .17), (.88, .25, .09), (.96, .15, .08), (1, .08, .08)]
STRAT = [(0, .14, .14), (.05, .32, .32), (.15, .37, .37), (.3, .35, .35), (.45, .3, .3), (.55, .29, .29), (.65, .32, .32), (.75, .31, .31), (.8, .27, .27)]
SIX = [40, 45, 50, 55, 59, 64]; STEEL, WOUND = (222, 224, 232), (196, 198, 208)
SC6 = [WOUND]*3 + [STEEL]*3
MAPLE, ROSE = (234, 202, 142), (40, 26, 22)
def burst(edge, center): return lambda u: mix(edge, center, clamp(1 - abs(u - .45)*2.2))
GC = {
 "pan":    dict(L=.5, d=.08, prof=PAN, top=(206, 210, 218), side=(150, 154, 164), neck=.95, fb0=.95, fbw=(.06, .055), fbcol=(42, 42, 50),
                frets=17, fret_col=(170, 170, 184), inlay_col=(230, 230, 240), head="pan", headcol=(190, 194, 204),
                strings=[45, 52, 57, 61, 64, 69], scol=SC6, sb=.06, sn=.034, slide=True,
                pickups=[(.5, "horseshoe", (226, 228, 236))], bridge="pan", bridge_u=.24, pluck_u=.36),
 "es150":  dict(L=.95, d=.16, prof=ARCH, top=(214, 140, 56), colfn=burst((62, 28, 14), (218, 146, 60)), side=(62, 28, 14),
                neck=.56, fb0=.72, fbcol=ROSE, frets=19, head="gibson", headcol=(24, 20, 20), strings=SIX, scol=SC6, sb=.07, sn=.036,
                hole=dict(type="f"), pickups=[(.38, "blade", None)], bridge="trapeze", bridge_u=.26, pluck_u=.44,
                guards=[([(.1, .5), (.27, .48), (.32, .6), (.2, .72), (.1, .66)], (64, 32, 22))], knobs=[(.22, .17), (.29, .24)],
                knob_col=(30, 24, 20)),
 "tele":   dict(L=.8, d=.09, prof=TELE, top=(234, 192, 112), side=(204, 152, 82), neck=.68, fb0=.82, fbcol=MAPLE, inlay_col=(30, 24, 22),
                frets=21, head="fender", headcol=MAPLE, strings=SIX, scol=SC6, sb=.075, sn=.034,
                guards=[([(-.08, .32), (.08, .3), (.1, .44), (.07, .82), (-.22, .8), (-.26, .66), (-.22, .45)], (24, 22, 26))],
                pickups=[(.62, "single", (216, 216, 224)), (.24, "single", (30, 30, 34))], bridge="tele", bridge_u=.2, pluck_u=.32,
                knobs=[(.22, .2), (.25, .1)]),
 "lespaul": dict(L=.78, d=.12, prof=LP, top=(224, 186, 82), side=(120, 64, 30), neck=.66, fb0=.82, fbcol=ROSE, frets=22,
                 head="gibson", headcol=(24, 20, 20), strings=SIX, scol=SC6, sb=.072, sn=.034,
                 guards=[([(.1, .46), (.26, .46), (.3, .6), (.2, .78), (.09, .7)], (232, 216, 172))],
                 pickups=[(.63, "hum", (232, 216, 170)), (.33, "hum", (232, 216, 170))], bridge="stop", bridge_u=.22, pluck_u=.36,
                 knobs=[(.22, .14), (.3, .2), (.18, .24), (.26, .3)], knob_col=(226, 196, 110), sparkle=False),
 "strat":  dict(L=.8, d=.09, prof=STRAT, top=(220, 150, 40), colfn=burst((44, 18, 10), (222, 152, 42)), side=(44, 18, 10),
                horns=[[(-.27, .79), (-.08, .79), (-.09, .95), (-.15, 1.08), (-.22, 1.02), (-.26, .9)],
                       [(.08, .79), (.27, .79), (.25, .88), (.18, .97), (.12, .93)]], horn_col=(70, 30, 14), neck_u=.82,
                neck=.7, fb0=.84, fbcol=MAPLE, inlay_col=(30, 24, 22), frets=21, head="fender", headcol=MAPLE, strings=SIX, scol=SC6,
                sb=.075, sn=.034, guards=[([(-.2, .78), (.08, .82), (.12, .6), (.24, .3), (.18, .15), (-.06, .22), (-.22, .45)], (242, 240, 232))],
                pickups=[(.64, "single", (240, 238, 228)), (.46, "single", (240, 238, 228)), (.28, "single", (240, 238, 228))],
                bridge="trem", bridge_u=.2, pluck_u=.38, knobs=[(.17, .3), (.21, .22), (.23, .14)], knob_col=(240, 238, 228)),
 "superstrat": dict(L=.78, d=.09, prof=STRAT, top=(255, 238, 70), side=(210, 186, 36),
                horns=[[(-.27, .79), (-.08, .79), (-.08, .98), (-.13, 1.18), (-.2, 1.08), (-.26, .92)],
                       [(.08, .79), (.27, .79), (.24, .92), (.16, 1.06), (.11, .98)]], horn_col=(236, 214, 40), neck_u=.82,
                neck=.7, fb0=.84, fbcol=(30, 22, 20), inlay_col=(240, 240, 240), frets=24, head="shark", headcol=(248, 228, 52),
                strings=SIX, scol=SC6, sb=.075, sn=.034, pickups=[(.6, "hum", (24, 24, 28)), (.28, "hum", (24, 24, 28))],
                bridge="floyd", bridge_u=.2, pluck_u=.36, knobs=[(.22, .17)], knob_col=(24, 24, 28)),
}
def build(g, t, ev, fx, s): G.build_guitar(g, t, ev, fx, s, GC[s["key"]])

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def amp(x, drive, tone=.5):
    if drive > 0: x = np.tanh(x*drive)/np.tanh(drive)
    for _ in range(2): x = lfilter([1 - tone], [1, -tone], x)
    return x
def damp(x, dur, rel=.06):
    t = _t(len(x)); return x*np.where(t < dur + .03, 1.0, np.exp(-(t - dur - .03)/rel))
def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]; f = midi_hz(m)
    if key == "pan":
        prev = midi_hz(s["ev"][idx-1][1]) if idx > 0 and nt - s["ev"][idx-1][0] < 2 else f
        n = int((dur + .3)*SR); t = _t(n)
        fr = f*((prev/f)**(1 - np.clip(t/.09, 0, 1)))*(1 + .006*np.sin(2*np.pi*5.5*t)*np.clip((t - .15)/.2, 0, 1))
        ph = 2*np.pi*np.cumsum(fr)/SR
        x = np.sin(ph) + .35*np.sin(2*ph) + .18*np.sin(3*ph) + .08*np.sin(4*ph)
        env = np.minimum(1, t/.01)*np.exp(-t*.8)*np.where(t < dur, 1, np.exp(-(t - dur)/.08))
        return amp(x*env*.6, 1.4, .35)*v
    if key == "es150": x = damp(amp(P.ks_pluck(f, 1.2, .45, .997, idx)*1.2, 1.2, .55), dur)
    elif key == "tele": x = damp(amp(P.ks_pluck(f, 1.0, 1.0, .9975, idx)*1.5, 1.8, .22), dur)
    elif key == "lespaul": x = damp(amp(P.ks_pluck(f, 1.2, .9, .998, idx)*3, 3.0, .45), dur)
    elif key == "strat": x = damp(amp((P.ks_pluck(f, 1.2, .95, .998, idx) + .4*P.ks_pluck(f*1.004, 1.2, .95, .998, idx + 9))*1.4, 2.0, .35), dur)
    else:
        x = P.ks_pluck(f, max(dur + .3, .5), 1.0, .999, idx)
        x = damp(amp(x*7, 7.0, .5), dur if cs < 3 else 2.4, .05)
    if cs >= 3:
        same = sorted(e[1] for e in s["ev"] if abs(e[0] - nt) < 1e-6)
        x = np.concatenate([np.zeros(int(.008*same.index(m)*SR)), x])
    return x*v

def tom(f0):
    def fn(v):
        n = int(.35*SR); t = _t(n); fr = f0*(1 + .4*np.exp(-t*20))
        return np.sin(2*np.pi*np.cumsum(fr)/SR)*np.exp(-t*9)*v
    return fn
KIT = {"kick": lambda v: P.kick(v)*1.1, "snare": lambda v: P.snare(v)*1.3, "hat": P.hat, "ohat": lambda v: P.hat(v, True),
       "crash": P.crash, "tom0": tom(200), "tom1": tom(160), "tom2": tom(125), "tom3": tom(95)}
KICK_KINDS, ACCENT_KINDS = ("kick",), ("snare",)
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT
    for b in range(16):
        T0 = b*B
        if b == 15: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "snare", 1.)]; continue
        if b == 0:
            ev += [(T0 + k*.5*Q, "hat", .45) for k in range(8)] + [(T0, "kick", .9), (T0 + 2*Q, "kick", .9)]
            continue
        ev += [(T0, "kick", 1.), (T0 + 2*Q, "kick", .95), (T0 + 2.5*Q, "kick", .8), (T0 + Q, "snare", 1.), (T0 + 3*Q, "snare", 1.)]
        ev += [(T0 + k*.5*Q, "hat", .6 if k % 2 == 0 else .4) for k in range(8) if k != 7] + [(T0 + 3.5*Q, "ohat", .5)]
        if b % 2 == 0: ev.append((T0, "crash", .75))
        if b == 13: ev += [(T0 + (3 + k*.25)*Q, f"tom{k}", .8) for k in range(4)]
    return ev
def bass_voice(m, dur, v):
    f = midi_hz(m); x = P.ks_pluck(f, dur + .1, .55, .998, int(m))
    x = lfilter([.25], [1, -.75], x); return np.tanh(2.2*x)*v
def bass_events(c):
    out = []
    for b in range(1, 16):
        r = CH[BARS[b]][0]; r = r if r < 41 else r - 12; T0 = b*c.BAR
        if b == 15: out.append((T0, r, 2.6, 1.)); continue
        out += [(T0 + k*.5*c.BEAT, r, .45*c.BEAT, .9 if k % 2 == 0 else .75) for k in range(8)]
    return out
pad_voice = None
MIX = dict(drums=-21, bass=-24, verb=-25, verb_len=1.2)

# ---------------------------------------------------------------- background: neon synthwave grid
THEMES = {   # sky top, horizon glow, neon grid, sun top, sun bottom
 "intro":   ((10, 4, 30), (150, 30, 120), (0, 240, 255), (255, 226, 90), (255, 60, 170)),
 "pan":     ((4, 18, 30), (30, 120, 130), (120, 255, 230), (230, 255, 240), (70, 200, 200)),
 "es150":   ((20, 10, 4), (140, 72, 20), (255, 190, 90), (255, 234, 170), (220, 110, 40)),
 "tele":    ((30, 6, 10), (176, 52, 30), (255, 150, 70), (255, 240, 130), (255, 80, 40)),
 "lespaul": ((4, 20, 12), (40, 120, 64), (190, 255, 130), (255, 240, 150), (140, 220, 90)),
 "strat":   ((6, 6, 38), (70, 44, 160), (160, 130, 255), (210, 230, 255), (130, 100, 255)),
}
HY = 240
SKY = {k: F.make_grad(v[0], v[1], y0=0, y1=HY) for k, v in THEMES.items()}
def ped_cols(th): return F.ped_cols_from(mix(THEMES[th][0], THEMES[th][1], .35))
_rr = np.random.default_rng(14)
STARS = [(int(_rr.integers(0, RW)), int(_rr.integers(4, 170)), _rr.random()*6.28) for _ in range(45)]
XS = np.arange(RW)
BANDX = np.minimum(13, (np.abs(XS - CX)/CX*13).astype(int))
SCAN = (YY % 3 == 0)

def background(c, x):
    th0, th1, kb = x["th0"], x["th1"], x["kb"]; T = x["T"]
    A, B = THEMES[th0], THEMES[th1]
    top, hor, neon, s_top, s_bot = (np.array(mix(A[i], B[i], kb), float) for i in range(5))
    bg = SKY[th0]*(1 - kb) + SKY[th1]*kb
    bg[HY:] = top*.55
    for sx, sy, ph in STARS:
        b = .5 + .5*math.sin(T*2.6 + ph)
        if b > .45: bg[sy, sx] = bg[sy, sx]*(1 - b) + np.array([240, 240, 255])*b
    cy = 214; r = 58 + 6*x["loud"] + 3*x["pulse"]
    d = np.hypot(XX - CX, YY - cy)
    inside = (d < r) & (YY < HY)
    k = np.clip((YY - (cy - r))/(2*r), 0, 1)
    sun = s_top[None, None, :]*(1 - k[..., None]) + s_bot[None, None, :]*k[..., None]
    rel = YY - cy
    period = 9; gap = np.clip((rel/r)*5, 0, 5)
    stripe = (rel > 0) & (((rel + T*8) % period) < gap)
    m = inside & ~stripe
    bg[m] = sun[m]
    halo = (d >= r) & (d < r + 10) & (BAY < .3*(1 - (d - r)/10)) & (YY < HY)
    bg[halo] = bg[halo]*.5 + s_bot*.5
    eq = x["eq"][BANDX]
    h = (8 + 34*eq)*(.55 + .45*np.abs(np.sin(XS*.085 + 1.0)))
    ridge = (HY - h).astype(int)
    mtn = (YY >= ridge[None, :]) & (YY < HY)
    bg[mtn] = top*.85
    bg[(YY == ridge[None, :]) & (YY < HY)] = neon
    bg[SCAN] *= .88
    im = Image.fromarray(np.clip(bg, 0, 255).astype(np.uint8)); dr = ImageDraw.Draw(im)
    gl = tuple(int(v) for v in np.minimum(255, neon*(.55 + .45*x["pulse"])))
    gd = tuple(int(v) for v in neon*.45)
    speed = .9 + 1.6*x["loud"]
    ph = (T*speed + x["cum"]*.6) % 1
    for i in range(12):
        z = (i + ph)/12
        y = int(HY + (RH - HY)*z**2.2)
        dr.line([(0, y), (RW, y)], fill=gl if z > .25 else gd)
    for i in range(-10, 11):
        dr.line([(CX + i*6, HY), (CX + i*60, RH)], fill=gd)
    dr.line([(0, HY), (RW, HY)], fill=gl)
    if x["accent"] > .2:
        a = x["accent"]; col = tuple(int(v) for v in np.minimum(255, s_bot*a + top*(1 - a)))
        dr.line([(0, HY), (CX - 40, 96)], fill=col); dr.line([(RW, HY), (CX + 40, 96)], fill=col)
    return np.asarray(im).copy()

if __name__ == "__main__":
    F.run(sys.modules[__name__])

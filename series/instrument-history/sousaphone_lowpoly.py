"""Pixel Pitch: the modern marching sousaphone (BB-flat, fiberglass bell) - low-poly model. No history video yet: built
for BALLFIGHT #53 (Flute vs Sousaphone), on the tuba video's tube / bell helpers (tuba60_lowpoly, used read-only as a
library) and used through projects/which_instrument_wins/pp_bridge.py ('SOUSAPHONE').

    python3 sousaphone_lowpoly.py            # face count + extent per scene

Reference photos (Claude in Chrome, 2026-10-30, Wikimedia Commons): "Yamaha Sousaphone YSH-411.jpg" (a brass
sousaphone, three-quarter front, on white) and "Sousaphones in marching band Freedom Festival 2011 Provo UT July
2011.jpg" (a line of them worn on the march). What makes it a sousaphone and not a tuba, measured there (H = the whole
instrument's height, the bottom of the wrap to the top of the bell rim):
  - the WRAP: one big, slightly oval loop of tubing (~0.62 H wide, ~0.52 H tall), worn round the player - its plane
    leans ~22 deg (left shoulder down to the right hip on the march), the tubing widening all the way round;
  - the GOOSENECK: the loop's top end rises over the shoulder, then bends a quarter turn so the bell faces straight
    ahead, above the player's head;
  - the BELL: huge and forward-facing, the rim ~0.50 H across (as wide as the wrap), its centre ~0.25 H below the
    top; a shallow, wide flare (a short cone that opens fast), the throat dark;
  - the VALVES: three top-action pistons at the front of the loop at waist height, leaning back toward the player, the
    tuning-slide loops hanging under them inside the wrap; the leadpipe climbs from them to a mouthpiece in front of the
    face (inside the top of the loop).
Finish: the marching-band kind - a white fiberglass bell and gooseneck on a lacquered brass wrap and valve set, a brass
band round the bell rim. Plain: no school letters or logos on the bell (as the brief asks).
Local model frame: y up, +z toward the camera (the way the bell faces), +x to the player's left (the bell's side).
"""
import math
import sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import tuba60_lowpoly as TU                          # read-only: path(), flare(), mouthpiece(), fing_of(), brass voice
import trumpet_lowpoly as T                          # read-only: tube(), bend(), brass()
import pp_shapes as S

OUT, TAG = "sousaphone_lowpoly.mp4", "su"
BPM = 126
SHOW_BARS = 2
HEAD_INTRO = "THE SOUSAPHONE"

_NOTES = [(0, 34, .5, .9), (.5, 41, .5, .85), (1, 34, .5, .9), (1.5, 41, .5, .85), (2, 36, .5, .9), (2.5, 38, .5, .85),
          (3, 39, 1, .9)]
SCENES = [
    dict(key="modern", theme="intro", name="SOUSAPHONE", credit="MARCHING BAND", notes=_NOTES),
    # the same horn for small sprites (BALLFIGHT balls): every tube's radius x `chunk`, the valve caps bigger
    dict(key="ball", theme="intro", name="SOUSAPHONE", credit="MARCHING BAND", notes=_NOTES, chunk=1.45),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


LACQUER = (232, 184, 84)           # the brass wrap, valves and slides
FIBER = (242, 240, 232)            # the white fiberglass bell + gooseneck
RIMB = (214, 168, 74)              # the brass band round the rim
NICKEL, PEARL, HOT = TU.NICKEL, TU.PEARL, TU.HOT
THROAT = (30, 20, 16)

LEAN = math.radians(22)            # the wrap's plane leans (left shoulder down to the right hip)


def _lean(p):
    """rotate a point of the upright wrap about the z axis (through the wrap's centre) by LEAN"""
    x, y, z = p
    cx, cy = 0.0, .40
    c, s = math.cos(LEAN), math.sin(LEAN)
    dx, dy = x - cx, y - cy
    return (cx + c * dx - s * dy, cy + s * dx + c * dy, z)


def sousa(g, m, chunk=1.0):
    k = chunk
    # the wrap: one oval loop, starting low at the front by the valves and going round (up the player's right side,
    # over the back, down the left), ending at its top-left where the gooseneck leaves; the tube widens all the way
    cx, cy, rx, ry = 0.0, .40, .31, .27
    ang = np.linspace(-80, 430, 44)                       # degrees, ccw from +x: ~1.4 turns, ending at the top
    pts = []
    for i, a in enumerate(ang):
        u = i / (len(ang) - 1)
        ar = math.radians(a)
        inner = min(1.0, (a + 80) / 300.0)                    # the first part runs a little inside and behind the wrap
        sc = .80 + .20 * inner * inner * (3 - 2 * inner)
        z = .07 * math.sin(ar) - .03 - .07 * (1 - inner)
        pts.append(_lean((cx + rx * sc * math.cos(ar), cy + ry * sc * math.sin(ar), z)))
    TU.path(g, pts, .017 * k, LACQUER, r1=.052 * k, n=10)
    # the gooseneck: up over the shoulder from the wrap's end, then a quarter turn to face forward (fiberglass)
    p0 = np.array(pts[-1])
    neck = [p0, p0 + [.012, .03, -.01], p0 + [.016, .065, -.03], p0 + [.008, .095, -.06]]
    TU.path(g, neck, .052 * k, FIBER, r1=.066 * k, n=12, spec=.35)
    k0 = neck[-1] + np.array([0, 0, .13])
    bend = [k0 + .13 * np.array([0, math.sin(a), -math.cos(a)]) for a in np.linspace(0, math.pi / 2, 9)]
    TU.path(g, bend, .066 * k, FIBER, r1=.085 * k, n=12, spec=.35)
    # the bell: a short wide cone that opens fast, rim ~0.5 H across, a brass band round it, the throat dark
    rim = TU.flare(g, bend[-1], (0, 0, 1), .30, .085 * k, .29, FIBER, x0=.10, m=1.05, r_mid=.10 * k, rim_col=RIMB)
    # the valves: three top-action pistons at the front of the loop, leaning back toward the player
    on = TU.fing_of(m, 3)
    base = np.array(_lean((-.03, .30, .10)))
    lean_v = np.array([0.0, math.cos(math.radians(28)), -math.sin(math.radians(28))])
    for i in range(3):
        c = base + np.array([.055 * i, -.012 * i, 0])
        down = i in on
        T.tube(g, c - lean_v * .05, c + lean_v * .05, .021 * min(k, 1.25), col=LACQUER, n=10)
        for e in (-.05, .05):
            T.tube(g, c + lean_v * (e - .006), c + lean_v * (e + .006), .025 * min(k, 1.25), col=NICKEL, n=10)
        top = c + lean_v * (.09 - (.016 if down else 0))
        T.tube(g, c + lean_v * .05, top, .004 * k, col=NICKEL, n=6)
        T.tube(g, top, top + lean_v * .012, .017 * min(k, 1.3), col=HOT if down else PEARL, n=10, spec=.4)
        lo = c - lean_v * .05                                   # the valve slide hanging under it, inside the wrap
        for dx in (-.011, .011):
            T.tube(g, lo + [dx, 0, -.01], lo + [dx, -.055 - .018 * i, -.03], .0075 * k, col=LACQUER, n=6)
        T.bend(g, tuple(lo + [0, -.055 - .018 * i, -.03]), .011, .0075 * k, 180, 360, col=LACQUER, segs=4)
    # the main tuning slide: a long U across the bottom of the loop
    a = np.array(pts[0])
    TU.path(g, [a, a + [.07, -.02, .03], base + [.14, -.06, 0], base + [.12, .0, 0]], .022 * k, LACQUER)
    # the leadpipe: from the first valve up inside the loop to the mouthpiece in front of the face
    mp = np.array(_lean((.03, .62, .15)))
    TU.path(g, [base + [0, .03, .02], base + [-.04, .12, .05], (mp + base) / 2 + [-.03, 0, .04], mp], .012 * k,
            LACQUER, r1=.009 * k)
    TU.mouthpiece(g, mp, (.25, .35, -.6), r=.008 * min(k, 1.3))
    # bell braces from the wrap's top to the gooseneck (two little struts)
    for q in (pts[-6], pts[-4]):
        T.tube(g, q, neck[2] + [0, -.02, 0], .006 * k, col=NICKEL, n=6)
    return np.asarray(rim, float)


POSE = None


def build(g, t, ev, fx, s):
    m = TU.cur(s, t, ev)
    rim = sousa(g, m, chunk=s.get("chunk", 1.0))
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(rim + [.32, -.1, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(rim + [.04, .02, .05]), age, 0))


# ---------------------------------------------------------------- sound: the marching sousaphone's punch
# Starts from the tuba video's voice (trumpet_lowpoly.brass + the extra fundamental, as the TUBA's bridge voice), made
# for the street: brighter (the fiberglass bell throws the upper harmonics forward), a marcato attack - fast (12 ms),
# a short accent bump that settles to ~80 %, and a tongued "tah" (a noisy upper-harmonic burst in the first 30 ms) -
# and short notes cut off clean so the line stays tight with the drumline. `blat` (0..1): the bell-blast accent - a
# bigger scoop and bump, a lip buzz (a fast rasp on the upper partials) and more top; 0 = a plain marching bass note.
from pp_framework import midi_hz, SR                 # noqa: E402


def sousa_tone(f, dur, v, blat=0.0, bright=.9):
    n = int((dur + .14) * SR)
    t = np.arange(n) / SR
    fr = f * (1 - (.018 + .05 * blat) * np.exp(-t / (.02 + .02 * blat)))         # the drop into the note
    ph = 2 * np.pi * np.cumsum(fr) / SR
    x = np.zeros(n)
    tot = 0.0
    br = bright + .5 * blat
    for k in range(1, 40):
        fk = f * k
        if fk > 8000:
            break
        a = k ** -.8 * math.exp(-k / (4.0 + 7.0 * br * v))
        x += a * np.sin(k * ph + .21 * k)
        tot += a
    x /= tot + 1e-9
    buzz = np.zeros(n)
    if blat:                                                                     # lip buzz on the upper partials
        for k in range(5, 14):
            buzz += np.sin(k * ph) / k
        buzz *= (1 + np.sin(2 * np.pi * 38 * t)) * .5 * blat * .55
    x = np.tanh((x + buzz) * (1.2 + 1.3 * blat)) / np.tanh(1.2 + 1.3 * blat)       # a little edge, more on a blat
    body = np.sin(2 * np.pi * np.cumsum(fr) / SR)                                # the round low end (as the TUBA)
    x = x + .38 * body
    rel = .025 if dur < .3 else .05
    env = np.minimum(1, t / .012) ** 1.2 * np.where(t < dur, 1, np.exp(-(t - dur) / rel))
    env *= .8 + (.35 + .3 * blat) * np.exp(-t / .07)                               # the accent bump, settling
    rng = np.random.default_rng(int(f * 7 + dur * 997) % 100003)
    tah = np.convolve(rng.standard_normal(n), np.ones(6) / 6, 'same') - np.convolve(rng.standard_normal(n),
                                                                                  np.ones(40) / 40, 'same')
    x = x + tah * .16 * (1 + blat) * np.exp(-t / .012)                             # the tongued "tah"
    return x * env * v * .8


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return sousa_tone(midi_hz(m), dur, v, blat=.25)


if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], dict(sc, ev=[(0.0, 34, .9, .5, True, 1)]))
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces", len(g.L), "lines, extent (m):", np.round(V.max(0) - V.min(0), 3))

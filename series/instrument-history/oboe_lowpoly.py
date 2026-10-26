"""Pixel Pitch: the modern oboe - low-poly model (and, below, its voice). No history video yet: built for BALLFIGHT #11
(Clarinet vs Oboe) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the other
instruments' modern showcase models are. A later Pixel Pitch oboe video can grow from this file (add SCENES rows,
backdrop, music) without touching the model.

    python3 oboe_lowpoly.py                # face count + extent (sprites: projects/which_instrument_wins/sprites.py)

Model from reference photos (Wikimedia Commons, found in the browser, 2026-10-26):
  - "Yamaha Oboe YOB-831.tif" (CC BY-SA 4.0): a straight-on front product shot, 863 x 6937 px, measured at 590 mm body
    length (0.147 mm per px at 500 px wide): crown bulb 25 mm across and 21 mm tall with a silver ring under it; upper
    joint 16.8 -> ~21 mm, its tenon ring at 39.5 % of the body; lower joint ~21 -> 26 mm, the bell ring at 79 %; the
    bell 28 mm at the ring, 32 mm low down, then the oboe's rounded bulb at the rim, 49 mm across and ~22 mm tall.
    Keywork: open plates down the front (the first one perforated: the half-hole plate), long rods down both sides,
    big teardrop pinky spatulas near the joint (left hand) and at ~56 % (right hand), two key cups on the bell.
  - "Oboe modern.jpg" (Lorée, Paris; GFDL / CC BY-SA 3.0): the same layout at three quarters, with a reed in: green
    thread over the staple, pale cane blades.
  - "Anches hautbois conservatoire et baroque.jpg" (Commons): a modern reed against a ruler: 67 mm long (blades 22 mm,
    tip 6.7 mm wide narrowing to 4.2 mm at the thread; thread wrap 15 mm; cork 29 mm, mostly inside the oboe)."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
from pp_framework import lerp, dim, mix

OUT, TAG = "oboe_lowpoly.mp4", "ob"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE OBOE"

# one scene for now: the modern conservatoire oboe (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 69, 2, .85), (2, 72, 1, .85), (3, 76, 1, .85), (4, 74, 2, .85), (6, 69, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="MODERN OBOE", credit="CONSERVATOIRE OBOE", notes=_NOTES),
    # the same oboe for small sprites (BALLFIGHT balls, ~25-90 px long): true lengths, every radius x `chunk` about the
    # bore (body, bell, keys and reed together), so the black body, the silver plates and the reed still read as more
    # than a one-pixel line. A Pixel Pitch video would use "modern".
    dict(key="ball", theme="intro", name="MODERN OBOE", credit="CONSERVATOIRE OBOE", notes=_NOTES, chunk=1.6),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- the instrument (real proportions, metres)
# local frame: x = the bore axis from the reed tip (x = 0) to the bell rim, the keys face +z, +y = the player's right
BLACK = (30, 24, 28)            # grenadilla: near black with a warm cast (the Yamaha's wood reads dark brown up close)
NICKEL = (214, 216, 226)        # silver-plated keywork
CANE = (226, 204, 148)          # the reed's scraped cane blades
CANE_D = (196, 168, 110)
THREAD = (46, 132, 74)          # the thread wrap (green, as on the Lorée's reed)
CORK = (196, 156, 106)
HOLE = (14, 10, 12)

REED = .042                     # the reed's visible length above the crown: blades 22, thread 15, cork 5 mm
BODY = .590                     # crown top to bell rim
X_TEN, X_BELL = REED + .395 * BODY, REED + .79 * BODY     # tenon ring, bell ring (39.5 %, 79 % of the body)
X_END = REED + BODY
FZ = np.array([0, 0, 1.])


def unit(v):
    v = np.asarray(v, float)
    return v / np.linalg.norm(v)


def lathe(g, prof, col, n=10, spec=.4, two=False):
    """a turned profile along x: prof = [(x, r), ...]"""
    for (x0, r0), (x1, r1) in zip(prof, prof[1:]):
        T.tube(g, (x0, 0, 0), (x1, 0, 0), r0, r1, col=col, n=n, spec=spec, two=two)


def ring_band(g, x, r, w=.005, col=NICKEL):
    T.tube(g, (x, 0, 0), (x + w, 0, 0), r, col=col, n=12, spec=.85)


def body_r(x):
    """outer radius of the conical body at x (crown and bell excluded): 8.4 mm under the crown -> 10.5 mm at the tenon ->
    13 mm at the bell ring (the photo's 16.8 / ~21 / 26 mm)"""
    if x < X_TEN:
        return lerp(.0084, .0105, (x - (REED + .021)) / (X_TEN - REED - .021))
    return lerp(.0107, .013, (x - X_TEN) / (X_BELL - X_TEN))


def reed(g):
    """two cane blades pressed together: flat and wide at the tip (6.7 mm), round where the thread binds them (4.2 mm);
    the green thread wrap; a sliver of cork at the crown"""
    T.tube(g, (REED - .006, 0, 0), (REED + .004, 0, 0), .0031, .0031, col=CORK, n=8, spec=.1)
    T.tube(g, (.022, 0, 0), (REED - .006, 0, 0), .00225, .0028, col=THREAD, n=8, spec=.3)
    for x in (.026, .030, .034):                                              # wraps of thread
        T.tube(g, (x, 0, 0), (x + .0012, 0, 0), .0029, col=dim(THREAD, .75), n=8, spec=.2)
    secs = [(.0, .00335, .00055), (.006, .0032, .0008), (.012, .0029, .0012), (.018, .0025, .0017), (.022, .0021, .0021)]
    m = 10
    ring = lambda x, a, b: [np.array([x, a * math.cos(2 * math.pi * k / m), b * math.sin(2 * math.pi * k / m)]) for k in range(m)]
    for (x0, a0, b0), (x1, a1, b1) in zip(secs, secs[1:]):
        A, B = ring(x0, a0, b0), ring(x1, a1, b1)
        for k in range(m):
            col = CANE if math.sin(2 * math.pi * (k + .5) / m) > 0 else CANE_D
            g.face([A[k], A[(k + 1) % m], B[(k + 1) % m], B[k]], col, ref=((x0 + x1) / 2, 0, 0), spec=.2)
    g.face(ring(0.0, .00335, .00055), CANE, ref=(.01, 0, 0))                  # the tip
    g.line((0.0, -.0031, 0), (0.0, .0031, 0), HOLE)                            # the opening between the blades


def crown(g):
    """the top of the upper joint: a turned bulb (25 mm x 21 mm), a silver ring at the top and one under it"""
    x0 = REED
    prof = [(x0, .0088), (x0 + .003, .0108), (x0 + .007, .0124), (x0 + .012, .0125), (x0 + .016, .0113),
            (x0 + .019, .0094), (x0 + .021, .0086)]
    lathe(g, prof, BLACK, n=12, spec=.5)
    g.face(S.ring((x0, 0, 0), (1, 0, 0), .0088, 12), BLACK, ref=(x0 + .01, 0, 0))
    ring_band(g, x0 - .0015, .0084, .002)
    ring_band(g, x0 + .021, .0091, .002)


def bell(g):
    """28 mm at the ring, 32 mm low down, then the rounded bulb at the rim (49 mm across, ~22 mm tall), open at the end"""
    x0 = X_BELL + .006
    prof = [(x0, .0140), (x0 + .03, .0142), (x0 + .06, .0152), (x0 + .085, .0170), (x0 + .093, .0198), (x0 + .100, .0232),
            (x0 + .106, .0245), (x0 + .111, .0238), (X_END - .002, .0212), (X_END, .0195)]
    lathe(g, prof, BLACK, n=14, spec=.45, two=True)
    g.face(S.ring((X_END - .004, 0, 0), (1, 0, 0), .0150, 12), HOLE, ref=(x0, 0, 0), emit=True, two=True)


def plate(g, x, side=0.0, r=.0050, perf=False, col=NICKEL, h=.0022):
    """an open key plate on the body surface (side = angle from the front, +z, toward +y)"""
    d = np.array([0, math.sin(side), math.cos(side)])
    c = np.array([x, 0, 0]) + d * (body_r(x) + .0008)
    top = S.prism(g, c, d, r, h, 10, col, dim(col, .8))
    if perf:                                                                    # the half-hole plate's perforation
        g.face(S.ring(top + d * .0002, d, r * .32, 6), HOLE, ref=c, emit=True)
    else:
        g.face(S.ring(top + d * .0002, d, r * .55, 8), mix(col, (255, 255, 255), .25), ref=c)
    return top


def knob(g, x, side, r=.0026, lift=.0035, col=NICKEL):
    """a small round key touch / pillar head off the body"""
    d = np.array([0, math.sin(side), math.cos(side)])
    p = np.array([x, 0, 0]) + d * (body_r(x) + lift)
    S.prism(g, p, d, r, .0022, 6, col, dim(col, .8))
    T.tube(g, np.array([x, 0, 0]) + d * body_r(x) * .9, p, .0009, col=dim(col, .85), n=4, spec=.6)


def rod(g, x0, x1, side, lift=.0032, r=.0011, col=NICKEL):
    d = np.array([0, math.sin(side), math.cos(side)])
    a = np.array([x0, 0, 0]) + d * (body_r(x0) + lift)
    b = np.array([x1, 0, 0]) + d * (body_r(x1) + lift)
    T.tube(g, a, b, r, col=col, n=5, spec=.8)
    for x in (x0, x1):                                                          # posts at each end
        p = np.array([x, 0, 0]) + d * body_r(x) * .95
        T.tube(g, p, p + d * (lift + .001), .0012, col=dim(col, .9), n=4, spec=.7)


def spatula(g, x, side, out=.016, ang=25.0, L=.026, w=.0065, col=NICKEL):
    """a big teardrop pinky spatula: an arm off the side of the body, then a flat touch angled back along the bore"""
    d = np.array([0, math.sin(side), math.cos(side)])
    base = np.array([x, 0, 0]) + d * body_r(x)
    tip = base + d * out
    T.tube(g, base, tip, .0012, col=col, n=4, spec=.8)
    a = math.radians(ang)
    ax = unit(np.array([math.cos(a), 0, 0]) + d * math.sin(a))                # leaning along +x and outward
    n_ = FZ * .8 + d * .2                                                       # the touch faces the front (fingers)
    n_ = unit(n_ - ax * (n_ @ ax))
    m = 8
    pts = []
    for k in range(m):
        u = 2 * math.pi * k / m
        rr = w * (.55 + .45 * math.cos(u))                                     # teardrop: fat end at the tip
        pts.append(tip + ax * (L * .5 * (1 + math.cos(u)) * .9) + np.cross(n_, ax) * rr * math.sin(u))
    g.face(pts, col, ref=tip - n_, spec=.9, two=True)
    g.face([p + n_ * .0016 for p in pts], mix(col, (255, 255, 255), .2), ref=tip, spec=.9)


def keys(g):
    """the conservatoire keywork, simplified to what reads at sprite size: front plates (the first perforated), the
    long side rods, the octave lever, pinky spatula clusters near the tenon and on the lower joint, bell keys"""
    xu0, xu1 = REED + .021, X_TEN
    xl0, xl1 = X_TEN + .006, X_BELL
    # upper joint: the octave keys' long lever up the right side, a thumb plate behind
    rod(g, xu0 + .012, xu0 + .075, .95, lift=.0034)
    knob(g, xu0 + .014, .55, r=.0032)
    knob(g, xu0 + .045, .25, r=.0024)
    plate(g, xu0 + .028, math.pi, r=.0042)                                      # thumb plate (back)
    for f, perf in ((.30, True), (.50, False), (.70, False)):                   # 1st (half-hole) .. 3rd finger
        plate(g, lerp(xu0, xu1, f), 0.0, r=.0052, perf=perf)
    for f in (.40, .60, .80):                                                   # small trill / side touches
        knob(g, lerp(xu0, xu1, f), -.85, r=.0022)
    rod(g, xu0 + .02, xu1 - .008, 1.05)                                          # rods down both sides
    rod(g, xu0 + .035, xu1 - .008, -1.05)
    spatula(g, xu1 - .030, 1.25, out=.015, ang=22, L=.024)                      # left little finger (3 touches)
    spatula(g, xu1 - .018, 1.55, out=.019, ang=28, L=.026)
    spatula(g, xu1 - .006, 1.10, out=.012, ang=18, L=.020)
    ring_band(g, X_TEN, .0117, .006)                                           # the tenon ring
    # lower joint
    for f in (.22, .42, .60):                                                   # 4th .. 6th finger plates
        plate(g, lerp(xl0, xl1, f), 0.0, r=.0056)
    for f in (.32, .52):
        knob(g, lerp(xl0, xl1, f), .9, r=.0022)
    rod(g, xl0 + .01, xl1 - .01, 1.05)
    rod(g, xl0 + .02, xl1 - .03, -1.05)
    spatula(g, lerp(xl0, xl1, .70), -1.30, out=.016, ang=24, L=.025)          # right little finger (C, Eb)
    spatula(g, lerp(xl0, xl1, .76), -1.62, out=.020, ang=30, L=.027)
    plate(g, lerp(xl0, xl1, .88), -.35, r=.0050)                               # low C# / Eb cups
    ring_band(g, X_BELL, .0145, .006)                                          # the bell ring
    # bell: two key cups on the upper bell (low B, Bb) on a rod from the lower joint
    xb = X_BELL + .006
    for x, s in ((xb + .022, .55), (xb + .045, -.55)):
        d = np.array([0, math.sin(s), math.cos(s)])
        c = np.array([x, 0, 0]) + d * .0152
        S.prism(g, c, d, .0058, .0030, 10, NICKEL, dim(NICKEL, .8))
    rod(g, xl1 - .025, xb + .045, .05, lift=.0042)


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])


POSE = rot_y(math.radians(-16)) @ rot_z(math.radians(-63))      # as the clarinet: reed up-left, bell down-right


def xform(g, f0, l0, R=None, t=(0, 0, 0)):
    t = np.asarray(t, float)
    for i in range(f0, len(g.F)):
        P_, n, col, two, emit, gid, spec = g.F[i]
        if R is not None:
            P_ = P_ @ R.T
            n = R @ n
        g.F[i] = (P_ + t, n, col, two, emit, gid, spec)
    for i in range(l0, len(g.L)):
        a, b, col, gid = g.L[i]
        if R is not None:
            a, b = R @ a, R @ b
        g.L[i] = (a + t, b + t, col, gid)


def oboe(g):
    """the whole instrument in its local frame; returns the bell mouth"""
    reed(g)
    crown(g)
    lathe(g, [(REED + .021, body_r(REED + .021)), (X_TEN, body_r(X_TEN - 1e-6))], BLACK, n=10, spec=.45)   # upper joint
    lathe(g, [(X_TEN + .006, body_r(X_TEN + .006)), (X_BELL, body_r(X_BELL))], BLACK, n=10, spec=.45)        # lower joint
    bell(g)
    keys(g)
    return np.array([X_END, 0, 0])


def build(g, t, ev, fx, s):
    f0, l0 = len(g.F), len(g.L)
    mouth = oboe(g)
    ch = s.get("chunk", 1.0)
    if ch != 1.0:                                                    # radii x chunk about the bore axis (lengths true)
        Sc, Sn = np.array([1.0, ch, ch]), np.array([ch, 1.0, 1.0])   # points; normals (inverse transpose, rescaled)
        for i in range(f0, len(g.F)):
            P_, n, col, two, emit, gid, spec = g.F[i]
            n = n * Sn
            g.F[i] = (P_ * Sc, n / np.linalg.norm(n), col, two, emit, gid, spec)
        for i in range(l0, len(g.L)):
            a, b, col, gid = g.L[i]
            g.L[i] = (a * Sc, b * Sc, col, gid)
    cen = np.array([X_END / 2, 0, 0])
    o = np.array([0, .39, 0]) - POSE @ cen
    xform(g, f0, l0, R=POSE, t=o)
    mouth = POSE @ mouth + o
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(mouth + [.08, .12, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(mouth + [.02, 0, 0]), age, 0))


# ---------------------------------------------------------------- sound: the nasal double reed
# A conical bore sounds every harmonic (the clarinet's cylinder mostly odd ones). The oboe's colour comes from its
# formants: a strong band around 1.1-1.4 kHz and a second one near 3 kHz, with a weak fundamental in the low register,
# so the 2nd-4th harmonics lead (the nasal, reedy sound). A tongued attack with a short buzz of reed noise, vibrato
# that blooms after the attack, and a quick release. Pitch may be fractional (midi 68.5 = a quarter-tone flat A).
SR = 44100
FORMANTS = ((1200., 420., 1.9), (2950., 650., 0.95), (4400., 900., 0.25))   # (centre Hz, width Hz, gain)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def double_reed(f, dur, v, bright=.6, vib=.0045, vrate=5.3, att=.022, buzz=.06, breath=.012):
    n = int((dur + .12) * SR)
    t = np.arange(n) / SR
    env = np.minimum(1, t / att) ** 1.2 * np.where(t < dur, 1, np.exp(-(t - dur) / .035))
    env *= 1 + .12 * np.exp(-t / .05)                                          # the tongue's little bump
    fr = f * (1 - .012 * np.exp(-t / .03)) * (1 + vib * np.sin(2 * np.pi * vrate * t) * np.clip((t - .16) / .22, 0, 1))
    ph = 2 * np.pi * np.cumsum(fr) / SR
    x = np.zeros(n)
    tot = 0.0
    for k in range(1, 48):
        fk = f * k
        if fk > 9500:
            break
        a = k ** -.45 * (.28 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        a *= math.exp(-fk / (2600 + 4200 * bright * v))                         # louder = brighter
        if k == 1:
            a *= min(.9, max(.4, (f - 200) / 400))                               # a weak fundamental down low
        x += a * np.sin(k * ph + .7 * k)
        tot += a
    x /= tot + 1e-9
    rng = np.random.default_rng(int(f * 10) % 100003)                          # deterministic noise per pitch
    nz = rng.standard_normal(n)
    from scipy.signal import butter, sosfilt
    nb = sosfilt(butter(2, [1800, 4200], 'band', fs=SR, output='sos'), nz)
    x += nb * (buzz * np.exp(-t / .03) + breath) * .6
    return x * env * v


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return double_reed(_midi_hz(m), dur, v)


if __name__ == "__main__":
    import piano_lowpoly as P
    g = P.Geo()
    build(g, 0.0, [], [], SCENES[0])
    V = np.concatenate([f[0] for f in g.F])
    print(len(g.F), "faces, extent (m):", np.round(V.max(0) - V.min(0), 3))

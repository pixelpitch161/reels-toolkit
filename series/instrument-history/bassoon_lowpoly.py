"""Pixel Pitch: the modern bassoon - low-poly model. No history video yet: built for BALLFIGHT #13
(Bassoon vs Bari Sax) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the oboe
(BALLFIGHT #11) is. A later Pixel Pitch bassoon video can grow from this file (add SCENES rows, backdrop, music) without
touching the model.

    python3 bassoon_lowpoly.py               # face count + extent (sprites: projects/which_instrument_wins/sprites.py)

Model from reference photos (Wikimedia Commons, found in Claude in Chrome, 2026-10-26):
  - "Yamaha Bassoon YFG-812 II.tif" (CC BY-SA 4.0): a straight-on front product shot (Heckel system, maple, silver-plated
    keys), measured on its 250 x 1661 px rendition and scaled to the bassoon's usual 134 cm (0.815 mm per px):
    boot 425 mm tall, an oval 73 mm across the two bores at its silver top band tapering to 63 mm at the silver boot cap
    (55 mm tall); the long joint (~42 mm, keys down its front-left) rising to 1012 mm; the bell joint above it (47 mm,
    a slight bulge to 54 mm at ~1175 mm, the pale cap ring 62 mm across at the top, 1340 mm); the wing joint (34 mm)
    beside the long joint, its silver crown at 849-894 mm; the bocal rising 57 mm out of the crown, arcing over and
    running down and out to the reed (red thread on the Yamaha's); the black hand rest on the boot's wing side at
    ~340 mm. Wood sampled at (120, 56, 52), deep red-brown flamed maple.
  - "Bassoon parts.jpg" (CC BY-SA 3.0): the five parts laid out (bell, long joint, wing joint, boot with hand rest and
    cap, bocal): the bocal's cork end, a ~180-degree arc, then a long, gently curving run to the reed (~30 cm in all).
Local frame: y up from the bottom of the boot cap (the bassoon standing), +z toward the camera (the finger side), +x
toward the wing joint (the bocal side). The long joint's axis is at x = -0.020, the wing joint's at x = +0.021."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_shapes as S
import trumpet_lowpoly as T
from pp_framework import lerp, dim, mix

OUT, TAG = "bassoon_lowpoly.mp4", "bn"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE BASSOON"

# one scene for now: the modern Heckel-system bassoon (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 46, 1, .85), (1, 50, 1, .85), (2, 53, 1, .85), (3, 58, 1, .85), (4, 57, 2, .85), (6, 46, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="MODERN BASSOON", credit="HECKEL SYSTEM", notes=_NOTES),
    # the same bassoon for small sprites (BALLFIGHT balls, ~30-60 px tall): true heights, every horizontal distance x
    # `chunk` about the instrument's centre line (joints, boot, bocal, hand rest together), so the twin tubes, the crook
    # and the cap ring still read as more than a two-pixel line; the crook's tube and the reed also x `crook` thicker (the
    # bocal is the bassoon's signature at that size). A Pixel Pitch video would use "modern".
    dict(key="ball", theme="intro", name="MODERN BASSOON", credit="HECKEL SYSTEM", notes=_NOTES, chunk=2.3, crook=3.4),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- the instrument (real proportions, metres)
MAPLE = (120, 56, 52)           # flamed maple, deep red-brown stain (sampled from the Yamaha shot)
MAPLE_D = (84, 38, 36)          # the darker flame stripes
NICKEL = (214, 216, 226)        # silver-plated keywork, rings, boot cap
IVORY = (232, 226, 210)         # the bell's pale cap ring
CANE = (226, 204, 148)
CANE_D = (196, 168, 110)
THREAD = (196, 40, 44)          # the reed's thread wrap (red on the Yamaha's)
CORK = (196, 156, 106)
BLACK = (34, 32, 34)            # the hand rest
HOLE = (16, 10, 10)

XL, XW = -.020, .021            # long joint / wing joint axes (x)
Y_BOOT = .425                   # top of the boot (its silver band)
Y_LONG = 1.012                  # top of the long joint (the bell's ring)
Y_TOP = 1.340                   # top of the bell's cap ring
Y_WING = .894                   # top of the wing joint's crown
FZ = np.array([0, 0, 1.])


def unit(v):
    v = np.asarray(v, float)
    return v / np.linalg.norm(v)


def lathe_y(g, x, z, prof, col, n=10, spec=.4, two=False):
    """a turned profile along y at (x, z): prof = [(y, r), ...]"""
    for (y0, r0), (y1, r1) in zip(prof, prof[1:]):
        T.tube(g, (x, y0, z), (x, y1, z), r0, r1, col=col, n=n, spec=spec, two=two)


def flamed(g, x, y0, y1, r, n=10, stripes=6):
    """a maple tube with darker flame bands (alternating face colours along y: the flame reads at sprite size)"""
    ys = np.linspace(y0, y1, stripes + 1)
    for j, (a, b) in enumerate(zip(ys, ys[1:])):
        ra, rb = r(a), r(b)
        T.tube(g, (x, a, 0), (x, b, 0), ra, rb, col=MAPLE if j % 2 == 0 else mix(MAPLE, MAPLE_D, .45), n=n, spec=.45)


def oval_ring(cx, y, a, b, n=16):
    return [np.array([cx + a * math.cos(2 * math.pi * k / n), y, b * math.sin(2 * math.pi * k / n)]) for k in range(n)]


def oval_tube(g, cx, prof, col, n=16, spec=.45, cap_top=None, cap_bot=None):
    """an oval prism along y (the boot): prof = [(y, half-width a (x), half-depth b (z)), ...]"""
    for (y0, a0, b0), (y1, a1, b1) in zip(prof, prof[1:]):
        A, B = oval_ring(cx, y0, a0, b0, n), oval_ring(cx, y1, a1, b1, n)
        for k in range(n):
            g.face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], col, ref=(cx, (y0 + y1) / 2, 0), spec=spec)
    if cap_bot is not None:
        y, a, b = prof[0]
        g.face(oval_ring(cx, y, a, b, n), cap_bot, ref=(cx, y + .01, 0))
    if cap_top is not None:
        y, a, b = prof[-1]
        g.face(oval_ring(cx, y, a, b, n), cap_top, ref=(cx, y - .01, 0))


BOOT_CX = (XL + XW) / 2


def boot(g):
    """the oval boot: 73 mm across the bores at the top band, 63 mm at the cap, ~46 mm deep; silver cap + band"""
    a = lambda y: lerp(.0315, .0365, y / Y_BOOT)          # half-width (x)
    b = lambda y: lerp(.0225, .0245, y / Y_BOOT)          # half-depth (z)
    oval_tube(g, BOOT_CX, [(0.0, a(0) * .93, b(0) * .93), (.006, a(0), b(0)), (.055, a(.055), b(.055))], NICKEL,
              spec=.85, cap_bot=dim(NICKEL, .6))
    ys = np.linspace(.055, Y_BOOT - .03, 7)
    for j, (y0, y1) in enumerate(zip(ys, ys[1:])):
        oval_tube(g, BOOT_CX, [(y0, a(y0), b(y0)), (y1, a(y1), b(y1))], MAPLE if j % 2 == 0 else mix(MAPLE, MAPLE_D, .45))
    oval_tube(g, BOOT_CX, [(Y_BOOT - .03, a(Y_BOOT) * 1.02, b(Y_BOOT) * 1.02), (Y_BOOT, a(Y_BOOT) * 1.02, b(Y_BOOT) * 1.02)],
              NICKEL, spec=.85, cap_top=dim(NICKEL, .75))


def long_joint(g):
    r = lambda y: lerp(.0205, .0225, (y - Y_BOOT) / (Y_LONG - Y_BOOT))
    flamed(g, XL, Y_BOOT, Y_LONG - .012, r, n=10, stripes=7)
    lathe_y(g, XL, 0, [(Y_LONG - .014, .0235), (Y_LONG + .004, .0235)], NICKEL, n=12, spec=.9)   # the bell's silver ring


def bell_joint(g):
    """47 mm, a slight bulge to 54 mm two-thirds up... (at ~1175 mm), then the pale cap ring (62 mm) at the top"""
    prof = [(Y_LONG + .004, .0235), (1.08, .0240), (1.13, .0258), (1.175, .0270), (1.22, .0258), (1.27, .0240),
            (1.305, .0244), (1.322, .0262)]
    for j, ((y0, r0), (y1, r1)) in enumerate(zip(prof, prof[1:])):
        T.tube(g, (XL, y0, 0), (XL, y1, 0), r0, r1, col=MAPLE if j % 2 == 0 else mix(MAPLE, MAPLE_D, .45), n=12, spec=.45)
    lathe_y(g, XL, 0, [(1.322, .029), (1.330, .031), (Y_TOP, .0305)], IVORY, n=14, spec=.5)
    g.face(S.ring((XL, Y_TOP + .0005, 0), (0, 1, 0), .0215, 12), HOLE, ref=(XL, Y_TOP - .02, 0), emit=True)


def wing_joint(g):
    r = lambda y: .0170
    flamed(g, XW, Y_BOOT, Y_WING - .045, r, n=10, stripes=5)
    lathe_y(g, XW, 0, [(Y_WING - .045, .0185), (Y_WING - .040, .0198), (Y_WING - .003, .0198), (Y_WING, .0188)], NICKEL,
            n=12, spec=.9)                                                      # the silver crown
    g.face(S.ring((XW, Y_WING + .0004, 0), (0, 1, 0), .0105, 10), dim(NICKEL, .55), ref=(XW, Y_WING - .02, 0))
    for y in (.60, .66, .72):                                                   # the three angled finger holes (front)
        c = np.array([XW, y, .0172])
        g.face(S.ring(c, FZ, .0036, 8), HOLE, ref=c - FZ, emit=True)
        T.tube(g, c - FZ * .001, c + FZ * .0018, .0046, col=dim(MAPLE, .8), n=8, spec=.3)


def bocal_path(k=1.0):
    """the crook: up out of the crown, a ~180-degree arc over toward +x / +z, then a long, gently curving run down and
    out to the reed (~30 cm in all). Returns (points, radii)."""
    pts, rad = [], []
    for y in np.linspace(Y_WING - .015, Y_WING + .045, 4):                      # cork end in the crown, rising
        pts.append(np.array([XW, y, 0.0]))
    c = np.array([XW + .021, Y_WING + .045, .004])                             # the arc
    for a in np.linspace(math.pi, math.pi * .12, 7)[1:]:
        pts.append(c + np.array([.021 * math.cos(a), .021 * math.sin(a) * 1.15, .012 * (1 - a / math.pi)]))
    p0 = pts[-1]
    end = np.array([XW + .135, Y_WING - .045, .150])
    ctrl = p0 + np.array([.045, -.015, .05])
    for u in np.linspace(0, 1, 8)[1:]:
        pts.append((1 - u) ** 2 * p0 + 2 * (1 - u) * u * ctrl + u * u * end)
    n = len(pts)
    for i in range(n):
        rad.append(lerp(.0048, .0031, i / (n - 1)) * k)
    return pts, rad


def bocal(g, k=1.0):
    """the nickel crook (with its cork end in the crown and the whisper-key nub) and the reed at its tip; k thickens the
    tube and the reed (sprites)"""
    import altosax60_lowpoly as AS                                             # its swept-tube helper (read-only use)
    pts, rad = bocal_path(k)
    rings, ax = AS.sweep(g, pts, rad, NICKEL, n=8, spec=.95)
    T.tube(g, pts[0], pts[1], .0062, col=CORK, n=8, spec=.1)
    nub = pts[4] + np.array([0, 0, .006])
    S.prism(g, nub, (0, 0, 1), .0028, .003, 6, NICKEL, dim(NICKEL, .8))         # the pianissimo (whisper) vent nub
    d = unit(ax[-1] - ax[-2])
    tip = ax[-1]
    # the reed: a thread-wrapped tube (red), two wires, then the wide, flat cane blades
    kr = 1 + (k - 1) * .5
    T.tube(g, tip - d * .004, tip + d * .022, .0046 * kr, .0050 * kr, col=THREAD, n=8, spec=.25)
    for f in (.026, .031):
        T.tube(g, tip + d * f, tip + d * (f + .0015), .0052 * kr, col=NICKEL, n=8, spec=.9)
    side = unit(np.cross(d, (0, 1, 0)))
    up = unit(np.cross(side, d))
    base = tip + d * .022
    m = 10
    secs = [(.000, .0050, .0040), (.012, .0058, .0022), (.024, .0068, .0013), (.034, .0072, .0007)]
    ring = lambda s, a_, b_: [base + d * s + side * a_ * kr * math.cos(2 * math.pi * j / m) +
                              up * b_ * kr * math.sin(2 * math.pi * j / m) for j in range(m)]
    for (s0, a0, b0), (s1, a1, b1) in zip(secs, secs[1:]):
        A, B = ring(s0, a0, b0), ring(s1, a1, b1)
        for j in range(m):
            col = CANE if math.sin(2 * math.pi * (j + .5) / m) > 0 else CANE_D
            g.face([A[j], A[(j + 1) % m], B[(j + 1) % m], B[j]], col, ref=base + d * (s0 + s1) / 2, spec=.2)
    g.face(ring(.034, .0072, .0007), CANE, ref=base)
    return tip + d * .056


def hand_rest(g):
    """the black crutch on the boot's wing side (~340 mm up): a stem out, then a curved paddle"""
    y = .340
    a0 = np.array([BOOT_CX + .034, y, .006])
    a1 = a0 + np.array([.022, .004, .004])
    T.tube(g, a0, a1, .0032, col=NICKEL, n=6, spec=.8)
    pts = [a1 + np.array([.004, .028 - .018 * k, .010 * math.sin(k * .9)]) for k in range(5)]
    for p, q in zip(pts, pts[1:]):
        T.tube(g, p, q, .0085, .0080, col=BLACK, n=8, spec=.35)


def keys(g):
    """the Heckel keywork, simplified to what reads at sprite size: long rods down the long joint's front-left, the low-A
    pad on the bell, the wing's ring key, the boot's big right-hand pad, spatulas and the cross-key guards"""
    def rod(x, z, y0, y1, r=.0012):
        T.tube(g, (x, y0, z), (x, y1, z), r, col=NICKEL, n=5, spec=.85)
        for y in (y0, y1):
            T.tube(g, (x, y, z * .8), (x, y, z), .0014, col=dim(NICKEL, .9), n=4, spec=.7)

    def pad(c, n_, r):
        S.prism(g, c, n_, r, .0028, 10, NICKEL, dim(NICKEL, .8))
        g.face(S.ring(np.asarray(c) + unit(n_) * .0016, n_, r * .5, 8), mix(NICKEL, (255, 255, 255), .25), ref=c)
    # long joint: three rods down the front-left, a few touches
    for x, z in ((XL - .016, .016), (XL - .011, .0195), (XL - .004, .0225)):
        rod(x, z, Y_BOOT + .05, Y_LONG - .06)
    for y in (.62, .72, .80, .90):
        pad((XL - .012, y, .021), unit((-.5, 0, 1)), .0034)
    # bell joint: the low-A pad on the front with its rod
    pad((XL + .002, 1.115, .025), FZ, .0075)
    rod(XL + .002, .026, Y_LONG - .04, 1.11)
    # wing joint: the ring key over the third hole, a rod down its outer side
    T.tube(g, (XW, .72, .0175), (XW, .72, .021), .0058, col=NICKEL, n=10, spec=.9)
    rod(XW + .012, .013, .48, Y_WING - .06)
    # boot: the big right-hand pad, a cross-key guard, spatulas, a rod
    pad((BOOT_CX + .004, .245, .0265), FZ, .0095)
    pad((BOOT_CX - .014, .175, .026), FZ, .0068)
    for y in (.115, .125):
        T.tube(g, (BOOT_CX - .018, y, .0275), (BOOT_CX + .014, y, .0275), .0011, col=NICKEL, n=4, spec=.8)
    for y, x in ((.30, BOOT_CX - .020), (.285, BOOT_CX - .012)):
        S.prism(g, (x, y, .029), FZ, .0045, .002, 6, NICKEL, dim(NICKEL, .8))
    rod(BOOT_CX + .022, .022, .09, Y_BOOT - .04)


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, 0, s_], [0, 1, 0], [-s_, 0, c]])


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return np.array([[c, -s_, 0], [s_, c, 0], [0, 0, 1]])


POSE = rot_z(math.radians(24)) @ rot_y(math.radians(-14))     # bell up-left, boot down-right, the crook out to the right


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


def bassoon(g, crook=1.0):
    """the whole instrument standing in its local frame; returns the reed tip"""
    boot(g)
    long_joint(g)
    bell_joint(g)
    wing_joint(g)
    hand_rest(g)
    keys(g)
    return bocal(g, crook)


def build(g, t, ev, fx, s):
    f0, l0 = len(g.F), len(g.L)
    reed = bassoon(g, s.get("crook", 1.0))
    ch = s.get("chunk", 1.0)
    if ch != 1.0:                                                    # x / z x chunk about the centre line (heights true)
        Sc, Sn = np.array([ch, 1.0, ch]), np.array([1.0, ch, 1.0])   # points; normals (inverse transpose, rescaled)
        c0 = np.array([BOOT_CX, 0, 0])
        for i in range(f0, len(g.F)):
            P_, n, col, two, emit, gid, spec = g.F[i]
            n = n * Sn
            g.F[i] = ((P_ - c0) * Sc + c0, n / np.linalg.norm(n), col, two, emit, gid, spec)
        for i in range(l0, len(g.L)):
            a, b, col, gid = g.L[i]
            g.L[i] = ((a - c0) * Sc + c0, (b - c0) * Sc + c0, col, gid)
        reed = (reed - c0) * Sc + c0
    cen = np.array([BOOT_CX, Y_TOP / 2, 0])
    o = np.array([0, .68, 0]) - POSE @ cen
    xform(g, f0, l0, R=POSE, t=o)
    reed = POSE @ reed + o
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(reed + [.08, .12, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(reed + [.02, 0, 0]), age, 0))


if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces, extent (m):", np.round(V.max(0) - V.min(0), 3))

"""Pixel Pitch: the kazoo - low-poly model. No history video yet: built for BALLFIGHT Kazoo vs Harmonica (IDEAS #43)
and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the other instruments' modern
showcase models are. A later Pixel Pitch kazoo video can grow from this file (add SCENES rows, backdrop, music) without
touching the model.

    python3 kazoo_lowpoly.py                # face count + extent

Model from reference photos (Wikimedia Commons, studied in Chrome, 2026-10-27), units = decimetres (L = 1.2 for a
120 mm kazoo):
  - "PlasticKazoo.JPG" (Simon Speed, own work): a classic plastic kazoo straight down on its top, 1210 x 416 px. Measured
    along the body: the wide (mouth) end 18 % of the length across, parallel to ~42 % of the length, then tapering to
    10 % at the narrow end (a 0.56 taper); the turret centred at 34 % of the length from the wide end, 16.5 % of the
    length across, its cap a grille of crossed slots over the membrane.
  - "2023 Kazoo (1).jpg" and "2023 Kazoo (3).jpg": a metal kazoo three quarters on: the body's section is a rounded
    rectangle about 1.5x as wide as it is tall at the mouth (a flat top the turret stands on), the open ends show dark;
    the turret stands ~40 % of its diameter above the top, with a rolled, flared rim (~1.15x its base) and the membrane
    (wax paper) a little below the rim.
The kazoo lies along x (the wide mouth end at -x), the turret up (+y), the front at +z. Classic plastic: a red body
(the BALLFIGHT KAZOO's colour) with a yellow turret cap, so the cap still reads inside a red-tinted glass ball."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files

OUT, TAG = "kazoo_lowpoly.mp4", "kz"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE KAZOO"

# one scene for now: the plastic kazoo (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 60, 1, .85), (1, 62, 1, .85), (2, 64, 2, .85), (4, 67, 2, .85), (6, 60, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="KAZOO", credit="PLASTIC KAZOO", notes=_NOTES),
    # the same kazoo for small sprites (BALLFIGHT balls ~25-60 px, the mini-kazoos ~8-14 px): the body `chunk` x as
    # thick, the turret `tur` x as big (x2 as tall), and tipped `tilt` deg toward the viewer so the yellow cap's grille
    # shows from the sprite camera's low angle; otherwise it reads as a red stick
    dict(key="ball", theme="intro", name="KAZOO", credit="PLASTIC KAZOO", notes=_NOTES, chunk=1.55, tur=1.22, tilt=30.0),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- measurements (fractions of the length L)
L = 1.20                                    # 120 mm
# (u from the wide mouth end, half-width / L, half-height / L): parallel to ~42 %, then a straight taper
PROF = [(0.0, .090, .060), (.08, .090, .060), (.25, .089, .059), (.42, .087, .057), (.60, .075, .050), (.80, .061, .043),
        (.96, .051, .038), (1.0, .050, .037)]
U_TUR = .34                                 # turret centre
TUR_R = .0825 * L                           # turret base radius (16.5 % of L across)
TUR_H = .034 * L                            # turret height above the top (~40 % of its diameter)
LIP_R, LIP_H = TUR_R * 1.15, .010 * L       # the rolled, flared rim

BODY = (214, 40, 38)                        # red plastic
BODY_HI = (240, 78, 62)                     # the flat top catching the light
BODY_LO = (150, 24, 28)                     # the underside
CAP = (252, 206, 44)                        # the yellow turret cap
CAP_HI = (255, 236, 120)
SLOT = (60, 18, 16)                         # the grille's slots: the membrane seen through them
MEMBRANE = (236, 214, 170)                  # wax paper
HOLE = (24, 8, 10)                          # the open ends


def section(w, h):
    """a rounded-rectangle section (z, y) with 45-degree chamfers: 8 points counter-clockwise from the front top"""
    c = min(w, h) * .38
    return [(w, h - c), (w - c, h), (-(w - c), h), (-w, h - c), (-w, -(h - c)), (-(w - c), -h), (w - c, -h), (w, -(h - c))]


def _face(g, pts, col, ref, **kw):
    g.face([tuple(p) for p in pts], col, ref=tuple(ref), **kw)


def body(g, chunk=1.0):
    """the tapered body: a loft of rounded-rectangle sections along x, both ends open (dark)"""
    rings = []
    for u, hw, hh in PROF:
        x = -L / 2 + u * L
        rings.append([(x, y * chunk, z * chunk) for z, y in section(hw * L, hh * L)])
    cen = (0.0, 0.0, 0.0)
    for A, B in zip(rings, rings[1:]):
        for k in range(8):
            k2 = (k + 1) % 8
            # top flat (k == 1) light, the chamfers + sides mid, the underside (k == 5) dark
            col = BODY_HI if k == 1 else (BODY_LO if k == 5 else BODY)
            _face(g, [A[k], A[k2], B[k2], B[k]], col, (A[k][0], 0.0, 0.0), spec=.35)
    for R_, sgn, (u, hw, hh) in ((rings[0], -1, PROF[0]), (rings[-1], 1, PROF[-1])):
        x = R_[0][0]
        _face(g, R_, BODY, (x - sgn, 0, 0))                                    # the rim
        inner = [(x + sgn * .002, y * .62, z * .70) for _, y, z in R_]
        _face(g, inner, HOLE, (x - sgn, 0, 0), emit=False)                      # the open end
    return rings


def top_y(u, chunk=1.0):
    hh = np.interp(u, [p[0] for p in PROF], [p[2] for p in PROF]) * L * chunk
    return hh


def turret(g, chunk=1.0, tur=1.0, n=12):
    """the turret: a short cylinder on the flat top, a rolled, flared rim, the yellow cap with its slotted grille"""
    TUR_R, LIP_R = globals()['TUR_R'] * tur, globals()['LIP_R'] * tur
    x0 = -L / 2 + U_TUR * L
    y0 = top_y(U_TUR, chunk) - .004                                             # sunk a hair into the top
    th = TUR_H * (1.0 + 5.0 * (tur - 1.0))
    y1 = y0 + th
    ang = [2 * math.pi * k / n + math.pi / n for k in range(n)]
    ring = lambda r, y: [(x0 + r * math.cos(a), y, r * math.sin(a)) for a in ang]
    lo, hi = ring(TUR_R, y0), ring(TUR_R, y1 - LIP_H)
    for k in range(n):
        k2 = (k + 1) % n
        _face(g, [lo[k], lo[k2], hi[k2], hi[k]], BODY, (x0, y0, 0), spec=.35)
    # the flared rim: an outward-sloping band, then its rolled top edge
    l0, l1 = ring(TUR_R, y1 - LIP_H), ring(LIP_R, y1)
    for k in range(n):
        k2 = (k + 1) % n
        _face(g, [l0[k], l0[k2], l1[k2], l1[k]], CAP, (x0, y1 - LIP_H - .05, 0), spec=.5)
    # the cap's top: a yellow annulus round a grille (dark membrane under two crossing yellow bars + a ring)
    inner = ring(TUR_R * .72, y1 + .001)
    for k in range(n):
        k2 = (k + 1) % n
        _face(g, [l1[k], l1[k2], inner[k2], inner[k]], CAP_HI if k < n // 2 else CAP, (x0, y1 - 1, 0), spec=.5)
    _face(g, inner, SLOT, (x0, y1 - 1, 0))
    yb = y1 + .003
    rb = TUR_R * .72
    for (dx, dz) in ((1, 0), (0, 1)):                                           # the grille's crossed bars
        w = .011
        if dx:
            pts = [(x0 - rb, yb, -w), (x0 + rb, yb, -w), (x0 + rb, yb, w), (x0 - rb, yb, w)]
        else:
            pts = [(x0 - w, yb, -rb), (x0 + w, yb, -rb), (x0 + w, yb, rb), (x0 - w, yb, rb)]
        _face(g, pts, CAP, (x0, yb - 1, 0), spec=.5)
    rr = TUR_R * .40                                                            # the grille's inner ring
    ring2 = [(x0 + rr * math.cos(a), yb + .001, rr * math.sin(a)) for a in ang]
    ring3 = [(x0 + rr * .70 * math.cos(a), yb + .001, rr * .70 * math.sin(a)) for a in ang]
    for k in range(n):
        k2 = (k + 1) % n
        _face(g, [ring2[k], ring2[k2], ring3[k2], ring3[k]], CAP, (x0, yb - 1, 0), spec=.5)
    return (x0, y1, 0.0)


def kazoo(g, chunk=1.0, tur=1.0):
    body(g, chunk)
    return turret(g, chunk, tur)


def tilt(g, i0, deg):
    """tip the faces built from index i0 on about the x axis (the top toward the viewer, +z)"""
    a = math.radians(deg)
    R = np.array([[1, 0, 0], [0, math.cos(a), -math.sin(a)], [0, math.sin(a), math.cos(a)]])
    for i in range(i0, len(g.F)):
        P_, n, *rest = g.F[i]
        g.F[i] = (np.asarray(P_, float) @ R.T, R @ np.asarray(n, float), *rest)


def build(g, t, ev, fx, s):
    i0 = len(g.F)
    top = kazoo(g, chunk=s.get("chunk", 1.0), tur=s.get("tur", 1.0))
    if s.get("tilt"):
        tilt(g, i0, s["tilt"])
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", (top[0], top[1] + .3, 0.0), age))


if __name__ == "__main__":
    import piano_lowpoly as P
    for s in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], s)
        V = np.concatenate([f[0] for f in g.F])
        print(s["key"], len(g.F), "faces, extent:", np.round(V.max(0) - V.min(0), 3))

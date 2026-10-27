"""Pixel Pitch: the harmonica (a 10-hole diatonic "blues harp") - low-poly model. No history video yet: built for
BALLFIGHT Kazoo vs Harmonica (IDEAS #43) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same
way the other instruments' modern showcase models are. A later Pixel Pitch harmonica video can grow from this file (add
SCENES rows, backdrop, music) without touching the model.

    python3 harmonica_lowpoly.py            # face count + extent

Model from reference photos (Wikimedia Commons, studied in Chrome, 2026-10-27), units = decimetres (L = 1.01 for a
101 mm harp):
  - "Harmonijka ustna 10301A Blues Session Steel A firmy Seydel.jpg" (a 10-hole diatonic three quarters on from the
    front, 2516 x 1383 px): the comb sandwiched between two stamped steel cover plates; ten square holes along the comb's
    front, centred, over ~70 % of the length (a 7.2 % pitch; each hole ~65 % of the pitch wide and ~60 % of the comb's
    height); the comb shows ~2 % in front of the covers; each cover is flat on top with its front edge rolled down in a
    smooth curve over the holes, a closed, rounded end at each side, a flat strip at the back, a pressed recess band
    along the top and a screw near each end.
  - "Special20.JPG" (a Hohner Special 20 close up): the rolled front edge, the stamped hole numbers 1-10 along it, the
    covers' end vents.
  - Sizes: 101 x 28 x 21 mm overall (the standard 10-hole), the comb ~7 mm thick.
The comb here is pearwood (the classic "Marine Band" blues harp: the dark holes read on tan, and the tan reads against
the BALLFIGHT HARMONICA's sky-blue glass), the covers polished steel. The harp lies along x (hole 1, the low end, at -x),
the holes facing the front (+z), the top cover up (+y)."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files

OUT, TAG = "harmonica_lowpoly.mp4", "hm"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE HARMONICA"

# one scene for now: the 10-hole diatonic (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 60, 1, .85), (1, 64, 1, .85), (2, 67, 2, .85), (4, 65, 2, .85), (6, 62, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="HARMONICA", credit="10-HOLE DIATONIC", notes=_NOTES),
    # the same harp for small sprites (BALLFIGHT balls, ~25-60 px): the comb `chunk` x as thick with bigger holes and
    # tipped `tilt` deg toward the viewer, so the ten holes read as a row of dark squares and the top cover shows
    dict(key="ball", theme="intro", name="HARMONICA", credit="10-HOLE DIATONIC", notes=_NOTES, chunk=1.5, tilt=18.0),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- measurements (decimetres)
L = 1.01                    # length (comb and covers)
DEPTH = .28                 # front to back (z)
COMB_T = .072               # comb thickness (y)
COVER_H = .068              # each cover's height above / below the comb
PITCH = .072 * L            # hole pitch
HOLE_W, HOLE_H = .65 * PITCH, .62 * COMB_T
COMB_LIP = .02              # the comb shows in front of the covers
END_R = .05                 # the covers' rounded ends (plan view)

WOOD = (204, 146, 84)       # pearwood comb
WOOD_HI = (214, 160, 98)
HOLE = (30, 16, 10)
STEEL = (196, 202, 214)
STEEL_HI = (236, 240, 248)
STEEL_LO = (128, 134, 150)
RECESS = (158, 164, 180)
SCREW = (90, 94, 108)
STAMP = (90, 96, 112)


def _face(g, pts, col, ref, **kw):
    g.face([tuple(p) for p in pts], col, ref=tuple(ref), **kw)


def box(g, x0, x1, y0, y1, z0, z1, col, spec=0.):
    g.box(min(x0, x1), max(x0, x1), min(y0, y1), max(y0, y1), min(z0, z1), max(z0, z1), col, spec=spec)


def comb(g, chunk=1.0):
    """the comb: a wooden slab with ten square chambers opening on the front face"""
    ct = COMB_T * chunk
    zf = DEPTH / 2                                    # the comb's front face
    zb = -DEPTH / 2 + .01
    hw, hh = HOLE_W * min(chunk, 1.25), HOLE_H * chunk
    xs = [(j - 4.5) * PITCH for j in range(10)]
    # the front face as strips between the holes (so the holes are real openings), then each chamber's dark back wall
    edges = [-L / 2] + [v for x in xs for v in (x - hw / 2, x + hw / 2)] + [L / 2]
    y0, y1 = -ct / 2, ct / 2
    for j in range(0, len(edges) - 1, 2):              # full-height strips between holes (and the two end blocks)
        _face(g, [(edges[j], y0, zf), (edges[j + 1], y0, zf), (edges[j + 1], y1, zf), (edges[j], y1, zf)], WOOD,
              (0, 0, 0), spec=.1)
    for x in xs:                                       # above / below each hole
        for ya, yb in ((y0, -hh / 2), (hh / 2, y1)):
            _face(g, [(x - hw / 2, ya, zf), (x + hw / 2, ya, zf), (x + hw / 2, yb, zf), (x - hw / 2, yb, zf)], WOOD,
                  (0, 0, 0), spec=.1)
        _face(g, [(x - hw / 2, -hh / 2, zf - .06), (x + hw / 2, -hh / 2, zf - .06), (x + hw / 2, hh / 2, zf - .06),
                  (x - hw / 2, hh / 2, zf - .06)], HOLE, (0, 0, -1))
        for sx in (-1, 1):                             # the chamber's side walls (a little depth when seen at an angle)
            xx = x + sx * hw / 2
            _face(g, [(xx, -hh / 2, zf), (xx, hh / 2, zf), (xx, hh / 2, zf - .06), (xx, -hh / 2, zf - .06)], WOOD_HI,
                  (xx + sx, 0, zf - .03))                      # normals into the chamber (seen from outside)
        for sy in (-1, 1):
            yy = sy * hh / 2
            _face(g, [(x - hw / 2, yy, zf), (x + hw / 2, yy, zf), (x + hw / 2, yy, zf - .06), (x - hw / 2, yy, zf - .06)],
                  (120, 78, 40) if sy > 0 else (96, 60, 30), (x, yy + sy, zf - .03))
    # the comb's ends and back (the top / bottom are under the covers)
    for sx in (-1, 1):
        _face(g, [(sx * L / 2, y0, zb), (sx * L / 2, y1, zb), (sx * L / 2, y1, zf), (sx * L / 2, y0, zf)], WOOD_HI,
              (0, 0, 0))
    _face(g, [(-L / 2, y0, zb), (L / 2, y0, zb), (L / 2, y1, zb), (-L / 2, y1, zb)], WOOD, (0, 0, 0))
    # the thin strip of comb top / bottom showing in front of the covers
    for sy in (-1, 1):
        y = sy * ct / 2
        _face(g, [(-L / 2, y, zf), (L / 2, y, zf), (L / 2, y, zf - COMB_LIP), (-L / 2, y, zf - COMB_LIP)], WOOD_HI,
              (0, 0, 0))
    return xs


def cover_section(sy, chunk=1.0):
    """a cover plate's section (z, y): a flat strip at the back, up to the flat top, the front rolled down in a curve
    to the comb (sy = +1 top cover, -1 bottom cover)"""
    ct = COMB_T * chunk / 2
    h = COVER_H * (1 + .25 * (chunk - 1))
    zb, zf = -DEPTH / 2, DEPTH / 2 - COMB_LIP
    pts = [(zb, ct + .2 * h), (zb + .06, ct + .25 * h), (zb + .085, ct + .92 * h), (zb + .11, ct + h)]
    for a in np.linspace(0, math.pi / 2, 5):          # the rolled front: a quarter curve from the top down to the comb
        pts.append((zf - .085 + .085 * math.sin(a), ct + .08 * h + .92 * h * math.cos(a)))
    return [(z, sy * y) for z, y in pts]


def covers(g, chunk=1.0):
    """two stamped steel cover plates: the profile extruded along x with rounded, closed ends; a recess band and a screw
    near each end on top"""
    hx = L / 2
    for sy in (1, -1):
        sec = cover_section(sy, chunk)
        n = len(sec)
        # along the length: the section between x = -(hx - END_R) and +(hx - END_R), then the rounded ends tuck in
        xs = [-hx, -hx + END_R * .3, -hx + END_R, hx - END_R, hx - END_R * .3, hx]
        shrink = [.80, .95, 1.0, 1.0, .95, .80]          # the ends' plan-view rounding: the profile scales in
        rings = []
        for x, k in zip(xs, shrink):
            cz = (sec[0][0] + sec[-1][0]) / 2
            base = sec[0][1]
            rings.append([(x, base + (y - base) * (.55 + .45 * k), cz + (z - cz) * k) for z, y in sec])
        cen = (0.0, sy * .02, 0.0)
        for A, B in zip(rings, rings[1:]):
            for j in range(n - 1):
                col = STEEL_HI if 3 <= j <= 4 else (STEEL_LO if j >= n - 3 else STEEL)
                if sy < 0:
                    col = STEEL_LO if j < n - 2 else STEEL
                _face(g, [A[j], A[j + 1], B[j + 1], B[j]], col, cen, spec=.9)
        for R_, sx in ((rings[0], -1), (rings[-1], 1)):  # closed ends
            _face(g, R_ + [(R_[-1][0], sec[-1][1] * .0 + sy * COMB_T * chunk / 2, R_[-1][2]),
                           (R_[0][0], sy * COMB_T * chunk / 2, R_[0][2])], STEEL_LO, (R_[0][0] - sx, sy * .02, 0),
                  spec=.9)
        if sy > 0:
            ytop = sec[3][1] + .002
            z0, z1 = sec[3][0] + .01, sec[4][0] - .005  # the pressed recess band along the top, toward the back
            zr0, zr1 = z0 + .006, z0 + .05
            _face(g, [(-hx + .12, ytop, zr0), (hx - .12, ytop, zr0), (hx - .12, ytop, zr1), (-hx + .12, ytop, zr1)],
                  RECESS, (0, 0, 0), spec=.6)
            for sx in (-1, 1):                           # the screws
                x = sx * (hx - .07)
                zc = (z0 + z1) / 2 - .02
                ring = [(x + .018 * math.cos(a), ytop + .002, zc + .018 * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 9)[:-1]]
                _face(g, ring, SCREW, (x, 0, zc), spec=.7)
            for j in range(10):                          # the stamped hole numbers: small dark ticks along the front roll
                x = (j - 4.5) * PITCH
                zt = sec[5][0]
                yt = sec[5][1] + .002
                _face(g, [(x - .006, yt, zt - .006), (x + .006, yt, zt - .006), (x + .006, yt - .002, zt + .006),
                          (x - .006, yt - .002, zt + .006)], STAMP, (x, 0, 0))


def harmonica(g, chunk=1.0):
    comb(g, chunk)
    covers(g, chunk)


def tilt(g, i0, deg):
    """tip the faces built from index i0 on about the x axis (the top toward the viewer, +z)"""
    a = math.radians(deg)
    R = np.array([[1, 0, 0], [0, math.cos(a), -math.sin(a)], [0, math.sin(a), math.cos(a)]])
    for i in range(i0, len(g.F)):
        P_, n, *rest = g.F[i]
        g.F[i] = (np.asarray(P_, float) @ R.T, R @ np.asarray(n, float), *rest)


def build(g, t, ev, fx, s):
    i0 = len(g.F)
    harmonica(g, chunk=s.get("chunk", 1.0))
    if s.get("tilt"):
        tilt(g, i0, s["tilt"])
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", (0.0, .3, DEPTH / 2), age))


if __name__ == "__main__":
    import piano_lowpoly as P
    for s in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], s)
        V = np.concatenate([f[0] for f in g.F])
        print(s["key"], len(g.F), "faces, extent:", np.round(V.max(0) - V.min(0), 3))

"""Pixel Pitch: the modern double bass (contrabass) - low-poly model. No history video yet: built for BALLFIGHT #47
(Cello vs Contrabass) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the other
instruments' modern showcase models are. A later Pixel Pitch double-bass video can grow from this file (add SCENES rows,
backdrop, music) without touching the model.

    python3 contrabass_lowpoly.py           # face count + extent, and the size next to cello60_lowpoly's modern cello

Model from reference photos (Wikimedia Commons, studied in Chrome, 2026-10-28), in violin_lowpoly's units (its 356 mm
violin body is L = .95, so 2.67 units per metre):
  - "AGK bass1 full.jpg" (CC BY-SA 3.0): one double bass photographed straight-on from the front AND the side on white,
    1100 x 1550 px. Measured per pixel row in a canvas: body 887 px (scroll top 37, body 545-1432, endpin tip 1535).
    Front: the violin-family outline with the viol's sloping shoulders (half-width .05 L at the neck root, .18 L at
    87 %, .23 L at the upper bout 66-72 %), small corners at 65 % and 48 %, C-bout .18 L, lower bout .31 L at 21-28 %
    (a 62 %-of-length-wide body); f-hole eyes at 58 % (+-.09 L) and 40 % (+-.17 L), the bridge between their notches at
    48 %, .12 L wide; tailpiece 3-30 %; fingerboard ending at 66 %. Side: ribs + plates ~.22 L deep, the FLAT back
    breaking forward over its top fifth (it slopes in by .10 L toward the neck from 80 %), the bridge standing ~.11 L
    proud of the top; endpin .12 L out (retracted); geared machine heads with keys sticking out of the pegbox sides.
  - "Double Bass MET DP217153 / 54 / 55 / DP218932.jpg" (CC0, the Met's double bass): front, back, scroll and three-
    quarter views: the same sloping shoulders, the flat back's break showing as a band under the shoulders, the tall
    scroll; its head + neck is .71 of its body, the AGK bass's .56 (a 3/4 bass is ~.62: used here).
A double bass body is ~111 cm (a cello's ~76 cm), so L = 2.96 here, 1.48x the cello's; the BALLFIGHT sprite makes it
read bigger than the cello with the bridge's size factor. No bow on the model (the BALLFIGHT steps are drawn separately)."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files

OUT, TAG = "contrabass_lowpoly.mp4", "cb"
BPM = 92
SHOW_BARS = 2
HEAD_INTRO = "THE DOUBLE BASS"

# one scene for now: the modern double bass (the bridge builds it at rest)
_NOTES = [(0, 28, 1, .9), (1, 35, 1, .9), (2, 40, 1, .9), (3, 43, 1, .9), (4, 33, 2, .9), (6, 28, 2, .9)]
SCENES = [
    dict(key="modern", theme="intro", name="MODERN DOUBLE BASS", credit="3/4 DOUBLE BASS", notes=_NOTES),
    # the same bass for small sprites (BALLFIGHT balls, ~40-60 px tall): true proportions, but the neck, fingerboard,
    # machine keys, scroll and endpin are thickened x`chunk` across so they read as more than a one-pixel line.
    dict(key="ball", theme="intro", name="MODERN DOUBLE BASS", credit="3/4 DOUBLE BASS", notes=_NOTES, chunk=1.6),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- measurements (fractions of the body length L)
U_PER_M = .95 / .356            # violin_lowpoly's scale: its modern violin's 356 mm body is L = .95
L = 1.11 * U_PER_M              # a 3/4 double bass body, 111 cm: 2.962
RIBS = .19 * L                  # rib height + plate edges (side view)
ARCH_T = .030 * L               # the top's arching above the rib line (the back is flat)
BREAK_U, BREAK_IN = .80, .10 * L   # the flat back breaks forward from 80 % up, in by .10 L at the neck root
Y0 = .4                         # body bottom (the endpin stands below it, its tip near y = 0)
# half-widths along the body, u from the bottom (end button) to the top (neck root), from "AGK bass1 full.jpg"
CBPROF = [(0, .06), (.018, .148), (.032, .184), (.059, .221), (.086, .249), (.113, .271), (.14, .286), (.167, .298),
          (.194, .307), (.221, .310), (.248, .311), (.275, .309), (.302, .304), (.329, .300), (.356, .293), (.383, .285),
          (.41, .276), (.437, .266), (.464, .244), (.478, .225), (.492, .194), (.505, .187), (.532, .180), (.572, .179),
          (.60, .180), (.613, .184), (.627, .189), (.640, .207), (.654, .227), (.681, .233), (.722, .232), (.749, .229),
          (.776, .223), (.803, .216), (.830, .204), (.857, .187), (.884, .165), (.911, .133), (.938, .095), (.965, .062),
          (1, .05)]
U_BRIDGE, BRIDGE_W, BRIDGE_H = .483, .118 * L, .11 * L
U_FB_END, FB_END_W, NUT_W = .656, .062 * L, .036 * L
NECK, HEAD = .34 * L, .26 * L                       # nut -> body, scroll top -> nut
TAIL = (.03, .30)                                   # tailpiece span (u)
F_EYES = ((.584, .091), (.395, .169))               # f-hole (u, |x| / L): upper eye, lower eye
ENDPIN = .12 * L

VARNISH = (138, 60, 30)         # a deep red-brown, darker than the cello's orange (188, 84, 40)
VARNISH_HI = (172, 90, 46)      # the arch's crest catching light (value contrast for small sprites)
RIB = (96, 42, 22)
BACK = (112, 50, 26)
EDGE = (190, 124, 70)           # the bare edge / purfling band round the top plate
EBONY = (26, 22, 24)
EBONY_HI = (58, 50, 54)
MAPLE = (230, 202, 152)         # the bridge
NECKWOOD = (152, 92, 50)        # the varnished maple neck + head
BRASS = (214, 176, 84)          # the machine heads' plates and gears
KEY = (236, 226, 196)           # the machine keys (bone-white)
STEEL = (196, 200, 212)         # the endpin
HOLE = (16, 10, 10)
STRING = (206, 200, 188)


def hw_at(u):
    return float(np.interp(u, [p[0] for p in CBPROF], [p[1] for p in CBPROF])) * L


def arch_a(u):
    return ARCH_T * min(1.0, 4.4 * u * (1 - u)) ** .55


def arch_z(x, u):
    """the top's surface height above the rib line at (x, body fraction u): a low-poly tent (edge -> shoulder -> crest)"""
    w = hw_at(u)
    a = arch_a(u)
    r = min(1.0, abs(x) / max(w, 1e-6))
    return a * (1 - .25 * r / .45) if r < .45 else a * .75 * (1 - r) / .55


def back_z(u):
    """the flat back's plane (z), breaking forward over the top fifth toward the neck"""
    zb = -RIBS / 2
    return zb + BREAK_IN * max(0.0, (u - BREAK_U) / (1 - BREAK_U))


def _face(g, pts, col, ref, **kw):
    g.face([tuple(p) for p in pts], col, ref=tuple(ref), **kw)


def body(g):
    """the body: ribs all round, the top plate as a three-step tent (crest, shoulders, edge), the flat back with its
    break toward the neck"""
    zf = RIBS / 2
    rows = []
    for u, hw in CBPROF:
        rows.append((u, Y0 + u * L, hw * L, arch_a(u), back_z(u)))
    cen = (0, Y0 + L / 2, 0)
    for (ua, ya, wa, ta, ba), (ub, yb, wb, tb, bb) in zip(rows, rows[1:]):
        ym = (ya + yb) / 2
        for sg in (-1, 1):
            e = [(sg * wa, ya, zf), (sg * wb, yb, zf)]
            sh = [(sg * .45 * wa, ya, zf + .75 * ta), (sg * .45 * wb, yb, zf + .75 * tb)]
            cr = [(0, ya, zf + ta), (0, yb, zf + tb)]
            _face(g, [e[0], e[1], sh[1], sh[0]], VARNISH, (sg * wa * .7, ym, zf - 1))
            _face(g, [sh[0], sh[1], cr[1], cr[0]], VARNISH_HI if sg > 0 else VARNISH, (sg * wa * .2, ym, zf - 1))
            # the flat back (one plane across, split at the centre so the break reads in two tones)
            _face(g, [(sg * wa, ya, ba), (sg * wb, yb, bb), (0, yb, bb), (0, ya, ba)], BACK, (sg * wa * .5, ym, ba + 1))
            # ribs, with a pale edge band along the top plate's rim (the purfling / bare edge reads as an outline)
            zr = zf - .03
            _face(g, [(sg * wa, ya, ba), (sg * wb, yb, bb), (sg * wb, yb, zr), (sg * wa, ya, zr)], RIB, cen)
            _face(g, [(sg * wa, ya, zr), (sg * wb, yb, zr), (sg * wb, yb, zf), (sg * wa, ya, zf)], EDGE, cen)
    for (u, y, w, a, bz), sgn in ((rows[0], -1), (rows[-1], 1)):         # the end walls
        _face(g, [(-w, y, bz), (w, y, bz), (w, y, zf), (-w, y, zf)], RIB, (0, y - sgn, 0))


def surf(x, u):
    return RIBS / 2 + arch_z(x, u)


def plate_decal(g, pts_xu, col, lift=.006, emit=True):
    """a flat decal following the top's arch (f-holes): points as (x, u)"""
    P = [(x, Y0 + u * L, surf(x, u) + lift) for x, u in pts_xu]
    c = np.mean(P, 0)
    _face(g, P, col, (c[0], c[1], c[2] - 1), emit=emit)


def ff_hole(g, sg):
    """an f-hole: two eyes + a slanted S-stem with the two notches (the bridge stands at the notches); a bass's are big"""
    (ua, xa), (ub, xb) = F_EYES
    xa, xb = sg * xa * L, sg * xb * L
    for (x, u), r in (((xa, ua), .03), ((xb, ub), .04)):
        ring = [(x + r * math.cos(2 * math.pi * k / 6 + .3), u + r / L * math.sin(2 * math.pi * k / 6 + .3)) for k in range(6)]
        plate_decal(g, ring, HOLE)
    n = 6
    for k in range(n):
        f0, f1 = k / n, (k + 1) / n

        def at(f):
            return xa + (xb - xa) * f + sg * .03 * math.sin(math.pi * f * 2), ua + (ub - ua) * f
        (x0, u0), (x1, u1) = at(f0), at(f1)
        w = .018 + .012 * math.sin(math.pi * (f0 + f1) / 2)
        plate_decal(g, [(x0 - w, u0), (x1 - w, u1), (x1 + w, u1), (x0 + w, u0)], HOLE)
    um, xm = (ua + ub) / 2, (xa + xb) / 2
    for dx in (-.026, .026):
        plate_decal(g, [(xm + dx - .009, um - .003), (xm + dx + .009, um - .003), (xm + dx + .009, um + .003),
                        (xm + dx - .009, um + .003)], HOLE)


def box(g, x0, x1, y0, y1, z0, z1, col, spec=0.):
    g.box(min(x0, x1), max(x0, x1), min(y0, y1), max(y0, y1), min(z0, z1), max(z0, z1), col, spec=spec)


def slab(g, pts, z0, z1, col, spec=0.):
    """a prism: the polygon pts (x, y) extruded from z0 to z1"""
    c = (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts), (z0 + z1) / 2)
    g.face([(x, y, z1) for x, y in pts], col, ref=c, spec=spec)
    g.face([(x, y, z0) for x, y in pts], col, ref=c, spec=spec)
    for k in range(len(pts)):
        (xa, ya), (xb, yb) = pts[k], pts[(k + 1) % len(pts)]
        g.face([(xa, ya, z0), (xb, yb, z0), (xb, yb, z1), (xa, ya, z1)], col, ref=c, spec=spec)


def wedge(g, bot, top, col, spec=0., top_col=None):
    """a hexahedron from two quads (each 4 (x, y, z) corners, same order): a slab whose thickness varies (the tilted
    fingerboard)"""
    c = tuple(np.mean(np.array(bot + top, float), 0))
    g.face(top, top_col or col, ref=c, spec=spec)
    g.face(bot, col, ref=c, spec=spec)
    for k in range(4):
        a, b = k, (k + 1) % 4
        g.face([bot[a], bot[b], top[b], top[a]], col, ref=c, spec=spec)


def xcyl(g, cy, cz, r, hl, col, n=8, spec=0., cx=0.):
    ring = [(cy + r * math.cos(2 * math.pi * k / n), cz + r * math.sin(2 * math.pi * k / n)) for k in range(n)]
    c = (cx, cy, cz)
    for x in (cx - hl, cx + hl):
        g.face([(x, y, z) for y, z in ring], col, ref=c, spec=spec)
    for k in range(n):
        (ya, za), (yb, zb) = ring[k], ring[(k + 1) % n]
        g.face([(cx - hl, ya, za), (cx + hl, ya, za), (cx + hl, yb, zb), (cx - hl, yb, zb)], col, ref=c, spec=spec)


def ycyl(g, cx, cz, r, y0, y1, col, n=6, spec=0.):
    """a vertical rod (the endpin)"""
    ring = [(cx + r * math.cos(2 * math.pi * k / n), cz + r * math.sin(2 * math.pi * k / n)) for k in range(n)]
    c = (cx, (y0 + y1) / 2, cz)
    for y in (y0, y1):
        g.face([(x, y, z) for x, z in ring], col, ref=c, spec=spec)
    for k in range(n):
        (xa, za), (xb, zb) = ring[k], ring[(k + 1) % n]
        g.face([(xa, y0, za), (xb, y0, zb), (xb, y1, zb), (xa, y1, za)], col, ref=c, spec=spec)


def contrabass(g, chunk=1.0):
    """the whole instrument standing on its endpin (y up, the top plate facing +z); returns string geometry"""
    zf = RIBS / 2
    yt = Y0 + L
    ch = chunk
    body(g)
    for sg in (-1, 1):
        ff_hole(g, sg)
    yn = yt + NECK                                                   # the nut
    # the fingerboard rises off the top toward the bridge: its underside .05 L above the top at the body's end, .10 L
    # above it where it stops (66 %), so the tall bridge's strings clear it
    yfb = Y0 + U_FB_END * L
    z_fb_end = surf(0, U_FB_END) + .07 * L
    z_root = zf + ARCH_T + .02 * L
    z_nut = z_root - .01 * L

    def fbz(y):
        return z_fb_end + (z_nut - z_fb_end) * (y - yfb) / (yn - yfb)
    hw0, hw1 = FB_END_W / 2 * min(ch, 1.3), NUT_W / 2 * ch
    th = .028 * L
    bot = [(-hw0, yfb, fbz(yfb)), (hw0, yfb, fbz(yfb)), (hw1, yn, fbz(yn)), (-hw1, yn, fbz(yn))]
    top = [(x, y, z + th) for x, y, z in bot]
    wedge(g, bot, top, EBONY, spec=.3, top_col=EBONY_HI)
    # neck (maple, varnished) behind the fingerboard, from the shoulders to the nut; the heel on the top block
    nk0, nk1 = .052 * L * ch / 1.6 + .02, .04 * L * ch / 1.6 + .015
    nb = [(-nk0, yt - .06 * L, zf - .04 * L), (nk0, yt - .06 * L, zf - .04 * L), (nk1, yn, fbz(yn) - .045 * L),
          (-nk1, yn, fbz(yn) - .045 * L)]
    nt = [(-nk0, yt - .06 * L, fbz(yt - .06 * L)), (nk0, yt - .06 * L, fbz(yt - .06 * L)), (nk1, yn, fbz(yn)),
          (-nk1, yn, fbz(yn))]
    wedge(g, nb, nt, NECKWOOD)
    box(g, -hw1 - .006, hw1 + .006, yn - .012, yn + .01, fbz(yn), fbz(yn) + th + .012, (232, 222, 196))   # the nut
    # pegbox + scroll
    ys = yn + .64 * HEAD
    zc = fbz(yn) - .01 * L
    slab(g, [(-.05 * L * ch / 1.6 - .012, yn), (.05 * L * ch / 1.6 + .012, yn), (.044 * L * ch / 1.6 + .01, ys),
             (-.044 * L * ch / 1.6 - .01, ys)], zc - .08 * L, zc + .015 * L, NECKWOOD)
    slab(g, [(-.03 * L, yn + .02), (.03 * L, yn + .02), (.026 * L, ys - .02), (-.026 * L, ys - .02)],
         zc + .001, zc + .016 * L, HOLE)                              # the open pegbox
    sc_y, sc_z = yn + .82 * HEAD, zc - .045 * L
    hl = .046 * L * ch / 1.6 + .01
    xcyl(g, sc_y, sc_z, .085 * L, hl, NECKWOOD)                       # the scroll: an outer turn ...
    xcyl(g, sc_y + .01, sc_z + .006, .05 * L, hl + .004, (110, 60, 32))   # ... the inner turn (darker)
    xcyl(g, sc_y + .016, sc_z + .012, .02 * L, hl + .008, (72, 38, 22))   # ... and the eye
    # machine heads: a brass plate down each side of the pegbox, two gears + a bone-white key per side, the keys
    # sticking straight out sideways (front view) and a little back
    pw = .05 * L * ch / 1.6 + .012
    for sg in (-1, 1):
        box(g, sg * pw, sg * (pw + .012), yn + .06 * HEAD, ys - .02, zc - .05 * L, zc - .01 * L, BRASS, spec=.7)
        for f in (.28, .62):
            y = yn + (ys - yn) * f
            box(g, sg * (pw + .012), sg * (pw + .045 * L), y - .006 * ch, y + .006 * ch, zc - .036 * L, zc - .024 * L,
                BRASS, spec=.6)                                       # the shaft
            box(g, sg * (pw + .045 * L), sg * (pw + .095 * L), y - .034 * ch, y + .034 * ch, zc - .048 * L,
                zc - .012 * L, KEY, spec=.4)                          # the key (thumb piece)
    # bridge (maple): tall, with feet on the arch at the f-holes' notches, an arched top
    yb = Y0 + U_BRIDGE * L
    zb0 = surf(0, U_BRIDGE)
    hb = BRIDGE_W / 2
    g.face([(-hb, yb, zb0), (hb, yb, zb0), (hb * .62, yb, zb0 + BRIDGE_H), (0, yb, zb0 + BRIDGE_H * 1.05),
            (-hb * .62, yb, zb0 + BRIDGE_H)], MAPLE, ref=(0, yb - 1, zb0), two=True)
    box(g, -hb, -hb * .55, yb - .012, yb + .012, zb0 - .01, zb0 + .03, MAPLE)          # its feet
    box(g, hb * .55, hb, yb - .012, yb + .012, zb0 - .01, zb0 + .03, MAPLE)
    # tailpiece (ebony), slanting up from the saddle toward the bridge
    ta, tb = Y0 + TAIL[0] * L, Y0 + TAIL[1] * L
    za, zb_ = surf(0, TAIL[0]) + .015, surf(0, TAIL[1]) + .05 * L
    tw0, tw1 = .02 * L, .043 * L
    pts = [(-tw0, ta, za), (tw0, ta, za), (tw1, tb, zb_), (-tw1, tb, zb_)]
    _face(g, pts, EBONY, (0, (ta + tb) / 2, za - 1), spec=.3)
    _face(g, [(x, y, z - .03) for x, y, z in pts], EBONY, (0, (ta + tb) / 2, za + 1))
    for (a, b) in ((0, 3), (1, 2)):
        _face(g, [pts[a], pts[b], (pts[b][0], pts[b][1], pts[b][2] - .03), (pts[a][0], pts[a][1], pts[a][2] - .03)],
              EBONY_HI, (0, (ta + tb) / 2, za))
    box(g, -.025 * L, .025 * L, Y0, Y0 + .02, zf - .01, zf + .02, EBONY)          # saddle
    # endpin: the collar on the end block, the steel rod, a dark rubber tip
    ycyl(g, 0, 0, .022 * L, Y0 - .035 * L, Y0 + .005, EBONY, n=8)
    ycyl(g, 0, 0, .007 * L * ch, Y0 - ENDPIN, Y0 - .03 * L, STEEL, n=6, spec=.6)
    ycyl(g, 0, 0, .012 * L * ch, Y0 - ENDPIN - .012, Y0 - ENDPIN + .02, (40, 36, 38), n=6)
    # strings: E A D G, tail -> bridge -> nut (lines: the Pixel Pitch video draws them; ball sprites drop them)
    TBN = []
    for i in range(4):
        f = i / 3
        tail = ((-1.5 + i) * tw1 * .5, tb - .02, zb_ + .004)
        brg = (-hb * .5 + hb * f, yb, zb0 + BRIDGE_H * (1.0 + .05 * math.sin(math.pi * f)))
        nut = (-hw1 * .7 + 1.4 * hw1 * f, yn, fbz(yn) + th + .012)
        g.line(tail, brg, (150, 146, 140))
        g.line(brg, nut, STRING)
        TBN.append((tail, brg, nut))
    return TBN


def build(g, t, ev, fx, s):
    contrabass(g, chunk=s.get("chunk", 1.0))
    yb = Y0 + U_BRIDGE * L
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", (.6, yb + .8, surf(0, U_BRIDGE)), age))


if __name__ == "__main__":
    import piano_lowpoly as P
    g = P.Geo()
    build(g, 0.0, [], [], SCENES[0])
    V = np.concatenate([f[0] for f in g.F])
    print(len(g.F), "faces, extent:", np.round(V.max(0) - V.min(0), 3), "(body L = %.3f)" % L)

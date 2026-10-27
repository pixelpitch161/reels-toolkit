"""Pixel Pitch: the modern viola - low-poly model (and, below, its voice). No history video yet: built for BALLFIGHT #16
(Viola vs Violin) and used read-only through projects/which_instrument_wins/pp_bridge.py, the same way the other
instruments' modern showcase models are. A later Pixel Pitch viola video can grow from this file (add SCENES rows,
backdrop, music) without touching the model.

    python3 viola_lowpoly.py                # face count + extent, and the size next to violin_lowpoly's modern violin

Model from reference photos (Wikimedia Commons, studied in Chrome, 2026-10-27), in the same units as violin_lowpoly's
modern violin (body L = .95 for a 356 mm violin, so 2.67 units per metre):
  - "Bratsche.jpg" (public domain): a 16.5" (419 mm) viola, straight-on front and side views on white, 700 x 1100 px.
    Measured per pixel row in a canvas (0.624 mm / px): body 672 px; upper bout 311 px (194 mm, 46.3 % of the length),
    C-bout 207 px (129 mm, 30.8 %), lower bout 395 px (246 mm, 58.8 %); upper corners at 64.7 % from the bottom, lower
    corners at 42.4 %; f-hole eyes at 53.6 % (+-6.5 %) and 38.7 % (+-15.7 %); bridge at 45.4 %, 53 mm wide; tailpiece
    3.7-30.5 %; fingerboard end at 63.2 % (44 mm wide, 24 mm at the nut); neck 34 % of the body, pegbox + scroll 24 %;
    pegs out to +-11 % of the body. Side view: ribs + plate edges 77 px (11.5 % of the length), the arching adds ~2.5 %
    on each side, so the body is ~16 % of its length deep (the violin model is a flat 10.5 %).
  - "Violin-Viola.jpg" (CC BY-SA 3.0): a violin and a viola side by side in one photo: the viola's body is 1.09-1.18x
    longer and its lower bout 1.18x wider (that viola is a small one; 16.5" is 1.18x a 14" violin).
  - "Viola Aldo Gallipoli 1990.jpg" (CC BY-SA 4.0): the outline, f-holes and the dark ebony fittings checked on a second
    instrument (black background).
The viola is a scaled-up violin with deeper ribs and a fuller arch: here L = 1.118 (419 mm), ribs .128 deep, top arch
.028, back arch .022, darker varnish; no bow on the model (the BALLFIGHT bow / weapon is drawn separately)."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files

OUT, TAG = "viola_lowpoly.mp4", "va"
BPM = 100
SHOW_BARS = 2
HEAD_INTRO = "THE VIOLA"

# one scene for now: the modern viola (the bridge builds it at rest, voice() plays single notes)
_NOTES = [(0, 60, 2, .85), (2, 64, 1, .85), (3, 67, 1, .85), (4, 65, 2, .85), (6, 60, 2, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="MODERN VIOLA", credit="16.5 INCH VIOLA", notes=_NOTES),
    # the same viola for small sprites (BALLFIGHT balls, ~25-60 px tall): true proportions, but the neck, fingerboard,
    # pegs and scroll are thickened x`chunk` across so they read as more than a one-pixel line next to the body.
    dict(key="ball", theme="intro", name="MODERN VIOLA", credit="16.5 INCH VIOLA", notes=_NOTES, chunk=1.5),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


# ---------------------------------------------------------------- measurements (fractions of the body length L)
U_PER_M = .95 / .356            # violin_lowpoly's scale: its modern violin's 356 mm body is L = .95
L = .419 * U_PER_M              # 16.5" viola body: 1.118
RIBS = .115 * L                 # rib height + plate edges (side view)
ARCH_T, ARCH_B = .025 * L, .020 * L     # top / back arching above the rib line
Y0 = .2                         # body bottom (as the violin model, which stands on a stand at y = 0)
# half-widths along the body, u from the bottom (end button) to the top (neck root), from "Bratsche.jpg"
VAPROF = [(0, .05), (.015, .085), (.031, .151), (.067, .214), (.103, .251), (.156, .280), (.21, .294), (.26, .291),
          (.32, .273), (.38, .246), (.418, .254), (.436, .238), (.455, .176), (.50, .161), (.56, .154), (.60, .153),
          (.628, .163), (.645, .217), (.665, .212), (.70, .208), (.74, .218), (.79, .228), (.83, .230), (.87, .224),
          (.915, .203), (.945, .178), (.975, .135), (.99, .092), (1, .05)]
U_BRIDGE, BRIDGE_W, BRIDGE_H = .454, .053 * U_PER_M, .040 * U_PER_M
U_FB_END, FB_END_W, NUT_W = .632, .044 * U_PER_M, .024 * U_PER_M
NECK, HEAD = .342 * L, .238 * L                     # nut -> body, scroll top -> nut
TAIL = (.037, .305)                                 # tailpiece span (u)
F_EYES = ((.536, .065), (.387, .157))               # f-hole (u, |x| / L): upper eye, lower eye

VARNISH = (128, 52, 30)         # darker than the violin's (156, 52, 30): a deep red-brown
VARNISH_HI = (164, 78, 40)      # the arch's crest catching light (value contrast for small sprites)
RIB = (88, 34, 20)
BACK = (104, 42, 24)
EDGE = (186, 120, 64)           # the bare edge / purfling band round the top plate
EBONY = (26, 22, 24)
EBONY_HI = (56, 48, 52)
MAPLE = (228, 198, 148)         # the bridge
NECKWOOD = (150, 90, 52)        # the varnished maple neck + head
HOLE = (16, 10, 10)
STRING = (214, 210, 200)


def arch_z(x, u, top=True):
    """surface height above the rib line at (x, body fraction u): a low-poly tent (edge -> shoulder -> crest)"""
    w = np.interp(u, [p[0] for p in VAPROF], [p[1] for p in VAPROF]) * L
    a = (ARCH_T if top else ARCH_B) * min(1.0, 4.4 * u * (1 - u)) ** .55
    r = min(1.0, abs(x) / max(w, 1e-6))
    return a * (1 - .25 * r / .45) if r < .45 else a * .75 * (1 - r) / .55


def _face(g, pts, col, ref, **kw):
    g.face([tuple(p) for p in pts], col, ref=tuple(ref), **kw)


def body(g):
    """the arched body: ribs all round, the top and back plates as three-step tents (crest, shoulders, edge)"""
    zf, zb = RIBS / 2, -RIBS / 2
    rows = []
    for u, hw in VAPROF:
        w = hw * L
        y = Y0 + u * L
        at, ab = ARCH_T * min(1.0, 4.4 * u * (1 - u)) ** .55, ARCH_B * min(1.0, 4.4 * u * (1 - u)) ** .55
        rows.append((y, w, at, ab))
    cen = (0, Y0 + L / 2, 0)
    for (ya, wa, ta, ba), (yb, wb, tb, bb) in zip(rows, rows[1:]):
        ym = (ya + yb) / 2
        # top plate: edge (x = +-w) at zf, shoulder (+-.45 w) at zf + .75 a, crest (0) at zf + a
        for sg in (-1, 1):
            e = [(sg * wa, ya, zf), (sg * wb, yb, zf)]
            sh = [(sg * .45 * wa, ya, zf + .75 * ta), (sg * .45 * wb, yb, zf + .75 * tb)]
            cr = [(0, ya, zf + ta), (0, yb, zf + tb)]
            _face(g, [e[0], e[1], sh[1], sh[0]], VARNISH, (sg * wa * .7, ym, zf - 1))
            _face(g, [sh[0], sh[1], cr[1], cr[0]], VARNISH_HI if sg > 0 else VARNISH, (sg * wa * .2, ym, zf - 1))
            eb = [(sg * wa, ya, zb), (sg * wb, yb, zb)]
            shb = [(sg * .45 * wa, ya, zb - .75 * ba), (sg * .45 * wb, yb, zb - .75 * bb)]
            crb = [(0, ya, zb - ba), (0, yb, zb - bb)]
            _face(g, [eb[0], eb[1], shb[1], shb[0]], BACK, (sg * wa * .7, ym, zb + 1))
            _face(g, [shb[0], shb[1], crb[1], crb[0]], BACK, (sg * wa * .2, ym, zb + 1))
            # ribs, with a pale edge band along the top plate's rim (the purfling / bare edge reads as an outline)
            zr = zf - .012
            _face(g, [(sg * wa, ya, zb), (sg * wb, yb, zb), (sg * wb, yb, zr), (sg * wa, ya, zr)], RIB, cen)
            _face(g, [(sg * wa, ya, zr), (sg * wb, yb, zr), (sg * wb, yb, zf), (sg * wa, ya, zf)], EDGE, cen)
    for (y, w, at, ab), sgn in ((rows[0], -1), (rows[-1], 1)):          # the end walls
        _face(g, [(-w, y, zb), (w, y, zb), (w, y, zf), (-w, y, zf)], RIB, (0, y - sgn, 0))


def surf(x, u):
    return RIBS / 2 + arch_z(x, u)


def plate_decal(g, pts_xu, col, lift=.003, emit=True):
    """a flat decal following the top's arch (f-holes, tailpiece shadow): points as (x, u)"""
    P = [(x, Y0 + u * L, surf(x, u) + lift) for x, u in pts_xu]
    c = np.mean(P, 0)
    _face(g, P, col, (c[0], c[1], c[2] - 1), emit=emit)


def ff_hole(g, sg):
    """an f-hole: two eyes + a slanted stem with the two notches (the bridge sits at the notches)"""
    (ua, xa), (ub, xb) = F_EYES
    xa, xb = sg * xa * L, sg * xb * L
    for (x, u), r in (((xa, ua), .011), ((xb, ub), .014)):
        ring = [(x + r * math.cos(2 * math.pi * k / 6 + .3), u + r / L * math.sin(2 * math.pi * k / 6 + .3)) for k in range(6)]
        plate_decal(g, ring, HOLE)
    n = 6
    for k in range(n):                                               # the stem: an S-curve from eye to eye
        f0, f1 = k / n, (k + 1) / n
        def at(f):
            return xa + (xb - xa) * f + sg * .012 * math.sin(math.pi * f * 2), ua + (ub - ua) * f
        (x0, u0), (x1, u1) = at(f0), at(f1)
        w = .006 + .004 * math.sin(math.pi * (f0 + f1) / 2)
        plate_decal(g, [(x0 - w, u0), (x1 - w, u1), (x1 + w, u1), (x0 + w, u0)], HOLE)
    um = (ua + ub) / 2                                               # the notches
    xm = (xa + xb) / 2
    for dx in (-.012, .012):
        plate_decal(g, [(xm + dx - .004, um - .004), (xm + dx + .004, um - .004), (xm + dx + .004, um + .004),
                        (xm + dx - .004, um + .004)], HOLE)


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


def xcyl(g, cy, cz, r, hl, col, n=8, spec=0.):
    ring = [(cy + r * math.cos(2 * math.pi * k / n), cz + r * math.sin(2 * math.pi * k / n)) for k in range(n)]
    c = (0, cy, cz)
    for x in (-hl, hl):
        g.face([(x, y, z) for y, z in ring], col, ref=c, spec=spec)
    for k in range(n):
        (ya, za), (yb, zb) = ring[k], ring[(k + 1) % n]
        g.face([(-hl, ya, za), (hl, ya, za), (hl, yb, zb), (-hl, yb, zb)], col, ref=c, spec=spec)


def octo(cx, cy, rx, ry, n=8):
    return [(cx + rx * math.cos(2 * math.pi * k / n + math.pi / n), cy + ry * math.sin(2 * math.pi * k / n + math.pi / n))
            for k in range(n)]


def viola(g, chunk=1.0):
    """the whole instrument, standing on its end button (y up, the top plate facing +z); returns string geometry"""
    zf = RIBS / 2
    yt = Y0 + L
    body(g)
    for sg in (-1, 1):
        ff_hole(g, sg)
    cz = zf + ARCH_T * .9                                            # the neck's top meets the fingerboard here
    yn = yt + NECK                                                   # the nut
    ch = chunk
    # neck (maple, varnished) from the body's top block to the nut, its heel on the top plate
    slab(g, [(-.042 * ch, yt - .035), (.042 * ch, yt - .035), (.034 * ch, yn), (-.034 * ch, yn)], zf - .045, cz,
         NECKWOOD)
    slab(g, [(-.05, yt - .05), (.05, yt - .05), (.044, yt + .02), (-.044, yt + .02)], zf - .02, zf + ARCH_T * .5, NECKWOOD)
    # fingerboard (ebony), overhanging the top down to 63 % of the body; a lighter top face so it reads on the body
    yfb = Y0 + U_FB_END * L
    hw0, hw1 = FB_END_W / 2 * min(ch, 1.25), NUT_W / 2 * ch
    slab(g, [(-hw0, yfb), (hw0, yfb), (hw1, yn), (-hw1, yn)], cz, cz + .038, EBONY, spec=.3)
    g.face([(-hw0 * .8, yfb + .01, cz + .0385), (hw0 * .8, yfb + .01, cz + .0385), (hw1 * .8, yn - .01, cz + .0385),
            (-hw1 * .8, yn - .01, cz + .0385)], EBONY_HI, ref=(0, (yfb + yn) / 2, cz), spec=.3)
    box(g, -hw1 - .004, hw1 + .004, yn - .008, yn + .006, cz, cz + .046, (232, 222, 196))     # the nut (bone)
    # pegbox + scroll
    yp = yn + .2 * HEAD                                              # pegbox walls from the nut up to the scroll
    ys = yn + .62 * HEAD
    slab(g, [(-.036 * ch, yn), (.036 * ch, yn), (.032 * ch, ys), (-.032 * ch, ys)], zf - .06, cz + .02, NECKWOOD)
    slab(g, [(-.022 * ch, yn + .01), (.022 * ch, yn + .01), (.019 * ch, ys - .01), (-.019 * ch, ys - .01)],
         cz + .002, cz + .021, HOLE)                                 # the open pegbox
    sc_y, sc_z = yn + .80 * HEAD, cz - .028
    xcyl(g, sc_y, sc_z, .062, .034 * ch, NECKWOOD)                   # the scroll: an outer turn ...
    xcyl(g, sc_y + .006, sc_z + .004, .036, .037 * ch, (106, 58, 32))   # ... the inner turn (darker)
    xcyl(g, sc_y + .01, sc_z + .008, .014, .039 * ch, (70, 36, 22))  # ... and the eye
    # pegs: two per side, staggered, shaft + a big thumb piece (ebony) so they read at sprite size
    for k, f in enumerate((.25, .42, .58, .75)):
        y = yn + (ys - yn) * f
        sg = -1 if k % 2 == 0 else 1
        z = cz - .012
        box(g, sg * .03 * ch, sg * .088, y - .007, y + .007, z - .007, z + .007, EBONY)
        box(g, sg * .088, sg * .132, y - .022, y + .022, z - .009, z + .009, EBONY, spec=.3)
        box(g, sg * .118, sg * .126, y - .016, y + .016, z + .009, z + .012, EBONY_HI)
    # bridge (maple) on the arch at 45 % of the body
    yb = Y0 + U_BRIDGE * L
    zb0 = surf(0, U_BRIDGE)
    hb = BRIDGE_W / 2
    g.face([(-hb, yb, zb0), (hb, yb, zb0), (hb * .7, yb, zb0 + BRIDGE_H), (-hb * .7, yb, zb0 + BRIDGE_H)], MAPLE,
           ref=(0, yb - 1, zb0), two=True)
    box(g, -hb, hb, yb - .006, yb + .006, zb0, zb0 + .016, MAPLE)                  # its feet
    # tailpiece (ebony), slanting up from the saddle toward the bridge
    ta, tb = Y0 + TAIL[0] * L, Y0 + TAIL[1] * L
    za, zb_ = surf(0, TAIL[0]) + .012, surf(0, TAIL[1]) + .03
    tw0, tw1 = .022 * L, .055 * L
    pts = [(-tw0, ta, za), (tw0, ta, za), (tw1, tb, zb_), (-tw1, tb, zb_)]
    _face(g, pts, EBONY, (0, (ta + tb) / 2, za - 1), spec=.3)
    _face(g, [(x, y, z - .018) for x, y, z in pts], EBONY, (0, (ta + tb) / 2, za + 1))
    for (a, b) in ((0, 3), (1, 2)):
        _face(g, [pts[a], pts[b], (pts[b][0], pts[b][1], pts[b][2] - .018), (pts[a][0], pts[a][1], pts[a][2] - .018)],
              EBONY_HI, (0, (ta + tb) / 2, za))
    for k in range(4):                                               # fine tuner / string-anchor dots (nickel)
        x = (-1.5 + k) * tw1 * .5
        box(g, x - .006, x + .006, tb - .03, tb - .016, zb_, zb_ + .01, (206, 208, 218), spec=.6)
    box(g, -.016, .016, Y0 - .012, Y0 + .004, -.016, .016, EBONY)                  # end button
    box(g, -.03, .03, Y0, Y0 + .012, zf - .005, zf + .008, EBONY)                  # saddle
    # chinrest (ebony), over the lower bout left of the tailpiece
    cx_, cu = -.082 * L, .088
    cz0 = surf(cx_, cu)
    slab(g, octo(cx_, Y0 + cu * L, .13 * L, .068 * L), cz0, cz0 + .042, EBONY, spec=.5)
    g.face([(x, y, cz0 + .0425) for x, y in octo(cx_, Y0 + cu * L, .09 * L, .042 * L)], EBONY_HI,
           ref=(cx_, Y0 + cu * L, cz0), spec=.5)
    # strings: C G D A, tail -> bridge -> nut (lines: the Pixel Pitch video draws them; ball sprites drop them)
    TBN = []
    for i in range(4):
        f = i / 3
        tail = ((-1.5 + i) * tw1 * .5, tb - .02, zb_ + .004)
        brg = (-hb * .62 + 1.24 * hb * f, yb, zb0 + BRIDGE_H)
        nut = (-hw1 * .7 + 1.4 * hw1 * f, yn, cz + .046)
        g.line(tail, brg, (150, 146, 140))
        g.line(brg, nut, STRING)
        TBN.append((tail, brg, nut))
    return TBN


def build(g, t, ev, fx, s):
    viola(g, chunk=s.get("chunk", 1.0))
    yb = Y0 + U_BRIDGE * L
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", (.3, yb + .5, surf(0, U_BRIDGE)), age))


# ---------------------------------------------------------------- sound: the darker bowed voice, a fifth below
# Tuned C3 G3 D4 A4, a fifth under the violin. The body is too small for its range (an ideal viola would be ~53 cm),
# so its main air and wood resonances sit low (~300 / ~600 Hz) and its bright band near 1.6 kHz, where the violin's is
# near 3 kHz: fewer high partials, a steeper roll-off, a husky, nasal, darker tone. A slightly slower bow attack, a
# little more bow noise, and a wider, slower vibrato than the violin's.
SR = 44100
FORM = [(300, .7, .45), (620, 1.0, .40), (1650, .55, .40)]     # (centre Hz, gain, log-width)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def bowed(f, dur, vel, att=.045, vib=.0055, vrate=5.2, vdel=.14, tilt=1.12, noise=.035):
    from scipy.signal import lfilter, butter, sosfilt
    rel = .16
    n = int((dur + rel) * SR)
    t = np.arange(n) / SR
    ph0 = (f * 7.13) % (2 * math.pi)                                 # deterministic vibrato phase per pitch
    fm = 1 + vib * np.clip((t - vdel) / .25, 0, 1) * np.sin(2 * np.pi * vrate * t + ph0)
    ph = 2 * np.pi * np.cumsum(f * fm) / SR
    sig = np.zeros(n)
    tot = 0.0
    for k in range(1, 40):
        fk = f * k
        if fk > 5000:
            break
        gk = (.22 + sum(a * math.exp(-(math.log(fk / fc) / w) ** 2) for fc, a, w in FORM)) / k ** tilt
        sig += gk * np.sin(k * ph)
        tot += gk
    sig /= tot
    rng = np.random.default_rng(int(f * 10) % 100003)               # bow hair noise, deterministic per pitch
    nz = sosfilt(butter(2, [700, 2600], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    sig += nz * noise * (.6 + .8 * np.exp(-t / .06))
    env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1.0, np.exp(-(t - dur) / (rel / 4)))
    env = env * (.86 + .14 * np.minimum(1, t / max(dur, .05)))
    sig = lfilter([.30], [1, -.70], sig)                            # a warmer low-pass than the violin's (.35 / .65)
    return sig * env * vel


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return bowed(_midi_hz(m), dur, v)


if __name__ == "__main__":
    import piano_lowpoly as P
    g = P.Geo()
    build(g, 0.0, [], [], SCENES[0])
    V = np.concatenate([f[0] for f in g.F])
    print(len(g.F), "faces, extent:", np.round(V.max(0) - V.min(0), 3))
    import violin_lowpoly as VL
    gv = P.Geo()
    VL.build_viol(gv, 0.0, [], [], dict(VL.SCENES[-1], ev=[]), "modern")
    Vv = np.concatenate([f[0] for f in gv.F])
    print("modern violin (incl. its bow) extent:", np.round(Vv.max(0) - Vv.min(0), 3))

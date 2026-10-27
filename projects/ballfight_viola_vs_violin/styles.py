"""#42 weapon styles. The VIOLIN looks exactly as in #28 (crossover rule), so its draw functions are imported from #28's
folder (series rule: styles live there so crossovers can import them), not copied:

    SV  projects/ballfight_drums_vs_violin/styles.py (#28): the bow (`bow_geom`, `draw_bow`, `bow_trail_polys`), the
        pizzicato darts (`dart_quads`, `draw_dart`), the stick / hair tones

New here, kept in this folder so later crossovers can import them:

    style 'wholenote'  COUNTING RESTS' whole note: the hollow oval of the notation, standing up as a heavy wheel and
                       rolling (the oval's turn bobs it up and down, its tilted hole spins), ink with a teal-lit rim;
                       `note_quads` / `draw_note` (and `note_shadow` for the canvas)
    'rest'             the giant quarter rest over the resting viola (a pixel glyph, `REST_GLYPH`, stamped by
                       `stamp_glyph`), with the small counted numbers above it (render.py)
    'eighth'           MELODY AT LAST's little eighth notes rising off the ball (`EIGHTH_GLYPH`)

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import importlib.util
import math
import os
import numpy as np
import scene as S

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, folder):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, '..', folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


SV = _load('styles28', 'ballfight_drums_vs_violin')        # VIOLIN (the bow, PIZZICATO's darts)
R = S.R_BALL

# ---------------------------------------------------------------- the whole note (a rolling wheel)
INK = np.array([44, 44, 66], float)            # the notehead's ink (a deep blue-black, so the teal rim reads)
NOTE_A, NOTE_B = 1.0, 0.68                     # outer oval (x the note's radius): wider than tall, as engraved
HOLE_A, HOLE_B = 0.36, 0.56                    # the hole: a narrow oval ...
HOLE_TILT = math.radians(-32)                  # ... leaning like "/" (thick at 10 and 4 o'clock)
THICK = 0.22                                   # slab thickness (x radius)
DRAW = 1.3                                     # drawn half-width x the hit radius: a crush lands as the glass touches the
                                               # oval's edge (the engine hits at r + 0.9 R between centres)
K_PTS = 20


def _oval(a, b, tilt, phi):
    th = np.linspace(0, 2 * math.pi, K_PTS + 1)[:-1]
    x, y = a * np.cos(th), b * np.sin(th)
    c, s = math.cos(tilt - phi), math.sin(tilt - phi)
    return np.stack([x * c - y * s, x * s + y * c], 1)


def support(phi):
    """half-height of the outer oval turned by phi (the wheel's centre height above the canvas, x radius)"""
    return math.sqrt((NOTE_A * math.sin(phi)) ** 2 + (NOTE_B * math.cos(phi)) ** 2)


def note_axes(cam, lean=0.25):
    """the notehead's plane: screen-right (a) and an up axis (b) between the camera's up and the world's up, so the
    oval faces the viewer like the engraved glyph (a true edge-on wheel reads as a sliver from this high camera) with
    a little lean that shows the top of its rim; n = its face normal (toward the camera)"""
    a = cam.M[0].copy()
    a[1] = 0.0
    a /= np.linalg.norm(a)
    b = (1 - lean) * cam.M[1] + lean * np.array([0.0, 1.0, 0.0])
    b -= a * (b @ a)
    b /= np.linalg.norm(b)
    n = np.cross(a, b)
    if n @ (cam.pos - cam.target) < 0:
        n = -n
    return a, b, n


def note_centre(c, r, phi, lift, b):
    """world centre of a note at ring point c: high enough that its lowest point touches the canvas"""
    hc = b[1] * r * support(phi) + lift
    return S.to_world(c[0], c[1], 0.0) + np.array([0.0, hc, 0.0]) - np.array([b[0], 0.0, b[2]]) * r * support(phi) * 0.0


def note_quads(cam, c, r, phi, lift=0.0, scale=1.0):
    """world quads [(Q (4, 3), normal (3,), kind)] of a whole note standing on the canvas at ring point c (radius r,
    turned by phi as it rolls), `lift` above its rolling height (a dropped note falls)"""
    r = r * scale * DRAW
    a, b, nrm = note_axes(cam)
    C = note_centre(c, r, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * r
    I = _oval(HOLE_A, HOLE_B, HOLE_TILT, phi) * r
    th = THICK * r / 2

    def Wp(pq, w):
        return C + a * pq[0] + b * pq[1] + nrm * w
    out = []
    for j in range(K_PTS):
        j1 = (j + 1) % K_PTS
        for sgn in (1, -1):
            out.append((np.array([Wp(O[j], sgn * th), Wp(O[j1], sgn * th), Wp(I[j1], sgn * th), Wp(I[j], sgn * th)]),
                        nrm * sgn, 'face'))
        mo = (O[j] + O[j1]) / 2
        rad = a * mo[0] + b * mo[1]
        rad = rad / (np.linalg.norm(rad) + 1e-9)
        out.append((np.array([Wp(O[j], th), Wp(O[j1], th), Wp(O[j1], -th), Wp(O[j], -th)]), rad, 'rim'))
        mi = (I[j] + I[j1]) / 2
        radi = -(a * mi[0] + b * mi[1])
        radi = radi / (np.linalg.norm(radi) + 1e-9)
        out.append((np.array([Wp(I[j], th), Wp(I[j1], th), Wp(I[j1], -th), Wp(I[j], -th)]), radi, 'hole'))
    return out


def draw_note(fr, cam, L, shade, c, r, phi, col, lift=0.0, scale=1.0, glow=0.0):
    """paint a whole note (far quads first); col = the owner's colour (the rim light); glow 0..1 brightens the rim"""
    quads = note_quads(cam, c, r, phi, lift, scale)
    ctr = np.array([Q.mean(0) for Q, n, k in quads])
    d = cam.depth(ctr)
    order = np.argsort(-d)
    nrm = np.array([n for Q, n, k in quads])
    base = np.array([INK if k == 'face' else (INK * 0.6 + col * 0.45 if k == 'rim' else INK * 0.7)
                     for Q, n, k in quads])
    cols = shade(L, nrm, ctr, base, rimk=0.7)
    for j in order:
        Q, n, kind = quads[j]
        cc = cols[j]
        if kind == 'rim':
            cc = cc + col * (0.35 + 0.45 * glow) + 20
        elif kind == 'face':
            cc = cc + col * 0.08 * (1 + 2 * glow)
        fr.poly(cam, Q, np.clip(cc, 0, 255))
    # a lit edge along the top of the outer oval (the spot catches it): reads on the dark canvas shadows
    a, b, nrm = note_axes(cam)
    rr = r * scale * DRAW
    C = note_centre(c, rr, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * rr
    top = [C + a * p + b * q + nrm * THICK * rr / 2 for p, q in O if q > 0.25 * rr]
    if len(top) > 1:
        sp = cam.wp(np.array(top))
        order = np.argsort(sp[:, 0])
        fr.draw.line([tuple(sp[j]) for j in order], fill=tuple(int(v) for v in np.clip(col * 0.6 + 110, 0, 255)), width=1)


def note_shadow(cam, c, r, phi, lift=0.0):
    """the note's outer oval as a world polygon (for the spot's shadow on the canvas)"""
    a, b, nrm = note_axes(cam)
    r = r * DRAW
    C = note_centre(c, r, phi, lift, b)
    O = _oval(NOTE_A, NOTE_B, 0.0, phi) * r
    return np.array([C + a * p + b * q for p, q in O[::2]])


# ---------------------------------------------------------------- pixel glyphs (rest, eighth note)
REST_GLYPH = ["..#....", "..##...", "...##..", "....##.", "...###.", "..###..", ".###...", ".##....", "..##...",
              "...##..", "...###.", "..#.##.", ".#.....", ".#.....", "..##...", "....#.."]      # the quarter rest
EIGHTH_GLYPH = ["..#..", "..##.", "..#.#", "..#..", "..#..", "###..", "###.."]               # a flagged eighth


def glyph(rows, scale=1):
    g = np.array([[ch == '#' for ch in r] for r in rows], bool)
    return g.repeat(scale, 0).repeat(scale, 1)


def stamp_glyph(img, g, cx, cy, col, alpha=1.0, outline=(16, 12, 26), shadow=True):
    """stamp a boolean glyph centred at (cx, cy) on a uint8 / float image with a 1 px dark outline (+ drop shadow)"""
    from reelkit.font import dilate
    H, W = img.shape[:2]
    o = dilate(g, 1)
    gh, gw = o.shape
    x0, y0 = int(round(cx - gw / 2)), int(round(cy - gh / 2))
    layers = ([(o, (0, 0, 0), 1, 2, 0.45)] if shadow else []) + [(o, outline, 0, 0, 1.0)]
    off = (o.shape[0] - g.shape[0]) // 2
    layers.append((np.pad(g, ((off, off), (off, off))), col, 0, 0, 1.0))
    for m, c, ox, oy, a0 in layers:
        xa, ya = max(0, x0 + ox), max(0, y0 + oy)
        xb, yb = min(W, x0 + ox + gw), min(H, y0 + oy + gh)
        if xb <= xa or yb <= ya:
            continue
        sub = m[ya - y0 - oy:yb - y0 - oy, xa - x0 - ox:xb - x0 - ox]
        reg = img[ya:yb, xa:xb].astype(np.float32)
        a = alpha * a0
        reg[sub] = reg[sub] * (1 - a) + np.array(c, float) * a
        img[ya:yb, xa:xb] = reg.astype(img.dtype)
    return x0, y0, gw, gh

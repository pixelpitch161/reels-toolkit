"""kit: what the renderer kit's own helpers give, for the ports in bf-picture.

studio/reference/picture_kit/ (every .npz has a sidecar .json: vectors-convention.md):

    config.npz   reelkit.config: BAYER, BAYER_FULL, XX and YY as they are, and ss, ease, lerp, mixc, whole_period,
                 loop_bpm, safe_x1 and safe_overshoot on fixed inputs. The sidecar's "constants" holds W, H, UP, FPS,
                 DUR, NF, SAFE, SAFE_BUTTONS and SAFE_CX
    font.npz     reelkit.font: every glyph of the 5 x 7 pixel font, text_mask for a spread of strings and scales
                 (every glyph, lower case, characters the font does not have), dilate for the radii the kit uses,
                 and stamp onto a seeded noise frame with and without outline, shadow and alpha
    np.npz       the numpy semantics the renderer leans on, element by element: np.clip (two bounds and one),
                 astype(np.uint8 / np.int32 / np.int64) from floats, the two floor dithers, np.roll, np.nonzero

The renderer kit (the 3D-to-2D machinery of reelkit, each function on seeded inputs):

    camera.npz   reelkit.camera: Cam (the constructor's fields, to_cam, proj, wp, depth, pixel_rays, ground_hit),
                 clip_near, interp_keys
    geometry.npz reelkit.geometry: face, box, prism, lathe, catenary, thin_quad
    raster.npz   reelkit.raster: Frame (the constructor, poly, screen_poly, line, disc, set_array, array, depth) at
                 the full 360 x 640, the 60 glow kernels, apply_glows
    lighting.npz reelkit.lighting: Lights (key, fill, point) and shade
    shadows.npz  reelkit.shadows: MaskGrid (its size, raster, sample), proj_point
    lens.npz     reelkit.lens: oct_kernel, coc_map, dof
    fftconv.npz  what dof stands on: scipy.fft.rfftn and irfftn with the padded shapes fftconvolve uses, and
                 scipy.signal.fftconvolve(mode='same') itself
    post.npz     reelkit.post: bloom, grade, vignette, top_scrim
    hull.npz     scipy.spatial.ConvexHull(points).vertices for 2-D points (the order the spot's beam is drawn in)

A group is one file. Each case stores the inputs and what Python gave; one call per element where the function takes
scalars (ss, ease, ...), so a row of the output is exactly one Python call. Floats that Rust must match bit for bit
are float64 / float32 arrays or float.hex() strings.

This module holds the Group class, the first three groups and the export itself. The renderer kit's nine groups are
in kit_renderer_vectors.py, which `run` calls after them."""
import os

import numpy as np

from . import common

OUT = common.ref('picture_kit')
SEED = 20261117


class Group:
    """one vector file being filled: the cases (sidecar rows) and their arrays (npz members)"""

    def __init__(self, name, about, **top):
        self.name, self.about, self.top = name, about, top
        self.cases, self.arrays = [], {}

    def add(self, call, args, **arrays):
        """a case: `call` is the Python expression, `args` its JSON arguments, `arrays` role -> array ("in" is
        spelled in_)"""
        cid = f'c{len(self.cases):03d}'
        names = {}
        for role, a in arrays.items():
            role = role.rstrip('_')
            assert isinstance(a, np.ndarray) and a.dtype.str in common.NPZ_DTYPES, (self.name, cid, role, type(a))
            self.arrays[f'{cid}__{role}'] = np.ascontiguousarray(a)
            names[role] = f'{cid}__{role}'
        self.cases.append(dict(id=cid, call=call, args=args, arrays=names))
        return cid

    def write(self):
        """write <name>.npz and <name>.json, read the archive back and compare, then list both in the manifest"""
        from . import manifest
        npz, js = os.path.join(OUT, self.name + '.npz'), os.path.join(OUT, self.name + '.json')
        common.write_npz(npz, self.arrays, register=False)
        common.write_json(js, dict(about=self.about, tool='kit', **self.top, cases=self.cases), register=False)
        with np.load(npz, allow_pickle=False) as z:
            named = {m for c in self.cases for m in c['arrays'].values()}
            assert set(z.files) == set(self.arrays) == named, f'{self.name}: members and sidecar disagree'
            for k, a in self.arrays.items():
                b = z[k]
                assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), (self.name, k)
        manifest.add(npz, tool=__name__)
        manifest.add(js, tool=__name__)
        size = os.path.getsize(npz) + os.path.getsize(js)
        print(f'kit: {self.name:8s} {len(self.cases):4d} cases  {size / 1e3:8.1f} kB')
        return size


def rng(*key):
    return np.random.default_rng([SEED, *key])


def f64(values):
    return np.array([float(v) for v in values], np.float64)


def i64(values):
    return np.array([int(v) for v in values], np.int64)


# ================================================================ reelkit.config
def group_config():
    from reelkit import config as C
    g = Group('config', 'reelkit.config: the dither tables as they are and the scalar helpers on fixed inputs',
              constants=dict(W=C.W, H=C.H, UP=C.UP, FPS=C.FPS, DUR=common.hexf(C.DUR), NF=C.NF, SAFE=list(C.SAFE),
                             SAFE_BUTTONS=list(C.SAFE_BUTTONS), SAFE_CX=C.SAFE_CX),
              note='Scalar helpers: one Python call per row, with Python floats (ints for the safe-zone helpers); '
                   'the inputs are the arrays named by the argument names, "out" the results. BAYER_FULL, XX and YY '
                   'are (640, 360): (height, width).')
    assert C.BAYER.dtype == np.float64 and C.BAYER_FULL.shape == (C.H, C.W) and C.XX.dtype == np.float32
    g.add('reelkit.config.BAYER', {}, out=C.BAYER)
    g.add('reelkit.config.BAYER_FULL', {}, out=C.BAYER_FULL)
    g.add('reelkit.config.XX', {}, out=C.XX)
    g.add('reelkit.config.YY', {}, out=C.YY)

    r = rng(1)
    # ss(a, b, x): random edges and positions (b below a too), then the corners: x on and past each edge
    a = list(r.uniform(-5, 5, 300))
    b = [v + s * w for v, s, w in zip(a, r.choice([-1.0, 1.0], 300), r.uniform(1e-3, 4, 300))]
    x = [lo + (hi - lo) * u for lo, hi, u in zip(a, b, r.uniform(-0.6, 1.6, 300))]
    for lo, hi in ((0.0, 1.0), (0.0, 0.25), (60.88, 61.0), (1.0, 0.0), (-2.0, 3.0), (0.12, 0.0)):
        for xv in (lo, hi, (lo + hi) / 2, lo - 1.0, hi + 1.0, lo + (hi - lo) * 1e-9, hi - (hi - lo) * 1e-9, -0.0, 0.0,
                   lo + (hi - lo) / 3, lo + (hi - lo) * 0.75):
            a.append(lo), b.append(hi), x.append(xv)
    out = [C.ss(float(p), float(q), float(v)) for p, q, v in zip(a, b, x)]
    g.add('reelkit.config.ss(a, b, x)', {}, a=f64(a), b=f64(b), x=f64(x), out=f64(out))

    x = list(r.uniform(-0.5, 1.5, 300)) + [0.0, -0.0, 1.0, 0.5, 0.25, 0.75, 1e-9, 1 - 1e-9, 1e-300, -1.0, 2.0, 1 / 3,
                                             2 / 3, 0.1, 0.9, 5e-324, 1.0000000000000002, 0.9999999999999999]
    g.add('reelkit.config.ease(x)', {}, x=f64(x), out=f64(C.ease(float(v)) for v in x))

    a, b, u = r.uniform(-300, 300, 200), r.uniform(-300, 300, 200), r.uniform(-0.5, 1.5, 200)
    g.add('reelkit.config.lerp(a, b, u)', {}, a=a, b=b, u=u,
          out=f64(C.lerp(float(p), float(q), float(v)) for p, q, v in zip(a, b, u)))

    # mixc(a, b, u): colours as the kit gives them (whole numbers and floats), u inside and outside 0..1
    a = np.where(r.random((200, 3)) < 0.5, np.floor(r.uniform(0, 256, (200, 3))), r.uniform(0, 255, (200, 3)))
    b = np.where(r.random((200, 3)) < 0.5, np.floor(r.uniform(0, 256, (200, 3))), r.uniform(0, 255, (200, 3)))
    u = np.concatenate([r.uniform(-0.2, 1.2, 190), [0.0, 1.0, 0.5, 0.25, 1e-9, 1 - 1e-9, -0.0, 2.0, 1 / 3, 0.1]])
    out = np.stack([C.mixc(tuple(float(v) for v in p), tuple(float(v) for v in q), float(w))
                    for p, q, w in zip(a, b, u)])
    assert out.dtype == np.float64 and out.shape == (200, 3)
    g.add('reelkit.config.mixc(a, b, u)', {}, a=a, b=b, u=u, out=out)

    # whole_period(period, dur): random, then exact ties of dur / period (round is half to even) and periods
    # longer than the video
    period = list(r.uniform(0.3, 90, 200)) + [2.0, 2.0, 2.0, 4.0, 122.0, 61.0, 7.3, 24.4, 200.0, 0.5, 40.0]
    dur = list(r.choice([61.0, 60.0], 200)) + [5.0, 7.0, 1.0, 10.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0, 60.0]
    g.add('reelkit.config.whole_period(period, dur)', {}, period=f64(period), dur=f64(dur),
          out=f64(C.whole_period(float(p), float(d)) for p, d in zip(period, dur)))

    bpm = list(r.uniform(40, 220, 150)) + [96.0, 120.0, 132.0, 146.0, 1.0, 94.42622950819673, 60.0, 118.0]
    dur = list(r.choice([61.0, 60.0], 150)) + [61.0, 61.0, 61.0, 60.0, 61.0, 61.0, 61.0, 60.0]
    bpb = list(r.choice([4, 3], 150)) + [4, 4, 4, 4, 4, 4, 3, 4]
    g.add('reelkit.config.loop_bpm(bpm, dur, beats_per_bar)', {}, bpm=f64(bpm), dur=f64(dur), beats_per_bar=i64(bpb),
          out=f64(C.loop_bpm(float(p), float(d), int(k)) for p, d, k in zip(bpm, dur, bpb)))

    y0 = list(r.integers(-20, 660, 150)) + [100, 280, 289, 290, 291, 0, 500, 290, 290]
    y1 = [v + int(d) for v, d in zip(y0[:150], r.integers(0, 80, 150))] + [121, 290, 290, 291, 300, 640, 501, 290, 289]
    g.add('reelkit.config.safe_x1(y0, y1)', {}, y0=i64(y0), y1=i64(y1),
          out=i64(C.safe_x1(int(p), int(q)) for p, q in zip(y0, y1)))

    x0 = list(r.integers(-40, 380, 300)) + [20, 20, 19, 20, 280, 300, 299, 301, 250, 0, 340, 100]
    yy0 = list(r.integers(-40, 660, 300)) + [96, 96, 96, 95, 280, 290, 290, 250, 480, 0, 500, 300]
    x1 = [v + int(d) for v, d in zip(x0[:300], r.integers(0, 120, 300))] + [340, 341, 340, 340, 300, 360, 300, 310, 320,
                                                                            360, 360, 200]
    yy1 = [v + int(d) for v, d in zip(yy0[:300], r.integers(0, 120, 300))] + [500, 500, 500, 500, 290, 640, 291, 300,
                                                                             520, 640, 640, 400]
    g.add('reelkit.config.safe_overshoot(x0, y0, x1, y1)', {}, x0=i64(x0), y0=i64(yy0), x1=i64(x1), y1=i64(yy1),
          out=i64(C.safe_overshoot(int(p), int(q), int(s), int(t)) for p, q, s, t in zip(x0, yy0, x1, yy1)))
    return g


# ================================================================ reelkit.font
# every string a caption, a label or the title card can meet: the whole table, lower case, characters the font does
# not have (drawn as a space), characters whose upper case is not one character or is a letter of the table
FONT_STRINGS = (
    'FLUTE', 'TROMBONE', 'K.O.', 'FIGHT!', 'ROUND 1', '100%', "DON'T BLINK", 'LV', 'LOST', '3 2 1', 'VS',
    'the quick brown fox jumps over the lazy dog', 'Who wins?', 'a', 'I', ' ', '  ', '', 'x2', '+15', '#1',
    'WAIT... WHAT?!', '(50/50)', 'A=B', '1:23', 'R&B', '20\u00b0', '3\u00d74', '8\u00f72', '"OK"', 'A<B>C', 'a-b,c',
    '@_[]{}~^|\\$;`', '\u00e9\u00c9\u00f1', '\u00df', '\u0131\u017f', '\u00b5\u01c6', '\U0001f600!', 'A\tB\nC',
    '\u00b0\u00d7\u00f7', '\ufb01', 'CLARINET', 'TUBA', 'PIANO')
FONT_SCALES = (1, 2, 3, 4)


def group_font():
    from reelkit import font as F
    chars = ''.join(F._G)
    assert len(chars) == len(F.GLYPHS) == 58 and all(F.GLYPHS[c].shape == (7, 5) for c in chars)
    g = Group('font', 'reelkit.font: the 5 x 7 pixel font, text_mask, dilate and stamp', ADV=F.ADV, glyphs=chars,
              note='Masks are bool arrays (height, width). text_mask: args s (the string) and scale. dilate: args r, '
                   'arrays m and out. stamp: arrays "in" (the uint8 RGB frame before), mask, out; args x, y, color '
                   'and the keywords exactly as passed (a keyword that is absent was not passed; outline null is '
                   'outline=None; alpha is a float.hex() string).')
    g.add('np.stack([reelkit.font.GLYPHS[ch] for ch in chars])', dict(chars=chars),
          out=np.stack([F.GLYPHS[c] for c in chars]))

    def mask_case(s, scale):
        m = F.text_mask(s, scale)
        assert m.dtype == np.bool_ and m.ndim == 2
        g.add('reelkit.font.text_mask(s, scale)', dict(s=s, scale=scale), out=m)

    for k in range(0, len(chars), 9):                     # the whole table, nine glyphs at a time
        mask_case(chars[k:k + 9], 1)
        mask_case(chars[k:k + 9].lower(), 2)
    mask_case(chars, 1)
    for n, s in enumerate(FONT_STRINGS):
        mask_case(s, 1)
        mask_case(s, FONT_SCALES[n % 4])
    for scale in (0, 1, 2, 3, 4, 5, 6, 8):
        mask_case('A', scale)
        mask_case('', scale)

    r = rng(2)
    masks = [F.text_mask('AB', 1), F.text_mask('K.O.', 2), F.text_mask('', 1), F.text_mask('!', 3),
             np.ones((3, 4), bool), np.zeros((2, 5), bool), np.ones((1, 1), bool)]
    masks += [r.random((int(r.integers(1, 9)), int(r.integers(1, 14)))) < p for p in (0.1, 0.3, 0.5, 0.05, 0.8, 0.2)]
    for m in masks:
        for rad in (0, 1, 2, 3, 4):                       # 1: stamp and the HUD; 2: the big captions; 3, 4: the title
            out = F.dilate(m, rad)
            assert out.dtype == np.bool_ and out.shape == (m.shape[0] + 2 * rad, m.shape[1] + 2 * rad)
            g.add('reelkit.font.dilate(m, r)', dict(r=rad), m=m, out=out)

    # stamp: a 64 x 40 noise frame; text inside, across every edge and outside; each keyword passed and left out
    fh, fw = 40, 64
    texts = [('HP', 1), ('K.O.', 1), ('A', 2), ('7', 3), ('', 1), ('WIN!', 1), ('x', 4)]
    spots = [(5, 6), (20, 14), (-3, 4), (4, -3), (55, 10), (10, 36), (-2, -2), (60, 37), (-40, 5), (5, -30), (64, 3),
             (3, 40), (63, 39), (0, 0), (-1, 1), (1, -1)]
    outlines = ['default', None, (10, 20, 26), (255, 255, 255), (0, 0, 0), (200, 30, 90)]
    alphas = ['default', 1.0, 0.999, 0.9989999, 0.5, 0.25, 0.0, 0.75, 0.1, 0.9]
    n = 0
    for (x, y) in spots:
        for rep in range(5):
            s, scale = texts[n % len(texts)]
            mask = F.text_mask(s, scale)
            frame = r.integers(0, 256, (fh, fw, 3), dtype=np.uint8)
            color = tuple(int(v) for v in r.integers(0, 256, 3))
            kw = {}
            o = outlines[(n * 5 + rep) % len(outlines)]
            if o != 'default':
                kw['outline'] = o
            if (n + rep) % 3 == 0:
                kw['shadow'] = bool((n + rep) % 2)
            a = alphas[(n * 3 + rep) % len(alphas)]
            if a != 'default':
                kw['alpha'] = a
            out = frame.copy()
            F.stamp(out, mask, x, y, color, **kw)
            assert out.dtype == np.uint8
            args = dict(x=x, y=y, color=list(color))
            for k, v in kw.items():
                args[k] = common.hexf(v) if k == 'alpha' else (list(v) if isinstance(v, tuple) else v)
            g.add('reelkit.font.stamp(frame, mask, x, y, color' + ''.join(f', {k}={k}' for k in kw) + ')', args,
                  in_=frame, mask=mask, out=out)
            n += 1
    return g


# ================================================================ numpy semantics
F64_SPECIALS = [0.0, -0.0, 0.5, -0.5, 0.999999, 1.0, 1.5, 2.5, -1.0, -1.5, 127.5, 254.999, 255.0, 255.5, 255.999999,
                256.0, 256.7, 257.0, 300.5, 511.99, -0.7, -0.999999, -255.0, -256.0, -257.2, 65535.9, 65536.0,
                65539.9, 1e6, -1e6, 2147483647.0, 2147483647.5, 2147483648.0, 2147483649.0, -2147483648.0,
                -2147483648.5, -2147483649.0, 3e9, 4294967296.0, 4294967301.0, 1e10, -1e10, 1e19, -1e19,
                9223372036854775807.0, -9223372036854775808.0, 1e300, -1e300, 5e-324, float('nan'), float('inf'),
                float('-inf')]


def cast_inputs(r):
    """values to cast: the specials twice (at different positions, so both the vector part and the tail of numpy's
    loops meet them) around ordinary values"""
    return np.concatenate([F64_SPECIALS, r.uniform(0, 256, 150), r.uniform(-600, 900, 77), F64_SPECIALS[::-1],
                           r.uniform(0, 256, 30), r.normal(0, 1e5, 13)])


def group_np():
    from reelkit import config as C
    g = Group('np', 'numpy semantics the renderer leans on, element by element',
              numpy=np.__version__,
              note='Every case is one numpy call on the arrays named in "call"; scalars are in args (floats as '
                   'float.hex() strings). Casts of NaN, the infinities and values outside the target type are what '
                   'numpy gives on the reference PC (it leaves them to the processor); the kit only relies on '
                   'casts of values that fit. np.clip(x, lo, None) is np.maximum(x, lo): for -0.0 against 0.0 its '
                   'result depends on where the element sits in the array, so those cases hold no zero of either '
                   'sign next to a zero bound.')
    r = rng(3)
    with np.errstate(invalid='ignore', over='ignore'):
        x = cast_inputs(r)
        for lo, hi in ((0, 255), (0, 1), (-1, 1), (0.0, 1e-3), (-0.0, 255.0), (0.25, 0.75)):
            out = np.clip(x, lo, hi)
            assert out.dtype == np.float64
            g.add('np.clip(x, lo, hi)', dict(lo=common.hexf(float(lo)), hi=common.hexf(float(hi))), x=x, out=out)
        x32 = x.astype(np.float32)
        for lo, hi in ((0, 255), (0, 1), (-1.5, 0.1)):
            out = np.clip(x32, lo, hi)
            assert out.dtype == np.float32
            g.add('np.clip(x32, lo, hi)', dict(lo=common.hexf(float(lo)), hi=common.hexf(float(hi))), x32=x32, out=out)
        nz = x[x != 0]                                    # no zero of either sign: see the note
        for lo in (0, 1e-3, -1, 255):
            g.add('np.clip(x, lo, None)', dict(lo=common.hexf(float(lo))), x=nz, out=np.clip(nz, lo, None))
        for hi in (0, 1, 255.0):
            g.add('np.clip(x, None, hi)', dict(hi=common.hexf(float(hi))), x=nz, out=np.clip(nz, None, hi))
        a, b = r.normal(0, 50, 257), r.normal(0, 50, 257)
        a[[3, 40, 200, 256]] = np.nan
        b[[5, 40, 201]] = np.nan
        b[[7, 8, 255]] = a[[7, 8, 255]]
        g.add('np.maximum(a, b)', {}, a=a, b=b, out=np.maximum(a, b))
        g.add('np.minimum(a, b)', {}, a=a, b=b, out=np.minimum(a, b))

        x = cast_inputs(r)
        x32 = x.astype(np.float32)
        g.add('x.astype(np.uint8)', {}, x=x, out=x.astype(np.uint8))
        g.add('x32.astype(np.uint8)', {}, x32=x32, out=x32.astype(np.uint8))
        g.add('x.astype(np.int32)', {}, x=x, out=x.astype(np.int32))
        g.add('x.astype(np.int64)', {}, x=x, out=x.astype(np.int64))
        g.add('np.clip(x, 0, 255).astype(np.uint8)', {}, x=x, out=np.clip(x, 0, 255).astype(np.uint8))
        g.add('np.clip(x32, 0, 255).astype(np.uint8)', {}, x32=x32, out=np.clip(x32, 0, 255).astype(np.uint8))

    # the two dithers, with the Bayer table as the renderer indexes it and the kit's own constants
    bayer = np.ascontiguousarray(C.BAYER_FULL[:16, :24])
    v = np.concatenate([r.uniform(0, 1.2, bayer.size - 40), np.repeat([0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0, 0.999], 5)]
                       ).reshape(bayer.shape)
    for levels, gain in ((5, 0.999), (6, 0.999), (4, 0.9), (10, 0.9), (3, 0.9)):
        g.add('np.floor(v * levels + bayer * gain) / levels', dict(levels=levels, gain=common.hexf(gain)), v=v,
              bayer=bayer, out=np.floor(v * levels + bayer * gain) / levels)
    x = np.concatenate([r.uniform(0, 255, bayer.size - 40), np.repeat([0.0, 5.0, 6.0, 7.0, 35.0, 42.0, 254.9, 255.0],
                                                                     5)]).reshape(bayer.shape)
    for step in (6, 5, 7):
        g.add('np.floor(x / step + bayer) * step', dict(step=step), x=x, bayer=bayer,
              out=np.floor(x / step + bayer) * step)
        g.add('np.floor(x / step + bayer * gain) * step', dict(step=step, gain=common.hexf(0.999)), x=x, bayer=bayer,
              out=np.floor(x / step + bayer * 0.999) * step)

    m2 = r.integers(0, 256, (5, 7), dtype=np.uint8)
    m3 = r.integers(0, 256, (4, 6, 3), dtype=np.uint8)
    m1 = r.integers(0, 256, 9, dtype=np.uint8)
    for shift in (0, 1, -1, 3, -4, 7, 12, -13):
        for axis in (0, 1):
            g.add('np.roll(a, shift, axis)', dict(shift=shift, axis=axis), a=m2, out=np.roll(m2, shift, axis))
            g.add('np.roll(a, shift, axis)', dict(shift=shift, axis=axis), a=m3, out=np.roll(m3, shift, axis))
        g.add('np.roll(a, shift)', dict(shift=shift), a=m1, out=np.roll(m1, shift))

    for p, shape in ((0.3, (6, 9)), (0.05, (12, 10)), (0.9, (3, 4)), (0.0, (4, 4)), (1.0, (2, 3)), (0.5, (1, 7)),
                     (0.5, (7, 1))):
        m = r.random(shape) < p
        ys, xs = np.nonzero(m)
        assert ys.dtype == np.int64 and xs.dtype == np.int64
        g.add('np.nonzero(m)', {}, m=m, ys=ys, xs=xs)
    for p, n in ((0.4, 17), (0.0, 5), (1.0, 4)):
        m = r.random(n) < p
        g.add('np.nonzero(m)[0]', {}, m=m, out=np.nonzero(m)[0])
    return g


def run():
    from . import kit_renderer_vectors                    # (it imports Group from this module)
    total = sum(make().write() for make in (group_config, group_font, group_np, *kit_renderer_vectors.GROUPS))
    print(f'kit: {total / 1e3:.1f} kB -> {common.rel(OUT)}')


EXPORTS = {'kit': run}

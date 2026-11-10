"""pillow: what Pillow 12.3.0 draws, for every primitive the renderer calls.

    studio/reference/pillow/calls.json     every ImageDraw / Image call the kit makes while it draws all 1,830 frames
                                           of the example episode, counted by call site with its argument shapes
    studio/reference/pillow/<name>.npz     seeded cases per primitive (+ <name>.json, the sidecar that lists them):
        polygon, polygon_outline, line, ellipse, rectangle, point          plain drawing on L, RGB, F and RGBA images
        blend_polygon, blend_line, blend_ellipse, blend_rectangle, blend_point
                                           ImageDraw.Draw(rgb_image, 'RGBA'): the ink is blended by its alpha
        gaussian_blur                      Image.filter(ImageFilter.GaussianBlur(radius))
        resize_nearest, resize_bilinear, resize_bicubic                    Image.resize(size, filter)
        as_used                            whole calls captured from the renderer at full size (shadow masks, the
                                           bloom, the spot beam, ink starbursts, the title card's crack)

A case stores the arguments and the resulting array (and the image drawn on, when it is not one flat colour). The
survey decides what must be covered: the export stops if the renderer makes a draw call whose image mode, draw mode,
ink kinds and width no case reproduces, or an Image call no file covers. Each file is read back without pickle and
compared with what was written before it is listed in the manifest."""
import inspect
import math
import multiprocessing as mp
import os
import sys

import numpy as np

from . import common
from . import frames as F

OUT = common.ref('pillow')
SEED = 20261110
PROCS = 8                                  # render workers of the survey
CHUNK = 15                                 # frames per task
MAX_SIDE = 96                              # the seeded cases draw on canvases up to 96 x 96
MIN_CASES = 200
WIDTHS = ('default', 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)          # line widths the cases draw ('default': not passed)
AS_USED_FRAMES = (16, 41, 54, 135, 136, 179, 920, 1287, 1310, 1501, 1502, 1642)
AS_USED_WHOLE = (54, 920, 1310, 1642)      # the frames whose shadow mask, bloom and beam are kept


# ================================================================ the survey: what the kit really calls
def ink_kind(v):
    """how a fill or outline was given: none, int, float, rgb, rgba"""
    if v is None:
        return 'none'
    if isinstance(v, (tuple, list)):
        return {3: 'rgb', 4: 'rgba'}.get(len(v), f'tuple{len(v)}')
    if isinstance(v, (bool, int, np.integer)):
        return 'int'
    if isinstance(v, (float, np.floating)):
        return 'float'
    return type(v).__name__


def flatten(xy):
    """(form, numbers) of a coordinate argument: 'pairs' for a sequence of (x, y), 'flat' for x0, y0, x1, y1, ..."""
    if len(xy) and isinstance(xy[0], (list, tuple, np.ndarray)):
        return 'pairs', [v for p in xy for v in p]
    return 'flat', list(xy)


class Trace:
    """wraps Pillow's drawing and image calls and tells its hooks about every call the KIT makes (a caller under
    projects/, reelkit/ or series/); calls Pillow makes of its own accord are not reported. hook(kind, name, obj,
    args, out, site, via): kind 'draw' (obj the ImageDraw), 'image' (obj the Image) or 'module' (obj None)"""
    MODULE_CALLS = (('Image', ('new', 'fromarray', 'open', 'merge', 'blend', 'composite', 'alpha_composite',
                               'frombytes', 'frombuffer', 'eval')), ('ImageDraw', ('Draw', 'floodfill')))

    def __init__(self):
        self.hooks, self._undo, self._sites = [], [], {}

    def _site(self, fr):
        code = fr.f_code
        got = self._sites.get(code)
        if got is None:
            try:
                rel = common.rel(code.co_filename)
            except ValueError:
                rel = ''
            got = self._sites[code] = (f'{rel}:{code.co_qualname}', rel.split('/')[0] in common.KIT_DIRS,
                                       rel == 'reelkit/raster.py')
        if not got[1]:
            return None, None
        via = None
        if got[2] and fr.f_back is not None:                 # a Frame method: name who asked the frame to draw
            via = self._site(fr.f_back)[0]
        return got[0], via

    def _wrap(self, owner, name, kind):
        orig = vars(owner)[name]
        params = list(inspect.signature(orig).parameters)
        if kind != 'module':
            params = params[1:]
        trace = self

        def call(*a, **kw):
            site, via = trace._site(sys._getframe(1))
            out = orig(*a, **kw)
            if site is not None and trace.hooks:
                obj, rest = (None, a) if kind == 'module' else (a[0], a[1:])
                args = dict(zip(params, rest))
                args.update(kw)
                for h in list(trace.hooks):
                    h(kind, name, obj, args, out, site, via)
            return out
        call.__name__ = name
        setattr(owner, name, call)
        self._undo.append((owner, name, orig))

    def install(self):
        from PIL import Image, ImageDraw
        for name, fn in list(vars(ImageDraw.ImageDraw).items()):
            if not name.startswith('_') and inspect.isfunction(fn):
                self._wrap(ImageDraw.ImageDraw, name, 'draw')
        for name, fn in list(vars(Image.Image).items()):
            if not name.startswith('_') and inspect.isfunction(fn):
                self._wrap(Image.Image, name, 'image')
        mods = dict(Image=Image, ImageDraw=ImageDraw)
        for mod, names in self.MODULE_CALLS:
            for name in names:
                self._wrap(mods[mod], name, 'module')
        return self

    def remove(self):
        for owner, name, orig in reversed(self._undo):
            setattr(owner, name, orig)
        self._undo.clear()


def stats_hook(rows):
    """a Trace hook that counts calls into `rows`: {key: stats}, mergeable across processes"""
    from PIL import Image

    def new(**kw):
        return dict(calls=0, points=None, coords=None, fractional=False, outside=False, forms=set(), types=set(),
                    combos=set(), alpha=None, sizes=set(), details=set(), **kw)

    def span(old, lo, hi):
        return (lo, hi) if old is None else (min(old[0], lo), max(old[1], hi))

    def hook(kind, name, obj, args, out, site, via):
        if kind == 'draw':
            im = obj._image
            key = ('draw', name, im.mode, obj.mode, site, via or '')
            r = rows.get(key) or rows.setdefault(key, new())
            r['calls'] += 1
            r['sizes'].add(im.size)
            fill, outline = args.get('fill'), args.get('outline')
            r['combos'].add((ink_kind(fill), ink_kind(outline), args.get('width', 'default')))
            for v in (fill, outline):
                if ink_kind(v) == 'rgba':
                    r['alpha'] = span(r['alpha'], int(v[3]), int(v[3]))
            if 'xy' in args:
                form, flat = flatten(args['xy'])
                r['forms'].add(form)
                r['types'] |= {type(v).__name__ for v in flat}
                v = np.array(flat, float)
                r['points'] = span(r['points'], len(flat) // 2, len(flat) // 2)
                if len(v):
                    r['coords'] = span(r['coords'], float(v.min()), float(v.max()))
                    r['fractional'] |= bool((v != np.trunc(v)).any())
                    x, y = v[0::2], v[1::2]
                    r['outside'] |= bool(((x < 0) | (x >= im.size[0]) | (y < 0) | (y >= im.size[1])).any())
        elif kind == 'image':
            key = ('image', name, obj.mode, '', site, via or '')
            r = rows.get(key) or rows.setdefault(key, new())
            r['calls'] += 1
            r['sizes'].add(obj.size)
            if name == 'resize':
                how = args.get('resample')
                r['details'].add(f"to {tuple(args['size'])}, "
                                 f"{'default filter' if how is None else Image.Resampling(how).name}")
            elif name == 'filter':
                flt = args['filter']
                r['details'].add(f"{type(flt).__name__}({getattr(flt, 'radius', '')})")
            elif name in ('convert', 'save'):
                r['details'].add(str(args.get('mode', args.get('format', ''))))
        else:
            if name == 'new':
                mode, detail = args['mode'], f"size {tuple(args['size'])}, color {ink_kind(args.get('color', 0))}"
            elif name == 'fromarray':
                a = args['obj']
                mode = out.mode
                detail = f"{a.dtype} array {a.shape}" + (f", mode={args['mode']!r}" if args.get('mode') else '')
            elif name == 'Draw':
                mode, detail = args['im'].mode, f"mode {args.get('mode') or 'of the image'}"
            else:
                mode, detail = '', ''
            key = ('module', name, mode, '', site, via or '')
            r = rows.get(key) or rows.setdefault(key, new())
            r['calls'] += 1
            r['details'].add(detail)
    return hook


def merge(into, rows):
    for key, r in rows.items():
        o = into.get(key)
        if o is None:
            into[key] = r
            continue
        o['calls'] += r['calls']
        for k in ('points', 'coords', 'alpha'):
            if r[k] is not None:
                o[k] = r[k] if o[k] is None else (min(o[k][0], r[k][0]), max(o[k][1], r[k][1]))
        for k in ('fractional', 'outside'):
            o[k] = o[k] or r[k]
        for k in ('forms', 'types', 'combos', 'sizes', 'details'):
            o[k] |= r[k]
    return into


_WORKER = {}


def warm(P):
    """build what the renderer builds once, on first use: both balls' sprites and the title card's close-ups. The
    Pixel Pitch bridge imports the series' modules for the model, and some of those paint posters for their own
    videos as they load: Pillow calls that are no part of any BALLFIGHT frame, so the survey starts after them"""
    for k in (0, 1):
        yc, sw, roll = P.CFG[k]['view']
        P.sprite(k, 16, yc, roll)
        P.TT.big_sprite(k)


def _survey_init(tag):
    """pool worker start: load the episode in this process (spawn starts from nothing) and trace Pillow"""
    _WORKER['P'] = F.picture(tag)
    warm(_WORKER['P'])
    _WORKER['trace'] = Trace().install()


def _survey_chunk(frames):
    rows = {}
    _WORKER['trace'].hooks[:] = [stats_hook(rows)]
    for f in frames:
        _WORKER['P'].render(f)
    return rows


def survey(P, frames):
    """every Pillow call the kit makes drawing `frames`, counted: {key: stats}. Counts, spans and sets only, so the
    result does not depend on how many workers drew the frames or in what order"""
    warm(P)                                               # the caches the workers read are written before they start
    tag = os.path.basename(os.path.dirname(P.EP.dir))
    frames = list(frames)
    chunks = [frames[i:i + CHUNK] for i in range(0, len(frames), CHUNK)]
    out = {}
    with mp.get_context('spawn').Pool(min(PROCS, len(chunks)), initializer=_survey_init, initargs=(tag,)) as pool:
        for rows in pool.map(_survey_chunk, chunks, chunksize=1):
            merge(out, rows)
    return out


def survey_doc(rows, frames, cover):
    """the survey as JSON: one row per (call, image mode, draw mode, call site), sorted; cover[key] names the
    vector files whose cases reproduce that row's calls"""
    def lo_hi(v, nd=None):
        return None if v is None else [round(x, nd) if nd is not None else x for x in v]

    def width_key(w):
        return (0, 0) if w == 'default' else (1, w)

    draw, image = [], []
    for key in sorted(rows):
        kind, name, mode, draw_mode, site, via = key
        r = rows[key]
        if kind == 'draw':
            draw.append(dict(
                call=f'ImageDraw.{name}', image_mode=mode, draw_mode=draw_mode, site=site, via=via, calls=r['calls'],
                image_sizes=sorted(list(s) for s in r['sizes']), xy_form=sorted(r['forms']),
                xy_types=sorted(r['types']), points=lo_hi(r['points']), coords=lo_hi(r['coords'], 3),
                fractional=r['fractional'], outside_the_image=r['outside'],
                ink=[dict(fill=c[0], outline=c[1], width=c[2])
                     for c in sorted(r['combos'], key=lambda c: (c[0], c[1], width_key(c[2])))],
                alpha=lo_hi(r['alpha']), cases=cover[key]))
        else:
            label = ('Image.Image.' if kind == 'image' else 'ImageDraw.' if name == 'Draw' else 'Image.') + name
            image.append(dict(call=label, image_mode=mode, site=site, via=via, calls=r['calls'],
                              image_sizes=sorted(list(s) for s in r['sizes']), details=sorted(r['details']),
                              cases=cover[key]))
    return dict(
        about='every Pillow call the kit makes while it draws the example episode, counted by call site',
        tool='pillow', episode=F.EPISODE, frames=len(frames), pillow=_pillow_version(),
        note='site is the kit function that calls Pillow; via is who called it when the site is a method of '
             'reelkit.raster.Frame. xy_types are the Python types of the coordinates (float64 is numpy\'s, a subclass '
             'of float). points and coords are the smallest and largest count and value seen. width "default" means '
             'the call passed none (ImageDraw.line then draws width 1). Pillow turns every coordinate into a C int '
             'by truncation toward zero before it draws. cases names the vector files in this folder whose cases '
             'reproduce the row (as_used has whole calls at full size besides); an empty list is a call that makes '
             'or wraps an image and draws nothing. np.asarray(image) is not a Pillow call and is not listed. The '
             'survey starts after the sprites and title close-ups are built: the Pixel Pitch modules the bridge '
             'imports for the models paint posters of their own as they load, which no frame shows.',
        draw=draw, image=image)


def _pillow_version():
    import PIL
    return PIL.__version__


# ================================================================ the cases
def enc(v):
    """an argument as JSON: a float as its hex string (every bit), an int as a number, sequences as lists"""
    if v is None or isinstance(v, str):
        return v
    if isinstance(v, (bool, np.bool_)):
        return bool(v)
    if isinstance(v, (int, np.integer)):
        return int(v)
    if isinstance(v, (float, np.floating)):
        return common.hexf(float(v))
    if isinstance(v, (list, tuple)):
        return [enc(x) for x in v]
    raise TypeError(type(v).__name__)


class Book:
    """one vector file being filled: the cases (sidecar rows) and their arrays (npz members)"""

    def __init__(self, name, about, **top):
        self.name, self.about, self.top = name, about, top
        self.cases, self.arrays, self.sigs = [], {}, set()

    def add(self, call, args, **arrays):
        cid = f'c{len(self.cases):03d}'
        names = {}
        for role, a in arrays.items():
            assert isinstance(a, np.ndarray) and a.dtype.str in common.NPZ_DTYPES, (self.name, cid, role)
            self.arrays[f'{cid}__{role}'] = np.ascontiguousarray(a)
            names[role] = f'{cid}__{role}'
        self.cases.append(dict(id=cid, call=call, args=args, arrays=names))

    def kinds(self):
        return {c['args'].get('kind') for c in self.cases}

    def write(self):
        """write <name>.npz and <name>.json, read the archive back and compare, then list both"""
        from . import manifest
        npz, js = os.path.join(OUT, self.name + '.npz'), os.path.join(OUT, self.name + '.json')
        common.write_npz(npz, self.arrays, register=False)
        doc = dict(about=self.about, tool='pillow', pillow=_pillow_version(), **self.top, cases=self.cases)
        common.write_json(js, doc, register=False)
        z = np.load(npz, allow_pickle=False)
        named = {m for c in self.cases for m in c['arrays'].values()}
        assert set(z.files) == set(self.arrays) == named, f'{self.name}: members and sidecar disagree'
        for k, a in self.arrays.items():
            b = z[k]
            assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), (self.name, k)
        manifest.add(npz, tool=__name__)
        manifest.add(js, tool=__name__)
        return os.path.getsize(npz) + os.path.getsize(js)


DRAW_NOTE = ('args: mode and size (width, height) of the image, background (its one colour before the call) or the '
             'array "in", xy as passed (xy_form "pairs": a list of (x, y) tuples; "flat": one list x0, y0, x1, y1, '
             '...), then the keyword arguments of the call exactly as passed (a keyword that is absent was not '
             'passed). Floats are float.hex() strings. Arrays are (height, width) for L and F, (height, width, 3) '
             'for RGB, (height, width, 4) for RGBA; F is float32. Pillow truncates every coordinate toward zero to '
             'a C int before drawing, and an F ink to float32.')


def draw_case(book, kind, mode, size, method, xy, bg, blend=False, **kw):
    """run one ImageDraw call on a fresh image and file it. bg: the image's one colour, or an (h, w, 3) array"""
    from PIL import Image, ImageDraw
    w, h = size
    assert max(w, h) <= MAX_SIDE
    has_in = isinstance(bg, np.ndarray)
    im = Image.fromarray(bg) if has_in else Image.new(mode, (w, h), bg)
    assert im.mode == mode and im.size == (w, h)
    d = ImageDraw.Draw(im, 'RGBA') if blend else ImageDraw.Draw(im)
    getattr(d, method)(xy, **kw)
    form = flatten(xy)[0]
    args = dict(kind=kind, mode=mode, size=[w, h], xy=enc(xy), xy_form=form)
    if blend:
        args['draw_mode'] = 'RGBA'
    if not has_in:
        args['background'] = enc(bg)
    for k, v in kw.items():
        args[k] = enc(v)
    call = f"ImageDraw.Draw(im{', ' + repr('RGBA') if blend else ''}).{method}(xy" + \
           ''.join(f', {k}={k}' for k in kw) + ')'
    arrays = dict(out=np.array(im))
    if has_in:
        arrays['in'] = bg
    book.add(call, args, **arrays)
    book.sigs.add((method, mode, 'RGBA' if blend else mode, ink_kind(kw.get('fill')), ink_kind(kw.get('outline')),
                   kw.get('width', 'default')))


class Rnd:
    """the seeded source of one file's cases: plain Python numbers out"""

    def __init__(self, *seed):
        self.g = np.random.default_rng([SEED, *seed])

    def u(self, a, b):
        return float(self.g.uniform(a, b))

    def i(self, a, b):
        return int(self.g.integers(a, b + 1))

    def pick(self, seq):
        return seq[int(self.g.integers(len(seq)))]

    def size(self, lo=8):
        return self.pick([(MAX_SIDE, MAX_SIDE), (64, 48), (48, 64), (32, 32), (24, 40), (self.i(lo, MAX_SIDE),
                         self.i(lo, MAX_SIDE)), (self.i(lo, MAX_SIDE), self.i(lo, MAX_SIDE)), (self.i(lo, 40),
                         self.i(lo, 40))])

    def rgb(self):
        return (self.i(0, 255), self.i(0, 255), self.i(0, 255))

    def ink(self, mode, rep=0, alpha=None):
        """a fill in the form the kit gives it for this image mode"""
        if mode == 'L':
            return 255 if rep % 2 == 0 else self.i(1, 254)
        if mode == 'F':
            return self.u(5.0, 40.0) if rep % 3 else self.u(-3.0, 1e5)
        if mode == 'RGB':
            return self.rgb()
        return self.rgb() + (self.i(0, 255) if alpha is None else alpha,)

    def paper(self, mode):
        """the colour an image of this mode starts with in the kit"""
        return {'L': 0, 'F': 1e6, 'RGB': self.rgb(), 'RGBA': (0, 0, 0, 0)}[mode]

    def stripes(self, w, h, rep):
        """an RGB image of random columns (even rep) or rows: every blend meets many different pixels, and the
        archive stays small"""
        if rep % 2 == 0:
            return np.ascontiguousarray(np.broadcast_to(self.g.integers(0, 256, (1, w, 3), dtype=np.uint8), (h, w, 3)))
        return np.ascontiguousarray(np.broadcast_to(self.g.integers(0, 256, (h, 1, 3), dtype=np.uint8), (h, w, 3)))


ALPHAS = (255, None, 128, None, 254, None, 127, None, 1, None, None, 0, None, None, None, None)   # None: random


def poly_xy(kind, r, w, h):
    """the vertices of one polygon case"""
    m = min(w, h)
    cx, cy = r.u(0.2 * w, 0.8 * w), r.u(0.2 * h, 0.8 * h)

    def around(n, lo, hi, jitter=True):
        a0 = r.u(0, 2 * math.pi)
        ang = sorted(a0 + r.u(0, 2 * math.pi) for _ in range(n)) if jitter else \
            [a0 + 2 * math.pi * k / n for k in range(n)]
        return [(cx + r.u(lo, hi) * m * math.cos(a), cy + r.u(lo, hi) * m * math.sin(a)) for a in ang]

    def anywhere(n, k=0.15):
        return [(r.u(-k * w, (1 + k) * w), r.u(-k * h, (1 + k) * h)) for _ in range(n)]

    if kind == 'quad':
        return anywhere(4)
    if kind == 'triangle':
        return anywhere(3)
    if kind == 'ngon':
        return around(r.i(5, 12), 0.1, 0.6)
    if kind == 'circle':                                  # a projected circle, as the shadows and hoods are
        n, rx, ry, rot = r.pick([12, 13, 16, 40]), r.u(0.05, 0.5) * w, r.u(0.03, 0.4) * h, r.u(0, 2 * math.pi)
        return [(cx + rx * math.cos(rot + 2 * math.pi * k / n), cy + ry * math.sin(rot + 2 * math.pi * k / n))
                for k in range(n)]
    if kind == 'star':                                    # reelkit.fx.starburst's shape
        n, rad, rot = 18, r.u(0.15, 0.48) * m, r.u(0, 2 * math.pi)
        return [(w / 2 + (rad * r.u(0.75, 1.0) if k % 2 == 0 else rad * r.u(0.32, 0.45)) *
                 math.cos(rot + k * 2 * math.pi / n + r.u(-0.12, 0.12)),
                 h / 2 + (rad * r.u(0.75, 1.0) if k % 2 == 0 else rad * r.u(0.32, 0.45)) *
                 math.sin(rot + k * 2 * math.pi / n + r.u(-0.12, 0.12))) for k in range(n)]
    if kind == 'int':
        return [(r.i(-5, w + 5), r.i(-5, h + 5)) for _ in range(r.i(3, 8))]
    if kind == 'outside_partly':
        cx, cy = r.pick([(0, cy), (w, cy), (cx, 0), (cx, h), (0, 0), (w, h)])
        return around(r.i(3, 9), 0.2, 0.7)
    if kind == 'outside_far':
        return [(r.u(-5 * w, 6 * w), r.u(-5 * h, 6 * h)) for _ in range(r.i(3, 6))]
    if kind == 'outside_all':
        dx, dy = r.pick([(-2 * w, 0), (2 * w, 0), (0, -2 * h), (0, 2 * h), (2 * w, 2 * h), (-w - 3, -h - 3)])
        return [(x + dx, y + dy) for x, y in anywhere(r.i(3, 6), 0.0)]
    if kind == 'cover_all':
        return [(-r.u(1, 50), -r.u(1, 50)), (w + r.u(1, 50), -r.u(1, 50)), (w + r.u(1, 50), h + r.u(1, 50)),
                (-r.u(1, 50), h + r.u(1, 50))]
    if kind == 'two_points':
        return anywhere(2)
    if kind == 'collinear':
        (x0, y0), (x1, y1) = anywhere(2)
        return [(x0 + (x1 - x0) * u, y0 + (y1 - y0) * u) for u in sorted(r.u(-0.2, 1.2) for _ in range(r.i(3, 5)))]
    if kind == 'repeated':
        pts = around(r.i(3, 6), 0.15, 0.5)
        return [p for p in pts for _ in range(r.i(1, 3))]
    if kind == 'same_point':
        return [(cx, cy)] * r.i(2, 5)
    if kind == 'closed':                                  # the first vertex given again at the end
        pts = around(r.i(3, 8), 0.15, 0.5)
        return pts + pts[:1]
    if kind == 'bowtie':
        a, b, c, d = around(4, 0.2, 0.6)
        return [a, c, b, d]
    if kind == 'pentagram':
        n = r.pick([5, 7, 9])
        pts = around(n, 0.45, 0.5, jitter=False)
        return [pts[(k * (n // 2)) % n] for k in range(n)]
    if kind == 'scrambled':
        return anywhere(r.i(5, 9), 0.05)
    if kind == 'axis_rect':
        x0, x1, y0, y1 = r.u(-4, w * 0.6), r.u(w * 0.4, w + 4), r.u(-4, h * 0.6), r.u(h * 0.4, h + 4)
        return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    if kind == 'staircase':                               # rectilinear, whole numbers
        n = r.i(2, 5)
        xs = sorted(r.i(0, w - 1) for _ in range(n + 1))
        ys = sorted(r.i(0, h - 1) for _ in range(n + 1))
        pts = [(xs[0], ys[0])]
        for k in range(n):
            pts += [(xs[k + 1], ys[k]), (xs[k + 1], ys[k + 1])]
        return pts + [(xs[0], ys[n])]
    if kind == 'horizontal_runs':                         # consecutive edges on one row, in both directions
        y0, y1 = sorted((r.i(1, h - 2), r.i(1, h - 2)))
        xs = sorted(r.u(0, w) for _ in range(r.i(3, 5)))
        f = r.u(0.0, 0.9)
        top = [(x, y0 + (f if k % 2 else 0.0)) for k, x in enumerate(xs)]
        return top + [(x, y1 + 1 + f) for x in reversed(xs)]
    if kind == 'flat_top':                                # edges that differ only below the decimal point
        y = r.i(1, h - 2)
        return [(r.u(0, w / 2), y + r.u(0.0, 0.99)), (r.u(w / 2, w), y + r.u(0.0, 0.99)),
                (r.u(0, w), r.u(0, h))] + ([(r.u(0, w), r.u(0, h))] if r.i(0, 1) else [])
    if kind == 'sliver_subpixel':                         # every vertex inside one pixel
        x, y = r.i(0, w - 1), r.i(0, h - 1)
        return [(x + r.u(0.0, 0.999), y + r.u(0.0, 0.999)) for _ in range(r.i(3, 5))]
    if kind == 'sliver_thin':                             # a long quad under a pixel wide (a rope's shadow)
        (x0, y0), (x1, y1) = anywhere(2, 0.0)
        ln = math.hypot(x1 - x0, y1 - y0) + 1e-9
        nx, ny, t = -(y1 - y0) / ln, (x1 - x0) / ln, r.u(0.05, 0.9)
        return [(x0, y0), (x1, y1), (x1 + nx * t, y1 + ny * t), (x0 + nx * t, y0 + ny * t)]
    if kind == 'negative_fraction':                       # -0.x truncates to 0, not to -1
        return [(-r.u(0.01, 0.99), r.u(0, h)), (r.u(0, w), -r.u(0.01, 0.99)), (r.u(0, w), r.u(0, h)),
                (-r.u(0.01, 0.99), -r.u(0.01, 0.99))][:r.i(3, 4)]
    if kind == 'half_pixel':
        frac = [0.5, 0.999999, 1e-9, 0.4999999999, 0.0]
        return [(r.i(0, w - 1) + r.pick(frac), r.i(0, h - 1) + r.pick(frac)) for _ in range(r.i(3, 6))]
    raise KeyError(kind)


POLY_KINDS = ('quad', 'triangle', 'ngon', 'circle', 'star', 'int', 'outside_partly', 'outside_far', 'outside_all',
              'cover_all', 'two_points', 'collinear', 'repeated', 'same_point', 'closed', 'bowtie', 'pentagram',
              'scrambled', 'axis_rect', 'staircase', 'horizontal_runs', 'flat_top', 'sliver_subpixel', 'sliver_thin',
              'negative_fraction', 'half_pixel')
POLY_NEEDS = {'float vertices': ('quad', 'triangle', 'ngon', 'circle'), 'integer vertices': ('int', 'staircase'),
              'vertices outside the canvas': ('outside_partly', 'outside_far', 'outside_all', 'cover_all'),
              'degenerate': ('two_points', 'collinear', 'repeated', 'same_point'),
              'self-intersecting': ('bowtie', 'pentagram', 'scrambled'),
              'horizontal and vertical edges': ('axis_rect', 'staircase', 'horizontal_runs', 'flat_top'),
              'sub-pixel slivers': ('sliver_subpixel', 'sliver_thin')}


def book_polygon():
    b = Book('polygon', 'ImageDraw.polygon(xy, fill=...): filled polygons on L, RGB, F and RGBA images',
             note=DRAW_NOTE)
    r = Rnd(1)
    modes = ('L', 'RGB', 'F', 'RGBA')
    for rep in range(8):
        for j, kind in enumerate(POLY_KINDS):
            mode = modes[(rep + j) % 4]
            w, h = r.size()
            draw_case(b, kind, mode, (w, h), 'polygon', poly_xy(kind, r, w, h), r.paper(mode), fill=r.ink(mode, rep))
    return b


def book_polygon_outline():
    b = Book('polygon_outline', 'ImageDraw.polygon(xy, fill=..., outline=...) and outline alone: the outline is '
             '1 px lines from vertex to vertex, drawn after the fill (reelkit.fx.starburst draws its ink stars so, '
             'on an RGBA image)', note=DRAW_NOTE)
    r = Rnd(2)
    kinds = ('star', 'quad', 'triangle', 'ngon', 'bowtie', 'axis_rect', 'two_points', 'outside_partly', 'int',
             'sliver_thin')
    modes = ('RGBA', 'RGBA', 'RGB', 'L', 'RGBA')
    for rep in range(10):
        for j, kind in enumerate(kinds):
            for hollow in (False, True):
                mode = modes[(rep + j) % 5]
                w, h = r.size()
                a = r.pick([255, 191, r.i(0, 255)])
                kw = dict(outline=r.ink(mode, rep, a)) if hollow else \
                    dict(fill=r.ink(mode, rep, a), outline=r.ink(mode, rep + 1, a))
                draw_case(b, kind, mode, (w, h), 'polygon', poly_xy(kind, r, w, h), r.paper(mode), **kw)
    return b


def line_xy(kind, r, w, h):
    def p(k=0.1):
        return (r.u(-k * w, (1 + k) * w), r.u(-k * h, (1 + k) * h))

    if kind == 'two_float':
        return [p(), p()]
    if kind == 'two_int':
        return [(r.i(-3, w + 3), r.i(-3, h + 3)), (r.i(-3, w + 3), r.i(-3, h + 3))]
    if kind == 'polyline':
        return [p() for _ in range(r.i(3, 7))]
    if kind == 'int_polyline':
        return [(r.i(0, w), r.i(0, h)) for _ in range(r.i(3, 7))]
    if kind == 'rope':                                    # 15 points, nearly straight, sagging (a ring rope)
        (x0, y0), (x1, y1) = p(0.0), p(0.0)
        sag = r.u(0.0, 4.0)
        return [(x0 + (x1 - x0) * k / 14, y0 + (y1 - y0) * k / 14 + sag * math.sin(math.pi * k / 14))
                for k in range(15)]
    if kind == 'ring':                                    # a closed loop: the last point is the first (a bell's lip)
        cx, cy, rx, ry = r.u(0.3 * w, 0.7 * w), r.u(0.3 * h, 0.7 * h), r.u(2, 0.45 * w), r.u(1, 0.3 * h)
        return [(cx + rx * math.cos(2 * math.pi * (k % 18) / 18), cy + ry * math.sin(2 * math.pi * (k % 18) / 18))
                for k in range(19)]
    if kind == 'zigzag':                                  # a jagged crack
        x, y, out = r.u(0, w), r.u(0, h), []
        a = r.u(0, 2 * math.pi)
        for _ in range(r.i(4, 9)):
            out.append((x, y))
            x, y = x + math.cos(a) * r.u(10, 18) + r.u(-4, 4), y + math.sin(a) * r.u(10, 18) + r.u(-4, 4)
        return out
    if kind == 'crossing':                                # a polyline that crosses itself
        a, b, c, d = p(0.0), p(0.0), p(0.0), p(0.0)
        return [a, b, c, d, a, c]
    if kind == 'horizontal':
        y = r.u(0, h)
        return [(r.u(-5, w + 5), y), (r.u(-5, w + 5), y)]
    if kind == 'vertical':
        x = r.u(0, w)
        return [(x, r.u(-5, h + 5)), (x, r.u(-5, h + 5))]
    if kind == 'diagonal':
        x, y, n, sx, sy = r.i(0, w - 1), r.i(0, h - 1), r.i(1, min(w, h)), r.pick([-1, 1]), r.pick([-1, 1])
        return [(x, y), (x + sx * n, y + sy * n)]
    if kind == 'steep':
        x, y = r.u(0, w), r.u(0, h)
        return [(x, y), (x + r.u(-6, 6), y + r.pick([-1, 1]) * r.u(10, h))]
    if kind == 'shallow':
        x, y = r.u(0, w), r.u(0, h)
        return [(x, y), (x + r.pick([-1, 1]) * r.u(10, w), y + r.u(-6, 6))]
    if kind == 'zero_length':
        q = p(0.0)
        return [q, q]
    if kind == 'subpixel':
        x, y = r.i(0, w - 1), r.i(0, h - 1)
        return [(x + r.u(0, 0.99), y + r.u(0, 0.99)), (x + r.u(0, 0.99), y + r.u(0, 0.99))]
    if kind == 'short':                                   # one or two pixels long
        x, y = r.u(1, w - 2), r.u(1, h - 2)
        return [(x, y), (x + r.u(-2.5, 2.5), y + r.u(-2.5, 2.5))]
    if kind == 'outside_partly':
        return [p(0.0), (r.u(-2 * w, 3 * w), r.u(-2 * h, 3 * h))]
    if kind == 'outside_far':
        return [(r.u(-6 * w, -w), r.u(-6 * h, 7 * h)), (r.u(2 * w, 7 * w), r.u(-6 * h, 7 * h))]
    if kind == 'outside_all':
        return [(-r.u(3, 30), r.u(0, h)), (-r.u(3, 30), r.u(0, h))]
    if kind == 'flat_form':                               # x0, y0, x1, y1 in one list
        (x0, y0), (x1, y1) = p(), p()
        return [x0, y0, x1, y1]
    if kind == 'negative_fraction':
        return [(-r.u(0.01, 0.99), r.u(0, h)), (r.u(0, w), -r.u(0.01, 0.99))]
    raise KeyError(kind)


LINE_KINDS = ('two_float', 'two_int', 'polyline', 'int_polyline', 'rope', 'ring', 'zigzag', 'crossing', 'horizontal',
              'vertical', 'diagonal', 'steep', 'shallow', 'zero_length', 'subpixel', 'short', 'outside_partly',
              'outside_far', 'outside_all', 'flat_form', 'negative_fraction')


def book_line():
    b = Book('line', 'ImageDraw.line(xy, fill=..., width=...) on L, RGB and F images: width 1 (or none given) is a '
             'Bresenham line through every vertex; wider lines are one filled quad per segment, no joints',
             note=DRAW_NOTE)
    r = Rnd(3)
    nk = len(LINE_KINDS)
    for mi, mode in enumerate(('RGB', 'F', 'L')):
        for wi, width in enumerate(WIDTHS):
            for q in range(12):
                kind = LINE_KINDS[(wi * 5 + mi * 3 + q * 2 + (q // 6)) % nk]
                w, h = r.size()
                kw = dict(fill=r.ink(mode, q))
                if width != 'default':
                    kw['width'] = width
                draw_case(b, kind, mode, (w, h), 'line', line_xy(kind, r, w, h), r.paper(mode), **kw)
    return b


def box_xy(kind, r, w, h):
    """x0, y0, x1, y1 of one ellipse or rectangle case (x0 <= x1 and y0 <= y1: Pillow refuses anything else)"""
    x, y = r.u(2, w - 2), r.u(2, h - 2)
    if kind == 'round':
        a = r.u(2, 0.45 * min(w, h))
        return [x - a, y - a, x + a, y + a]
    if kind == 'wide':
        a = r.u(3, 0.5 * w)
        return [x - a, y - a * r.u(0.1, 0.5), x + a, y + a * r.u(0.1, 0.5)]
    if kind == 'tall':
        a = r.u(3, 0.5 * h)
        return [x - a * r.u(0.1, 0.5), y - a, x + a * r.u(0.1, 0.5), y + a]
    if kind == 'box':
        x0, x1 = sorted((r.u(0, w), r.u(0, w)))
        y0, y1 = sorted((r.u(0, h), r.u(0, h)))
        return [x0, y0, x1, y1]
    if kind == 'tiny':
        return [x, y, x + r.u(0, 3.5), y + r.u(0, 3.5)]
    if kind == 'zero_w':
        return [x, y, x, y + r.u(1, 20)]
    if kind == 'zero_h':
        return [x, y, x + r.u(1, 20), y]
    if kind == 'point':
        return [x, y, x, y]
    if kind == 'outside_partly':
        x, y = r.pick([(0.0, y), (float(w), y), (x, 0.0), (x, float(h)), (0.0, 0.0)])
        a, c = r.u(3, 0.6 * w), r.u(3, 0.6 * h)
        return [x - a, y - c, x + a, y + c]
    if kind == 'outside_all':
        dx, dy = r.pick([(-3.0 * w, 0.0), (2.0 * w, 0.0), (0.0, -3.0 * h), (0.0, 2.0 * h)])
        return [x + dx, y + dy, x + dx + r.u(1, w), y + dy + r.u(1, h)]
    if kind == 'cover':
        return [-r.u(1, 60), -r.u(1, 60), w + r.u(1, 60), h + r.u(1, 60)]
    if kind == 'int':
        x0, x1 = sorted((r.i(-2, w + 2), r.i(-2, w + 2)))
        y0, y1 = sorted((r.i(-2, h + 2), r.i(-2, h + 2)))
        return [x0, y0, x1, y1]
    if kind == 'negative_fraction':
        return [-r.u(0.01, 0.99), -r.u(0.01, 0.99), r.u(0.5, w), r.u(0.5, h)]
    if kind == 'head':                                    # a crowd head: x - rh .. x + rh
        rh = r.u(0.8, 4.5)
        return [x - rh, y - rh, x + rh, y + rh]
    if kind == 'body':                                    # a crowd body
        rh = r.u(0.8, 4.5)
        return [x - 1.9 * rh, y + 0.6 * rh, x + 1.9 * rh, y + 3.4 * rh]
    if kind == 'bulb':                                    # a flashbulb
        return [x - 1.6, y - 1.6, x + 1.6, y + 1.6]
    if kind == 'phone':                                   # a phone held up
        rh = r.u(0.8, 4.5)
        return [x + rh * 0.3, y - max(1, rh * 0.7), x + rh * 0.3 + max(1, rh * 0.5), y]
    if kind == 'pairs_form':                              # ((x0, y0), (x1, y1))
        x0, x1 = sorted((r.u(0, w), r.u(0, w)))
        y0, y1 = sorted((r.u(0, h), r.u(0, h)))
        return [(x0, y0), (x1, y1)]
    raise KeyError(kind)


ELLIPSE_KINDS = ('round', 'wide', 'tall', 'tiny', 'zero_w', 'zero_h', 'point', 'outside_partly', 'outside_all',
                 'cover', 'int', 'negative_fraction', 'head', 'body', 'bulb', 'pairs_form')
RECT_KINDS = ('box', 'tiny', 'zero_w', 'zero_h', 'point', 'outside_partly', 'outside_all', 'cover', 'int',
              'negative_fraction', 'phone', 'pairs_form')


def book_ellipse():
    b = Book('ellipse', 'ImageDraw.ellipse(xy, fill=...) and ellipse(xy, outline=...) on RGB, L and RGBA images',
             note=DRAW_NOTE)
    r = Rnd(4)
    modes = ('RGB', 'L', 'RGB', 'RGBA')
    for rep in range(13):
        for j, kind in enumerate(ELLIPSE_KINDS):
            mode = modes[(rep + 3 * j) % 4]
            w, h = r.size()
            v = (rep + j) % 5
            kw = dict(fill=r.ink(mode, rep)) if v < 3 else dict(outline=r.ink(mode, rep)) if v == 3 else \
                dict(fill=r.ink(mode, rep), outline=r.ink(mode, rep + 1))
            draw_case(b, kind, mode, (w, h), 'ellipse', box_xy(kind, r, w, h), r.paper(mode), **kw)
    return b


def book_rectangle():
    b = Book('rectangle', 'ImageDraw.rectangle(xy, fill=...) and rectangle(xy, outline=...) on RGB and L images',
             note=DRAW_NOTE)
    r = Rnd(5)
    for rep in range(17):
        for j, kind in enumerate(RECT_KINDS):
            mode = ('RGB', 'L', 'RGB')[(rep + j) % 3]
            w, h = r.size()
            v = (rep + 2 * j) % 6
            kw = dict(fill=r.ink(mode, rep)) if v < 4 else dict(outline=r.ink(mode, rep)) if v == 4 else \
                dict(fill=r.ink(mode, rep), outline=r.ink(mode, rep + 1))
            draw_case(b, kind, mode, (w, h), 'rectangle', box_xy(kind, r, w, h), r.paper(mode), **kw)
    return b


def point_xy(kind, r, w, h):
    if kind == 'pair':                                    # (x, y): one point as a bare pair of floats
        return (r.u(0, w), r.u(0, h))
    if kind == 'list_one_int':                            # [(xi, yi)]
        return [(r.i(0, w - 1), r.i(0, h - 1))]
    if kind == 'many':
        return [(r.u(-2, w + 2), r.u(-2, h + 2)) for _ in range(r.i(2, 12))]
    if kind == 'flat_many':
        return [v for _ in range(r.i(2, 6)) for v in (r.u(0, w), r.u(0, h))]
    if kind == 'outside':
        return r.pick([(-r.u(1, 9), r.u(0, h)), (w + r.u(0, 9), r.u(0, h)), (r.u(0, w), -r.u(1, 9)),
                       (r.u(0, w), h + r.u(0, 9)), (float(w), float(h))])
    if kind == 'negative_fraction':                       # truncates to column 0 / row 0
        return r.pick([(-r.u(0.01, 0.99), r.u(0, h)), (r.u(0, w), -r.u(0.01, 0.99)),
                       (-r.u(0.01, 0.99), -r.u(0.01, 0.99))])
    if kind == 'edge':
        return r.pick([(w - r.u(0.0, 0.999), h - r.u(0.0, 0.999)), (0.0, 0.0), (w - 1, h - 1), (w - 1.0, 0.5)])
    if kind == 'repeated':                                # the same pixel twice (a blend then lands twice)
        q = (r.u(0, w), r.u(0, h))
        return [q, q]
    raise KeyError(kind)


POINT_KINDS = ('pair', 'list_one_int', 'many', 'flat_many', 'outside', 'negative_fraction', 'edge', 'repeated')


def book_point():
    b = Book('point', 'ImageDraw.point(xy, fill=...) on RGB, L and F images', note=DRAW_NOTE)
    r = Rnd(6)
    for rep in range(25):
        for j, kind in enumerate(POINT_KINDS):
            mode = ('RGB', 'L', 'RGB', 'F')[(rep + j) % 4]
            w, h = r.size(1) if rep % 5 == 4 else r.size()
            draw_case(b, kind, mode, (w, h), 'point', point_xy(kind, r, w, h), r.paper(mode), fill=r.ink(mode, rep))
    return b


BLEND_NOTE = DRAW_NOTE + (' Here the image is RGB and the drawing mode is "RGBA": every pixel the shape covers '
                          'becomes the blend of the pixel and the ink by the ink\'s alpha, once per time it is '
                          'drawn. "in" is the image before the call.')


def blend_book(name, about, seed, method, kinds, xy_of, variants):
    """a file of blended draws: kinds x variants (each variant: fn(r, alpha) -> the call's keywords)"""
    b = Book(name, about, note=BLEND_NOTE)
    r = Rnd(seed)
    n = 0
    while len(b.cases) < MIN_CASES:
        for kind in kinds:
            alpha = ALPHAS[n % len(ALPHAS)]
            alpha = r.i(2, 253) if alpha is None else alpha
            w, h = r.size()
            kw = variants[n % len(variants)](r, alpha)
            draw_case(b, kind, 'RGB', (w, h), method, xy_of(kind, r, w, h), r.stripes(w, h, n), blend=True, **kw)
            n += 1
    return b


def book_blend_polygon():
    kinds = ('triangle', 'quad', 'ngon', 'circle', 'star', 'bowtie', 'pentagram', 'scrambled', 'axis_rect',
             'horizontal_runs', 'flat_top', 'sliver_thin', 'two_points', 'outside_partly', 'int', 'closed',
             'collinear', 'staircase', 'half_pixel')
    return blend_book('blend_polygon', "ImageDraw.Draw(rgb, 'RGBA').polygon(xy, fill=rgba): a blended fill never "
                      'blends a pixel twice within one polygon, however its edges cross', 7, 'polygon', kinds,
                      poly_xy, [lambda r, a: dict(fill=r.rgb() + (a,))])


def book_blend_line():
    kinds = ('two_float', 'polyline', 'crossing', 'horizontal', 'vertical', 'diagonal', 'steep', 'shallow',
             'zero_length', 'subpixel', 'short', 'outside_partly', 'flat_form', 'zigzag', 'two_int')
    variants = [lambda r, a, w=w: dict(fill=r.rgb() + (a,), **({} if w == 'default' else dict(width=w)))
                for w in ('default', 1, 2, 2, 3, 1, 2)]
    return blend_book('blend_line', "ImageDraw.Draw(rgb, 'RGBA').line(xy, fill=rgba, width=...): particle trails and "
                      'speed lines', 8, 'line', kinds, line_xy, variants)


def book_blend_ellipse():
    variants = [lambda r, a: dict(fill=r.rgb() + (a,)), lambda r, a: dict(fill=r.rgb() + (a,)),
                lambda r, a: dict(outline=r.rgb() + (a,))]
    return blend_book('blend_ellipse', "ImageDraw.Draw(rgb, 'RGBA').ellipse(xy, fill=rgba) and ellipse(xy, "
                      'outline=rgba): particle heads, halos and rings', 9, 'ellipse', ELLIPSE_KINDS, box_xy, variants)


def book_blend_rectangle():
    variants = [lambda r, a: dict(fill=r.rgb() + (a,)), lambda r, a: dict(fill=r.rgb() + (a,)),
                lambda r, a: dict(fill=r.rgb() + (a,)), lambda r, a: dict(outline=r.rgb() + (a,))]
    return blend_book('blend_rectangle', "ImageDraw.Draw(rgb, 'RGBA').rectangle(xy, fill=rgba)", 10, 'rectangle',
                      RECT_KINDS, box_xy, variants)


def book_blend_point():
    return blend_book('blend_point', "ImageDraw.Draw(rgb, 'RGBA').point(xy, fill=rgba)", 11, 'point', POINT_KINDS,
                      point_xy, [lambda r, a: dict(fill=r.rgb() + (a,))])


# ---------------------------------------------------------------- pictures to blur and resize
def picture_for(kind, mode, r, w, h):
    """an (h, w) or (h, w, c) uint8 test picture"""
    from PIL import Image, ImageDraw
    c = {'L': 1, 'RGB': 3, 'RGBA': 4}[mode]
    g = r.g
    if kind == 'noise':
        a = g.integers(0, 256, (h, w, c), dtype=np.uint8)
    elif kind == 'flat':
        a = np.empty((h, w, c), np.uint8)
        a[:] = g.integers(0, 256, c, dtype=np.uint8)
    elif kind == 'impulse':                               # single bright pixels on black: the kernel itself
        a = np.zeros((h, w, c), np.uint8)
        for _ in range(r.i(1, 3)):
            a[r.pick([0, h - 1, r.i(0, h - 1)]), r.pick([0, w - 1, r.i(0, w - 1)])] = \
                g.integers(128, 256, c, dtype=np.uint8)
    elif kind == 'step':                                  # hard edges
        a = np.zeros((h, w, c), np.uint8)
        a[:, r.i(0, w):] = g.integers(0, 256, c, dtype=np.uint8)
        a[r.i(0, h):, :] //= 2
    elif kind == 'checker':
        n = r.i(1, 4)
        yy, xx = np.mgrid[0:h, 0:w]
        a = np.where((((yy // n) + (xx // n)) % 2)[..., None] > 0, g.integers(0, 256, c, dtype=np.uint8),
                     g.integers(0, 256, c, dtype=np.uint8)).astype(np.uint8)
    elif kind == 'gradient':
        yy, xx = np.mgrid[0:h, 0:w]
        a = np.stack([(xx * r.u(0, 255) / max(1, w - 1) + yy * r.u(0, 255) / max(1, h - 1)) % 256
                      for _ in range(c)], -1).astype(np.uint8)
    elif kind == 'mask':                                  # polygons at 255 on black: a shadow mask
        im = Image.new('L', (w, h), 0)
        d = ImageDraw.Draw(im)
        for _ in range(r.i(1, 5)):
            d.polygon(poly_xy(r.pick(['quad', 'circle', 'sliver_thin', 'triangle']), r, w, h), fill=255)
        m = np.array(im)
        a = np.repeat(m[..., None], c, 2)
    elif kind == 'spots':                                 # a few bright blobs on black: what the bloom blurs
        a = np.zeros((h, w, c), np.uint8)
        for _ in range(r.i(1, 6)):
            y, x, s = r.i(0, h - 1), r.i(0, w - 1), r.i(1, 4)
            a[y:y + s, x:x + s] = g.integers(60, 256, c, dtype=np.uint8)
    else:
        raise KeyError(kind)
    if mode == 'RGBA' and kind in ('mask', 'spots', 'impulse', 'step'):
        a[..., 3] = np.maximum(a[..., 3], g.integers(0, 256, (h, w), dtype=np.uint8) // 2)
    return np.ascontiguousarray(a[..., 0] if mode == 'L' else a)


def book_gaussian_blur():
    from PIL import Image, ImageFilter
    b = Book('gaussian_blur', 'Image.filter(ImageFilter.GaussianBlur(radius)): three passes of an extended box blur '
             '(BoxBlur.c). The renderer blurs its L shadow masks by 0.7 and the RGB bloom source (90 x 160) by 3.0',
             note='args: mode, size (width, height), radius as passed (a float.hex() string; Pillow reads it as a C '
                  'float). Arrays "in" and "out" are (height, width) for L, (height, width, 3) for RGB.')
    r = Rnd(12)
    plan = [('L', 0.7, k) for k in ('mask', 'mask', 'mask', 'impulse', 'noise', 'step', 'checker', 'flat')] * 6 + \
           [('RGB', 3.0, k) for k in ('spots', 'spots', 'impulse', 'noise', 'step', 'gradient', 'flat')] * 6 + \
           [(m, rad, k) for m in ('L', 'RGB') for rad in (0.3, 0.5, 1.0, 1.5, 2.0, 2.5, 4.0, 5.0, 8.0, 0.05)
            for k in ('impulse', 'noise', 'mask', 'spots', 'step')] + \
           [('L', 0.7, 'impulse'), ('L', 0.7, 'noise'), ('RGB', 3.0, 'impulse'), ('RGB', 3.0, 'noise')] * 3
    tiny = [(1, 1), (1, 7), (7, 1), (2, 2), (3, 2), (2, 5), (4, 4), (5, 3)]
    for n, (mode, rad, kind) in enumerate(plan):
        w, h = tiny[(n // 7) % len(tiny)] if n % 7 == 6 else r.size(6)
        if kind == 'noise':
            w, h = min(w, 32), min(h, 32)
        a = picture_for(kind, mode, r, w, h)
        out = np.array(Image.fromarray(a).filter(ImageFilter.GaussianBlur(rad)))
        b.add('Image.fromarray(a).filter(ImageFilter.GaussianBlur(radius))',
              dict(kind=kind, mode=mode, size=[w, h], radius=enc(float(rad))), out=out, **{'in': a})
    return b


def resize_book(name, how, seed, plan_extra):
    from PIL import Image
    flt = getattr(Image.Resampling, how)
    b = Book(name, f'Image.resize(size, Image.{how}) on L, RGB and RGBA images',
             note='args: mode, size_in and size_out (width, height). Arrays "in" and "out" are (height, width[, '
                  'channels]). An RGBA image is resized with its colour premultiplied by alpha (Pillow converts to '
                  'RGBa and back) for every filter but NEAREST.')
    r = Rnd(seed)
    plan = list(plan_extra)
    kinds = ('noise', 'gradient', 'spots', 'checker', 'step', 'flat', 'impulse', 'mask')
    modes = ('RGB', 'L', 'RGB', 'RGBA')
    n = 0
    while len(plan) < MIN_CASES:
        mode, kind = modes[n % 4], kinds[(n // 4 + n) % len(kinds)]
        w, h = r.size(1) if n % 9 == 8 else r.size(4)
        if kind == 'noise':                                                # (noise does not compress: keep it small)
            w, h = min(w, 40), min(h, 40)
        v = n % 6
        if v == 0:
            ow, oh = r.i(1, MAX_SIDE), r.i(1, MAX_SIDE)                    # any size
        elif v == 1:
            ow, oh = max(1, w // r.i(2, 5)), max(1, h // r.i(2, 5))        # down by whole factors
        elif v == 2:
            k = r.i(2, 4)
            w, h = max(1, w // k), max(1, h // k)
            ow, oh = w * k, h * k                                          # up by a whole factor
        elif v == 3:
            ow, oh = max(1, int(w * r.u(0.3, 0.99))), max(1, int(h * r.u(0.3, 0.99)))   # down a little
        elif v == 4:
            ow, oh = min(MAX_SIDE, int(w * r.u(1.01, 2.5)) + 1), min(MAX_SIDE, int(h * r.u(1.01, 2.5)) + 1)
        else:
            ow, oh = r.pick([(w, max(1, h // 2)), (max(1, w // 2), h), (1, 1), (w, 1), (1, h)])
        plan.append((mode, kind, (w, h), (ow, oh)))
        n += 1
    for mode, kind, (w, h), (ow, oh) in plan:
        if (ow, oh) == (w, h):
            ow = ow + 1 if ow < MAX_SIDE else ow - 1                       # (the same size is a plain copy)
        a = picture_for(kind, mode, r, w, h)
        out = np.array(Image.fromarray(a).resize((ow, oh), flt))
        b.add(f'Image.fromarray(a).resize(size_out, Image.{how})',
              dict(kind=kind, mode=mode, size_in=[w, h], size_out=[ow, oh], resample=how), out=out, **{'in': a})
    return b


def book_resize_bilinear():
    # the bloom's two calls at a quarter of their real size: RGB down by 4, then up by 4
    extra = [('RGB', k, (w, h), (w // 4, h // 4)) for k in ('spots', 'noise', 'gradient', 'spots')
             for w, h in ((96, 64), (64, 96), (88, 92), (40, 72))]
    extra += [('RGB', k, (w, h), (w * 4, h * 4)) for k in ('spots', 'noise', 'gradient', 'spots')
              for w, h in ((24, 16), (16, 24), (22, 23), (10, 18))]
    return resize_book('resize_bilinear', 'BILINEAR', 13, extra)


def book_resize_nearest():
    extra = [(m, k, (w, h), (w * 3, h * 3)) for m in ('RGB', 'L') for k in ('noise', 'checker')
             for w, h in ((32, 20), (12, 21), (30, 32))]                   # the 3x upscale to 1080 x 1920
    return resize_book('resize_nearest', 'NEAREST', 14, extra)


def book_resize_bicubic():
    return resize_book('resize_bicubic', 'BICUBIC', 15, [])


# ---------------------------------------------------------------- whole calls, as the renderer makes them
def book_as_used(P):
    """render a few frames with Pillow traced and keep whole calls with their real arguments, at full size"""
    from reelkit import fx
    from reelkit.shadows import MaskGrid
    b = Book('as_used', 'whole Pillow calls captured from kit.picture.render at full size: the canvas shadow mask '
             '(polygons then a 0.7 blur), the bloom (down by 4, a 3.0 blur, up by 4), the spot beam\'s hull, ink '
             'starbursts, the title card\'s crack',
             note='Each case names the frame it was captured from and lists the calls in order. Coordinates are '
                  'the exact floats the kit passed (float.hex() strings). Arrays keep the image\'s own shape: '
                  '(height, width[, channels]).')
    trace = Trace().install()
    scope = []                                            # (what is being captured, its calls so far), innermost last
    kept = {}

    def hook(kind, name, obj, args, out, site, via):
        if not scope:
            return
        what, log = scope[-1]
        if kind == 'draw' and name in ('polygon', 'line'):
            if what == 'beam' and not (site.endswith(':render') and obj._image.mode == 'L'):
                return
            op = dict(method=name, xy=enc([tuple(p) for p in args['xy']]))
            for k in ('fill', 'outline', 'width'):
                if k in args:
                    op[k] = enc(args[k])
            log.append((op, obj._image))
        elif kind == 'image' and name in ('filter', 'resize') and what in ('shadow', 'bloom'):
            log.append((dict(method=name, radius=getattr(args.get('filter'), 'radius', None),
                             size=args.get('size'), resample=args.get('resample')), np.array(obj), np.array(out)))

    trace.hooks.append(hook)
    undo = []

    def scoped(owner, name, what, done):
        orig = getattr(owner, name)

        def call(*a, **kw):
            scope.append((what, []))
            try:
                out = orig(*a, **kw)
            finally:
                log = scope.pop()[1]
            done(log, a, out)
            return out
        undo.append((owner, name, orig, name in vars(owner)))
        setattr(owner, name, call)

    def got_shadow(ops, a, out):
        polys = [x[0] for x in ops if x[0].get('method') == 'polygon']
        blur = [x for x in ops if x[0].get('method') == 'filter']
        assert len(blur) == 1 and blur[0][0]['radius'] == 0.7 and len(polys) + 1 == len(ops)
        assert np.array_equal(np.asarray(blur[0][2], np.float32) / 255.0, out)
        kept.setdefault('shadow', []).append((polys, blur[0][1], blur[0][2]))

    def got_bloom(ops, a, out):
        assert [op['method'] for op, *_ in ops] == ['resize', 'filter', 'resize'], ops
        (r0, src, small), (f1, small1, blur), (r2, blur2, up) = ops
        assert np.array_equal(small, small1) and np.array_equal(blur, blur2) and f1['radius'] == 3.0
        assert tuple(r0['size']) == (F.W // 4, F.H // 4) and tuple(r2['size']) == (F.W, F.H)
        kept.setdefault('bloom', []).append((src, small, blur, up))

    def got_burst(ops, a, out):
        kept.setdefault('burst', []).append(([op for op, im in ops], np.array(ops[0][1]) if ops else None))

    def got_crack(ops, a, out):
        if ops:
            kept.setdefault('crack', []).append(([op for op, im in ops], np.array(ops[-1][1])))

    try:
        scoped(MaskGrid, 'raster', 'shadow', got_shadow)
        scoped(P, 'bloom', 'bloom', got_bloom)
        scoped(fx, 'starburst', 'burst', got_burst)
        scoped(P.TT, 'draw_crack', 'crack', got_crack)
        for f in AS_USED_FRAMES:
            kept.clear()
            scope.append(('beam', []))                    # outside every other scope: render's own L polygon
            try:
                P.render(f)
            finally:
                log = scope.pop()[1]
            beam = [(op, np.array(im)) for op, im in log]
            if f in AS_USED_WHOLE:
                (polys, mask, out), = kept['shadow']
                b.add("d = ImageDraw.Draw(Image.new('L', size, 0)); d.polygon(xy, fill=255) for each polygon in "
                      "order; then .filter(ImageFilter.GaussianBlur(blur))  [reelkit.shadows.MaskGrid.raster]",
                      dict(kind='shadow_mask', frame=f, mode='L', size=[mask.shape[1], mask.shape[0]],
                           polygons=[p['xy'] for p in polys], fill=255, blur=enc(0.7)), mask=mask, out=out)
                (src, small, blur, up), = kept['bloom']
                b.add('small = Image.fromarray(a).resize(small_size, Image.BILINEAR); blur = small.filter('
                      'ImageFilter.GaussianBlur(radius)); out = blur.resize(size, Image.BILINEAR)  '
                      '[reelkit.post.bloom]',
                      dict(kind='bloom', frame=f, mode='RGB', size=[F.W, F.H], small_size=[F.W // 4, F.H // 4],
                           radius=enc(3.0)), small=small, blur=blur, out=up, **{'in': src})
                (op, m), = beam
                b.add("ImageDraw.Draw(Image.new('L', size, 0)).polygon(xy, fill=255)  [the spot beam's hull, "
                      "kit.picture.render]",
                      dict(kind='beam', frame=f, mode='L', size=[F.W, F.H], xy=op['xy'], fill=op['fill']), out=m)
            for ops, patch in kept.get('burst', []):
                b.add("d = ImageDraw.Draw(Image.new('RGBA', size, (0, 0, 0, 0))); each op in order: "
                      "d.polygon(xy, fill=..., outline=...)  [reelkit.fx.starburst]",
                      dict(kind='starburst', frame=f, mode='RGBA', size=[patch.shape[1], patch.shape[0]],
                           background=[0, 0, 0, 0], ops=ops), out=patch)
            for ops, m in kept.get('crack', [])[:1]:      # (drawn once per door: the same crack twice)
                b.add("d = ImageDraw.Draw(Image.new('L', size, 0)); each op in order: d.line(xy, fill=255, width=2)"
                      "  [title.draw_crack]",
                      dict(kind='crack', frame=f, mode='L', size=[F.W, F.H], ops=ops), out=m)
    finally:
        for owner, name, orig, had in reversed(undo):
            if had:
                setattr(owner, name, orig)
            else:
                delattr(owner, name)
        trace.remove()
    return b


# ================================================================ checks
IMAGE_COVER = {                                  # Image / ImageDraw calls of the kit -> the file that covers them
    ('image', 'resize'): 'resize_bilinear', ('image', 'filter'): 'gaussian_blur',
    ('module', 'new'): None, ('module', 'fromarray'): None, ('module', 'Draw'): None,     # no pixels of their own
}
DRAW_FILES = {'polygon': ('polygon', 'polygon_outline', 'blend_polygon'), 'line': ('line', 'blend_line'),
              'ellipse': ('ellipse', 'blend_ellipse'), 'rectangle': ('rectangle', 'blend_rectangle'),
              'point': ('point', 'blend_point')}


def check_cover(rows, books):
    """every Pillow call found in the renderer has cases: a draw call by its image mode, draw mode, ink kinds and
    width; an Image call by the file that covers it. Returns (the lines of the cover table, {row: its files})"""
    by_name = {b.name: b for b in books}
    lines, missing, cover = [], [], {}
    for key in sorted(rows):
        kind, name, mode, draw_mode, site, via = key
        cover[key] = []
        if kind == 'draw':
            if name not in DRAW_FILES:
                missing.append(f'ImageDraw.{name} on {mode} (from {site}): no vector file draws it')
                continue
            files = [f for f in DRAW_FILES[name] if f in by_name]
            for fill, outline, width in sorted(rows[key]['combos'], key=str):
                sig = (name, mode, draw_mode, fill, outline, width)
                have = [f for f in files if sig in by_name[f].sigs]
                cover[key] = sorted(set(cover[key]) | set(have))
                if not have:
                    missing.append(f'ImageDraw.{name} on {mode} (draw mode {draw_mode}) with fill {fill}, outline '
                                   f'{outline}, width {width} (from {site}): no case')
            lines.append(f"  ImageDraw.{name:9s} {mode:4s} draw {draw_mode:4s} {rows[key]['calls']:9d} calls  "
                         f"{site}{' via ' + via if via else ''}")
        else:
            if (kind, name) not in IMAGE_COVER:
                missing.append(f'Image.{name} on {mode or "no image"} (from {site}): no vector file covers it')
                continue
            f = IMAGE_COVER[(kind, name)]
            if f is not None and f not in by_name:
                missing.append(f'Image.{name}: its file {f} was not made')
            cover[key] = [f] if f is not None else []
            if (kind, name) == ('image', 'resize'):
                bad = [d for d in rows[key]['details'] if 'BILINEAR' not in d]
                if bad:
                    missing.append(f'Image.resize {bad} (from {site}): only BILINEAR is covered as used')
            if (kind, name) == ('image', 'filter'):
                bad = [d for d in rows[key]['details'] if not d.startswith('GaussianBlur(')]
                if bad:
                    missing.append(f'Image.filter {bad} (from {site}): only GaussianBlur is covered')
            label = ('Image.Image.' if kind == 'image' else 'ImageDraw.' if name == 'Draw' else 'Image.') + name
            lines.append(f"  {label:19s} {mode:4s} {'':9s} {rows[key]['calls']:9d} calls  {site}"
                         f"{' via ' + via if via else ''}  {sorted(rows[key]['details'])}")
    assert not missing, 'the renderer calls Pillow in ways no case covers:\n  ' + '\n  '.join(missing)
    return lines, cover


def check_books(books):
    """each seeded file has its 200 cases on canvases up to 96 x 96, with the hard kinds the brief lists"""
    by_name = {b.name: b for b in books}
    want = ('polygon', 'polygon_outline', 'line', 'ellipse', 'rectangle', 'point', 'blend_polygon', 'blend_line',
            'blend_ellipse', 'blend_rectangle', 'blend_point', 'gaussian_blur', 'resize_nearest', 'resize_bilinear',
            'resize_bicubic', 'as_used')
    assert sorted(by_name) == sorted(want), f'files: {sorted(by_name)}; wanted: {sorted(want)}'
    for b in books:
        ids = [c['id'] for c in b.cases]
        assert len(set(ids)) == len(ids)
        if b.name == 'as_used':
            kinds = b.kinds()
            assert {'shadow_mask', 'bloom', 'beam', 'starburst', 'crack'} <= kinds, kinds
            continue
        assert len(b.cases) >= MIN_CASES, f'{b.name}: {len(b.cases)} cases, the brief asks for {MIN_CASES}'
        for c in b.cases:
            for k in ('size', 'size_in', 'size_out'):
                if k in c['args']:
                    assert max(c['args'][k]) <= MAX_SIDE, (b.name, c['id'])
    kinds = by_name['polygon'].kinds()
    for need, any_of in POLY_NEEDS.items():
        assert all(k in kinds for k in any_of), f'polygon: no cases for {need}'
    modes = {(c['args']['mode'], c['args'].get('width', 'default')) for c in by_name['line'].cases}
    assert all((m, w) in modes for m in ('L', 'RGB', 'F') for w in ('default', 1, 2, 3)), 'line: widths 1, 2, 3'
    ell = by_name['ellipse'].sigs
    assert any(s[3] != 'none' and s[4] == 'none' for s in ell) and any(s[3] == 'none' for s in ell), 'ellipse'
    for how in ('nearest', 'bilinear', 'bicubic'):
        got = {c['args']['mode'] for c in by_name['resize_' + how].cases}
        assert got == {'L', 'RGB', 'RGBA'}, (how, got)


# ================================================================ the export
BOOKS = (book_polygon, book_polygon_outline, book_line, book_ellipse, book_rectangle, book_point,
         book_blend_polygon, book_blend_line, book_blend_ellipse, book_blend_rectangle, book_blend_point,
         book_gaussian_blur, book_resize_nearest, book_resize_bilinear, book_resize_bicubic)


def run():
    P = F.picture('pillow')
    frames = range(F.NF)
    books = [make() for make in BOOKS] + [book_as_used(P)]
    check_books(books)
    rows = survey(P, frames)
    table, cover = check_cover(rows, books)
    print('pillow: every Pillow call the kit made drawing all', len(frames), 'frames, and it has cases:')
    print('\n'.join(table))
    common.write_json(os.path.join(OUT, 'calls.json'), survey_doc(rows, frames, cover))
    total = os.path.getsize(os.path.join(OUT, 'calls.json'))
    for b in books:
        size = b.write()
        total += size
        print(f'pillow: {b.name:16s} {len(b.cases):4d} cases  {size / 1e3:8.1f} kB')
    print(f'pillow: {sum(len(b.cases) for b in books)} cases in {len(books)} files, {total / 1e6:.2f} MB -> '
          f'{common.rel(OUT)}')


EXPORTS = {'pillow': run}

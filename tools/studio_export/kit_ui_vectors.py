"""kit_ui: what reelkit.ui draws and notes (captions, the fight HUD, labels, panels), for the port in bf-picture.

studio/reference/picture_kit/ (every .npz has a sidecar .json: vectors-convention.md):

    ui_label.npz     label (every scale, with and without an outline, inside, across and outside the picture, and
                     the HP numbers kit.picture really stamps into the balls) and panel
    ui_puff.npz      _hash01 on its own (with the number its sine is taken of), and _sand_puff: the grains under
                     the example episode's real caption lines through their life, and seeded ones at the edges of
                     the safe box
    ui_captions.npz  draw_captions(style='impact') with the caption lists and colours kit.picture really passes
                     for the example episode (the ability names, *K.O.*, the winner's name and WINS!, COMMENT THE
                     *NEXT MATCH-UP*) at frame times across each caption's life, seeded captions (mark-up, long and
                     empty lines, a window that never opens, text across the picture's edge), impact_landings and
                     caption_scale
    ui_hud.npz       fight_hud: the states the plan lists on a mid-grey picture (full HP, damage with its trail,
                     low HP, a flash, K.O., the winner's frame, levels 1 to 3; each with and without ability rows),
                     the calls kit.picture really makes at chosen frames of the example episode, and seeded ones

A drawing case holds no picture to draw on: `args.canvas` says how to make it (`canvas()` below), a flat colour or
a "weave" that holds every byte value in every channel and still packs small, all over or in a band of rows. `out`
is the picture after the call. What the call returned and what it filed in reelkit.ui.DRAWN (the boxes the
safe-zone check reads) are in the case's `expect`. Floats that Rust must match are float.hex() strings.

The real calls are recorded, not rebuilt: the export renders the chosen frames with kit.picture's fight_hud,
draw_captions and label wrapped, and keeps the arguments each was given."""
import copy
import math
import os

import numpy as np

from . import common

OUT = common.ref('picture_kit')
SEED = 20261119
W, H, FPS, NF = 360, 640, 30, 1830
GREY = (128, 128, 128)
HASH_K = (12.9898, 78.233, 37.719, 4.581)             # reelkit.ui._hash01's multipliers
CANVAS_NOTE = ('args.canvas makes the picture the call draws on, w x h, uint8 RGB: with only "flat", that colour '
               'everywhere; with "mul" and "add", pixel i (counted row by row from the top left) has the channels '
               '(mul[c] * i + add[c]) % 256, and if "rows" [ya, yb] is there too only rows ya to yb - 1 are woven '
               'like that and the rest is "flat". "out" is the picture after the call. ')


# ================================================================ the files
class Group:
    """one vector file being filled: the cases (sidecar rows) and their arrays (npz members)"""

    def __init__(self, name, about, **top):
        self.name, self.about, self.top = name, about, top
        self.cases, self.arrays = [], {}

    def add(self, call, args, expect=None, **arrays):
        """a case: `call` is the Python expression, `args` its JSON arguments, `expect` what it returned and noted,
        `arrays` role -> array"""
        cid = f'c{len(self.cases):03d}'
        names = {}
        for role, a in arrays.items():
            assert isinstance(a, np.ndarray) and a.dtype.str in common.NPZ_DTYPES, (self.name, cid, role, type(a))
            self.arrays[f'{cid}__{role}'] = np.ascontiguousarray(a)
            names[role] = f'{cid}__{role}'
        case = dict(id=cid, call=call, args=args, arrays=names)
        if expect is not None:
            case['expect'] = expect
        self.cases.append(case)
        return cid

    def write(self):
        """write <name>.npz and <name>.json, read the archive back and compare, then list both in the manifest"""
        from . import manifest
        npz, js = os.path.join(OUT, self.name + '.npz'), os.path.join(OUT, self.name + '.json')
        common.write_npz(npz, self.arrays, register=False)
        common.write_json(js, dict(about=self.about, tool='kit_ui', **self.top, cases=self.cases), register=False)
        with np.load(npz, allow_pickle=False) as z:
            named = {m for c in self.cases for m in c['arrays'].values()}
            assert set(z.files) == set(self.arrays) == named, f'{self.name}: members and sidecar disagree'
            for k, a in self.arrays.items():
                b = z[k]
                assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), (self.name, k)
        manifest.add(npz, tool=__name__)
        manifest.add(js, tool=__name__)
        size = os.path.getsize(npz) + os.path.getsize(js)
        print(f'kit_ui: {self.name:12s} {len(self.cases):4d} cases  {size / 1e3:8.1f} kB')
        return size


def rng(*key):
    return np.random.default_rng([SEED, *key])


# ================================================================ the pictures the calls draw on
def flat(w=W, h=H, rgb=GREY):
    return dict(w=w, h=h, flat=list(rgb))


def weave(r, w=W, h=H, rows=None):
    """a picture spec with seeded odd multipliers and offsets: every channel walks through all 256 values. With
    `rows` = (ya, yb) only those rows are woven and the rest is mid-grey (a whole woven frame packs to 7 kB, a
    band of it to a fraction)"""
    spec = dict(w=w, h=h, mul=[int(v) * 2 + 1 for v in r.integers(0, 128, 3)],
                add=[int(v) for v in r.integers(0, 256, 3)])
    if rows is not None:
        spec.update(flat=list(GREY), rows=[max(0, min(h, int(rows[0]))), max(0, min(h, int(rows[1])))])
    return spec


def canvas(spec):
    """the uint8 RGB picture a spec stands for. Only `flat`: that colour everywhere. With `mul` and `add`: pixel i
    (counted row by row from the top left of the whole picture) has the channels (mul[c] * i + add[c]) % 256; if
    the spec also has `rows` = [ya, yb], only rows ya to yb - 1 are woven like that and the rest is `flat`"""
    w, h = spec['w'], spec['h']
    if 'mul' not in spec:
        return np.full((h, w, 3), spec['flat'], np.uint8)
    i = np.arange(w * h, dtype=np.int64)[:, None]
    woven = ((i * np.array(spec['mul'], np.int64) + np.array(spec['add'], np.int64)) % 256).astype(np.uint8)
    woven = woven.reshape(h, w, 3)
    if 'rows' not in spec:
        return woven
    img = np.full((h, w, 3), spec['flat'], np.uint8)
    ya, yb = spec['rows']
    img[ya:yb] = woven[ya:yb]
    return img


def noted(ui, fn):
    """run fn() on an empty reelkit.ui.DRAWN; returns (what fn returned, what it noted as JSON rows)"""
    ui.reset_drawn()
    ret = fn()
    rows = [list(b) for b in ui.DRAWN]
    ui.reset_drawn()
    for b in rows:
        assert all(type(v) is int for v in b[:4]) and type(b[4]) is str and type(b[5]) is str, b
    return ret, rows


def kw_call(head, kw):
    """the call text: `head` and the keywords passed, each as name=name"""
    return head + ''.join(f', {k}={k}' for k in kw) + ')'


# ================================================================ the real calls of kit.picture
def real_frames(P):
    """{frame: why} for the frames whose HUD, caption and label calls the real cases replay"""
    from . import frames as F
    T = P.T
    picks = {}

    def at(t, why):
        picks.setdefault(min(NF - 1, F.first_at(t)), why)

    f = 0
    while P.TT.door_offset(f / FPS) == 0.0:
        f += 1
    picks[f] = 'the title card starts to open: the HUD appears'
    at(T.T_FIGHT - 0.5, 'the count-in: full HP, every ability locked')
    for k in (0, 1):
        for slot, tag in P.SLOTS:
            tu = P.ABIL_UNLOCK[k].get(slot)
            if tu is not None:
                at(tu + 0.1, f'{P.NAMES[k]} unlocks {tag}: the sweep starts')
                at(tu + 0.3, f'{P.NAMES[k]} unlocks {tag}: the sweep, further on')
            uses = P.ABIL_USES[k][slot]
            if uses:
                at(uses[0] + 0.05, f'{P.NAMES[k]} uses {tag}: lit')
                at(uses[0] + P.ABIL_HOLD[k][slot] + 0.25, f'{P.NAMES[k]} used {tag}: the glow eases back')
    hits = [e for e in P.EV if e[1] == 'hit' and e[0] < P.T_KO]
    for e in hits[:4] + hits[len(hits) // 2:len(hits) // 2 + 4] + hits[-4:]:
        at(e[0] + 0.01, 'a hit: the bar may flash')
    for k in (0, 1):
        low = [f for f in range(NF) if f / FPS < P.T_KO and P.hp_at(k, f / FPS) <= 25.0]
        if low:
            picks.setdefault(low[0], f'{P.NAMES[k]} at low HP: the bar pulses')
            picks.setdefault(min(low[0] + 7, low[-1]), f'{P.NAMES[k]} at low HP, a little later in the pulse')
    at(P.T_KO, 'the K.O.')
    at(P.T_KO + 0.6, 'after the K.O.')
    at(P.T_KO + 1.4, "the winner's gold frame")
    at(P.T_KO + 3.0, "the winner's gold frame, later")
    at(T.T_GLIDE + 0.1, 'the glide home: HP refills')
    picks.setdefault(NF - 1, 'the last frame')
    return dict(sorted(picks.items()))


def record(P, frames):
    """render the frames with kit.picture's three ways into reelkit.ui wrapped; {frame: [(name, args, kwargs)]} with
    the arguments as they were given (the picture, each call's first argument, is left out)"""
    seen, now = {}, [None]

    def wrap(name, fn):
        def inner(img, *a, **kw):
            seen[now[0]].append((name, copy.deepcopy(a), copy.deepcopy(kw)))
            return fn(img, *a, **kw)
        return inner

    saved = {n: getattr(P, n) for n in ('fight_hud', 'draw_captions', 'label')}
    try:
        for n, fn in saved.items():
            setattr(P, n, wrap(n, fn))
        for f in frames:
            now[0] = f
            seen[f] = []
            P.render(f)
    finally:
        for n, fn in saved.items():
            setattr(P, n, fn)
    return seen


_REAL = {}


def real():
    """(P, {frame: why}, {frame: recorded calls}), rendered once per run"""
    if not _REAL:
        from . import frames as F
        P = F.picture('kit_ui')
        why = real_frames(P)
        _REAL.update(P=P, why=why, calls=record(P, list(why)))
    return _REAL['P'], _REAL['why'], _REAL['calls']


# ================================================================ label and panel
LABEL_TEXTS = ('FLUTE', 'TROMBONE', 'K.O.', '100', '7', 'LV', 'VS', '', 'low hp', 'A', 'WINS!', '3\u00d74=12')


def group_label():
    from PIL import Image, ImageDraw
    from reelkit import ui
    g = Group('ui_label', 'reelkit.ui.label and reelkit.ui.panel on small pictures, and the HP numbers of the example '
                          'episode as kit.picture stamps them',
              note=CANVAS_NOTE + 'label: a keyword that is absent was not passed (scale defaults to 2, outline to '
                   '(10, 20, 26); outline null is outline=None); expect.returns is the width it returned. panel: '
                   'draw is ImageDraw.Draw of the picture; fill and edge default to (16, 20, 38) and (62, 74, 112). '
                   'expect.noted is what the call appended to reelkit.ui.DRAWN: [x0, y0, x1, y1, kind, info].')
    r = rng(1)
    spots = [(8, 10), (30, 20), (-3, 6), (5, -4), (100, 12), (12, 40), (-60, 5), (6, -40), (119, 47), (0, 0), (125, 3),
             (4, 50)]
    outlines = ['default', None, (10, 20, 26), (255, 255, 255), (60, 8, 120)]
    n = 0
    for x, y in spots:
        for rep in range(4):
            text = LABEL_TEXTS[n % len(LABEL_TEXTS)]
            col = tuple(int(v) for v in r.integers(0, 256, 3))
            spec = weave(r, 128, 48) if n % 5 else flat(128, 48)
            args, kw, pos = dict(canvas=spec, text=text, x=x, y=y, col=list(col)), {}, ()
            scale = (None, 1, 2, 3)[(n + rep) % 4]
            if scale is not None:
                pos = (scale,)
                args['scale'] = scale
            o = outlines[(n * 2 + rep) % len(outlines)]
            if o != 'default':
                kw['outline'] = o
                args['outline'] = None if o is None else list(o)
            img = canvas(spec)
            ret, rows = noted(ui, lambda: ui.label(img, text, x, y, col, *pos, **kw))
            assert type(ret) is int
            g.add(kw_call('reelkit.ui.label(img, text, x, y, col' + (', scale' if pos else ''), kw), args,
                  dict(returns=ret, noted=rows), out=img)
            n += 1

    # the HP numbers in the balls, exactly as kit.picture.render asks for them
    P, why, calls = real()
    stamped = [(f, a, kw) for f, got in calls.items() for name, a, kw in got if name == 'label']
    assert len(stamped) >= 8, f'the recorded frames stamp only {len(stamped)} HP numbers'
    for f, a, kw in stamped[::len(stamped) // 8][:8]:
        text, x, y, col, scale = a
        assert not kw and type(x) is int and type(y) is int and type(scale) is int, (a, kw)
        spec = weave(r, rows=(y - 10, y + 18))
        img = canvas(spec)
        ret, rows = noted(ui, lambda: ui.label(img, text, x, y, col, scale))
        g.add('reelkit.ui.label(img, text, x, y, col, scale)',
              dict(canvas=spec, text=text, x=x, y=y, col=list(col), scale=scale, frame=f, source='kit.picture'),
              dict(returns=ret, noted=rows), out=img)

    boxes = [(10, 8, 90, 30), (4, 4, 4, 4), (20, 10, 21, 11), (-6, -5, 40, 20), (60, 20, 140, 60), (0, 0, 127, 47),
             (30, 40, 70, 44), (200, 10, 240, 30), (5, 30, 6, 46), (50, 12, 52, 13), (-30, -30, -10, -8),
             (40, 5, 100, 6)]
    for n, (x0, y0, x1, y1) in enumerate(boxes):
        for rep in range(2):
            spec = weave(r, 128, 48) if (n + rep) % 3 else flat(128, 48)
            args, kw = dict(canvas=spec, x0=x0, y0=y0, x1=x1, y1=y1), {}
            if (n + rep) % 2:
                kw['fill'] = tuple(int(v) for v in r.integers(0, 256, 3))
            if (n // 2 + rep) % 2:                        # (sometimes the fill's own colour)
                same = 'fill' in kw and n % 4 == 3
                kw['edge'] = kw['fill'] if same else tuple(int(v) for v in r.integers(0, 256, 3))
            args.update({k: list(v) for k, v in kw.items()})
            im = Image.fromarray(canvas(spec))
            _, rows = noted(ui, lambda: ui.panel(ImageDraw.Draw(im), x0, y0, x1, y1, **kw))
            g.add(kw_call('reelkit.ui.panel(draw, x0, y0, x1, y1', kw), args, dict(noted=rows), out=np.array(im))
    return g


# ================================================================ the sand puff
def puff_key(pl, k, t0):
    """the key _draw_impact gives a line's puff"""
    return hash(pl) % 997 + 13 * k + int(t0 * 10)


def real_lines(P):
    """every caption line of the example episode as _draw_impact lays it out:
    (plain text, line index, t0, landing time, x0, x1, ybot, puff colour, key)"""
    from reelkit import ui
    out = []
    for name, a, kw in real_caption_forms(P):
        caps = a[1]
        puff = kw.get('puff', (236, 204, 150))
        for t0, t1, lines in caps:
            scale = ui.caption_scale(lines)
            lh = 7 * scale
            for k, ln in enumerate(lines):
                pl = ui._plain(ln)
                w0 = len(pl) * ui.ADV * scale - scale
                xl = int(round(ui.SAFE_CX - w0 / 2))
                out.append((pl, k, t0, t0 + k * ui.IMPACT_STAGGER + ui.IMPACT_FALL, xl, xl + w0,
                            ui.CAP_Y0 + k * (lh + 7) + lh, puff, puff_key(pl, k, t0)))
    return out


def group_puff():
    from reelkit import ui
    g = Group('ui_puff', 'reelkit.ui._hash01 and reelkit.ui._sand_puff',
              HASH_K=[common.hexf(v) for v in HASH_K], PUFF_T=common.hexf(ui.PUFF_T),
              note='_hash01: one Python call per row of the int arrays a, b, c, d (as many as the call names); '
                   '"phase" is the number its sine is taken of, sum(x * k for x, k in zip(xs, HASH_K)) + 0.5 '
                   "(Python's sum() of floats is compensated, so this is not the plain left-to-right sum), and "
                   '"out" what it returned. math.sin comes from the C runtime of the reference PC: a port with '
                   'another sine can differ from "out" in the last bits, never from "phase". _sand_puff: t and tl '
                   'are float.hex() strings, col null is col=None. ' + CANVAS_NOTE)
    P, why, calls = real()
    lines = real_lines(P)
    r = rng(2)

    def hash_case(names, rows):
        cols = list(zip(*rows))
        phase = [sum(x * k for x, k in zip(xs, HASH_K)) + 0.5 for xs in rows]
        out = [ui._hash01(*xs) for xs in rows]
        for ph, o in zip(phase, out):
            v = math.sin(ph) * 43758.5453
            assert o == v - math.floor(v) and 0.0 <= o < 1.0
        g.add(f"reelkit.ui._hash01({', '.join(names)})", {}, None,
              **{nm: np.array(c, np.int64) for nm, c in zip(names, cols)},
              phase=np.array(phase, np.float64), out=np.array(out, np.float64))

    # the grains of the real caption lines (key, grain 0..n, draw 1..4), then seeded keys, then other argument counts
    rows = [(ln[8], k, j) for ln in lines for k in range(12 + int((ln[5] - ln[4]) / 7)) for j in (1, 2, 3, 4)]
    hash_case(('a', 'b', 'c'), rows)
    rows = [(int(a), int(b), int(c)) for a, b, c in zip(r.integers(0, 1800, 3000), r.integers(0, 64, 3000),
                                                         r.integers(1, 5, 3000))]
    hash_case(('a', 'b', 'c'), rows + [(0, 0, 0), (0, 0, 1), (996, 0, 4), (-5, 3, 2), (10 ** 6, 7, 1)])
    hash_case(('a',), [(int(v),) for v in r.integers(-50, 3000, 200)] + [(0,)])
    hash_case(('a', 'b'), [(int(a), int(b)) for a, b in zip(r.integers(0, 3000, 200), r.integers(-9, 99, 200))])
    hash_case(('a', 'b', 'c', 'd'), [tuple(int(v) for v in row) for row in r.integers(0, 500, (300, 4))])

    def puff_case(spec, t, tl, x0, x1, ybot, col, key, **more):
        img = canvas(spec)
        before = img.copy()
        ret, rows_ = noted(ui, lambda: ui._sand_puff(img, t, tl, x0, x1, ybot, col, key))
        assert ret is None and not rows_
        g.add('reelkit.ui._sand_puff(img, t, tl, x0, x1, ybot, col, key)',
              dict(canvas=spec, t=common.hexf(t), tl=common.hexf(tl), x0=x0, x1=x1, ybot=ybot,
                   col=None if col is None else list(col), key=key, **more), None, out=img)
        return int((img != before).any(2).sum())

    # the real lines at the frames of their puff's life (the first frame at or after the landing, then on)
    from . import frames as F
    changed = 0
    for n, (pl, k, t0, tl, x0, x1, ybot, puff, key) in enumerate(lines):
        f0 = F.first_at(tl)
        for step in ((0, 1, 3, 7, 8, 12, 13, 14) if n % 2 else (0, 2, 5, 7, 10, 13)):
            t = (f0 + step) / FPS
            spec = flat() if (n + step) % 3 == 0 else weave(r, rows=(ybot - 44, ybot + 40))
            changed += puff_case(spec, t, tl, x0, x1, ybot, puff, key, line=pl, frame=f0 + step,
                                 source='kit.picture')
    assert changed > 400, 'the real puffs change too few pixels to be a test'

    # seeded: every edge of the safe box, the thresholds of u, widths of every kind, no colour
    us = [0.0, 1e-9, 0.01, 0.1, 0.2, 0.2499999, 0.25, 0.2500001, 0.3, 0.4, 0.4499999, 0.45, 0.46, -0.01, -1e-12,
          0.05, 0.15, 0.35]
    spans = [(60, 300), (20, 340), (0, 360), (150, 210), (20, 27), (300, 339), (180, 180), (200, 190), (330, 420),
             (-40, 30), (20, 26), (100, 114)]
    ybots = [121, 149, 96, 100, 495, 499, 300, 90, 498, 470]
    for n, u in enumerate(us * 2):
        x0, x1 = spans[n % len(spans)]
        ybot = ybots[n % len(ybots)]
        tl = float(r.uniform(0, 60))
        t = tl + u
        if n % 4 == 1:                                    # u as a frame time would give it, not as a sum
            t = float(round(t * FPS)) / FPS
        col = None if n % 11 == 10 else tuple(int(v) for v in r.integers(0, 256, 3))
        puff_case(flat() if n % 4 == 0 else weave(r, rows=(ybot - 44, ybot + 40)), t, tl, x0, x1, ybot, col,
                  int(r.integers(0, 1700)))
    # a picture smaller than the safe box: the grains past its edge are cut off
    for n in range(4):
        spec = weave(r, 200 + 20 * n, 140)
        tl = float(r.uniform(0, 60))
        puff_case(spec, tl + 0.05 + 0.08 * n, tl, 40, 330, 136, (236, 204, 150), int(r.integers(0, 1700)))
    return g


# ================================================================ captions
SEEDED_LINES = ('FLUTTER TONGUE', 'SNAKE DANCE', 'PICCOLO', 'SAD TROMBONE', 'POWER SLIDE', 'PEDAL TONE', '*K.O.*',
                'WINS!', 'COMMENT THE', '*NEXT MATCH-UP*', 'who ~wins~ *this*?', '~FLUTE~ VS *TROMBONE*', 'A', '', '*',
                'ROUND *2*: ~FIGHT~!', 'THIS LINE IS FAR TOO LONG TO FIT IN', "DON'T *BLINK*", 'x\u00d72 = 4\u00b0',
                '*unclosed', '~a~*b*c', '21 CHARS EXACTLY HERE', '14 CHARS HERE!', '15 CHARS HERE!!', '~~', 'I')


def real_caption_forms(P):
    """the draw_captions calls of one frame of kit.picture.render, as (name, args, kwargs) with args (t, caps);
    they are the same on every frame but for t, which is checked"""
    _, _, calls = real()
    forms = None
    for f, got in calls.items():
        mine = [(name, a, kw) for name, a, kw in got if name == 'draw_captions']
        assert len(mine) == 6 and all(len(a) == 2 and a[0] == f / FPS for _, a, _ in mine), (f, len(mine))
        shape = [(a[1], kw) for _, a, kw in mine]
        if forms is None:
            forms = mine
        assert shape == [(a[1], kw) for _, a, kw in forms], f'frame {f}: the caption calls changed'
    return forms


def caps_json(caps):
    return [dict(t0=common.hexf(float(t0)), t1=common.hexf(float(t1)), lines=list(lines)) for t0, t1, lines in caps]


def caption_case(g, ui, spec, t, caps, kw, **more):
    assert kw.get('style') == 'impact'
    img = canvas(spec)
    before = img.copy()
    ret, rows = noted(ui, lambda: ui.draw_captions(img, t, caps, **kw))
    assert ret is None
    other = {k: v for k, v in kw.items() if k != 'style'}
    args = dict(canvas=spec, t=common.hexf(t), caps=caps_json(caps), **more)
    for k, v in other.items():
        args[k] = v if v is None or isinstance(v, int) else list(v)
    g.add(kw_call("reelkit.ui.draw_captions(img, t, caps, style='impact'", other), args, dict(noted=rows), out=img)
    return int((img != before).any(2).sum()), rows


def group_captions():
    from reelkit import ui
    from . import frames as F
    g = Group('ui_captions', "reelkit.ui.draw_captions(style='impact'), impact_landings and caption_scale",
              constants=dict(CAP_Y0=ui.CAP_Y0, CAP_BIG_MAX=ui.CAP_BIG_MAX, CAP_SMALL_MAX=ui.CAP_SMALL_MAX,
                             IMPACT_DROP=ui.IMPACT_DROP, IMPACT_FALL=common.hexf(ui.IMPACT_FALL),
                             IMPACT_STAGGER=common.hexf(ui.IMPACT_STAGGER), IMPACT_PUNCH=common.hexf(ui.IMPACT_PUNCH),
                             IMPACT_PUNCH_T=common.hexf(ui.IMPACT_PUNCH_T), IMPACT_SHAKE=common.hexf(ui.IMPACT_SHAKE),
                             IMPACT_SHAKE_T=common.hexf(ui.IMPACT_SHAKE_T), PUFF_T=common.hexf(ui.PUFF_T),
                             CREAM=list(ui.CREAM), AMBER=list(ui.AMBER), SKYB=list(ui.SKYB), MINT=list(ui.MINT),
                             CORAL=list(ui.CORAL), PINK=list(ui.PINK), CYAN=list(ui.CYAN), INK=list(ui.INK)),
              note=CANVAS_NOTE + 'draw_captions: caps is the list of (t0, t1, lines) with the times as float.hex() '
                   'strings; a keyword that is absent was not passed (y0 100, cx 180, base CREAM, hi AMBER, alt '
                   'SKYB, puff (236, 204, 150); puff null is puff=None); expect.noted is what the call appended to '
                   'reelkit.ui.DRAWN: [x0, y0, x1, y1, kind, info]. Cases with source "kit.picture" '
                   'hold the caption list and keywords kit.picture.render passes for the example episode, at the '
                   'time of the frame named. impact_landings: expect.returns rows are [t_land, line, chars]. '
                   'Python scalar maths (math.sin for the shake, ** 2 for the punch) is the C runtime of the '
                   "reference PC's.")
    P, why, calls = real()
    forms = real_caption_forms(P)
    r = rng(3)
    assert [len(a[1]) for _, a, _ in forms] == [0, 3, 2, 1, 1, 1], 'the caption lists of the example episode changed'
    texts = [ln for _, a, _ in forms for _, _, lines in a[1] for ln in lines]
    assert texts == ['FLUTTER TONGUE', 'SNAKE DANCE', 'PICCOLO', 'SAD TROMBONE', 'POWER SLIDE', '*K.O.*', 'TROMBONE',
                     'WINS!', 'COMMENT THE', '*NEXT MATCH-UP*'], texts

    # the real calls: each caption at frames across its life (the frame before it, its fall, the landing with its
    # punch, shake and puff, the middle, the fade, the first frame after it)
    n = drew = 0
    for name, a, kw in forms:
        caps = a[1]
        if not caps:                                      # CAP_TITLE is empty: the call must draw nothing
            _, rows = caption_case(g, ui, flat(), 12.0, caps, kw, frame=360, source='kit.picture')
            assert not rows
            continue
        for t0, t1, lines in caps:
            fa, fb = F.first_at(t0), F.first_at(t1)
            steps = [-1, 0, 1, 2, 3, 4, 6, 9, 13]
            if len(lines) > 1:                            # the second line starts 0.22 s later
                steps += [7, 10, 11, 16, 22]
            picked = sorted({fa + s for s in steps} | {(fa + fb) // 2, fb - 3, fb - 2, fb - 1, fb})
            for f in picked:
                spec = flat() if n % 3 == 0 else weave(r, rows=(84, 196))
                changed, rows = caption_case(g, ui, spec, f / FPS, caps, kw, frame=f, source='kit.picture')
                assert bool(rows) == (t0 <= f / FPS < t1), (f, lines)
                drew += changed
                n += 1
    assert drew > 100000, 'the real captions change too few pixels to be a test'

    # seeded captions on pictures of several sizes: mark-up, both sizes, lines too long for the safe box, empty
    # lines, other colours and places, no puff, text across the picture's edges, and windows that never open
    sizes = [(W, H), (W, 220), (300, 200), (240, 260), (W, H)]
    for n in range(44):
        w, h = sizes[n % len(sizes)]
        spec = flat(w, h) if n % 4 == 0 else weave(r, w, h, rows=(84, 232) if h == H and n % 7 != 5 else None)
        lines = [SEEDED_LINES[int(v)] for v in r.integers(0, len(SEEDED_LINES), 1 + n % 3)]
        t0 = float(r.uniform(0.0, 58.0))
        if n % 5 == 2:
            t0 = round(t0 * FPS) / FPS                    # a caption that starts on a frame: t == ts and t == tl happen
        t1 = t0 + float(r.choice([2.4, 1.25, 0.7, 0.3]))
        kw = dict(style='impact')
        if n % 3 == 1:
            kw['base'] = tuple(int(v) for v in r.integers(0, 256, 3))
        if n % 4 == 2:
            kw['hi'] = tuple(int(v) for v in r.integers(0, 256, 3))
        if n % 5 == 3:
            kw['alt'] = tuple(int(v) for v in r.integers(0, 256, 3))
        if n % 6 == 4:
            kw['puff'] = None if n % 12 == 4 else tuple(int(v) for v in r.integers(0, 256, 3))
        if n % 7 == 5:
            kw['y0'] = int(r.choice([96, 60, 130, h - 30, 470, -10]))
        if n % 8 == 6:
            kw['cx'] = int(r.choice([180, 20, w - 15, 120, 330]))
        stage = n % 11
        if stage < 8:                                     # a time inside the window, at a frame or between frames
            k = int(r.integers(0, len(lines)))
            ts = t0 + k * ui.IMPACT_STAGGER
            t = ts + [0.0, 0.03, 0.07, ui.IMPACT_FALL, 0.11, 0.16, 0.24, 0.5][stage]
            if stage == 3:
                t = ts + ui.IMPACT_FALL                   # exactly the landing time, as _draw_impact computes it
            caps = [(t0, t1, lines)]
        elif stage == 8:                                  # in the fade
            t, caps = t1 - float(r.uniform(0.0, 0.12)), [(t0, t1, lines)]
        elif stage == 9:                                  # a window that never opens (t1 <= t0), beside a live one
            t = t0 + 0.4
            caps = [(t0, t0, lines), (t0, t0 - 1.0, lines), (t0 + 0.2, t1 + 1.0, lines[:1])]
        else:                                             # two captions open at once
            t = t0 + 0.35
            caps = [(t0, t1, lines), (t0 + 0.1, t1 + 0.5, [SEEDED_LINES[int(r.integers(0, 12))]])]
        caption_case(g, ui, spec, float(t), caps, kw)
    # the punch is held inside the safe box: lines 298 and 310 px wide may grow to 316 px and no further, and a line
    # wider than the box does not grow at all
    for line in ('A LINE OF 25 CHARACTERS!!', '*A LINE* OF ~26 CHARACTERS~!!!', 'THIS LINE IS FAR TOO LONG TO FIT IN'):
        assert len(ui._plain(line)) in (25, 26, 35), line
        for du in (0.0, 0.02, 0.06):                      # at the landing itself, and 0.02 and 0.06 s after it
            caption_case(g, ui, weave(r, W, 220), 20.0 + ui.IMPACT_FALL + du, [(20.0, 22.0, [line])],
                         dict(style='impact'))
    # the fade: while it is at 0.999 or above the text replaces the picture, below that it is mixed in
    for back in (0.118, 0.117, 0.1, 0.03):
        caption_case(g, ui, weave(r, W, 220), 22.0 - back, [(20.0, 22.0, ['~FADE~ *OUT*', 'slowly'])],
                     dict(style='impact'))
    # no line is drawn above the top of the safe box (row 96): a caption placed there or higher is held at it while
    # it falls (y0 96), once it has landed (y0 60, y0 -10) and when the punch and the shake would lift it (y0 97)
    held = 0
    for y0, dt in ((96, 0.03), (96, 0.07), (60, 0.6), (60, 0.05), (-10, 0.4), (97, 0.105), (97, 0.12), (97, 0.14)):
        _, rows = caption_case(g, ui, weave(r, W, 220), 20.0 + dt, [(20.0, 22.0, ['*K.O.*', 'ok'])],
                               dict(style='impact', y0=y0))
        held += rows[0][1] == 96
    assert held >= 6, f'only {held} of the captions at the top of the safe box are held at row 96'
    # only dead windows: nothing may be drawn or noted
    for caps, t in (([(5.0, 5.0, ['*K.O.*'])], 5.0), ([(5.0, 4.0, ['*K.O.*'])], 4.5), ([(5.0, 4.0, ['*K.O.*'])], 5.0),
                    ([(5.0, 6.0, ['WINS!'])], 6.0), ([(5.0, 6.0, ['WINS!'])], 4.999), ([], 3.0)):
        changed, rows = caption_case(g, ui, weave(r, W, 220), t, caps, dict(style='impact'))
        assert changed == 0 and not rows, (caps, t)

    def landings_case(caps, **kw):
        ret = ui.impact_landings(caps, **kw)
        args = dict(caps=caps_json(caps), **{k: common.hexf(v) for k, v in kw.items()})
        g.add(kw_call('reelkit.ui.impact_landings(caps', kw), args,
              dict(returns=[[common.hexf(float(t)), int(k), int(c)] for t, k, c in ret]))

    landings_case(list(P.CAPTIONS))
    assert len(ui.impact_landings(P.CAPTIONS)) == 10
    landings_case([(1.0, 3.0, ['A', '*B*c', '~d e~']), (4.0, 4.0, ['NEVER']), (5.0, 5.05, ['TOO', 'SHORT']),
                   (6.0, 6.33, ['ONE', 'TWO', 'THREE']), (7.0, 6.0, ['BACKWARDS']), (8.0, 9.0, [])])
    landings_case([(1.0, 3.0, ['A', 'B', 'C']), (2.5, 2.9, ['x', 'y'])], stagger=0.5, fall=0.25)
    landings_case([(0.0, 1.0, ['A', 'B'])], stagger=0.0, fall=0.0)
    landings_case([(float(t), float(t) + 1.0, ['*LINE*', 'two']) for t in r.uniform(0, 60, 12)], fall=0.1)
    # a line that would land exactly as its caption ends does not land; one float earlier it does
    landings_case([(1.0, 1.25, ['EDGE']), (2.0, 2.2500000000000004, ['IN'])], fall=0.25)
    assert len(ui.impact_landings([(1.0, 1.25, ['EDGE']), (2.0, 2.2500000000000004, ['IN'])], fall=0.25)) == 1

    for lines in (['FLUTTER TONGUE'], ['15 CHARS HERE!!'], ['*NEXT MATCH-UP*', 'COMMENT THE'], ['TROMBONE', 'WINS!'],
                  [''], ['*~*~'], ['~21 CHARS~ *EXACTLY* HERE'], ['a', 'THIS LINE IS FAR TOO LONG TO FIT IN'],
                  ['x\u00d72 = 4\u00b0 14 ok'], ['\u00e9' * 15]):
        g.add('reelkit.ui.caption_scale(lines)', dict(lines=lines), dict(returns=ui.caption_scale(lines)))
    return g


# ================================================================ the fight HUD
def fighter_json(d):
    out = {}
    for k, v in d.items():
        if k in ('name', 'status'):
            out[k] = str(v)
        elif k in ('ko', 'win'):
            out[k] = bool(v)
        elif k in ('level', 'max_level'):
            out[k] = int(v)
        elif k == 'color':
            out[k] = [common.hexf(float(c)) for c in v]
        elif k in ('hp', 'hp_max', 'trail', 'status_k', 'flash', 't'):
            out[k] = None if v is None else common.hexf(float(v))
        else:
            raise KeyError(f'a fighter key this export does not know: {k}')
    return out


def rows_json(rows):
    out = []
    for row in rows:
        j = {}
        for k, v in row.items():
            if k in ('tag', 'name', 'state'):
                j[k] = str(v)
            elif k in ('glow', 'sweep'):
                j[k] = None if v is None else common.hexf(float(v))
            else:
                raise KeyError(f'an ability row key this export does not know: {k}')
        out.append(j)
    return out


def hud_case(g, ui, spec, left, right, kw, **more):
    img = canvas(spec)
    before = img.copy()
    ret, rows = noted(ui, lambda: ui.fight_hud(img, copy.deepcopy(left), copy.deepcopy(right), **copy.deepcopy(kw)))
    args = dict(canvas=spec, left=fighter_json(left), right=fighter_json(right), **more)
    for k, v in kw.items():
        if k == 'abilities':
            args[k] = None if v is None else [rows_json(v[0]), rows_json(v[1])]
        elif k == 'alpha':
            args[k] = common.hexf(float(v))
        elif k == 'ink':
            args[k] = list(v)
        else:
            assert k in ('x0', 'x1', 'y1', 'bar_h', 'center'), k
            args[k] = v
    returns = dict(top=int(ret['top']), bars=[[int(v) for v in b] for b in ret['bars']])
    if 'rows' in ret:
        returns['rows'] = [[[int(v) for v in b] for b in side] for side in ret['rows']]
    assert set(ret) == set(returns)
    g.add(kw_call('reelkit.ui.fight_hud(img, left, right', kw), args, dict(returns=returns, noted=rows), out=img)
    return int((img != before).any(2).sum())


class _WideNumpy:
    """numpy as reelkit.ui sees it, but with an empty_like that makes a float64 array: _ability_rows then keeps the
    pill's body colour in float64 instead of rounding it to float32 on the way. Only used to find cases that tell the
    two apart"""

    def __getattr__(self, name):
        return getattr(np, name)

    @staticmethod
    def empty_like(a):
        return np.empty(a.shape, np.float64)


def hud_pixels(ui, base, left, right, kw, wide=False):
    """the picture fight_hud draws on a copy of `base`, as it is or (wide) with the pill colour kept in float64"""
    img = base.copy()
    saved = ui.np
    try:
        if wide:
            ui.np = _WideNumpy()
        ui.fight_hud(img, copy.deepcopy(left), copy.deepcopy(right), **copy.deepcopy(kw))
    finally:
        ui.np = saved
        ui.reset_drawn()
    return img


HUD_NAMES = ('FLUTE', 'TROMBONE', 'PIANO', 'TUBA', 'CLARINET', 'OBOE', 'CELLO', 'HORN', 'BASS DRUM', 'x', '')
HUD_ABILITIES = ('FLUTTER TONGUE', 'SNAKE DANCE', 'PICCOLO', 'SAD TROMBONE', 'POWER SLIDE', 'PEDAL TONE', 'SUSTAIN',
                 '88 KEYS', 'x', 'NOT A TUBA')


def group_hud():
    from reelkit import ui
    g = Group('ui_hud', 'reelkit.ui.fight_hud: the states of the plan on mid-grey, the calls kit.picture makes for '
                        'the example episode, and seeded ones',
              constants=dict(ABIL_ROW=ui.ABIL_ROW, ABIL_HOT=list(ui.ABIL_HOT),
                             ABIL_GREY=[list(c) for c in ui.ABIL_GREY], AMBER=list(ui.AMBER), INK=list(ui.INK)),
              note=CANVAS_NOTE + 'left and right are the fighter dicts: a key that is absent was not passed (hp_max '
                   '100, trail none, level 1, max_level 3, status "", status_k 0, flash 0, ko and win false, t 0); '
                   'floats are float.hex() strings, trail null is trail=None. The keywords named in "call" were '
                   'passed (x0, x1, y1 default to the safe box, alpha 1, ink INK, bar_h 6, center "VS", '
                   'abilities); abilities is [left rows, right rows] of dicts with tag, name, state, glow, sweep '
                   '(absent: tag "", state "locked", glow 0, sweep none). expect.returns is the dict fight_hud '
                   'returned (top, bars, and rows when abilities were given); expect.noted what it appended to '
                   'reelkit.ui.DRAWN. Cases with "state" are made by hand (the plan\'s list first, then thresholds '
                   'and edges); cases with source "kit.picture" are the arguments kit.picture.render passed at that '
                   'frame of the example episode; the rest are seeded.')
    P, why, calls = real()
    r = rng(4)
    names, cols = list(P.NAMES), [np.array(c, float) for c in P.COL]

    def fighter(k, **kw):
        return dict(name=names[k], color=cols[k].copy(), **kw)

    def rows(k, *states):
        """the fighter's three real rows with (state, glow, sweep) each"""
        base = P.ability_rows(k, 0.0)
        assert [b['tag'] for b in base] == ['LV2', 'LV3', 'LOW']
        return [dict(tag=b['tag'], name=b['name'], state=s, glow=gl, sweep=sw)
                for b, (s, gl, sw) in zip(base, states)]

    lock, on = ('locked', 0.0, None), ('unlocked', 0.0, None)
    lit, half, sweep = ('unlocked', 1.0, None), ('unlocked', 0.4, None), ('unlocked', 0.0, 0.35)
    plan = [
        ('full HP', fighter(0, hp=100.0), fighter(1, hp=100.0), (lock, lock, lock), (lock, lock, lock)),
        ('damaged, with the trail', fighter(0, hp=62.5, trail=74.0, level=2), fighter(1, hp=81.0, trail=81.0, level=2),
         (on, lock, lock), (sweep, lock, lock)),
        ('low HP', fighter(0, hp=18.0, trail=21.5, level=3, t=31.4), fighter(1, hp=24.0, trail=24.0, level=3, t=31.4),
         (on, lit, lock), (half, on, sweep)),
        ('a flash', fighter(0, hp=55.0, trail=63.0, flash=0.7, level=2, t=20.1),
         fighter(1, hp=70.0, trail=70.0, flash=0.25, level=2, t=20.1), (lit, lock, lock), (on, on, lock)),
        ('K.O.', fighter(0, hp=0.0, trail=3.5, ko=True, level=3, t=50.1),
         fighter(1, hp=31.0, trail=31.0, level=3, t=50.1), (on, on, on), (on, on, lit)),
        ('the winner', fighter(0, hp=0.0, trail=0.0, ko=True, level=3, t=52.0),
         fighter(1, hp=31.0, trail=31.0, win=True, level=3, t=52.0), (on, on, on), (on, on, on)),
        ('level 1', fighter(0, hp=96.0, level=1), fighter(1, hp=91.0, level=1), (lock, lock, lock), (lock, lock, lock)),
        ('level 2', fighter(0, hp=80.0, level=2, status='FLUTTER TONGUE', status_k=1.0),
         fighter(1, hp=77.0, level=2, status='SAD TROMBONE', status_k=0.45), (lit, lock, lock), (half, lock, lock)),
        ('level 3', fighter(0, hp=44.0, level=3, status='SNAKE DANCE', status_k=0.005),
         fighter(1, hp=40.0, level=3, status='POWER SLIDE', status_k=0.9995), (on, sweep, lock), (on, lit, lock)),
    ]
    for state, left, right, ra, rb in plan:
        ab = (rows(0, *ra), rows(1, *rb))
        hud_case(g, ui, flat(), left, right, dict(abilities=ab), state=state + ', with ability rows')
        hud_case(g, ui, flat(), left, right, {}, state=state + ', without ability rows')

    # the real calls: what kit.picture.render passed at the chosen frames
    seen = dict(sweep_left=0, sweep_right=0, glow_part=0, glow_full=0, flash=0, ko=0, win=0, low=0, trail=0, locked=0)
    n = 0
    for f, got in calls.items():
        for name, a, kw in got:
            if name != 'fight_hud':
                continue
            left, right = a
            assert set(kw) == {'abilities'} and len(a) == 2, (f, kw)
            for side, F_ in enumerate((left, right)):
                assert isinstance(F_['color'], np.ndarray) and F_['color'].dtype == np.float64
                rws = kw['abilities'][side]
                seen['sweep_left' if side == 0 else 'sweep_right'] += any(x['sweep'] is not None for x in rws)
                seen['glow_part'] += any(0.0 < x['glow'] < 1.0 for x in rws)
                seen['glow_full'] += any(x['glow'] == 1.0 for x in rws)
                seen['locked'] += all(x['state'] == 'locked' for x in rws)
                seen['flash'] += F_['flash'] > 0
                seen['ko'] += bool(F_['ko'])
                seen['win'] += bool(F_['win'])
                seen['low'] += F_['hp'] <= 25.0 and not F_['ko']
                seen['trail'] += F_['trail'] > F_['hp']
            hud_case(g, ui, flat() if n % 4 == 0 else weave(r, rows=(436, 506)), left, right, kw, frame=f, why=why[f],
                     source='kit.picture')
            n += 1
    assert all(seen.values()) and n >= 20, f'the recorded frames miss a HUD state: {seen} in {n} calls'

    # seeded: every argument, in and out of its range
    for n in range(48):
        def one(side):
            hp_max = float(r.choice([100.0, 100.0, 80.0, 250.5]))
            hp = float(r.choice([r.uniform(0, hp_max), r.uniform(0, 0.25 * hp_max), hp_max, 0.0, hp_max + 9.0, -3.0,
                                 0.25 * hp_max, 1e-7, 0.5, 99.9999995]))
            color = np.round(r.uniform(0, 255, 3), int(r.integers(0, 3)))       # whole numbers, or 1 or 2 decimals
            d = dict(name=str(r.choice(HUD_NAMES)), color=tuple(float(v) for v in color), hp=hp)
            if r.random() < 0.5:
                d['hp_max'] = hp_max
            else:
                hp_max = 100.0
            if r.random() < 0.6:
                d['trail'] = None if r.random() < 0.2 else float(hp + r.uniform(-10, 25))
            if r.random() < 0.6:
                d['level'] = int(r.integers(0, 7))
            if r.random() < 0.3:
                d['max_level'] = int(r.integers(1, 6))
            if r.random() < 0.6:
                d['status'] = str(r.choice(['', 'FLUTTER TONGUE', 'SAD TROMBONE', 'x', 'A VERY LONG STATUS LINE']))
                d['status_k'] = float(r.choice([0.0, 0.005, 0.01, 0.0100001, 0.3, 0.77, 0.9989999, 0.999, 1.0]))
            if r.random() < 0.5:
                d['flash'] = float(r.choice([0.0, 1.0, r.uniform(0, 1), 0.5]))
            if r.random() < 0.3:
                d['ko'] = bool(r.random() < 0.7)
            if r.random() < 0.3:
                d['win'] = bool(r.random() < 0.7)
            if r.random() < 0.7:
                d['t'] = float(r.choice([r.uniform(0, 61), round(float(r.uniform(0, 61)) * FPS) / FPS, 0.125, 30.5]))
            return d

        def ability_rows():
            out = []
            for _ in range(int(r.integers(0, 5))):
                row = dict(name=str(r.choice(HUD_ABILITIES)))
                if r.random() < 0.85:
                    row['tag'] = str(r.choice(['LV2', 'LV3', 'LOW', '', 'x']))
                if r.random() < 0.85:
                    row['state'] = str(r.choice(['locked', 'unlocked', 'unlocked', 'unlocked']))
                if r.random() < 0.8:
                    row['glow'] = float(r.choice([0.0, 0.003, 0.004, 0.0040001, 0.5, 1.0, 1.3, -0.2,
                                                  r.uniform(0, 1)]))
                if r.random() < 0.7:
                    sweeps = [0.0, 0.3, 0.999, 1.0, -0.1, r.uniform(0, 1), r.uniform(0, 1)]
                    row['sweep'] = None if r.random() < 0.3 else float(r.choice(sweeps))
                out.append(row)
            return out

        kw = {}
        if n % 3:
            kw['abilities'] = (ability_rows(), ability_rows())
        elif n % 9 == 3:
            kw['abilities'] = None
        if n % 5 == 1:
            kw['alpha'] = float(r.choice([1.0, 0.999, 0.998, 0.5, 0.25, 0.0]))
        if n % 6 == 2:
            kw['ink'] = tuple(int(v) for v in r.integers(0, 256, 3))
        if n % 7 == 3:
            kw['bar_h'] = int(r.integers(3, 10))
        if n % 4 == 1:
            kw['center'] = str(r.choice(['VS', '', 'X', 'ROUND 2']))
        w, h, band = W, H, (428, 506)
        if n % 8 == 4:                                    # its own place, on a picture of another size
            w, h, band = 320, 120, None
            kw.update(x0=int(r.integers(8, 30)), x1=int(r.integers(250, 310)), y1=int(r.integers(96, 112)))
        elif n % 8 == 6:
            kw.update(y1=int(r.integers(200, 290)))       # above the button column: x1 defaults to the safe box edge
            band = (kw['y1'] - 68, kw['y1'] + 10)
        elif n % 16 == 7:
            kw.update(x0=40, x1=330)
        hud_case(g, ui, flat(w, h) if n % 4 == 0 else weave(r, w, h, rows=band), one(0), one(1), kw)

    hud_case(g, ui, flat(), fighter(0, hp=100.0), fighter(1, hp=64.0, trail=70.0), dict(abilities=([], [])),
             state='abilities given, but no rows')

    # thresholds. The status line: mixed in below 0.999, not drawn at 0.01 or less; it is drawn when it ends exactly
    # at the bar's inner end (15 letters here) and left out when it is one letter longer. The whole HUD: mixed in
    # below 0.999
    for sk in (0.999, 0.9989999, 0.0100001, 0.01):
        hud_case(g, ui, weave(r, rows=(436, 506)), fighter(0, hp=80.0, status='FLUTTER TONGUE', status_k=sk),
                 fighter(1, hp=77.0, level=2, status='SAD TROMBONE', status_k=sk), {}, state=f'status_k {sk}')
    for status in ('FIFTEEN LETTERS', 'SIXTEEN LETTERS!'):
        assert len(status) in (15, 16)
        hud_case(g, ui, weave(r, rows=(436, 506)), fighter(0, hp=80.0, status=status, status_k=1.0),
                 fighter(1, hp=77.0, level=3, status=status, status_k=1.0), dict(x0=24, x1=285),
                 state=f'a status line of {len(status)} letters between the pips and the middle')
    for alpha in (0.999, 0.9989999):
        hud_case(g, ui, weave(r, rows=(436, 506)), fighter(0, hp=80.0), fighter(1, hp=77.0),
                 dict(alpha=alpha, abilities=(rows(0, on, lit, lock), rows(1, half, on, sweep))),
                 state=f'alpha {alpha}')

    # a sweep runs from 0 up to, not including, 1; a locked row has neither glow nor sweep, whatever it is given
    ends = (rows(0, ('unlocked', 0.0, 1.0), ('unlocked', 0.0, 0.0), ('locked', 1.0, 0.5)),
            rows(1, ('unlocked', 0.5, 1.0), ('unlocked', 0.0, 0.999), ('locked', 0.7, 0.2)))
    hud_case(g, ui, weave(r, rows=(436, 506)), fighter(0, hp=80.0), fighter(1, hp=77.0), dict(abilities=ends),
             state='sweeps of exactly 1 and 0, and locked rows given a glow and a sweep')

    # the pill's body colour is rounded to float32 on its way into the picture (np.empty_like of a float32 array).
    # Glows at which that rounding changes a byte, found by drawing the HUD both ways
    spec, found = weave(r, rows=(436, 506)), 0
    base, left, right = canvas(spec), fighter(0, hp=70.0), fighter(1, hp=60.0)
    for step in range(1, 4000):
        glow = step / 4000
        kw = dict(abilities=(rows(0, ('unlocked', glow, None), on, lock), rows(1, on, ('unlocked', glow, None), lock)))
        if not np.array_equal(hud_pixels(ui, base, left, right, kw), hud_pixels(ui, base, left, right, kw, wide=True)):
            hud_case(g, ui, spec, left, right, kw, state=f'glow {glow}: the float32 rounding of the pill colour shows')
            found += 1
            if found == 3:
                break
    assert found == 3, f'only {found} glows show the float32 rounding of the pill colour'

    # places no caller should ask for, drawn as Python's slices and indices draw them: a HUD whose backing starts
    # left of the picture (the slice start -2 counts from the right edge; the pill and the sweep's haze of a row
    # without a tag are cut at the left edge, the haze around a negative centre), a winner's frame at column -1, and
    # a HUD against the right and bottom edges (the pills are cut at the right edge)
    edge_rows = [dict(tag='', name='PICCOLO', state='unlocked', glow=1.0, sweep=0.03),
                 dict(tag='', name='SNAKE DANCE', state='unlocked', glow=0.6, sweep=0.0),
                 dict(tag='LOW', name='FLUTTER TONGUE', state='unlocked', glow=0.0, sweep=0.5)]
    odd = dict(abilities=(edge_rows, rows(1, on, lit, lit)))
    hud_case(g, ui, weave(r, 320, 120), fighter(0, hp=50.0, win=True), fighter(1, hp=20.0, t=3.3),
             dict(x0=2, x1=300, y1=100, **odd), state='x0 = 2: the backing slice starts at -2')
    hud_case(g, ui, weave(r, 320, 120), fighter(0, hp=50.0, win=True), fighter(1, hp=20.0, t=3.3),
             dict(x0=1, x1=300, y1=100), state='x0 = 1: the gold frame is at column -1')
    hud_case(g, ui, weave(r, 320, 120), fighter(0, hp=50.0), fighter(1, hp=20.0, win=True),
             dict(x0=30, x1=318, y1=117, **odd), state='the right edge and the bottom, as far as indices allow')
    return g


def run():
    total = sum(make().write() for make in (group_label, group_puff, group_captions, group_hud))
    print(f'kit_ui: {total / 1e3:.1f} kB -> {common.rel(OUT)}')


EXPORTS = {'kit_ui': run}

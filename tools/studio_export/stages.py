"""stages: the buffers of one frame after each stage of kit.picture.render, for finding where a frame first differs.

    python -m tools.studio_export stages 150 600 1500

writes studio/reference/.cache/stages/flute_vs_trombone/f<frame>.npz + f<frame>.json (git-ignored, not in the
manifest): for every stage in STAGES, in pipeline order, the colour buffer (and the depth buffer where the stage has
one) exactly as the renderer holds it at that point, in its own dtype. Nothing in the kit is edited: `tap` wraps the
functions render calls (and three methods of reelkit.raster.Frame) for the length of one render and puts them back.

Every dump is checked before it is kept: each stage fired once and in STAGES order, the last stage is the frame
render returns, the same frame drawn without the taps is identical, and every array reads back without pickle."""
import contextlib
import os
import sys
import time

import numpy as np

from . import common
from . import frames as F

OUT = os.path.join(common.CACHE, 'stages', F.EPISODE)

# (name, what the buffer is, the hook that captures it), in the order render reaches them
STAGES = (
    ('floor', 'the arena floor, lit per pixel and dithered: the float buffers handed to Frame.set_array',
     'the 1st Frame.set_array call made by render'),
    ('arena', 'after the stands, the crowd and the LED boards (drawn on the PIL canvas)', 'the return of arena()'),
    ('skirt', "after the ring platform's skirt", 'the 2nd Frame.array call made by render'),
    ('canvas', "the canvas, lit per pixel (spot, shadows, corner lights, the looks' decals) and dithered, plus the "
               "looks' canvas overlays: the float buffers handed to Frame.set_array",
     'the 2nd Frame.set_array call made by render'),
    ('painter', "after the painter pass: back post and ropes, the balls, the looks' items, the side posts, front "
                'ropes and post', "the arguments of the first look's over_ring"),
    ('over_ring', "after both looks' over-ring effects and the K.O. shards", 'the arguments of sparks()'),
    ('sparks', 'after the hit sparks', 'the return of sparks()'),
    ('particles', 'after the reelkit.fx particles: the float buffers handed to Frame.set_array (the picture the '
                  'lens sees is this, clipped and cut to whole numbers)',
     'the last Frame.set_array call made by render'),
    ('coc', 'the circle-of-confusion radius per pixel (px)', 'the return of coc_map()'),
    ('lens', 'after the depth of field', 'the return of dof()'),
    ('shimmer', "after the looks' heat shimmers", 'the return of shimmers()'),
    ('glows', 'after the glow sprites (corner lights, balls, the looks, fresh hits, the K.O.)',
     'the return of the first apply_glows call made by render'),
    ('beam', "after the spot's beam and the can at the top of frame, clipped to 0..255", 'the argument of bloom()'),
    ('bloom', 'after the bloom', 'the return of bloom()'),
    ('grade', 'after the grade', 'the return of grade()'),
    ('scrim', 'after the vignette and the top scrim, clipped to 0..255',
     "the argument of the first look's after_grade"),
    ('post_fx', "after both looks' crisp overlays and the post effects (fringe, ability moments, bursts)",
     'the return of post_fx()'),
    ('countdown', 'the picture cut to uint8, with the count-in drawn', 'the image after countdown() returns'),
    ('hud', 'after the HP numbers, the fight HUD and the captions', 'the argument of title.draw'),
    ('title', 'after the VS title card: the frame render returns', 'the return of title.draw'),
)
NAMES = tuple(s[0] for s in STAGES)


class Tap:
    """what one render handed over: `got[stage] = {role: array}` and `order`, the stages in the order they fired"""

    def __init__(self):
        self.got, self.order = {}, []

    def put(self, stage, **arrays):
        if stage in self.got:
            # render calls Frame.set_array once per ball before the call that ends the particles: the last one counts
            assert stage == 'particles', f'stage {stage} fired twice'
            self.order.remove(stage)
        self.order.append(stage)
        self.got[stage] = {k: np.array(v) for k, v in arrays.items()}      # copies: the renderer works in place


@contextlib.contextmanager
def tap(P):
    """wrap the stage boundaries of P.render; yields the Tap that fills during the next render; restores on exit"""
    from reelkit.raster import Frame
    TT, rec = P.TT, Tap()
    render_code = P.render.__code__
    count = dict(set_array=0, array=0, glows=0)
    undo = []

    def swap(obj, name, make):
        old = getattr(obj, name)
        had = name in vars(obj)
        undo.append((obj, name, old, had))
        setattr(obj, name, make(old))

    def by_render():
        return sys._getframe(2).f_code is render_code

    def frame_set_array(old):
        def set_array(self, arr, depth=None):
            if by_render():
                count['set_array'] += 1
                stage = {1: 'floor', 2: 'canvas'}.get(count['set_array'], 'particles')
                rec.put(stage, rgb=arr, depth=depth)
            return old(self, arr, depth)
        return set_array

    def frame_array(old):
        def array(self):
            if by_render():
                count['array'] += 1
                if count['array'] == 2:
                    rec.put('skirt', rgb=np.asarray(self.pil), depth=np.asarray(self.dep))
            return old(self)
        return array

    def after(stage, role='rgb', frame_arg=None):
        def make(old):
            def call(*a, **kw):
                out = old(*a, **kw)
                if frame_arg is None:
                    rec.put(stage, **{role: out})
                else:                                     # drawn in place on an argument
                    x = a[frame_arg]
                    rec.put(stage, **(dict(rgb=np.asarray(x.pil), depth=np.asarray(x.dep)) if hasattr(x, 'pil')
                                      else {role: x}))
                return out
            return call
        return make

    def sparks(old):
        def call(arr, cam, t):
            rec.put('over_ring', rgb=arr)
            out = old(arr, cam, t)
            rec.put('sparks', rgb=arr)
            return out
        return call

    def glows(old):
        def call(img, gl):
            out = old(img, gl)
            if by_render():
                count['glows'] += 1
                if count['glows'] == 1:
                    rec.put('glows', rgb=out)
            return out
        return call

    def bloom(old):
        def call(img, *a, **kw):
            rec.put('beam', rgb=img)
            out = old(img, *a, **kw)
            rec.put('bloom', rgb=out)
            return out
        return call

    def over_ring(old):
        def call(Fc, arr, depth):
            rec.put('painter', rgb=arr, depth=depth)
            return old(Fc, arr, depth)
        return call

    def after_grade(old):
        def call(img, Fc, drawn):
            rec.put('scrim', rgb=img)
            return old(img, Fc, drawn)
        return call

    def title(old):
        def call(img, t):
            rec.put('hud', rgb=img)
            out = old(img, t)
            rec.put('title', rgb=out)
            return out
        return call

    try:
        swap(Frame, 'set_array', frame_set_array)
        swap(Frame, 'array', frame_array)
        swap(P, 'arena', after('arena', frame_arg=0))
        swap(P.LOOKS[0], 'over_ring', over_ring)
        swap(P, 'sparks', sparks)
        swap(P, 'coc_map', after('coc', role='map'))
        swap(P, 'dof', after('lens'))
        swap(P, 'shimmers', after('shimmer'))
        swap(P, 'apply_glows', glows)
        swap(P, 'bloom', bloom)
        swap(P, 'grade', after('grade'))
        swap(P.LOOKS[0], 'after_grade', after_grade)
        swap(P, 'post_fx', after('post_fx'))
        swap(P, 'countdown', after('countdown', frame_arg=0))
        swap(TT, 'draw', title)
        yield rec
    finally:
        for obj, name, old, had in reversed(undo):
            if had:
                setattr(obj, name, old)
            else:
                delattr(obj, name)                        # (a bound method we shadowed on an instance)


def check(rec, img):
    """the dump is whole: every stage fired, in pipeline order, and the last one is the frame"""
    assert rec.order == list(NAMES), f'stages fired: {rec.order}; the pipeline has: {list(NAMES)}'
    for name in NAMES:
        for role, a in rec.got[name].items():
            assert a.dtype.str in common.NPZ_DTYPES, (name, role, a.dtype)
            want = (F.H, F.W) if role in ('depth', 'map') else (F.H, F.W, 3)
            assert a.shape == want, (name, role, a.shape)
    assert np.array_equal(rec.got['title']['rgb'], img), 'the last stage is not the frame render returned'


def capture(P, f):
    """(Tap, frame): render frame f with the stage taps on"""
    with tap(P) as rec:
        img = P.render(f)
    check(rec, img)
    return rec, img


def dump(P, f):
    t0 = time.time()
    rec, img = capture(P, f)
    assert np.array_equal(P.render(f), img), f'frame {f}: the taps changed the picture'
    arrays, cases = {}, []
    for j, (name, what, hook) in enumerate(STAGES):
        got = rec.got[name]
        for role, a in got.items():
            arrays[f'{name}__{role}'] = a
        cases.append(dict(id=name, call=hook, args=dict(frame=f, stage=j), what=what,
                          arrays={role: f'{name}__{role}' for role in got},
                          dtype={role: a.dtype.str for role, a in got.items()}))
    base = os.path.join(OUT, f'f{f:04d}')
    common.write_npz(base + '.npz', arrays, register=False)
    doc = dict(
        about=f'kit.picture.render({f}): the buffers after each stage, in pipeline order (a local debugging dump, '
              f'not a reference file)',
        tool='stages', episode=F.EPISODE, frame=f, t=f / F.FPS, size=[F.W, F.H], stages=list(NAMES),
        note='rgb arrays are (640, 360, 3), depth and map arrays (640, 360); each keeps the dtype the renderer '
             'holds it in (float32 or float64 buffers in 0..255 that are not yet clipped unless the stage says so, '
             'uint8 where the renderer has already cut to bytes). Frame.set_array clips to 0..255 and truncates '
             'to uint8 for the PIL canvas, so the next drawing stage starts from whole numbers.',
        cases=cases)
    common.write_json(base + '.json', doc, register=False)
    z = np.load(base + '.npz', allow_pickle=False)
    assert sorted(z.files) == sorted(arrays)
    for k, a in arrays.items():
        b = z[k]
        assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), k
    print(f"stages: {common.rel(base + '.npz')}  {os.path.getsize(base + '.npz') / 1e6:.1f} MB  "
          f"{len(NAMES)} stages  {time.time() - t0:.1f} s")


def run(args=()):
    """stages <frame>...: dump those frames. With no frame named there is nothing to do (it is no reference export)"""
    usage = 'python -m tools.studio_export stages 150 600 1500'
    if not args:
        print(f'stages: no frames named, nothing dumped ({usage})')
        return
    try:
        frames = [int(a) for a in args]
    except ValueError:
        frames = [-1]
    if any(not 0 <= f < F.NF for f in frames):
        raise SystemExit(f'stages: name the frames to dump as numbers 0..{F.NF - 1}: {usage}')
    P = F.picture('stages')
    for f in frames:
        dump(P, f)
    print('stages: ' + ', '.join(NAMES))


EXPORTS = {'stages': run}
TAKES_ARGS = ('stages',)            # the entry point hands this export the rest of the command line; `all` skips it

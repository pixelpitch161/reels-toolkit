"""kit_fx: what reelkit.fx gives (the effects kit: flares, impact bursts, camera shake, heat shimmer, chromatic
fringing, particles, floor marks, ability moments), for the port in bf-picture.

studio/reference/picture_kit/ (every .npz has a sidecar .json: vectors-convention.md):

    fx_timing.npz     env_ad; FlashBudget.assign; Shake.offset; Flares.at, tint and apply; Moments.env, active and
                      dim_k; Marks.live
    fx_bursts.npz     starburst on float32 and float64 pictures (bright, dark, hollow, across the edges);
                      Bursts.draw and Bursts.bright_times
    fx_warp.npz       ellipse_of, shimmer_world, shimmer and fringe
    fx_particles.npz  Particles: the particles a list of emit calls makes (every draw of the generator), alive,
                      _pos and draw
    fx_marks.npz      Marks.apply for every kind of mark, with and without a dither
    fx_moments.npz    Moments.apply_dim and Moments.speed_lines

Three kinds of case, told apart by `args.source`: "kit.picture" holds what the example episode really does (the
lists of add and emit calls its effects are built from, and the arguments kit.picture.render passed at chosen
frames, recorded by wrapping the functions); "plan" is the plan's script of 20 add or emit calls looked at 6 times;
"seeded" are edges and thresholds.

The objects with a history (Flares, Bursts, Shake, Particles, Marks, Moments) are described by a script: the list
of their add or emit calls in the order they were made, the arguments by name; an argument that is not there has
Python's default. Scripts are in the sidecar's top-level `scripts`; a case names its script in `args.script`.

A case that draws holds no picture to draw on: `args.canvas` says how to make it (`canvas()` below). What comes
back is stored as the part that differs from a baseline (`expect.box` and the array `out`): see RESULT_NOTE. Floats
that Rust must match are float.hex() strings or float64 arrays."""
import contextlib
import copy
import inspect
import math
import os

import numpy as np

from . import common
from .kit_renderer_vectors import cam_args, cam_of, hx

OUT = common.ref('picture_kit')
SEED = 20261121
W, H, FPS, NF = 360, 640, 30, 1830
CANVAS_NOTE = ('args.canvas makes the picture, w x h x 3: bytes first (with only "flat", that colour everywhere; '
               'with "mul" and "add", pixel i, counted row by row from the top left, has the channels '
               '(mul[c] * i + add[c]) % 256), then, in float64, byte * gain + bias (when "gain" is there; both '
               'float.hex()), then the cast to "dtype" (float32 or float64). ')
RESULT_NOTE = ('The result: expect.returned is "img" when Python returned the very array it was given (in place, '
               'or untouched) and "new" when it made one; expect.dtype is the dtype of what came back. The picture '
               'is stored as its difference from a baseline: expect.baseline is "canvas" (the picture as made) or '
               '"bytes" (np.clip(canvas, 0, 255).astype(np.uint8), which is what a drawing call that draws starts '
               'from; one that draws nothing returns the picture itself, and its baseline is "canvas"); '
               'expect.box [x0, y0, x1, y1] bounds the pixels that differ from the baseline (null: none do) and '
               '"out" is that part of the result, stored as uint8 when every value in it is a whole number from 0 '
               'to 255 (expect.stored says which dtype). Outside the box the result equals the baseline. ')
CAM_NOTE = ('args.cam holds a reelkit.camera.Cam by its fields: pos, M (rows right, up, fwd), f, cx, cy, near, '
            'target. ')


# ================================================================ the files
class Group:
    """one vector file being filled: the cases (sidecar rows), their arrays (npz members) and the scripts"""

    def __init__(self, name, about, **top):
        self.name, self.about, self.top = name, about, top
        self.cases, self.arrays, self.scripts = [], {}, {}

    def script(self, name, **what):
        if name not in self.scripts:
            self.scripts[name] = what
        return name

    def add(self, call, args, expect=None, **arrays):
        cid = f'c{len(self.cases):03d}'
        names = {}
        for role, a in arrays.items():
            role = role.rstrip('_')
            assert isinstance(a, np.ndarray) and a.dtype.str in common.NPZ_DTYPES, (self.name, cid, role, type(a))
            self.arrays[f'{cid}__{role}'] = np.ascontiguousarray(a)
            names[role] = f'{cid}__{role}'
        case = dict(id=cid, call=call, args=args, arrays=names)
        if expect is not None:
            case['expect'] = expect
        self.cases.append(case)
        return cid

    def write(self):
        from . import manifest
        npz, js = os.path.join(OUT, self.name + '.npz'), os.path.join(OUT, self.name + '.json')
        common.write_npz(npz, self.arrays, register=False)
        common.write_json(js, dict(about=self.about, tool='kit_fx', **self.top, scripts=self.scripts,
                                   cases=self.cases), register=False)
        with np.load(npz, allow_pickle=False) as z:
            named = {m for c in self.cases for m in c['arrays'].values()}
            assert set(z.files) == set(self.arrays) == named, f'{self.name}: members and sidecar disagree'
            for k, a in self.arrays.items():
                b = z[k]
                assert b.dtype == a.dtype and b.shape == a.shape and b.tobytes() == a.tobytes(), (self.name, k)
        manifest.add(npz, tool=__name__)
        manifest.add(js, tool=__name__)
        size = os.path.getsize(npz) + os.path.getsize(js)
        calls = {}
        for c in self.cases:
            calls[c['call']] = calls.get(c['call'], 0) + 1
        print(f'kit_fx: {self.name:13s} {len(self.cases):4d} cases  {size / 1e3:8.1f} kB')
        for call, n in calls.items():
            print(f'            {n:4d}  {call}')
        return size


def rng(*key):
    return np.random.default_rng([SEED, *key])


def f64(values):
    return np.array([float(v) for v in values], np.float64)


def i64(values):
    return np.array([int(v) for v in values], np.int64)


def rows(values, width):
    return np.asarray(values, np.float64).reshape(-1, width)


# ================================================================ pictures
def flat(rgb, dtype='float64', w=W, h=H, gain=None, bias=0.0):
    spec = dict(w=w, h=h, flat=[int(v) for v in rgb], dtype=dtype)
    if gain is not None:
        spec.update(gain=common.hexf(float(gain)), bias=common.hexf(float(bias)))
    return spec


def weave(r, dtype='float64', w=W, h=H, gain=None, bias=0.0):
    spec = dict(w=w, h=h, mul=[int(v) * 2 + 1 for v in r.integers(0, 128, 3)],
                add=[int(v) for v in r.integers(0, 256, 3)], dtype=dtype)
    if gain is not None:
        spec.update(gain=common.hexf(float(gain)), bias=common.hexf(float(bias)))
    return spec


def canvas(spec):
    """the float picture a spec stands for (CANVAS_NOTE)"""
    w, h = spec['w'], spec['h']
    if 'mul' in spec:
        i = np.arange(w * h, dtype=np.int64).reshape(h, w)
        px = np.stack([(spec['mul'][c] * i + spec['add'][c]) % 256 for c in range(3)], -1).astype(np.float64)
    else:
        px = np.empty((h, w, 3), np.float64)
        px[:] = spec['flat']
    if 'gain' in spec:
        px = px * float.fromhex(spec['gain']) + float.fromhex(spec['bias'])
    return np.ascontiguousarray(px.astype({'float32': np.float32, 'float64': np.float64}[spec['dtype']]))


def result(spec, img, got, baseline):
    """(expect, arrays) for a picture call: `img` is the array handed in, `got` what came back (RESULT_NOTE)"""
    assert got.dtype in (np.float32, np.float64) and got.shape == img.shape, (got.dtype, got.shape)
    base = canvas(spec)
    if got is img and baseline == 'bytes':
        baseline = 'canvas'                               # nothing was drawn: the picture itself came back
    if baseline == 'bytes':
        base = np.clip(base, 0, 255).astype(np.uint8)
    expect = dict(returned='img' if got is img else 'new', dtype=got.dtype.name, baseline=baseline, box=None)
    diff = np.any(base != got, axis=-1)
    if not diff.any():
        return expect, {}
    ys, xs = np.nonzero(diff)
    x0, y0, x1, y1 = int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1
    crop = np.ascontiguousarray(got[y0:y1, x0:x1])
    as_bytes = crop.astype(np.uint8)
    if np.array_equal(as_bytes.astype(crop.dtype), crop):
        crop = as_bytes
    expect.update(box=[x0, y0, x1, y1], stored=crop.dtype.name)
    return expect, dict(out=crop)


@contextlib.contextmanager
def frame_size(fx, w, h):
    """reelkit.fx with another picture size (its W and H are module globals; the file is not edited)"""
    old = fx.W, fx.H
    fx.W, fx.H = w, h
    try:
        yield
    finally:
        fx.W, fx.H = old


def cam_fields(cam):
    assert cam.M.dtype == np.float64 and cam.M.shape == (3, 3)
    return dict(pos=hx(cam.pos), M=hx(cam.M), f=hx(cam.f), cx=hx(cam.cx), cy=hx(cam.cy), near=hx(cam.near),
                target=hx(cam.target))


def cam_from(fields):
    """a Cam with exactly these fields (what Rust builds from args.cam)"""
    from reelkit.camera import Cam
    cam = Cam(np.zeros(3), 10.0, 0.0, 30.0, 40.0)
    un = lambda v: np.array([[float.fromhex(x) for x in row] for row in v]) if isinstance(v[0], list) \
        else np.array([float.fromhex(x) for x in v])                                       # noqa: E731
    cam.pos, cam.M, cam.target = un(fields['pos']), un(fields['M']), un(fields['target'])
    cam.f, cam.cx, cam.cy, cam.near = (float.fromhex(fields[k]) for k in ('f', 'cx', 'cy', 'near'))
    return cam


RING = cam_args((0.12, 0.25, 0.0), 27.5, 0.0, 58.0, 36.0, 288.0)          # the ring's first camera key
RING_B = cam_args((0.12, 0.25, 0.0), 27.4, 0.7, 57.7, 36.0, 288.0)        # a later key (yaw not 0)
NEAR_CAM = cam_args((0.0, 0.5, 0.0), 6.0, 25.0, 35.0, 50.0, 320.0)        # close in: big particles, the near plane


# ================================================================ scripts: the add / emit calls of an object
def bound(fn, args, kwargs):
    """the arguments of a recorded call by name (self left out); what the caller left out is left out here"""
    b = inspect.signature(fn).bind(None, *args, **kwargs)
    out = dict(b.arguments)
    out.pop('self')
    return out


def js_call(a):
    """a bound call as JSON: floats as float.hex(), arrays as lists, ints, bools, strings and None as they are"""
    return {k: hx(v.tolist() if isinstance(v, np.ndarray) else v) for k, v in a.items()}


def py_call(a):
    """the arguments back from JSON, as Python values (hex strings are floats; lists of them tuples)"""
    def un(v):
        if isinstance(v, str) and v[:2] in ('0x', '-0') and 'p' in v:
            return float.fromhex(v)
        if isinstance(v, list):
            return tuple(un(x) for x in v)
        return v
    return {k: un(v) for k, v in a.items()}


def build(make, method, calls):
    """an object from `make()` after the script's calls"""
    obj = make()
    for a in calls:
        a = py_call(a)
        if 'key' in a and 'rgb' in a:
            a['rgb'] = np.array(a['rgb'], float)
        getattr(obj, method)(**a)
    return obj


_REAL = {}


def real():
    """the example episode's effects: the scripts kit.picture builds them from, and the calls its render makes at
    chosen frames. dict(P, scripts={name: [calls]}, frames={frame: why}, calls={frame: [(name, info)]})"""
    if _REAL:
        return _REAL
    from reelkit import fx
    from . import frames as F
    P = F.picture('kit_fx')
    # the scripts: _fx_build run once more into fresh objects whose add / emit calls are recorded
    rec = {n: [] for n in ('flares', 'bursts', 'particles', 'marks', 'moments')}

    def recording(cls, method, name):
        class Rec(cls):
            pass

        def inner(self, *a, **kw):
            rec[name].append(js_call(bound(getattr(cls, method), copy.deepcopy(a), copy.deepcopy(kw))))
            return getattr(cls, method)(self, *a, **kw)
        setattr(Rec, method, inner)
        return Rec

    old = {n: getattr(P, n) for n in ('FL', 'BU', 'PA', 'MK', 'MO')}
    try:
        P.FL = recording(fx.Flares, 'add', 'flares')(max_n=6)
        P.BU = recording(fx.Bursts, 'add', 'bursts')()
        P.PA = recording(fx.Particles, 'emit', 'particles')(seed=P.T.FX_SEED)
        P.MK = recording(fx.Marks, 'add', 'marks')()
        P.MO = recording(fx.Moments, 'add', 'moments')()
        P._fx_build()
        again = {n: getattr(P, n) for n in old}
    finally:
        for n, v in old.items():
            setattr(P, n, v)
    A, B = old['PA']._arrays(), again['PA']._arrays()
    assert all(np.array_equal(A[k], B[k]) for k in A), 'the rebuilt particles are not the episode\'s'
    for n in ('FL', 'MK', 'MO'):
        old[n].tl._sort(), again[n].tl._sort()
        assert old[n].tl.t0 == again[n].tl.t0, n
    assert [t for t, _ in old['BU']._ev] == [t for t, _ in again['BU']._ev]
    assert P.T.FX_SEED == 41 and len(rec['particles']) > 100
    P.SHK.tl._sort()
    rec['shake'] = [dict(t0=hx(t), px=hx(px)) for t, px in zip(P.SHK.tl.t0, P.SHK.tl.items)]

    # the frames, and what render asks of reelkit.fx on them
    why = real_frames(P)
    seen, now = {}, [None]

    def note(name, **info):
        seen[now[0]].append((name, info))

    def w_fringe(img, cx, cy, r, px=2.0, k=1.0):
        assert img.dtype == np.float64 and type(cx) is np.float64 and type(cy) is np.float64
        note('fringe', cx=float(cx), cy=float(cy), r=r, px=px, k=float(k))
        return saved['fringe'](img, cx, cy, r, px, k)

    def w_shimmer(img, cam, c, r, width, amp, t=0.0, k=1.0, seed=0):
        note('shimmer_world', dtype=img.dtype.name, cam=cam_fields(cam), c=[float(v) for v in c], r=float(r),
             width=float(width), amp=float(amp), t=float(t), k=float(k), seed=int(seed))
        return saved['shimmer_world'](img, cam, c, r, width, amp, t, k, seed)

    def w_dim(self, img, t, keep=()):
        assert img.dtype == np.float64 and all(type(x) is np.float64 and type(y) is np.float64 for x, y, _ in keep)
        note('apply_dim', t=float(t), keep=[[float(v) for v in q] for q in keep], k=self.dim_k(t))
        return saved['apply_dim'](self, img, t, keep)

    def w_lines(self, img, t, key, x, y, r, direction=(0.0, 1.0), n=16, length=(16, 46), seed=0):
        out = saved['speed_lines'](self, img, t, key, x, y, r, direction, n, length, seed)
        note('speed_lines', dtype=img.dtype.name, t=float(t), key=int(key), x=float(x), y=float(y), r=float(r),
             direction=[float(v) for v in direction], drew=out is not img)
        return out

    def w_bursts(self, img, cam, t):
        note('Bursts.draw', dtype=img.dtype.name, cam=cam_fields(cam), t=float(t),
             live=sum(1 for t0, _ in self.tl.alive(t) if int((t - t0) * FPS + 1e-6) < self.frames))
        return saved['Bursts.draw'](self, img, cam, t)

    def w_particles(self, img, cam, t, light=None):
        assert img.dtype == np.float32 and light is not None
        note('Particles.draw', cam=cam_fields(cam), t=float(t), live=len(self.alive(t)[1]))
        return saved['Particles.draw'](self, img, cam, t, light)

    def w_marks(self, alb, U, V, t, dither=None):
        assert alb.dtype == np.float64 and U.dtype == np.float64 and dither is not None and dither.dtype == np.float64
        note('Marks.apply', t=float(t), live=len(self.live(t)), points=len(U))
        return saved['Marks.apply'](self, alb, U, V, t, dither)

    saved = {'fringe': fx.fringe, 'shimmer_world': fx.shimmer_world, 'apply_dim': fx.Moments.apply_dim,
             'speed_lines': fx.Moments.speed_lines, 'Bursts.draw': fx.Bursts.draw,
             'Particles.draw': fx.Particles.draw, 'Marks.apply': fx.Marks.apply}
    try:
        fx.fringe, fx.shimmer_world = w_fringe, w_shimmer
        fx.Moments.apply_dim, fx.Moments.speed_lines = w_dim, w_lines
        fx.Bursts.draw, fx.Particles.draw, fx.Marks.apply = w_bursts, w_particles, w_marks
        for f in why:
            now[0] = f
            seen[f] = []
            P.render(f)
    finally:
        fx.fringe, fx.shimmer_world = saved['fringe'], saved['shimmer_world']
        fx.Moments.apply_dim, fx.Moments.speed_lines = saved['apply_dim'], saved['speed_lines']
        fx.Bursts.draw, fx.Particles.draw = saved['Bursts.draw'], saved['Particles.draw']
        fx.Marks.apply = saved['Marks.apply']
    _REAL.update(P=P, scripts=rec, frames=why, calls=seen)
    return _REAL


def real_frames(P):
    """{frame: why}: frames of the example episode on which each effect is at work"""
    from . import frames as F
    picks = {}

    def at(t, why):
        picks.setdefault(max(0, min(NF - 1, F.first_at(t))), why)

    for t_, who, name in P.STORY['caps']:
        for dt in (0.05, 0.4, 0.95):
            at(t_ + dt, f'the ability moment of {name}')
    for dt in (0.0, 1 / 30, 0.2):
        at(P.T_KO + dt, 'the K.O.')
    for dt in (0.1, 0.5, 1.5):
        at(P.T_HS + dt, 'after the hit-stop')
    for e in [e for e in P.EV if e[1] == 'slam' and e[0] < P.T_KO][:2]:
        for dt in (0.02, 0.3):
            at(e[0] + dt, 'a slam: its dust, its shimmer, its crack')
    fringed = [e for e in P.EV if e[1] == 'hit' and e[0] < P.T_KO and P.LOOKS[e[2]].fringe(e)]
    for e in fringed[:3] + fringed[-2:]:
        for dt in (0.005, 0.07):
            at(e[0] + dt, f'a fringed hit ({e[7]})')
    seen = {}
    for f in range(NF):
        t = f / FPS
        if P.T.T_FIGHT <= t < P.T_HS:
            for w in P.RUN['WAVES'][P.sim_i(min(t, P.T_KO))]:
                seen.setdefault((int(w[0]), w[5]), []).append(f)
    for (who, kind), fs in sorted(seen.items()):
        for f in (fs[0], fs[min(3, len(fs) - 1)], fs[len(fs) // 2], fs[-1]):
            picks.setdefault(f, f'a ring ({kind}) of {P.NAMES[who]}')
    hits = [e for e in P.EV if e[1] == 'hit' and e[0] < P.T_KO]
    for e in hits[:3] + hits[len(hits) // 2:len(hits) // 2 + 3] + hits[-3:]:
        at(e[0], 'a hit: its burst')
        at(e[0] + 1 / 30, 'a hit: the hollow burst of its second frame')
    return dict(sorted(picks.items()))


def real_calls(name):
    """[(frame, info)] of one recorded function, in frame order"""
    R = real()
    return [(f, info) for f in R['frames'] for n, info in R['calls'][f] if n == name]


def spread(items, n):
    """at most n of the items, evenly through the list, first and last kept"""
    if len(items) <= n:
        return list(items)
    return [items[round(i * (len(items) - 1) / (n - 1))] for i in range(n)]


# ================================================================ fx_timing
def make_flares(script):
    from reelkit import fx
    return build(lambda: fx.Flares(script['max_n'], float.fromhex(script['span'])), 'add', script['calls'])


def flare_script(g, name, calls, max_n=6, span=3.0):
    return g.script(name, kind='Flares', max_n=max_n, span=hx(span), calls=calls)


def plan_flares(r):
    """the plan's script: 20 adds"""
    calls = []
    for i in range(20):
        rgb = [(196, 52, 104), (120, 240, 184), (226, 234, 248), (300.0, 150.5, 20.25), (0.4, 0.9, 0.2)][i % 5]
        calls.append(dict(t0=hx(float(r.uniform(0, 1.6)) if i % 7 else 0.5), pos=hx(r.uniform(-3, 3, 3).tolist()),
                          rgb=hx([float(v) for v in rgb]), radius=hx(float(r.uniform(1.0, 3.0))),
                          peak=hx(float(r.uniform(0.3, 1.4))), attack=hx(0.03 if i % 3 else float(r.uniform(0, 0.1))),
                          decay=hx(float(r.uniform(0.1, 0.6))), hold=hx(0.0 if i % 4 else float(r.uniform(0, 0.3)))))
    return calls


PLAN_TIMES = (0.0, 0.25, 0.5, 0.52, 1.0, 1.75)


def group_timing():
    from reelkit import fx
    from reelkit.lighting import Lights
    R = real()
    P = R['P']
    g = Group('fx_timing', 'reelkit.fx: env_ad, FlashBudget, Shake, Flares, the envelopes of Moments and Marks.live',
              note='Row-wise calls: one Python call per row of the input arrays. Shake(...).offset(t): out is (n, 2). '
                   'Flares(...).at(t): per time of "t", "count" lights; their positions "p", colours "c" and radii '
                   '"r" follow one another. tint: "P" (k, 3) points, out (k, 3). apply: the points of a fresh '
                   'Lights after FL.apply(L, t), as at. Moments(...).active(t): per time, "count" rows of "which" '
                   '(the index of the add call in the script), "e" and "u"; dim_k and env one value per row. '
                   'Marks(...).live(t): per time, "count" rows of "which", "k" and "age".')
    r = rng(1)
    # env_ad
    u = np.concatenate([r.uniform(-0.05, 1.5, 300), [0.0, -0.0, 0.03, 0.029999999, 0.015, -1e-9, 1e-12, 5.0, 40.0]])
    for name, (a, d, h) in (('defaults', (0.03, 0.25, 0.0)), ('flute swat', (0.03, 0.2, 0.0)),
                            ('a held flare', (0.05, 0.6, 0.2))):
        g.add('env_ad(u, attack, decay, hold)', dict(attack=hx(a), decay=hx(d), hold=hx(h), source='seeded', what=name),
              u=u, out=f64(fx.env_ad(float(x), a, d, h) for x in u))
    a, d, h = r.uniform(1e-3, 0.2, 300), r.uniform(0.05, 1.0, 300), np.where(r.random(300) < 0.5, 0.0,
                                                                             r.uniform(0, 0.4, 300))
    uu = r.uniform(-0.1, 2.0, 300)
    g.add('env_ad(u, attack, decay, hold)', dict(source='seeded', what='every argument per row'), u=uu, attack=a,
          decay=d,
          hold=h, out=f64(fx.env_ad(*(float(v) for v in q)) for q in zip(uu, a, d, h)))

    # FlashBudget
    def budget(times, pr, max_per_s=3, window=1.0, **more):
        out = fx.FlashBudget(max_per_s, window).assign([float(t) for t in times],
                                                       None if pr is None else [float(p) for p in pr])
        arrays = dict(times=f64(times), out=np.array(out, np.bool_))
        if pr is not None:
            arrays['priority'] = f64(pr)
        g.add('FlashBudget(max_per_s, window).assign(times, priority)',
              dict(max_per_s=max_per_s, window=hx(window), priority=pr is not None, **more), **arrays)

    bright = [c for c in R['scripts']['bursts'] if c.get('bright', True)]
    budget([float.fromhex(c['t0']) for c in bright], [float.fromhex(c.get('strength', hx(1.0))) for c in bright],
           source='kit.picture',
           what='the bright bursts of the example episode')
    sets = ((40, 6.0, 3, 1.0), (60, 4.0, 3, 1.0), (30, 2.0, 1, 0.5), (50, 5.0, 5, 1.0), (25, 3.0, 2, 2.0),
            (12, 30.0, 3, 1.0))
    for n, (count, top, mx, win) in enumerate(sets):
        times = np.round(r.uniform(0, top, count), 3)
        # equal after round(t, 4)
        times[count // 2:count // 2 + 4] = times[0] + np.array([0.0, 0.00004, 0.00005, 0.00006])
        budget(times, None if n % 2 else r.integers(0, 4, count) * 0.5, mx, win, source='seeded')
    budget([], None, source='seeded')
    budget([1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0], [0.0, 1.0, 3.0, 3.0, 2.0, 0.0, 0.0], source='seeded',
           what='ties: priority, then the order given; an event exactly one window later')
    budget([0.25, 0.75, 1.25, 1.25, 1.75], None, 2, 1.0, source='seeded')
    budget([1.0, 1.00004, 3.0, 3.00006, 3.0], [0.0, 2.0, 1.0, 5.0, 1.0], 1, 1.0, source='seeded',
           what='times that are equal once rounded to 4 places are taken by priority: the later, stronger one wins')

    # Shake
    def shake(script, ts, cap=2.5, decay=0.09, span=0.45, **more):
        sh = fx.Shake(cap, decay, span)
        for c in g.scripts[script]['calls']:
            sh.add(float.fromhex(c['t0']), float.fromhex(c['px']))
        g.add('Shake(cap, decay, span).offset(t)',
              dict(script=script, cap=hx(cap), decay=hx(decay), span=hx(span), **more),
              t=f64(ts), out=rows([sh.offset(float(t)) for t in ts], 2))

    g.script('shake kit.picture', kind='Shake', calls=R['scripts']['shake'])
    t0s = [float.fromhex(c['t0']) for c in R['scripts']['shake']]
    ts = sorted({f / FPS for t0 in t0s for f in range(int(t0 * FPS), int(t0 * FPS) + 16, 3) if f < NF})
    shake('shake kit.picture', ts, source='kit.picture', what='the frames after each jolt of the example episode')
    shake('shake kit.picture', t0s + [t + 0.45 for t in t0s[:20]] + [t - 1e-9 for t in t0s[:20]], source='kit.picture',
          what='at each jolt itself, at the end of its look-back and just before it')
    g.script('shake plan', kind='Shake', calls=[dict(t0=hx(float(t)), px=hx(float(p))) for t, p in
                                               zip(r.uniform(0, 1.5, 20), r.uniform(0.2, 4.0, 20))])
    shake('shake plan', PLAN_TIMES, source='plan')
    shake('shake plan', r.uniform(-0.2, 2.2, 150), 1.5, 0.2, 0.8, source='seeded')
    g.script('shake none', kind='Shake', calls=[])
    shake('shake none', [0.0, 1.0], source='seeded')

    # Flares
    flare_script(g, 'flares kit.picture', R['scripts']['flares'])
    flare_script(g, 'flares plan', plan_flares(r))
    flare_script(g, 'flares two', plan_flares(r)[:9], max_n=2, span=1.0)

    def at_arrays(fl, ts):
        got = [fl.at(float(t)) for t in ts]
        every = [q for one in got for q in one]
        return dict(t=f64(ts), count=i64(len(one) for one in got), p=rows([q[0] for q in every], 3),
                    c=rows([q[1] for q in every], 3), r=f64(q[2] for q in every))

    real_t = sorted({P.fx_clock(f / FPS) for f in R['frames']})
    for script, ts, src in (('flares kit.picture', real_t, 'kit.picture'), ('flares plan', PLAN_TIMES, 'plan'),
                            ('flares two', PLAN_TIMES, 'plan'), ('flares plan', r.uniform(-0.5, 5.5, 60), 'seeded')):
        fl = make_flares(g.scripts[script])
        g.add('Flares(max_n, span).at(t)', dict(script=script, source=src), **at_arrays(fl, ts))
        for t in spread(list(ts), 6):
            pts = r.uniform(-4, 4, (12, 3))
            gain = 1.0 if src != 'seeded' else float(r.uniform(0.2, 2.0))
            g.add('Flares(max_n, span).tint(P, t, gain)', dict(script=script, t=hx(t), gain=hx(gain), source=src),
                  P=pts, out=np.ascontiguousarray(fl.tint(pts, float(t), gain)))
        for t in spread(list(ts), 3):
            L = Lights(amb=(0.36, 0.33, 0.44), sky=(0.10, 0.08, 0.14))
            fl.apply(L, float(t))
            g.add('Flares(max_n, span).apply(L, t)', dict(script=script, t=hx(t), source=src),
                  p=rows([q[0] for q in L.points], 3), c=rows([q[1] for q in L.points], 3),
                  r=f64(q[2] for q in L.points))
    fl = make_flares(g.scripts['flares plan'])
    g.add('Flares(max_n, span).tint(P, t, gain)', dict(script='flares plan', t=hx(0.5), gain=hx(1.0), source='seeded',
                                                      what='one point, as np.atleast_2d makes it'),
          P=np.array([[0.2, 0.6, -0.4]]), out=np.ascontiguousarray(fl.tint([0.2, 0.6, -0.4], 0.5)))

    # Moments: env, active, dim_k
    def moments_script(name, calls, **kw):
        return g.script(name, kind='Moments', max_dim=hx(kw.get('max_dim', 0.3)), hold=hx(kw.get('hold', 0.55)),
                        t_in=hx(kw.get('t_in', 0.14)), t_out=hx(kw.get('t_out', 0.4)), calls=calls)

    moments_script('moments kit.picture', R['scripts']['moments'])
    plan = [dict(t0=hx(float(t) if i % 6 else 0.75), key=int(i % 3), rgb=hx(r.uniform(0, 255, 3).tolist()),
                 k=hx(1.0 if i % 4 else float(r.uniform(0.2, 1.5)))) for i, t in enumerate(r.uniform(0, 1.6, 20))]
    moments_script('moments plan', plan)
    moments_script('moments deep', plan[:8], max_dim=0.8, hold=0.2, t_in=0.3, t_out=0.1)
    cap_t = [float.fromhex(c['t0']) for c in R['scripts']['moments']]
    for script, ts, src in (('moments kit.picture', sorted({f / FPS for t0 in cap_t for f in
                                                           range(int(t0 * FPS) - 1,
                                                                 int(t0 * FPS) + 40)}), 'kit.picture'),
                            ('moments plan', PLAN_TIMES, 'plan'), ('moments deep', PLAN_TIMES, 'plan'),
                            ('moments plan', r.uniform(-0.3, 3.2, 80), 'seeded')):
        mo = make_moments(g.scripts[script])
        mo.tl._sort()
        order = sorted(range(len(g.scripts[script]['calls'])),
                       key=lambda i: (float.fromhex(g.scripts[script]['calls'][i]['t0']), i))
        ident = {id(m): order[j] for j, m in enumerate(mo.tl.items)}
        act = [mo.active(float(t)) for t in ts]
        g.add('Moments(max_dim, hold, t_in, t_out).active(t)', dict(script=script, source=src), t=f64(ts),
              count=i64(len(a) for a in act), which=i64(ident[id(m)] for a in act for m, _, _ in a),
              e=f64(e for a in act for _, e, _ in a), u=f64(u for a in act for _, _, u in a))
        g.add('Moments(max_dim, hold, t_in, t_out).dim_k(t)', dict(script=script, source=src), t=f64(ts),
              out=f64(mo.dim_k(float(t)) for t in ts))
    mo = make_moments(g.scripts['moments plan'])
    tt, t0 = r.uniform(-0.2, 2.0, 200), r.uniform(0, 0.5, 200)
    g.add('Moments(max_dim, hold, t_in, t_out).env(t, t0)', dict(script='moments plan', source='seeded'), t=tt, t0=t0,
          out=f64(mo.env(float(a), float(b)) for a, b in zip(tt, t0)))

    # Marks.live
    mark_scripts(g, r)
    for script, ts, src in (('marks kit.picture', real_t + [P.T_KO + 4.0, 55.0, 60.9], 'kit.picture'),
                            ('marks plan', PLAN_TIMES, 'plan'), ('marks few', PLAN_TIMES, 'plan'),
                            ('marks plan', r.uniform(-0.3, 9.0, 80), 'seeded')):
        mk, ident = make_marks(g.scripts[script])
        live = [mk.live(float(t)) for t in ts]
        g.add('Marks(span, max_live).live(t)', dict(script=script, source=src), t=f64(ts),
              count=i64(len(a) for a in live), which=i64(ident[id(m)] for a in live for m, _, _ in a),
              k=f64(k for a in live for _, k, _ in a), age=f64(q for a in live for _, _, q in a))
    return g


def make_moments(script):
    from reelkit import fx
    f = float.fromhex
    return build(lambda: fx.Moments(f(script['max_dim']), f(script['hold']), f(script['t_in']), f(script['t_out'])),
                 'add', script['calls'])


def make_marks(script):
    """(the Marks, {id(mark dict): index of its add call})"""
    from reelkit import fx
    mk = build(lambda: fx.Marks(float.fromhex(script['span']), script['max_live']), 'add', script['calls'])
    items = list(mk.tl.items)                              # in the order added, before the first sort
    return mk, {id(m): i for i, m in enumerate(items)}


def plan_marks(r):
    from reelkit import fx
    calls = []
    for i in range(20):
        kind = fx.MARK_KINDS[i % 7]
        calls.append(dict(t0=hx(float(r.uniform(0, 1.2)) if i % 5 else 0.25), kind=kind,
                          u=hx(float(r.uniform(-2.2, 2.2))),
                          v=hx(float(r.uniform(-2.2, 2.2))), size=hx(float(r.uniform(0.3, 1.3))),
                          angle=hx(float(r.uniform(-3.2, 3.2)) if i % 3 else 0.0),
                          col=hx([float(v) for v in ((40, 30, 34), (230, 220, 200), (176, 140, 96))[i % 3]]),
                          life=hx(float(r.uniform(0.8, 6.0))), n=int(r.integers(0, 7)), seed=int(r.integers(0, 5000)),
                          strength=hx(1.0 if i % 4 else float(r.uniform(0.3, 1.2)))))
    return calls


def mark_scripts(g, r):
    R = real()
    g.script('marks kit.picture', kind='Marks', span=hx(12.0), max_live=40, calls=R['scripts']['marks'])
    plan = plan_marks(rng(77))
    g.script('marks plan', kind='Marks', span=hx(12.0), max_live=40, calls=plan)
    g.script('marks few', kind='Marks', span=hx(1.0), max_live=3, calls=plan)


# ================================================================ fx_bursts
def burst_script(g, name, calls, **kw):
    return g.script(name, kind='Bursts', max_per_s=kw.get('max_per_s', 3), base_px=hx(kw.get('base_px', 7.0)),
                    per_strength=hx(kw.get('per_strength', 9.0)), frames=kw.get('frames', 2), calls=calls)


def make_bursts(script):
    from reelkit import fx
    f = float.fromhex
    bu = fx.Bursts(script['max_per_s'], f(script['base_px']), f(script['per_strength']), script['frames'])
    for a in script['calls']:
        bu.add(**py_call(a))
    return bu


def group_bursts():
    from reelkit import fx
    R = real()
    g = Group('fx_bursts', 'reelkit.fx: starburst on float pictures, Bursts.draw and Bursts.bright_times',
              note=CANVAS_NOTE + RESULT_NOTE + CAM_NOTE +
              'starburst: every argument by name. Bursts(...).draw(img, cam, t): the script in args.script. '
              'bright_times: out holds the times.')
    r = rng(2)

    def star(spec, **kw):
        img = canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = fx.starburst(img, kw['x'], kw['y'], kw['r'], np.array(kw['rgb'], float), kw['k'], kw['spikes'],
                               kw['seed'],
                               kw['hollow'], kw['bright'])
        expect, arrays = result(spec, img, got, 'canvas')
        src = kw.pop('source', 'seeded')
        g.add('starburst(img, x, y, r, rgb, k, spikes, seed, hollow, bright)', dict(canvas=spec, source=src, **hx(kw)),
              expect, **arrays)
        return expect

    cols = ((222.0, 232.0, 246.0), (120.0, 240.0, 184.0), (226.0, 234.0, 248.0), (255.0, 226.0, 150.0),
            (18.0, 10.0, 26.0),
            (255.0, 255.0, 255.0), (300.5, -20.0, 128.0))
    n = 0
    for dtype in ('float64', 'float32'):
        for hollow, bright in ((False, True), (False, False), (True, True)):
            for j in range(7):
                spec = (weave(r, dtype, 96, 96, gain=0.97, bias=3.3) if j % 2 else
                        flat(r.integers(0, 256, 3), dtype, 96, 96, gain=1.013, bias=-1.7))
                rad = (7.0 + 9.0 * 0.55, 7.0 + 9.0 * 0.8, 16.0, (7.0 + 9.0 * 0.6) * 1.3, 25.0 * 1.3, 3.2, 25.0)[j]
                star(spec, x=float(r.uniform(30, 66)), y=float(r.uniform(30, 66)), r=float(rad),
                     rgb=list(cols[(n + j) % 7]),
                     k=0.75 if hollow else (1.0 if j % 3 else float(r.uniform(0.2, 1.3))), spikes=9 if j != 2 else 5,
                     seed=int(r.integers(0, 10 ** 6)) if j else n * 7919 + 13, hollow=hollow, bright=bright)
                n += 1
    # across every edge, off the picture, a tie of round(x), a radius of 0 and a whole radius
    for x, y, rad in ((2.0, 3.0, 12.0), (93.5, 48.0, 14.0), (48.0, -4.0, 10.0), (50.5, 99.0, 15.0), (-40.0, 30.0, 10.0),
                      (48.0, 160.0, 10.0), (47.5, 48.5, 9.0), (46.5, 49.5, 9.0), (40.0, 40.0, 0.0), (40.0, 40.0, 11.0),
                      (-9.0, -9.0, 9.0), (104.0, 104.0, 8.0)):
        for dtype in ('float64', 'float32'):
            star(weave(r, dtype, 96, 96), x=x, y=y, r=rad, rgb=list(cols[0]), k=1.0, spikes=9,
                 seed=int(r.integers(0, 10 ** 6)), hollow=False, bright=True)
    for k in (0.0, 0.5, 1.0 / 255.0, 0.999, 1.0, 7.0):
        star(weave(r, 'float64', 64, 64, gain=1.0), x=31.0, y=33.0, r=12.0, rgb=list(cols[1]), k=k, spikes=9, seed=5,
             hollow=False, bright=True)
    star(flat((90, 80, 120), 'float64'), x=180.2, y=320.7, r=25.0, rgb=list(cols[0]), k=1.0, spikes=9, seed=13,
         hollow=False, bright=True, source='seeded')

    # Bursts
    burst_script(g, 'bursts kit.picture', R['scripts']['bursts'])
    plan = []
    for i in range(20):
        plan.append(dict(t0=hx(float(np.round(r.uniform(0, 0.6), 3)) if i % 6 else 0.2),
                         pos=hx([float(r.uniform(-3, 3)), 0.6, float(r.uniform(-3, 3))]),
                         rgb=hx(list(cols[i % 5])), strength=hx(float(r.uniform(0.3, 2.0))),
                         seed=None if i % 4 else int(r.integers(0, 10 ** 5)), bright=bool(i % 3)))
    burst_script(g, 'bursts plan', plan)
    burst_script(g, 'bursts long', plan, max_per_s=1, base_px=5.0, per_strength=4.0, frames=4)
    # six bright bursts at one time: the budget's three go to the strongest, whatever the order they were added in
    ties = [dict(t0=hx(1.0), pos=hx([-2.0 + 0.8 * i, 0.6, 0.5 * (i % 2)]), rgb=hx(list(cols[i % 3])), strength=hx(s_))
            for i, s_ in enumerate((0.5, 2.0, 1.0, 1.5, 0.3, 1.8))]
    burst_script(g, 'bursts ties', ties)
    for script in ('bursts kit.picture', 'bursts plan', 'bursts long', 'bursts ties'):
        bu = make_bursts(g.scripts[script])
        g.add('Bursts(max_per_s, base_px, per_strength, frames).bright_times()',
              dict(script=script, source='kit.picture' if 'kit' in script else 'plan'), out=f64(bu.bright_times()))

    def draw(script, spec, cam, t, **more):
        bu, img = make_bursts(g.scripts[script]), canvas(spec)
        got = bu.draw(img, cam_from(cam), float(t))
        expect, arrays = result(spec, img, got, 'canvas')
        g.add('Bursts(max_per_s, base_px, per_strength, frames).draw(img, cam, t)',
              dict(script=script, canvas=spec, cam=cam, t=hx(t), **more), expect, **arrays)
        return expect

    recorded = [(f, c) for f, c in real_calls('Bursts.draw') if c['live']]
    assert len(recorded) >= 8, f'only {len(recorded)} recorded frames draw a burst'
    drew = 0
    for n, (f, c) in enumerate(spread(recorded, 12)):
        # after the grade the picture is float64; after speed lines it is float32 and holds whole numbers
        spec = (flat(r.integers(20, 236, 3), 'float32') if n % 3 == 2 else
                flat(r.integers(20, 236, 3), 'float64', gain=float(r.uniform(0.9, 1.0)), bias=float(r.uniform(0, 3))))
        assert c['dtype'] in ('float32', 'float64')
        drew += draw('bursts kit.picture', spec, c['cam'], c['t'], frame=f, source='kit.picture',
                     seen=c['dtype'])['box'] is not None
    assert drew >= 8, drew
    ring = cam_fields(cam_of(RING))
    for t in PLAN_TIMES[:5] + (0.2 + 1 / 30,):
        draw('bursts plan', flat((70, 66, 96), 'float64', gain=0.99, bias=1.5), ring, t, source='plan')
    for t in (0.2, 0.2 + 1 / 30, 0.2 + 2 / 30, 0.2 + 3 / 30, 0.2 + 4 / 30, 0.1999999):
        draw('bursts long', flat((70, 66, 96), 'float32'), cam_fields(cam_of(RING_B)), t, source='plan')
    assert make_bursts(g.scripts['bursts ties']).bright_times() == [1.0] * 3
    assert draw('bursts ties', flat((70, 66, 96), 'float64'), ring, 1.0, source='seeded')['box']
    return g


# ================================================================ fx_warp
def group_warp():
    from reelkit import fx
    R = real()
    g = Group('fx_warp', 'reelkit.fx: ellipse_of, shimmer_world, shimmer and fringe',
              note=CANVAS_NOTE + RESULT_NOTE + CAM_NOTE +
              'ellipse_of(cam, c, r): out is [cx, cy, ax, ay]. shimmer and fringe are called as kit.picture calls '
              'them: the centre and the half-axes are numpy float64 scalars (what cam.wp and ellipse_of give), '
              'which makes every intermediate float64; with Python floats numpy would compute in float32. The '
              'pictures hold whole numbers, so a result is stored as bytes: both functions only move pixels.')
    r = rng(3)
    F8 = np.float64

    def world(spec, cam, c, rad, width, amp, t, k, seed, **more):
        img = canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = fx.shimmer_world(img, cam_from(cam), tuple(c), rad, width, amp, t, k, seed)
        expect, arrays = result(spec, img, got, 'canvas')
        assert got is img and expect.get('stored', 'uint8') == 'uint8'
        g.add('shimmer_world(img, cam, c, r, width, amp, t, k, seed)',
              dict(canvas=spec, cam=cam, c=hx(list(c)), r=hx(rad), width=hx(width), amp=hx(amp), t=hx(t), k=hx(k),
                   seed=seed,
                   **more), expect, **arrays)
        return expect

    recorded = real_calls('shimmer_world')
    kinds = {}
    for f, c in recorded:
        kinds.setdefault((c['width'], c['amp']), []).append((f, c))
    # the piccolo ring, the shockwave, the slam
    assert len(kinds) >= 3, f'the recorded frames show {sorted(kinds)} only'
    moved = 0
    for key in sorted(kinds):
        for n, (f, c) in enumerate(spread(kinds[key], 6)):
            assert c['dtype'] in ('float32', 'float64')
            e = world(weave(r, c['dtype']), c['cam'], c['c'], c['r'], c['width'], c['amp'], c['t'], c['k'], c['seed'],
                      frame=f, source='kit.picture')
            moved += e['box'] is not None
    assert moved >= 8, moved
    ring, near = cam_fields(cam_of(RING)), cam_fields(cam_of(NEAR_CAM))
    for n in range(10):
        cam = ring if n % 2 else near
        e = world(weave(r, 'float64' if n % 3 else 'float32'), cam,
                  [float(r.uniform(-1, 1)), 0.05, float(r.uniform(-1, 1))],
                  float(r.uniform(0.05, 1.2)), float(r.uniform(0.1, 0.6)), float(r.uniform(1.0, 4.0)),
                  float(r.uniform(0, 60)),
                  float(r.uniform(0.3, 1.0)), int(n % 2), source='seeded')
        assert e['box'] is not None
    # thresholds: k at 0.01; amp * k at 0.4 (which no picture can show: a push under half a pixel moves nothing);
    # a radius of 0 and one under the 1e-3 floor; a front off the picture
    for k, amp, moves in ((0.01, 100.0, False), (0.0100001, 100.0, True), (0.2, 2.0, False), (0.2, 1.9999, False),
                          (1.0, 0.4, False), (-1.0, 3.0, False)):
        e = world(weave(r, 'float64'), near, [0.0, 0.05, 0.0], 0.3, 0.3, amp, 1.0, k, 0, source='seeded')
        assert (e['box'] is not None) == moves, (k, amp)
    # (a circle whose half-axes are next to nothing has every pixel "on the front", and none is pushed)
    for rad in (0.0, 1e-4):
        assert not world(weave(r, 'float64'), near, [0.0, 0.05, 0.0], rad, 0.3, 3.0, 1.0, 1.0, 0,
                         source='seeded')['box']
    assert not world(weave(r, 'float64'), near, [40.0, 0.05, 0.0], 0.5, 0.3, 3.0, 1.0, 1.0, 0, source='seeded')['box']
    # a front narrower than 2 px is 2 px wide
    assert world(weave(r, 'float64'), ring, [0.3, 0.05, -0.4], 1.5, 0.02, 3.0, 2.0, 1.0, 1, source='seeded')['box']

    for cam, c, rad in [(ring, [0.0, 0.05, 0.0], 1.0), (near, [0.3, 0.05, -0.2], 0.4), (ring, [2.0, 0.05, -3.0], 2.5),
                        (near, [0.0, 0.0, 0.0], 0.0)] + [(ring if n % 2 else near, r.uniform(-2, 2, 3).tolist(),
                                                          float(r.uniform(0, 3)))
                                                         for n in range(12)]:
        g.add('ellipse_of(cam, c, r)', dict(cam=cam, c=hx(c), r=hx(rad), source='seeded'),
              out=f64(fx.ellipse_of(cam_from(cam), c, rad)))
    for f, c in spread(recorded, 8):
        g.add('ellipse_of(cam, c, r)', dict(cam=c['cam'], c=hx(c['c']), r=hx(c['r']), source='kit.picture', frame=f),
              out=f64(fx.ellipse_of(cam_from(c['cam']), c['c'], c['r'])))

    def shimmer(spec, cx, cy, ax, ay, width, amp, t, k, seed):
        img = canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = fx.shimmer(img, F8(cx), F8(cy), F8(ax), F8(ay), F8(width), amp, t, k, seed)
        expect, arrays = result(spec, img, got, 'canvas')
        g.add('shimmer(img, cx, cy, ax, ay, width, amp, t, k, seed)',
              dict(canvas=spec, cx=hx(cx), cy=hx(cy), ax=hx(ax), ay=hx(ay), width=hx(width), amp=hx(amp), t=hx(t),
                   k=hx(k),
                   seed=seed, source='seeded'), expect, **arrays)

    for n in range(10):
        shimmer(weave(r, 'float64', 100, 90), float(r.uniform(10, 90)), float(r.uniform(10, 80)),
                float(r.uniform(0.3, 40)),
                float(r.uniform(0.3, 25)), float(r.uniform(2, 8)), float(r.uniform(1, 4)), float(r.uniform(0, 60)),
                float(r.uniform(0.4, 1)), n)
    # the box is off the left
    shimmer(weave(r, 'float64', 100, 90), -30.0, 45.0, 10.0, 10.0, 3.0, 3.0, 0.0, 1.0, 0)
    shimmer(weave(r, 'float64', 100, 90), 99.0, 89.0, 30.0, 20.0, 4.0, 3.0, 2.0, 1.0, 1)         # across the corner
    shimmer(weave(r, 'float64', 100, 90), 50.0, 45.0, 0.5, 0.25, 2.0, 3.0, 2.0, 1.0, 1)          # half-axes under 1
    # a push so strong that the band's faint rim moves too (pixels are moved where the profile is above 0.02)
    shimmer(weave(r, 'float64', 100, 90), 50.0, 45.0, 22.0, 15.0, 6.0, 80.0, 2.0, 1.0, 1)

    def fringe(spec, cx, cy, rad, px, k, **more):
        img = canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = fx.fringe(img, F8(cx), F8(cy), rad, px, k)
        expect, arrays = result(spec, img, got, 'canvas')
        assert got is img and expect.get('stored', 'uint8') == 'uint8'
        g.add('fringe(img, cx, cy, r, px, k)',
              dict(canvas=spec, cx=hx(cx), cy=hx(cy), r=hx(float(rad)), px=hx(float(px)),
                                                     k=hx(k), **more), expect, **arrays)
        return expect

    recorded = real_calls('fringe')
    sizes = {(c['r'], c['px']) for _, c in recorded}
    assert {(26, 2.5), (34, 3.0)} <= sizes, f'the recorded frames fringe {sorted(sizes)} only'
    for f, c in spread(recorded, 14):
        fringe(weave(r, 'float64'), c['cx'], c['cy'], c['r'], c['px'], c['k'], frame=f, source='kit.picture')
    for n in range(10):
        fringe(weave(r, 'float64' if n % 2 else 'float32', 100, 90), float(r.uniform(-5, 105)),
               float(r.uniform(-5, 95)),
               float(r.uniform(3, 40)), float(r.uniform(0.5, 4)), float(r.uniform(0.2, 1)), source='seeded')
    for k in (0.01, 0.0100001, 0.0, -0.5, 0.12, 0.13):                      # px * k crosses 0.3 between the last two
        fringe(weave(r, 'float64', 100, 90), 50.0, 45.0, 26.0, 2.5, k, source='seeded')
    fringe(weave(r, 'float64', 100, 90), 50.0, 45.0, 0.4, 2.5, 1.0, source='seeded')
    # k of exactly 0.01 does nothing, however far the shift would reach
    assert not fringe(weave(r, 'float64', 100, 90), 50.0, 45.0, 26.0, 200.0, 0.01, source='seeded')['box']
    assert fringe(weave(r, 'float64', 100, 90), 50.0, 45.0, 26.0, 200.0, 0.0100001, source='seeded')['box']
    fringe(weave(r, 'float64', 100, 90), 300.0, 45.0, 26.0, 2.5, 1.0, source='seeded')
    return g


# ================================================================ fx_particles
COLUMNS = ('t0', 'p', 'v', 'life', 'size', 'grow', 'col', 'g', 'drag', 'wa', 'wf', 'ph', 'tw', 'shape', 'alpha',
           'ground', 'spin', 'glow', 'tn', 'tdt')


def particle_script(g, name, seed, calls, max_alive=600):
    return g.script(name, kind='Particles', seed=seed, max_alive=max_alive, calls=calls)


def make_particles(script):
    from reelkit import fx
    pa = fx.Particles(script['seed'], script['max_alive'])
    with np.errstate(all='ignore'):                        # a direction of length 0 divides 0 by 0, as in the kit
        for a in script['calls']:
            a = py_call(a)
            a['pos'] = np.array(a['pos'], float)
            if a.get('col') is not None:
                a['col'] = [tuple(c) for c in a['col']] if isinstance(a['col'][0], tuple) else tuple(a['col'])
            pa.emit(**a)
    return pa


def emit(t0, pos, kind, n=None, dir=(0.0, 1.0, 0.0), vel=(0.0, 0.0, 0.0), col=None, scale=1.0, speed=None,
         spread=None, life=None):
    """an emit call for a script: the arguments that differ from Python's defaults"""
    f = lambda v: hx([float(x) for x in v])                                                # noqa: E731
    out = dict(t0=hx(float(t0)), pos=f(pos), kind=kind)
    if n is not None:
        out['n'] = n
    if tuple(dir) != (0.0, 1.0, 0.0):
        out['dir'] = f(dir)
    if tuple(vel) != (0.0, 0.0, 0.0):
        out['vel'] = f(vel)
    if col is not None:
        out['col'] = hx([[float(x) for x in c] for c in col] if np.ndim(col) == 2 else [float(x) for x in col])
    if scale != 1.0:
        out['scale'] = hx(float(scale))
    if speed is not None:
        out['speed'] = f(speed)
    if spread is not None:
        out['spread'] = hx(float(spread))
    if life is not None:
        out['life'] = f(life)
    return out


def group_particles():
    from reelkit import fx
    R = real()
    P = R['P']
    g = Group('fx_particles', 'reelkit.fx.Particles: what emit makes, alive, _pos and draw',
              presets=list(fx.PRESETS), shapes=list(fx._SHAPES), columns=list(COLUMNS),
              note=CANVAS_NOTE + RESULT_NOTE + CAM_NOTE +
              '"Particles(seed, max_alive) after the script": the arrays are the columns of _arrays() (sorted by t0, '
              'stably), by name; p, v and col are (n, 3); shape, tn are int64, ground and glow bool. alive(t): per '
              'time of "t", "count" indices into those columns, one after another in "idx". _pos: for the '
              'particles alive at t, at their ages u = t - t0: out (k, 3). draw(img, cam, t, light): args.light is '
              'null or names a Flares script and a time; the light is then lambda P: 1.0 + gain * FL.tint(P, t), '
              'as kit.picture passes it.')
    r = rng(4)
    particle_script(g, 'particles kit.picture', P.T.FX_SEED, R['scripts']['particles'])
    flare_script(g, 'flares kit.picture', R['scripts']['flares'])
    # the plan's script: 20 emits, every preset once, each with its defaults
    particle_script(g, 'particles plan', 7,
                    [emit(0.02 * (i % 4), (-1.9 + 0.2 * i, 0.3 + 0.05 * (i % 3), -0.6 + 0.3 * (i % 5)), kind)
                                              for i, kind in enumerate(fx.PRESETS)])
    # every keyword: n (0 and negative emit nothing), a direction along x (the other helper axis), a velocity, one
    # colour and several, scale, speed, spread (0, 180, beyond), life; a direction of length 0
    kinds = list(fx.PRESETS)
    opts = []
    for i in range(20):
        kind = kinds[(i * 7) % 20]
        kw = dict(n=(None, 3, 0, 5, 1, -2, 12)[i % 7])
        if i % 2:
            kw['dir'] = ((1.0, 0.0, 0.0), (0.95, 0.2, 0.1), (-1.0, 0.1, 0.0), (0.0, 0.0, 0.0), (0.3, 0.4, -2.0))[i % 5]
        if i % 3 == 0:
            kw['vel'] = tuple(float(v) for v in r.uniform(-1.5, 1.5, 3))
        if i % 4 == 1:
            kw['col'] = (200.0, 120.5, 40.0)
        if i % 4 == 2:
            kw['col'] = [(196, 150, 92), (150, 112, 64), (226, 196, 140)]
        if i % 5 == 0:
            kw['scale'] = float(r.uniform(0.5, 2.0))
        if i % 3 == 1:
            kw['speed'] = (0.3, 0.8) if i % 2 else (1.0, 1.0)
        if i % 6 in (1, 2, 4):
            kw['spread'] = (0.0, 180.0, 30.0, 270.0, 90.0, 180.0)[i % 6]
        if i % 5 == 3:
            kw['life'] = (0.4, 0.9)
        opts.append(emit(float(np.round(r.uniform(0, 0.4), 2)), r.uniform(-1.2, 1.2, 3) + [0, 1.3, 0], kind, **kw))
    particle_script(g, 'particles options', 20261121, opts)
    particle_script(g, 'particles few', 3, opts, max_alive=9)
    particle_script(g, 'particles none', 3, [])

    def state(script, src):
        pa = make_particles(g.scripts[script])
        A = pa._arrays()
        arrays = {}
        for k in COLUMNS:
            a = A[k]
            if not len(a):
                a = np.zeros((0, 3) if k in ('p', 'v', 'col') else 0,
                             np.int64 if k in ('shape', 'tn') else np.bool_ if k in ('ground', 'glow') else np.float64)
            if a.dtype not in (np.float64, np.bool_):
                assert a.dtype.kind == 'i', (k, a.dtype)
                a = a.astype(np.int64)
            arrays[k] = np.ascontiguousarray(a)
        g.add('Particles(seed, max_alive) after the script', dict(script=script, source=src), **arrays)
        return pa

    made = {s: state(s, src) for s, src in (('particles kit.picture', 'kit.picture'), ('particles plan', 'plan'),
                                            ('particles options', 'seeded'), ('particles few', 'seeded'),
                                            ('particles none', 'seeded'))}
    assert len(made['particles kit.picture']._arrays()['t0']) == len(P.PA._arrays()['t0'])
    real_draws = real_calls('Particles.draw')
    real_t = sorted({c['t'] for _, c in real_draws})
    plan_t = (0.0, 0.05, 0.15, 0.3, 0.6, 1.1)
    for script, ts, src in (('particles kit.picture', real_t, 'kit.picture'), ('particles plan', plan_t, 'plan'),
                            ('particles options', plan_t + (0.41, 0.4099, 2.0, -1.0), 'seeded'),
                            ('particles few', plan_t, 'seeded'), ('particles none', plan_t[:2], 'seeded')):
        pa = made[script]
        got = [pa.alive(float(t))[1] for t in ts]
        g.add('Particles(seed, max_alive).alive(t)', dict(script=script, source=src), t=f64(ts),
              count=i64(len(i) for i in got), idx=i64(i for one in got for i in one))
        for t in spread([t for t, one in zip(ts, got) if len(one)], 4):
            A, idx = pa.alive(float(t))
            g.add('Particles._pos(A, idx, t - t0)', dict(script=script, t=hx(t), source=src),
                  out=np.ascontiguousarray(fx.Particles._pos(A, idx, float(t) - A['t0'][idx])))

    def draw(script, spec, cam, t, light=None, **more):
        pa, img = made[script], canvas(spec)
        fn = None
        if light is not None:
            fl = make_flares(g.scripts[light['script']])
            gain, tl = float.fromhex(light['gain']), float.fromhex(light['t'])
            fn = lambda pts: 1.0 + gain * fl.tint(pts, tl)                                  # noqa: E731
        with frame_size(fx, spec['w'], spec['h']):
            got = pa.draw(img, cam_from(cam), float(t), light=fn)
        expect, arrays = result(spec, img, got, 'bytes')
        assert expect['returned'] == 'img' or (got.dtype == np.float32 and expect.get('stored', 'uint8') == 'uint8')
        g.add('Particles(seed, max_alive).draw(img, cam, t, light)',
              dict(script=script, canvas=spec, cam=cam, t=hx(t), light=light, **more), expect, **arrays)
        return expect

    busy = sorted(real_draws, key=lambda q: -q[1]['live'])
    picks = sorted(busy[:7] + spread(busy[7:], 5), key=lambda q: q[0])
    assert len(picks) >= 10 and picks[0][1]['live'] != picks[-1][1]['live']
    for n, (f, c) in enumerate(picks):
        # the picture before the lens is float32 and not whole; a third of the cases leave the byte range
        spec = (weave(r, 'float32', gain=1.02, bias=-2.5) if n % 3 == 0 else
                flat(r.integers(10, 200, 3), 'float32', gain=float(r.uniform(0.8, 1.0)), bias=float(r.uniform(0, 4))))
        draw('particles kit.picture', spec, c['cam'], c['t'],
             dict(script='flares kit.picture', gain=hx(0.6), t=hx(c['t'])), frame=f, source='kit.picture')
    flare_script(g, 'flares plan', plan_flares(rng(1)))
    ring, ring_b, near = (cam_fields(cam_of(c)) for c in (RING, RING_B, NEAR_CAM))
    for t in plan_t:
        draw('particles plan', flat((60, 56, 84), 'float32', gain=0.98, bias=0.7), near, t, source='plan')
        draw('particles plan', weave(r, 'float64', gain=0.99, bias=0.3), ring, t, source='plan')
    for n, t in enumerate(plan_t + (0.41,)):
        draw('particles options', flat((60, 56, 84), 'float32'), near if n % 2 else ring_b, t,
             dict(script='flares plan', gain=hx(0.6 if n % 2 else 300.0), t=hx(0.25 + 0.05 * n)), source='seeded')
    draw('particles few', flat((60, 56, 84), 'float32'), near, 0.3, source='seeded')
    draw('particles none', flat((60, 56, 84), 'float32'), near, 0.3, source='seeded')
    draw('particles plan', flat((60, 56, 84), 'float32'), near, 9.0, source='seeded')                # none alive
    behind = cam_fields(cam_of(cam_args((0.0, 0.5, 30.0), 6.0, 180.0, 35.0, 50.0, 320.0)))
    # all behind the camera
    draw('particles plan', flat((60, 56, 84), 'float32'), behind, 0.3, source='seeded')
    draw('particles plan', weave(r, 'float32', 200, 150), near, 0.3, source='seeded')                # a smaller picture
    # a near plane through the cloud (some particles are in front of it, some behind), and a camera so far away
    # that every particle is under half a pixel (its radius is held at 0.5)
    cut = cam_of(cam_args((0.0, 0.5, 0.0), 6.0, 25.0, 35.0, 50.0, 320.0, near=6.0))
    A, idx = made['particles plan'].alive(0.3)
    z = cut.to_cam(fx.Particles._pos(A, idx, 0.3 - A['t0'][idx]))[:, 2]
    assert (z > 6.0).any() and ((z > 0) & (z <= 6.0)).sum() >= 10, z
    draw('particles plan', flat((60, 56, 84), 'float32'), cam_fields(cut), 0.3, source='seeded')
    far = cam_of(cam_args((0.0, 0.5, 0.0), 120.0, 25.0, 35.0, 50.0, 320.0))
    z = far.to_cam(fx.Particles._pos(A, idx, 0.3 - A['t0'][idx]))[:, 2]
    assert (A['size'][idx] * (1 + A['grow'][idx] * 0.3) * far.f / z < 0.5).sum() >= 20
    draw('particles plan', flat((60, 56, 84), 'float32'), cam_fields(far), 0.3, source='seeded')
    return g


# ================================================================ fx_marks
def grid(u0, u1, nu, v0, v1, nv):
    """floor points on a grid, as (spec, U, V): point i has column i % nu and row i // nu; U = u0 + column * du"""
    du, dv = (u1 - u0) / (nu - 1), (v1 - v0) / (nv - 1)
    i = np.arange(nu * nv)
    U, V = u0 + (i % nu) * du, v0 + (i // nu) * dv
    return dict(u0=hx(u0), du=hx(du), nu=nu, v0=hx(v0), dv=hx(dv), nv=nv), U, V


def group_marks():
    from reelkit import fx
    from reelkit.config import BAYER
    R = real()
    P = R['P']
    g = Group('fx_marks', 'reelkit.fx.Marks.apply: every kind of mark on a grid of floor points',
              kinds=list(fx.MARK_KINDS),
              note='args.grid: point i (0 to nu * nv - 1) has column i % nu and row i // nu, U = u0 + column * du, '
                   'V = v0 + row * dv. The albedo of point i is ((mul[c] * i + add[c]) % 256) as float64 '
                   '(args.alb). args.dither true: dither[i] = reelkit.config.BAYER[row % 4, column % 4]; false: '
                   'None. The result: "changed" lists the points whose albedo differs from the input, "out" their '
                   'rows (k, 3); every other row is the input. expect.returned is "alb" when Python returned the '
                   'array it was given (no mark alive).')
    r = rng(5)
    mark_scripts(g, r)

    def apply(script, gspec, U, V, t, dither, **more):
        mk, _ = make_marks(g.scripts[script])
        n = len(U)
        alb_spec = dict(mul=[int(v) * 2 + 1 for v in r.integers(0, 128, 3)],
                        add=[int(v) for v in r.integers(0, 256, 3)])
        i = np.arange(n, dtype=np.int64)
        alb = np.stack([(alb_spec['mul'][c] * i + alb_spec['add'][c]) % 256 for c in range(3)], -1).astype(np.float64)
        before = alb.copy()
        dq = BAYER[(i // gspec['nu']) % 4, (i % gspec['nu']) % 4] if dither else None
        got = mk.apply(alb, U, V, float(t), dq)
        assert np.array_equal(alb, before) and got.dtype == np.float64
        ch = np.nonzero(np.any(got != before, axis=1))[0]
        g.add('Marks(span, max_live).apply(alb, U, V, t, dither)',
              dict(script=script, grid=gspec, alb=alb_spec, t=hx(t), dither=bool(dither), **more),
              dict(returned='alb' if got is alb else 'new', live=len(mk.live(float(t)))),
              changed=ch.astype(np.int64), out=np.ascontiguousarray(got[ch]))
        return len(ch)

    # every kind on its own, around the mark: early in its fade-in, held, and fading out; dithered and not
    kinds = {}
    for n, kind in enumerate(fx.MARK_KINDS):
        for variant in range(2):
            m = dict(t0=hx(1.0), kind=kind, u=hx(0.3 * n - 0.8), v=hx(0.2 - 0.1 * n), size=hx(0.45 + 0.3 * variant),
                     angle=hx(0.4 * n * (1 - 2 * variant)),
                     col=hx([230.0, 220.0, 200.0] if variant else [40.0, 30.0, 34.0]),
                     life=hx(4.0), n=(3, 2, 8, 0, 1, 5, 6)[(n + 3 * variant) % 7], seed=17 * n + 1000 * variant,
                     strength=hx(1.0 if variant else 0.7))
            name = g.script(f'marks one {kind} {variant}', kind='Marks', span=hx(12.0), max_live=40, calls=[m])
            u, v, s = (float.fromhex(m[k]) for k in ('u', 'v', 'size'))
            gs, U, V = grid(u - 1.4 * s, u + 1.4 * s, 26, v - 1.4 * s, v + 1.4 * s, 26)
            kinds[kind] = kinds.get(kind, 0) + apply(name, gs, U, V, 1.5, True, source='seeded')
            if variant == 0:
                apply(name, gs, U, V, 1.03, True, source='seeded')
                apply(name, gs, U, V, 4.3, False, source='seeded')
    assert all(kinds[k] > 8 for k in fx.MARK_KINDS), kinds
    # a claw of no gouges is one gouge; the middle of a crack, where the rays stop narrowing (r under 0.05)
    m = dict(t0=hx(1.0), kind='claw', u=hx(0.2), v=hx(-0.3), size=hx(0.5), angle=hx(0.7), life=hx(4.0), n=0, seed=5)
    gs, U, V = grid(0.2 - 0.7, 0.2 + 0.7, 26, -0.3 - 0.7, -0.3 + 0.7, 26)
    assert apply(g.script('marks claw of none', kind='Marks', span=hx(12.0), max_live=40, calls=[m]), gs, U, V, 1.5,
                 True,
                 source='seeded') > 8
    m = dict(t0=hx(1.0), kind='crack', u=hx(0.2), v=hx(-0.3), size=hx(1.5), life=hx(7.0), n=8, seed=141)
    gs, U, V = grid(0.2 - 0.14, 0.2 + 0.14, 20, -0.3 - 0.14, -0.3 + 0.14, 20)
    assert apply(g.script('marks crack middle', kind='Marks', span=hx(12.0), max_live=40, calls=[m]), gs, U, V, 1.5,
                 False,
                 source='seeded') > 100
    # the example episode's marks at the effect clock of recorded frames, on the ring's canvas (7 x 7 units)
    S = P.S
    recorded = [(f, c) for f, c in real_calls('Marks.apply') if c['live']]
    assert len(recorded) >= 6
    seen = set()
    for f, c in spread(recorded, 8):
        gs, U, V = grid(-S.AP, S.AP, 44, -S.AP, S.AP, 44)
        apply('marks kit.picture', gs, U, V, c['t'], True, frame=f, source='kit.picture')
        mk, ident = make_marks(g.scripts['marks kit.picture'])
        seen |= {m['kind'] for m, _, _ in mk.live(c['t'])}
    assert {'claw', 'dust', 'glass', 'crack', 'scorch'} <= seen, seen
    gs, U, V = grid(-2.6, 2.6, 40, -2.6, 2.6, 40)
    for t in PLAN_TIMES:
        apply('marks plan', gs, U, V, t, True, source='plan')
    apply('marks plan', gs, U, V, 1.0, False, source='plan')
    apply('marks few', gs, U, V, 1.0, True, source='plan')
    apply('marks plan', gs, U, V, -1.0, True, source='seeded')                 # nothing alive: the same array back
    gs, U, V = grid(40.0, 45.0, 6, 40.0, 45.0, 6)
    apply('marks plan', gs, U, V, 1.0, True, source='seeded')                  # marks alive, none near these points
    return g


# ================================================================ fx_moments
def group_moments():
    from reelkit import fx
    R = real()
    g = Group('fx_moments', 'reelkit.fx.Moments: apply_dim and speed_lines',
              note=CANVAS_NOTE + RESULT_NOTE +
              'apply_dim(img, t, keep): keep is a list of [x, y, r]; x and y are numpy float64 scalars, as '
              'kit.picture passes them (cam.wp), so the lit discs are computed in float64. The result is float64 '
              'and differs from the picture nearly everywhere, so it is stored thinned: expect.step n means "out" '
              'holds the pixels i = 0, n, 2 n, ... (counted row by row), (k, 3); expect.box is then absent. '
              'speed_lines(img, t, key, x, y, r, direction, n, length, seed): x, y and r as numpy float64.')
    r = rng(6)
    F8 = np.float64
    g.script('moments kit.picture', kind='Moments', max_dim=hx(0.3), hold=hx(0.55), t_in=hx(0.14), t_out=hx(0.4),
             calls=R['scripts']['moments'])
    plan = [dict(t0=hx(float(t) if i % 6 else 0.75), key=int(i % 3), rgb=hx(r.uniform(0, 255, 3).tolist()),
                 k=hx(1.0 if i % 4 else float(r.uniform(0.2, 1.5)))) for i, t in enumerate(r.uniform(0, 1.6, 20))]
    g.script('moments plan', kind='Moments', max_dim=hx(0.3), hold=hx(0.55), t_in=hx(0.14), t_out=hx(0.4), calls=plan)

    def dim(script, spec, t, keep, step, **more):
        mo, img = make_moments(g.scripts[script]), canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = mo.apply_dim(img, float(t), [(F8(x), F8(y), rr) for x, y, rr in keep])
        expect = dict(returned='img' if got is img else 'new', dtype=got.dtype.name, k=hx(mo.dim_k(float(t))))
        arrays = {}
        if got is not img:
            assert got.dtype == np.float64
            expect['step'] = step
            arrays['out'] = np.ascontiguousarray(got.reshape(-1, 3)[::step])
        g.add('Moments(max_dim, hold, t_in, t_out).apply_dim(img, t, keep)',
              dict(script=script, canvas=spec, t=hx(t), keep=hx([list(map(float, q)) for q in keep]), **more), expect,
              **arrays)
        return got is not img

    recorded = real_calls('apply_dim')
    dimmed = [(f, c) for f, c in recorded if c['k'] > 0.005]
    assert len(dimmed) >= 6 and all(len(c['keep']) == 2 for _, c in dimmed)
    for n, (f, c) in enumerate(spread(dimmed, 5)):
        spec = flat(r.integers(30, 226, 3), 'float64', gain=float(r.uniform(0.9, 1.0)), bias=float(r.uniform(0, 3)))
        assert dim('moments kit.picture', spec, c['t'], c['keep'], 37, frame=f, source='kit.picture')
    f, c = next((f, c) for f, c in recorded if c['k'] <= 0.005)
    assert not dim('moments kit.picture', flat((90, 80, 70), 'float64'), c['t'], c['keep'], 37, frame=f,
                   source='kit.picture')
    for n, t in enumerate(PLAN_TIMES):
        keep = [(float(r.uniform(0, 64)), float(r.uniform(0, 48)), float(r.uniform(0.2, 9))) for _ in range(n % 4)]
        dim('moments plan', weave(r, 'float64', 64, 48, gain=0.996, bias=0.25), t, keep, 1, source='plan')
    dim('moments plan', weave(r, 'float64', 64, 48), 0.5, [(20.0, 20.0, 0.0), (20.5, 30.25, 1.0), (-5.0, 60.0, 14.0)],
        1,
        source='seeded')
    # a dim of 0.005 or less is no dim: 0.004 and 0.0057, early in a moment's ease-in
    g.script('moments one', kind='Moments', max_dim=hx(0.3), hold=hx(0.55), t_in=hx(0.14), t_out=hx(0.4),
             calls=[dict(t0=hx(1.0), key=0, rgb=hx([222.0, 232.0, 246.0]))])
    mo = make_moments(g.scripts['moments one'])
    assert 0.003 < mo.dim_k(1.0095) < 0.005 < mo.dim_k(1.0115) < 0.007
    assert not dim('moments one', weave(r, 'float64', 64, 48), 1.0095, [(20.0, 20.0, 6.0)], 1, source='seeded')
    assert dim('moments one', weave(r, 'float64', 64, 48), 1.0115, [(20.0, 20.0, 6.0)], 1, source='seeded')

    def lines(script, spec, t, key, x, y, rad, direction, n=16, length=(16, 46), seed=0, **more):
        mo, img = make_moments(g.scripts[script]), canvas(spec)
        with frame_size(fx, spec['w'], spec['h']):
            got = mo.speed_lines(img, float(t), key, F8(x), F8(y), F8(rad), tuple(direction), n, length, seed)
        expect, arrays = result(spec, img, got, 'bytes')
        assert expect['returned'] == 'img' or (got.dtype == np.float32 and expect.get('stored', 'uint8') == 'uint8')
        g.add('Moments(max_dim, hold, t_in, t_out).speed_lines(img, t, key, x, y, r, direction, n, length, seed)',
              dict(script=script, canvas=spec, t=hx(t), key=key, x=hx(x), y=hx(y), r=hx(rad),
                   direction=hx(list(direction)),
                   n=n, length=hx([float(v) for v in length]), seed=seed, **more), expect, **arrays)
        return expect['returned'] == 'new'

    recorded = real_calls('speed_lines')
    drew = [(f, c) for f, c in recorded if c['drew']]
    assert len(drew) >= 6 and {c['key'] for _, c in drew} == {0, 1}, 'the recorded frames lack speed lines of a fighter'
    assert {c['dtype'] for _, c in recorded} == {'float32', 'float64'}       # the second fighter's call gets float32
    for n, (f, c) in enumerate(spread(drew, 10)):
        spec = (weave(r, 'float32') if c['dtype'] == 'float32' else
                weave(r, 'float64', gain=float(r.uniform(0.9, 1.02)), bias=float(r.uniform(-2, 3))))
        assert lines('moments kit.picture', spec, c['t'], c['key'], c['x'], c['y'], c['r'], c['direction'], frame=f,
                     source='kit.picture')
    f, c = next((f, c) for f, c in recorded if not c['drew'])
    assert not lines('moments kit.picture', flat((90, 80, 70), 'float64'), c['t'], c['key'], c['x'], c['y'], c['r'],
                     c['direction'], frame=f, source='kit.picture')
    for n, t in enumerate(PLAN_TIMES):
        lines('moments plan', weave(r, 'float64', 160, 200, gain=1.01, bias=-1.0), t, n % 3, float(r.uniform(40, 120)),
              float(r.uniform(40, 160)), float(r.uniform(6, 24)),
              [(0.6, -0.8), (0.0, 0.0), (-3.0, 1.0), (1e-7, 0.0), (0.0, 5.0),
                                                                   (2.0, 2.0)][n],
              n=(16, 5, 16, 1, 30, 16)[n], length=((16, 46), (4, 9), (16, 46), (30, 30), (16, 46), (16, 46))[n],
              seed=n * 3,
              source='plan')
    # no such key
    lines('moments plan', weave(r, 'float64', 160, 200), 0.5, 7, 80.0, 100.0, 12.0, (1.0, 0.0), source='seeded')
    # at a corner
    lines('moments plan', weave(r, 'float32', 160, 200), 0.5, 0, 158.0, 3.0, 12.0, (1.0, 1.0), source='seeded')
    return g


def run():
    total = sum(make().write() for make in (group_timing, group_bursts, group_warp, group_particles, group_marks,
                                            group_moments))
    print(f'kit_fx: {total / 1e3:.1f} kB -> {common.rel(OUT)}')


EXPORTS = {'kit_fx': run}

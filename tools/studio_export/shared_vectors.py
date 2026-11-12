"""shared: the look and voice helpers that several kit fighters share, against the kit's own functions (milestone 3,
Task 4), so each is ported once, in a shared file, before the fighters are.

    python -m tools.studio_export shared

writes studio/reference/shared/ (top-level manifest), every file a vector file (.npz + .json sidecar, the
convention of dsp_vectors.Cases):

    reeds.npz      kit/fighters/_reeds.py: _mask, blend, add, ribbon, ring_exit, beam_lane, draw_woodwind_shard,
                   tail_theta; the same-code copies other fighters keep (bass_clarinet, bassoon, euphonium: _mask,
                   blend; bassoon: add); acoustic's and trumpet's mask_of; horn's and tuba's mask_of; organ's
                   mask_of and line_mask
    padded.npz     contrabass.padded_sprite (sousaphone's too) and bass_drum._render: the RGBA bytes' SHA-256
    ndimage.npz    scipy.ndimage.gaussian_filter as TRUMPET calls it, its kernel weights, and the edge cases
    pp_blocks.npz  the Pixel Pitch voice blocks: trumpet_lowpoly.brass (TRUMPET, TUBA, HORN), piano_lowpoly.ks_pluck
                   (ACOUSTIC, ELECTRIC), altosax60_lowpoly.sax_tone (ALTO SAX, and the alto voice the mambo bed
                   plays) with the module's own generator `_RN`, drums60_lowpoly.bp, bassclarinet_lowpoly.slap
                   (BASS CLARINET, EUPHONIUM), the start of the series' module generators, and the parts' shared
                   hash h01 (acoustic h01_33, horn _h01_34, organ h01_35, trumpet h01_29)

Where the arguments come from. The picture helpers are wrapped (at run time; nothing in the kit is edited) while
each fighter that uses one draws a fight against FLUTE on the big top (the m3refs base config, handicap 1.0, 1.0):
the first seed in 1..SEED_SCAN where the fighter loses with all three of its abilities used (so its shards fly), a
frame every second plus 0.2 s and 0.6 s after each ability's first use and the K.O. Each wrapped call is kept as a
case (the first few of each function and fighter, then a spread, at most CAP). The voice blocks are wrapped while
pp_bridge.voice plays each user's register (4 pitches x velocities 0.6, 0.9 x held lengths 0.12, 0.25, 1.0, as the
fighter template's voice gate) in one fresh interpreter, then edge cases are added (a 1 ms pluck, a note of no
held length, a pitch with no harmonic under the cap, ...).

What each case holds. The cases of a file share one array per call and role (`Packed`: a case's `at` gives its
offset and shape there), because a zip member per case costs more than the data. Small arrays a function returns
are stored as they are. A full-frame picture is not stored: its inputs are rebuilt from a formula both sides compute
(`synthetic`, `synthetic_mask`, a fresh generator's normals) or from the case's own polygons, and only its SHA-256
is kept. Identical calls are kept once. A note is stored as a fingerprint: its length, the RMS of each 882-sample
block (20 ms) and 64 of its samples (indices (k * (n - 1)) // 63), all float64, plus the SHA-256 of its float64 samples.
The tests' bars: pictures, masks and sprites byte for byte; plain arithmetic to the bit; a float that passes through
the camera (ruling R34) value by value; a note within -120 dBFS sample by sample (the voices' bar of milestone 1)."""
import hashlib
import math
import re
import multiprocessing as mp
import os
import sys
import time

from . import common
from .dsp_vectors import Cases, jarg

OUT = common.ref('shared')
TAG = 'shared'
SEED_SCAN = 40                                           # seeds tried for each fighter's picture fight
CAP = 8                                                  # cases kept per function and fighter (picture)
WRITE_CAP = dict(blend=4, add=3, reeds=6)               # ... of those written, for the commonest
SOUND_CAP = 6                                            # ... per block and user voice
KEEP_FIRST = 4                                           # ... the first ones, then every SPREAD-th call
SPREAD = 23
PICKS = 64                                               # samples of a note kept
BLOCK = 882                                              # 20 ms at 44.1 kHz: the RMS blocks of a note
WORKERS = 3
# the fighters whose looks call a shared picture helper, and the names they call it by in their module
PICTURE_USERS = {
    'CLARINET': ('_mask', 'blend', 'add', 'ribbon', 'beam_lane', 'draw_woodwind_shard', 'tail_theta'),
    'OBOE': ('_mask', 'blend', 'add', 'ribbon', 'ring_exit', 'beam_lane', 'draw_woodwind_shard', 'tail_theta'),
    'ALTO SAX': ('_mask', 'blend', 'add'),
    'TENOR SAX': ('_mask', 'blend', 'add'),
    'CELLO': ('_mask', 'blend', 'add'),
    'CONTRABASS': ('_mask', 'blend', 'add', 'padded_sprite'),
    'ELECTRIC': ('_mask', 'blend', 'add', 'beam_lane'),
    'SOUSAPHONE': ('_mask', 'blend', 'padded_sprite'),
    'BASS DRUM': ('_mask', 'blend', '_render'),
    'BASS CLARINET': ('_mask', 'blend'),
    'BASSOON': ('_mask', 'blend', 'add'),
    'EUPHONIUM': ('_mask', 'blend'),
    'ACOUSTIC': ('mask_of',),
    'TRUMPET': ('mask_of', 'gaussian_filter'),
    'HORN': ('mask_of',),
    'TUBA': ('mask_of',),
    'ORGAN': ('mask_of', 'line_mask'),
}
# which family of mask each fighter's mask function is (the code, not the name, decides)
MASK_FAMILY = {('ACOUSTIC', 'mask_of'): 'mask_of', ('TRUMPET', 'mask_of'): 'mask_of',
               ('HORN', 'mask_of'): 'mask_all', ('TUBA', 'mask_of'): 'mask_all',
               ('ORGAN', 'mask_of'): 'mask_polys', ('ORGAN', 'line_mask'): 'line_mask'}
SPRITE_MESHES = ('TROMBONE', 'FLUTE')                    # the models the Studio has: padded sprites are replayed on them
VOICE_DURS = (0.12, 0.25, 1.0)
VOICE_VELS = (0.6, 0.9)


# ---------------------------------------------------------------- small helpers
def sha(a):
    """SHA-256 of an array's bytes in C order, in its own dtype"""
    import numpy as np
    return hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()


def synthetic(n, dtype):
    """a picture's samples both sides can make: ((i * 7919 + 13) % 25601) / 100.0 (float64), then the dtype"""
    import numpy as np
    x = ((np.arange(n, dtype=np.int64) * 7919 + 13) % 25601) / 100.0
    return x.astype(dtype)


def synthetic_mask(n, dtype):
    """a mask's samples: ((j * 104729 + 7) % 256) / 255.0 (float64, 0 and 1 among them), then the dtype"""
    import numpy as np
    x = ((np.arange(n, dtype=np.int64) * 104729 + 7) % 256) / 255.0
    return x.astype(dtype)


def keep(counter, key, cap=CAP):
    """whether the n-th call of `key` is kept: the first KEEP_FIRST, then every SPREAD-th, at most `cap`"""
    n = counter.get(key, 0)
    counter[key] = n + 1
    kept = counter.get(('kept',) + key, 0)
    if kept >= cap or not (n < KEEP_FIRST or n % SPREAD == 0):
        return False
    counter[('kept',) + key] = kept + 1
    return True


def thirds(lane):
    """beam_lane's output, floor(...) / 3, as its whole numbers of thirds (checked to give the floats back)"""
    import numpy as np
    k = np.rint(np.asarray(lane, float) * 3)
    assert ((k / 3) == lane).all() and k.min() >= 0 and k.max() <= 3, 'beam_lane is not whole thirds'
    return k.astype(np.uint8)


def points(seq):
    """a list of point lists as (concatenated (m, 2) float64 points, their lengths int64)"""
    import numpy as np
    arrs = [np.asarray([tuple(map(float, p)) for p in pts], float).reshape(-1, 2) for pts in seq]
    lens = np.array([len(a) for a in arrs], np.int64)
    cat = np.concatenate(arrs) if arrs else np.zeros((0, 2))
    return np.ascontiguousarray(cat, float), lens


def mask_arrays(polys, lines):
    """a mask's lists as two arrays: `pts` ((m, 2) float64: the polygons' points, then the lines') and `lens`
    (int64: how many polygons, each polygon's length, each line's length)"""
    import numpy as np
    pp_, pl = points(polys)
    lp_, ll = points(lines)
    return dict(pts=np.concatenate([pp_, lp_]), lens=np.concatenate([[len(pl)], pl, ll]).astype(np.int64))


def cam_of(cam):
    """a kit camera's numbers: pos, M (3 x 3, rows right, up, forward), f, cx, cy, near, target"""
    import numpy as np
    return dict(pos=np.asarray(cam.pos, float).copy(), M=np.asarray(cam.M, float).copy(),
                scal=np.array([cam.f, cam.cx, cam.cy, cam.near], float), target=np.asarray(cam.target, float).copy())


def fingerprint(x):
    """a note's fingerprint: (length, block RMS, picks, SHA-256 of its float64 samples)"""
    import numpy as np
    x = np.asarray(x, float)
    n = len(x)
    nb = (n + BLOCK - 1) // BLOCK
    rms = np.array([math.sqrt(float(np.mean(x[b * BLOCK:(b + 1) * BLOCK] ** 2))) for b in range(nb)], float)
    idx = [(k * (n - 1)) // (PICKS - 1) for k in range(PICKS)] if n else []
    picks = np.array([x[i] for i in idx], float)
    return n, rms, picks, hashlib.sha256(np.ascontiguousarray(x, '<f8').tobytes()).hexdigest()


class Packed:
    """a vector file whose cases share one array per (call, role): each case's arrays are appended to the shared
    member `<call>.<role>` (flattened, C order), and the case's `at` gives each role's [offset, *shape] in it. One
    npz member per case and role would cost far more in zip and npy headers than the data"""

    def __init__(self, cases):
        self.c, self.parts = cases, {}

    def add(self, call, args, arrays, **more):
        import numpy as np
        at, roles = {}, {}
        for role, a in arrays.items():
            a = np.ascontiguousarray(a)
            name = f"{re.sub(r'[^A-Za-z0-9_]+', '_', call).strip('_')[:28]}.{role}.{a.dtype.str[1:]}"
            got = self.parts.setdefault(name, [a.dtype, 0, []])
            assert got[0] == a.dtype, (name, got[0], a.dtype)
            at[role] = [got[1]] + list(a.shape)
            got[1] += a.size
            got[2].append(a.ravel())
            roles[role] = name
        self.c.cases.append(dict(id=f'c{len(self.c.cases):03d}', call=call, args=args, arrays=roles, at=at, **more))

    def register(self):
        return self.c.register()

    def write(self):
        import numpy as np
        for name, (dt, n, parts) in self.parts.items():
            self.c.shared(name, np.concatenate(parts).astype(dt) if parts else np.zeros(0, dt))
        return self.c.write()


def gen_state(g):
    s = g.bit_generator.state
    assert s['bit_generator'] == 'PCG64' and s['has_uint32'] == 0, s
    return dict(state=common.dec(s['state']['state']), inc=common.dec(s['state']['inc']))


# ---------------------------------------------------------------- the picture: one fighter's fight, wrapped
def _worker_init():
    from . import __main__ as entry
    from . import m3refs
    for k in m3refs.THREAD_VARS:
        os.environ[k] = '1'
    assert 'numpy' not in sys.modules, 'numpy was imported before the worker set its thread count'
    assert hash(entry.HASH_PROBE[0]) == entry.HASH_PROBE[1], 'the worker does not hash strings as the reference does'


def _choose_seed(cfg, name):
    """the first seed where `name` (left) loses with its three abilities used; else the first it loses; else 1"""
    from . import m3refs, stats
    _, story = cfg.load()
    want = {f'ability/{a}' for a in m3refs.ability_names(name)}
    loses = None
    for seed in range(1, SEED_SCAN + 1):
        r, items = stats.row(story, seed)
        if r['winner'] == 1:
            if want <= set(items):
                return seed, True
            loses = loses or seed
    return (loses or 1), False


def _picture_child(name):
    """in a spawn worker: `name` against FLUTE, drawn with the shared helpers wrapped; returns the cases"""
    import numpy as np
    from . import m3refs
    t0 = time.time()
    cfg = m3refs.Config(f'm3_shared_{m3refs.slug(name)}', 'flute_vs_trombone', (name, 'FLUTE'), (1.0, 1.0), 1.0,
                        m3refs.pair_rules(name, 'FLUTE'))
    seed, all_three = _choose_seed(cfg, name)
    P = m3refs.fighter_picture(cfg.with_seed(seed))
    look = P.LOOKS[0]
    mod = sys.modules[type(look).__module__]
    from ballfight.kit.fighters import _reeds as R
    import scene as S
    out = dict(name=name, seed=seed, all_three=all_three, cases=[])
    counter = {}

    def add(fn, **case):
        out['cases'].append(dict(fn=fn, fighter=name, **case))

    orig = {}

    def wrap(where, fname, maker):
        f = getattr(where, fname)
        orig[(where.__name__, fname)] = f
        setattr(where, fname, maker(f))

    seen_masks = set()
    last_masks = {}

    def mask_case(fam, polys, lines, width, m):
        h = sha(m)
        last_masks.clear()                                 # the gaussian of TRUMPET's beam reads the mask just made
        last_masks[h] = (fam, polys, lines, width)
        if h not in seen_masks and keep(counter, (fam,)):
            seen_masks.add(h)
            add(fam, args=dict(width=width), arrays=mask_arrays(polys, lines), sha256=h, dtype=str(m.dtype),
                nonzero=int(np.count_nonzero(m)))
        return m

    def w_mask(f, fam):
        if fam == 'reeds':                               # _mask(polys, lines=(), width=1)
            def g(polys, lines=(), width=1):
                return mask_case(fam, polys, lines, width, f(polys, lines, width))
        elif fam == 'mask_of':                           # mask_of(polys, width=None)
            def g(polys, width=None):
                return mask_case(fam, polys, (), width, f(polys, width))
        elif fam in ('mask_all', 'mask_polys'):          # mask_of(polys)
            def g(polys):
                return mask_case(fam, polys, (), None, f(polys))
        else:                                            # line_mask(segs, width=1)
            def g(segs, width=1):
                return mask_case(fam, [], segs, width, f(segs, width))
        return g

    seen_blends = set()

    def w_blend(f, fn):
        def g(img, mk, col, alpha):
            what = (fn, str(img.dtype), str(mk.dtype), type(alpha).__name__, float(alpha),
                    tuple(np.asarray(col, float).ravel()))
            if what not in seen_blends and keep(counter, (fn, str(img.dtype), str(mk.dtype))):
                seen_blends.add(what)
                prod = (np.asarray(mk).ravel()[:1] * alpha).dtype if fn == 'blend' else None
                add(fn, args=dict(img=str(img.dtype), mk=str(mk.dtype), alpha=jarg(float(alpha)),
                                  alpha_type=type(alpha).__name__, prod=None if prod is None else str(prod)),
                    arrays=dict(col=np.asarray(col, float).ravel().copy()))
            return f(img, mk, col, alpha)
        return g

    def w_ribbon(f):
        def g(cam, Pp, hw):
            r = f(cam, Pp, hw)
            if keep(counter, ('ribbon',)):
                c = cam_of(cam)
                add('ribbon', args=dict(hw_scalar=bool(np.ndim(hw) == 0)),
                    arrays=dict(c, P=np.asarray(Pp, float).copy(), hw=np.atleast_1d(np.asarray(hw, float)).copy(),
                                out=np.asarray(r, float).copy()))
            return r
        return g

    def w_ring_exit(f):
        def g(o, d, A=None):
            r = f(o, d, A)
            if keep(counter, ('ring_exit',)):
                add('ring_exit', args=dict(A=None if A is None else jarg(float(A)), S_A=jarg(float(S.A)),
                                           out=jarg(float(r))),
                    arrays=dict(o=np.asarray(o, float)[:2].copy(), d=np.asarray(d, float)[:2].copy()))
            return r
        return g

    def lane_grid():
        """canvas pixels both sides can make: a 48 x 48 grid across the ring and past the ropes"""
        A = float(S.A) + 0.3
        u = np.linspace(-A, A, 48)
        sx = np.repeat(u, 48)
        sz = np.tile(u, 48)
        idx = np.arange(48 * 48)
        return sx, sz, (idx // 48) % 640, (idx * 7) % 360

    def w_beam_lane(f):
        def g(sxp, szp, o, aim, length, width, ys, xs):
            r = f(sxp, szp, o, aim, length, width, ys, xs)
            if keep(counter, ('beam_lane',)):
                sx, sz, yy, xx = lane_grid()
                syn = f(sx, sz, o, aim, length, width, yy, xx)
                add('beam_lane', args=dict(aim=jarg(float(aim)), length=jarg(float(length)),
                                           width=jarg(float(width))),
                    arrays=dict(o=np.asarray(o, float)[:2].copy(), thirds=thirds(syn)))
            return r
        return g

    def w_shard(f):
        def g(P_, arr, cam, kind, p, rot, size):
            if keep(counter, ('draw_woodwind_shard',)):
                shard_case(f, P_, cam, kind, p, rot, size, 'recorded')
            return f(P_, arr, cam, kind, p, rot, size)
        return g

    def shard_case(f, P_, cam, kind, p, rot, size, why):
        arr = synthetic(640 * 360 * 3, np.float32).reshape(640, 360, 3)
        f(P_, arr, cam, kind, p, rot, size)
        add('draw_woodwind_shard', args=dict(kind=kind, rot=jarg(float(rot)), size=jarg(float(size)), why=why),
            arrays=dict(cam_of(cam), p=np.asarray(p, float).copy()), sha256=sha(arr))

    def w_tail(f):
        def g(P_, k, st, t, gl):
            r = f(P_, k, st, t, gl)
            if keep(counter, ('tail_theta',)):
                add('tail_theta', args=dict(th_end=jarg(float(P_.RUN['TH'][-1, k])), th0=jarg(float(P_.TH0[k])),
                                            gl=jarg(float(gl)), th=jarg(float(st['th']))), arrays={})
            return r
        return g

    def w_padded(f, fn):
        import sprites as SP
        from ballfight.kit.fighters import contrabass as CB

        def g(*a, **kw):
            r = f(*a, **kw)
            if keep(counter, (fn,)):
                if fn == 'padded_sprite':
                    nm, rr, yaw, roll = a[:4]
                    pad = a[4] if len(a) > 4 else kw.get('pad', CB.PAD)
                else:
                    _, rr, yaw, roll = a[:4]
                    pad = a[4] if len(a) > 4 else kw.get('pad', mod.PAD)
                replay = {m: sha(CB.padded_sprite(m, rr, yaw, roll, pad)) for m in SPRITE_MESHES}
                add('padded', args=dict(caller=fn, r=int(rr), yaw=jarg(float(yaw)), roll=jarg(float(roll)),
                                        pad=jarg(float(pad)), replay=replay, own=sha(r)),
                    arrays=dict(side=np.array([r.shape[0]], np.int64)))
            return r
        return g

    def w_gauss(f):
        def g(inp, sigma, *a, **kw):
            r = f(inp, sigma, *a, **kw)
            src = last_masks.get(sha(inp))
            if src is not None and keep(counter, ('gaussian_filter',)):
                fam, polys, lines, width = src
                add('gaussian_filter', args=dict(sigma=jarg(float(sigma)), extra=len(a) + len(kw),
                                                 dtype=str(inp.dtype), shape=list(inp.shape), mask=fam,
                                                 width=width),
                    arrays=mask_arrays(polys, lines), input_sha256=sha(inp), sha256=sha(r), out_dtype=str(r.dtype))
            return r
        return g

    names = PICTURE_USERS[name]
    for fname in names:
        if fname == 'gaussian_filter':
            import scipy.ndimage as ND
            wrap(ND, 'gaussian_filter', w_gauss)
            continue
        if not hasattr(mod, fname):
            raise SystemExit(f'shared: {name}\'s look module has no {fname}')
        if fname == '_mask':
            wrap(mod, fname, lambda f: w_mask(f, 'reeds'))
        elif fname in ('mask_of', 'line_mask'):
            wrap(mod, fname, lambda f, fam=MASK_FAMILY[(name, fname)]: w_mask(f, fam))
        elif fname in ('blend', 'add'):
            wrap(mod, fname, lambda f, fn=fname: w_blend(f, fn))
        elif fname == 'ribbon':
            wrap(mod, fname, w_ribbon)
        elif fname == 'ring_exit':
            wrap(mod, fname, w_ring_exit)
        elif fname == 'beam_lane':
            wrap(mod, fname, w_beam_lane)
        elif fname == 'draw_woodwind_shard':
            wrap(mod, fname, w_shard)
        elif fname == 'tail_theta':
            wrap(mod, fname, w_tail)
        elif fname in ('padded_sprite', '_render'):
            wrap(mod, fname, lambda f, fn=fname: w_padded(f, fn))
    if 'beam_lane' in names:                                  # beam_lane's own call of ring_exit
        wrap(R, 'ring_exit', w_ring_exit)

    # the frames: every second, each ability 0.2 s and 0.6 s after its first use, the K.O. and its shards
    FPS = 30
    times = [s + 0.5 for s in range(61)]
    firsts = {}
    for e in P.EV:
        if e[1] == 'ability' and e[2] == 0 and e[3] not in firsts:
            firsts[e[3]] = e[0]
    for t in firsts.values():
        times += [t + 0.2, t + 0.6]
    times += [P.T_KO + 0.1, P.T_KO + 0.4, P.T_KO + 0.9]
    frames = sorted({min(int(round(t * FPS)), 61 * FPS - 1) for t in times if t >= 0})
    for f in frames:
        P.render(f)
    out['frames'] = len(frames)
    out['abilities'] = sorted(firsts)

    if name == 'CLARINET':                                    # every kind of woodwind shard, wherever it falls
        f = orig.get((mod.__name__, 'draw_woodwind_shard'), R.draw_woodwind_shard)
        for i, kind in enumerate(('wood', 'reed', 'key', 'bell')):
            for j, (t, rot, size) in enumerate(((P.T_KO + 0.3, 0.3 + i, 0.05), (P.T_KO + 0.8, -2.1 + i, 0.09),
                                                (20.0, 3.0 * i + 0.7, 0.07))):
                cam = P.camera(t)
                p = (0.4 * i - 0.7 + 0.2 * j, 0.15 + 0.1 * j, 0.5 - 0.3 * i)
                shard_case(f, P, cam, kind, p, rot, size, 'made')
    print(f'shared: {name} vs FLUTE seed {seed} (loses with all three abilities: {all_three}), {len(frames)} frames, '
          f'{len(out["cases"])} cases, {time.time() - t0:.0f} s', flush=True)
    return out


# ---------------------------------------------------------------- the voice blocks, in one fresh interpreter
def _sound_child():
    """in a spawn worker: the voice blocks wrapped while their users' voices play, then the edge cases"""
    import numpy as np
    from . import m3refs
    E, _ = common.kit()
    E.activate_paths()
    import fighters
    import pp_bridge as B
    out = dict(cases=[], starts={})
    users = ('TRUMPET', 'TUBA', 'HORN', 'ACOUSTIC', 'ELECTRIC', 'ALTO SAX', 'BASS CLARINET', 'EUPHONIUM', 'CELLO')
    alto = [e[0] for e in B.ENTRANTS if e[1] == 'altosax60_lowpoly']
    assert len(alto) == 1, alto
    for nm in users:
        B.module(B.entrant(fighters.FIGHTERS[nm]['pp'])[1])
        B.scene(fighters.FIGHTERS[nm]['pp'])
    B.module('altosax60_lowpoly')
    B.scene(alto[0])
    import trumpet_lowpoly as T
    import piano_lowpoly as PL
    import altosax60_lowpoly as AS
    import drums60_lowpoly as D
    import violin_lowpoly as V
    import bassclarinet_lowpoly as BCL
    # the series' module generators where a soundtrack starts: a fresh generator of each one's seed (the imports
    # draw nothing)
    for label, g, seed in (('drums60_lowpoly._rng', D._rng, 77), ('altosax60_lowpoly._RN', AS._RN, 23),
                           ('violin_lowpoly.rng', V.rng, 9)):
        st = gen_state(g)
        assert st == gen_state(np.random.default_rng(seed)), f'{label} is not a fresh generator of seed {seed}'
        out['starts'][label] = dict(seed=seed, **st)
    who = {'v': None}
    counter = {}

    def rec(fn, args, x, **more):
        n, rms, picks, h = fingerprint(x)
        bad = int(np.count_nonzero(~np.isfinite(np.asarray(x, float))))
        arrays = dict(rms=rms, picks=picks) if bad == 0 else {}
        out['cases'].append(dict(fn=fn, user=who['v'], args=args, n=n, sha256=h, nonfinite=bad, arrays=arrays,
                                 **more))

    brass0, pluck0, sax0 = T.brass, PL.ks_pluck, AS.sax_tone

    def brass(f, dur, vel, bright=1., vib=.004, scoop=.03, rough=0., tilt=.75):
        x = brass0(f, dur, vel, bright, vib, scoop, rough, tilt)
        if keep(counter, ('brass', who['v']), SOUND_CAP):
            rec('brass', dict(f=jarg(float(f)), dur=jarg(float(dur)), vel=jarg(float(vel)), bright=jarg(float(bright)),
                              vib=jarg(float(vib)), scoop=jarg(float(scoop)), rough=jarg(float(rough)),
                              tilt=jarg(float(tilt))), x)
        return x

    def ks_pluck(f, dur, bright=0.5, decay=0.996, seed=0):
        x = pluck0(f, dur, bright, decay, seed)
        if keep(counter, ('ks_pluck', who['v']), SOUND_CAP):
            rec('ks_pluck', dict(f=jarg(float(f)), dur=jarg(float(dur)), bright=jarg(float(bright)),
                                 decay=jarg(float(decay)), seed=int(seed)), x)
        return x

    def sax_tone(f, dur, v, **kw):
        before = gen_state(AS._RN)
        x = sax0(f, dur, v, **kw)
        if keep(counter, ('sax_tone', who['v']), SOUND_CAP):
            rec('sax_tone', dict(f=jarg(float(f)), dur=jarg(float(dur)), v=jarg(float(v)),
                                 **{k: (None if val is None else jarg(val)) for k, val in kw.items()}), x,
                rn_before=before, rn_after=gen_state(AS._RN))
        return x

    T.brass, PL.ks_pluck, AS.sax_tone = brass, ks_pluck, sax_tone
    # (the mambo bed's alto voice is named only by its module here: its roster name is a retired fighter's)
    for nm, pp in [(u, fighters.FIGHTERS[u]['pp']) for u in users[:6]] + [('mambo alto', alto[0])]:
        lo, hi = B.entrant(pp)[3]
        who['v'] = nm
        for midi in (lo, lo + (hi - lo) // 3, lo + 2 * (hi - lo) // 3, hi):
            for vel in VOICE_VELS:
                for dur in VOICE_DURS:
                    B.voice(pp, midi, vel, dur)
    T.brass, PL.ks_pluck, AS.sax_tone = brass0, pluck0, sax0

    # the edge cases, straight to the functions
    who['v'] = 'edge'
    CAPS = dict(counter)
    counter.clear()
    for f, dur, vel, kw in ((440.0, 0.001, 0.8, {}), (440.0, 0.0, 0.8, {}), (220.0, -0.05, 0.9, {}),
                            (7500.0, 0.2, 0.8, {}), (3600.0, 0.3, 1.0, dict(rough=0.25, bright=0.8)),
                            (55.0, 0.5, 0.95, dict(tilt=1.1, vib=0.0, scoop=0.0)), (260.0, 0.4, 1.4, {})):
        x = brass0(f, dur, vel, **kw)
        rec('brass', dict(f=jarg(f), dur=jarg(dur), vel=jarg(vel),
                          **{k: jarg(float(v)) for k, v in dict(dict(bright=1., vib=.004, scoop=.03, rough=0.,
                                                                     tilt=.75), **kw).items()}), x)
    for f, dur, bright, decay, seed in ((440.0, 0.001, 0.5, 0.996, 0), (82.4, 0.001, 0.5, 0.996, 3),
                                        (110.0, 0.4, 0.0, 0.997, 11), (3999.0, 0.2, 1.0, 0.999, 2),
                                        (1000.0, 0.05, 0.35, 0.9978, 7), (44100.0 / 7, 0.02, 0.9, 0.998, 5),
                                        (65.4, 1.0, 0.75, 0.9988, 1234)):
        x = pluck0(f, dur, bright, decay, seed)
        rec('ks_pluck', dict(f=jarg(f), dur=jarg(dur), bright=jarg(bright), decay=jarg(decay), seed=seed), x)
    for f, dur, v, kw in ((392.0, 0.3, 0.8, dict(gl=(-2.0, 0.08))), (196.0, 0.5, 0.9, dict(growl=0.3, slap=0.55)),
                          (523.25, 0.12, 0.7, dict(breath=0.0)), (880.0, 0.0, 0.8, {}),
                          (7000.0, 0.2, 0.9, dict(bright=0.82, odd=0.3)), (330.0, 0.4, 1.0, dict(slap=0.4))):
        before = gen_state(AS._RN)
        x = sax0(f, dur, v, **kw)
        rec('sax_tone', dict(f=jarg(f), dur=jarg(dur), v=jarg(v),
                             **{k: (list(map(jarg, val)) if isinstance(val, tuple) else jarg(val))
                                for k, val in kw.items()}), x, rn_before=before, rn_after=gen_state(AS._RN))
    # bassclarinet_lowpoly.slap as both parts call it: slap(_midi_hz(m), vel) over each register, and edge cases
    for nm in ('BASS CLARINET', 'EUPHONIUM'):
        lo, hi = fighters.FIGHTERS[nm]['register']
        who['v'] = nm
        for m in (lo, (lo + hi) // 2, hi):
            for vel in (0.7, 1.0):
                x = BCL.slap(BCL._midi_hz(m), vel)
                rec('slap', dict(f=jarg(float(BCL._midi_hz(m))), v=jarg(float(vel)), dur=jarg(0.09), m=m), x)
    who['v'] = 'edge'
    for f, v, dur in ((55.0, 1.0, 0.0), (1760.0, 0.5, 0.2), (98.0, 0.9, -0.05)):
        x = BCL.slap(f, v, dur)
        rec('slap', dict(f=jarg(f), v=jarg(v), dur=jarg(dur), m=None), x)
    # drums60_lowpoly.bp on the series' own noise, at the bands sax_tone uses and two more
    for j, (lo_, hi_) in enumerate(((1600, 6500), (800, 4000), (900, 7000), (60.0, 21000.0))):
        x = np.random.default_rng(5 + j).normal(0, 1, 3000)          # the input: Rust draws the same normals
        y = D.bp(x, lo_, hi_)
        n, rms, picks, h = fingerprint(y)
        out['cases'].append(dict(fn='bp', user='edge', args=dict(lo=jarg(float(lo_)), hi=jarg(float(hi_)),
                                                                 noise_seed=5 + j, noise_n=3000),
                                 n=n, sha256=h, nonfinite=0, arrays=dict(rms=rms, picks=picks)))
    # the parts' hash: h01_29 (trumpet), h01_33 (acoustic), _h01_34 (horn), h01_35 (organ): the same code
    from ballfight.kit.fighters import trumpet as TR, acoustic as AC, horn as HO, organ as OR
    h01s = (TR.h01_29, AC.h01_33, HO._h01_34, OR.h01_35)
    rows = []
    for xs in ((12.345, 4), (0.0,), (), (7,), (3, 5), (-2.5, 1), (61.0, 7), (1e-4, 3), (123456.789, 2),
               (0.0011, 0.0019, 0.0020), (-1e-9, 5), (2.9999999, 4), (1e9, 1e9, 3), (59.875, 0.5, 7, 11)):
        vals = {f.__name__: f(*xs) for f in h01s}
        assert len(set(vals.values())) == 1, (xs, vals)
        rows.append(dict(xs=[jarg(x) for x in xs], out=jarg(vals['h01_29'])))
    out['h01'] = rows
    out['caps'] = {f'{k[0]}/{k[1]}': n for k, n in CAPS.items() if k[0] != 'kept'}
    return out


# ---------------------------------------------------------------- ndimage: synthetic cases and the kernels
def ndimage_cases(c):
    import numpy as np
    from scipy.ndimage import gaussian_filter
    from scipy.ndimage._filters import _gaussian_kernel1d
    sigmas = (0.5, 0.9, 1.0, 1.5, 2.4, 3.0, 4.5, 6.0, 0.125, 0.12, 0.3, 1e-3)
    for s in sigmas:
        lw = int(4.0 * s + 0.5)
        w = _gaussian_kernel1d(s, 0, lw)[::-1]
        c.add('_gaussian_kernel1d(sigma, 0, int(4.0 * sigma + 0.5))[::-1]', dict(sigma=jarg(s), radius=lw),
              dict(weights=np.ascontiguousarray(w)))
    shapes = ((1, 1), (1, 7), (7, 1), (5, 3), (6, 4), (13, 9), (2, 40), (31, 2))
    for i, (h, w) in enumerate(shapes):
        for dt in (np.float32, np.float64):
            x = (synthetic(h * w, np.float64) / 255.0).astype(dt).reshape(h, w)
            for s in (0.5, 1.0, 2.4, 6.0) if i % 2 == 0 else (0.9, 3.0, 0.125):
                y = gaussian_filter(x, s)
                assert y.dtype == x.dtype
                c.add('scipy.ndimage.gaussian_filter(x, sigma)', dict(sigma=jarg(s), dtype=str(x.dtype)),
                      dict(x=x.copy(), out=y))
    for dt in (np.float32, np.float64):                        # sigma 0 and below the 1e-15 floor: a copy
        x = (synthetic(20, np.float64) / 255.0).astype(dt).reshape(4, 5)
        for s in (0.0, 1e-16, 1e-3):
            c.add('scipy.ndimage.gaussian_filter(x, sigma)', dict(sigma=jarg(s), dtype=str(x.dtype)),
                  dict(x=x.copy(), out=gaussian_filter(x, s)))
    # full frames: a mask of TRUMPET's kind and a busy picture, by their SHA-256
    for dt in (np.float32, np.float64):
        x = synthetic_mask(640 * 360, dt).reshape(640, 360)
        for s in (2.4, 0.9, 0.5, 6.0):
            y = gaussian_filter(x, s)
            c.add('scipy.ndimage.gaussian_filter(synthetic_mask, sigma)', dict(sigma=jarg(s), dtype=str(x.dtype),
                                                                             shape=[640, 360], sha256=sha(y)),
                  dict(corner=np.ascontiguousarray(y[:3, :3])))


# ---------------------------------------------------------------- writing the files
def _write_reeds(pictures):
    import numpy as np
    c = Packed(Cases(os.path.join(OUT, 'reeds.npz'),
              'kit/fighters/_reeds.py and the same-code mask and blend helpers of the other looks, each call as the '
              'kit made it while a fighter drew a fight against FLUTE (the "fighter" of each case); blend and add '
              'on synthetic pictures and masks (shared_vectors.synthetic, synthetic_mask: 37 x 23 here), '
              'draw_woodwind_shard on a synthetic float32 frame', 'shared', __name__,
              synthetic_shape=[37, 23]))
    n = 0
    taken = {}
    for pic in pictures:
        for case in pic['cases']:
            fn = case['fn']
            if fn in ('padded', 'gaussian_filter'):
                continue
            key = (case['fighter'], fn)                  # a thinner spread of the commonest calls (size)
            taken[key] = taken.get(key, 0) + 1
            if taken[key] > WRITE_CAP.get(fn, CAP) and case.get('args', {}).get('why') != 'made':
                continue
            arrays = dict(case.get('arrays', {}))
            more = {k: v for k, v in case.items() if k not in ('fn', 'args', 'arrays', 'fighter')}
            args = dict(case.get('args', {}))
            if fn in ('blend', 'add'):
                # the kit's own function on synthetic inputs of the recorded dtypes
                from ballfight.kit.fighters import _reeds as R
                h, w = 37, 23
                img = synthetic(h * w * 3, np.dtype(args['img'])).reshape(h, w, 3)
                mk = synthetic_mask(h * w, np.dtype(args['mk'])).reshape(h, w)
                alpha = common_alpha(args)
                (R.blend if fn == 'blend' else R.add)(img, mk, arrays['col'], alpha)
                more['sha256'] = sha(img)
                more['out_dtype'] = str(img.dtype)
            c.add(fn, args, arrays, fighter=case['fighter'], **more)
            n += 1
    # blend and add at the corners: alpha 0 and below (blend returns at once), 1, NaN; both dtypes
    from ballfight.kit.fighters import _reeds as R
    for fn in ('blend', 'add'):
        for dt_img in ('float32', 'float64'):
            for dt_mk in ('float32', 'float64'):
                for alpha, kind in ((0.0, 'float'), (-0.25, 'float'), (0.37, 'float'), (0.37, 'float64'),
                                    (float('nan'), 'float')):
                    h, w = 37, 23
                    img = synthetic(h * w * 3, np.dtype(dt_img)).reshape(h, w, 3)
                    mk = synthetic_mask(h * w, np.dtype(dt_mk)).reshape(h, w)
                    a = np.float64(alpha) if kind == 'float64' else alpha
                    col = np.array([255.0, 12.5, 3e2])
                    (R.blend if fn == 'blend' else R.add)(img, mk, col, a)
                    prod = (mk.ravel()[:1] * a).dtype
                    c.add(fn, dict(img=dt_img, mk=dt_mk, alpha=jarg(alpha), alpha_type=type(a).__name__,
                                   prod=str(prod) if fn == 'blend' else None), dict(col=col), fighter='edge',
                          sha256=sha(img), out_dtype=str(img.dtype))
    # masks at the edges: nothing, a polygon of 2 points, a line of 1, wide lines, points off the frame
    from ballfight.kit.fighters import acoustic as AC, horn as HO, organ as OR
    edge = [([], (), 1), ([[(10, 10), (40, 12)]], (), 1), ([], [[(5.5, 7.25)]], 3),
            ([[(-50.0, -20.0), (400.0, 30.0), (180.0, 700.0)]], [[(0, 0), (359.9, 639.9)]], 4),
            ([[(100.25, 100.75), (130.5, 101.0), (120.0, 140.0), (99.0, 135.5)]], [[(3, 3), (3, 3)]], 0),
            ([[(12.0, 30.0), (60.0, 30.0), (36.0, 70.0)]], [[(1.0, 600.0), (300.0, 2.0), (350.0, 630.0)]], 2)]
    for polys, lines, width in edge:
        m = R._mask(polys, lines, width)
        c.add('reeds', dict(width=width), mask_arrays(polys, lines),
              fighter='edge', sha256=sha(m), dtype=str(m.dtype), nonzero=int(np.count_nonzero(m)))
        if polys and all(len(p) >= 2 for p in polys):
            for fam, fnc in (('mask_all', HO.mask_of), ('mask_polys', OR.mask_of)):
                m = fnc(polys)
                c.add(fam, dict(width=None), mask_arrays(polys, []),
                      fighter='edge', sha256=sha(m), dtype=str(m.dtype), nonzero=int(np.count_nonzero(m)))
        for wd in (None, width):
            m = AC.mask_of(polys + [list(q) for q in lines], wd)
            c.add('mask_of', dict(width=wd), mask_arrays(polys + [list(q) for q in lines], []),
                  fighter='edge', sha256=sha(m), dtype=str(m.dtype), nonzero=int(np.count_nonzero(m)))
        if lines:
            segs = [q for q in lines if len(q) == 2]
            if segs:
                m = OR.line_mask(segs, max(1, width))
                c.add('line_mask', dict(width=max(1, width)), mask_arrays([], segs),
                      fighter='edge', sha256=sha(m), dtype=str(m.dtype), nonzero=int(np.count_nonzero(m)))
    # ribbon at its edges: two points, repeated points, a width per point
    import scene as S
    from reelkit.camera import Cam
    cam = Cam((0.0, 0.3, 0.0), 6.2, 18.0, 31.0, 34.0)
    for Pp, hw in ((np.array([[0.0, 0.0, 0.2], [0.5, 0.25, 0.2]]), 0.1),
                   (np.array([[0.3, -0.2, 0.1], [0.3, -0.2, 0.1], [0.9, 0.4, 0.3]]), 0.07),
                   (np.array([[-1.0, 0.5, 0.2], [-0.4, 0.9, 0.25], [0.2, 1.0, 0.3], [0.9, 0.6, 0.3]]),
                    np.array([0.02, 0.05, 0.08, 0.11]))):
        r = R.ribbon(cam, Pp, hw)
        c.add('ribbon', dict(hw_scalar=bool(np.ndim(hw) == 0)),
              dict(cam_of(cam), P=Pp, hw=np.atleast_1d(np.asarray(hw, float)).copy(), out=np.asarray(r, float)),
              fighter='edge')
    for o, d, A in (((0.0, 0.0), (1.0, 0.0), None), ((0.5, -0.5), (1e-10, 1.0), None), ((3.0, 3.0), (1.0, 1.0), None),
                    ((0.2, 0.1), (-0.6, 0.8), 1.0), ((0.0, 0.0), (0.0, 0.0), None), ((-1.0, 2.0), (-1.0, -1e-9), 2.5)):
        r = R.ring_exit(np.array(o), np.array(d), A)
        c.add('ring_exit', dict(A=None if A is None else jarg(float(A)), S_A=jarg(float(S.A)), out=jarg(float(r))),
              dict(o=np.array(o, float), d=np.array(d, float)), fighter='edge')
    c.write()
    return c, n


def common_alpha(args):
    """the alpha a case recorded, as the kit passed it (a Python float, or a numpy float64 scalar)"""
    import numpy as np
    a = float.fromhex(args['alpha']) if isinstance(args['alpha'], str) else float(args['alpha'])
    return np.float64(a) if args['alpha_type'] == 'float64' else a


def _write_padded(pictures):
    import numpy as np
    from ballfight.kit.fighters import contrabass as CB
    c = Packed(Cases(os.path.join(OUT, 'padded.npz'),
              'contrabass.padded_sprite (the CONTRABASS and the SOUSAPHONE) and bass_drum._render, each call as the '
              'kit made it while the fighter drew a fight against FLUTE, replayed on the models the Studio has '
              '(TROMBONE, FLUTE): the SHA-256 of the RGBA bytes', 'shared', __name__))
    for pic in pictures:
        for case in pic['cases']:
            if case['fn'] == 'padded':
                c.add('padded', case['args'], case['arrays'], fighter=case['fighter'])
    for r, yaw, roll, pad in ((12, 0.0, 0.0, 1.25), (20, 30.0, -12.0, 1.22), (16, -45.0, 5.0, 1.0),
                              (24, 200.0, 0.0, 1.1), (18, 10.0, 90.0, 1.25), (12, 20.0, 0.0, 1.22)):
        sprites = {m: CB.padded_sprite(m, r, yaw, roll, pad) for m in SPRITE_MESHES}
        c.add('padded', dict(caller='edge', r=r, yaw=jarg(yaw), roll=jarg(roll), pad=jarg(pad),
                             replay={m: sha(x) for m, x in sprites.items()}),
              dict(side=np.array([sprites['TROMBONE'].shape[0]], np.int64)), fighter='edge')
    c.write()
    return c


def _write_ndimage(pictures):
    import numpy as np
    c = Packed(Cases(os.path.join(OUT, 'ndimage.npz'),
              'scipy.ndimage.gaussian_filter (mode reflect, truncate 4.0, the input\'s dtype) and its kernel weights: '
              'synthetic pictures (shared_vectors.synthetic / 255), the full-frame mask of synthetic_mask by its '
              'SHA-256, and every call TRUMPET\'s look made (its input is the mask_of case in reeds.npz with that '
              'SHA-256)', 'shared', __name__))
    ndimage_cases(c)
    for pic in pictures:
        for case in pic['cases']:
            if case['fn'] == 'gaussian_filter':
                more = {k: v for k, v in case.items() if k not in ('fn', 'args', 'arrays', 'fighter')}
                c.add('gaussian_filter', case['args'], case['arrays'], fighter=case['fighter'], **more)
    c.write()
    return c


def _write_pp(snd):
    import numpy as np
    c = Packed(Cases(os.path.join(OUT, 'pp_blocks.npz'),
              'the Pixel Pitch voice blocks several kit voices share, each call as the kit made it while its users\' '
              'voices played (pp_bridge.voice, 4 pitches x velocities x held lengths) or an edge case; a note as '
              'its fingerprint (length, the RMS of each 882-sample block, 64 samples at (k * (n - 1)) // 63, '
              'the SHA-256 of its float64 samples); sax_tone with the state of altosax60_lowpoly._RN around it',
              'shared', __name__, starts=snd['starts'], h01=snd['h01'], caps=snd['caps']))
    for case in snd['cases']:
        more = {k: v for k, v in case.items() if k not in ('fn', 'args', 'arrays')}
        c.add(case['fn'], case['args'], case['arrays'], **more)
    c.write()
    return c


def run():
    import numpy as np                                    # noqa: F401  (after the environment check of __main__)
    from . import m3refs
    t0 = time.time()
    for k in m3refs.THREAD_VARS:
        os.environ[k] = '1'
    names = list(PICTURE_USERS)
    with mp.get_context('spawn').Pool(WORKERS, initializer=_worker_init, maxtasksperchild=1) as pool:
        snd_job = pool.apply_async(_sound_child)
        pictures = pool.map(_picture_child, names, chunksize=1)
        snd = snd_job.get()
    print(f'shared: pictures and voices collected in {time.time() - t0:.0f} s', flush=True)
    common.kit()[0].activate_paths()
    files = [_write_reeds(pictures)[0].c, _write_padded(pictures), _write_ndimage(pictures), _write_pp(snd)]
    for f in files:
        f.register()
    seeds = ', '.join(f"{p['name']} {p['seed']}{'' if p['all_three'] else ' (not all three)'}" for p in pictures)
    print(f'shared: fights (seed): {seeds}')
    print(f'shared: {time.time() - t0:.0f} s')


EXPORTS = {'shared': run}

"""older_vectors: the shared helpers of the older looks and the picture opt-ins (plan m4, Task 4).

    python -m tools.studio_export older_vectors      (about 15 minutes: seven episodes draw a frame a second)

writes, under studio/reference/older/vectors/ (top-level manifest):

helpers.json   the episode helpers more than one older look calls (studio/crates/bf-content/src/fighters/shared/
               older.rs), each on the arguments the episodes give it. The arguments are collected by wrapping the
               helpers (at run time, in the episode's module) while each episode's own render.py draws one frame a
               second in its scratch copy (older/origin.py: studio/reference/.cache/m4/origin/<episode>/; nothing under
               projects/ is written), the first few calls of each helper and episode then a spread, plus edge cases:
                 mask_of     the three families the episodes keep (`all`: every list filled, #30 and #28; `polys`: lists
                             of 3 points or more, #31 and #35; `lines`: with a width, #33): the mask's SHA-256
                 blend       the three families (`into`, #30; `early`: no blend for alpha <= 0, #39; `clip`: the colour
                             clipped first, #33) on synthetic pictures and masks of the recorded dtypes (the formulas
                             of the shared export, shared_vectors.synthetic*): the result's SHA-256
                 annulus_polys   #30's (with an arc a0..a1; #28's is it with the defaults): the camera's numbers and
                             every quad's points
                 dart_quads  #28 = #35 = #33 (the same code): the head's and the body's world points
                 draw_dart   #28's (VIOLIN's PIZZICATO darts) and #35's (HARP's): each recorded dart drawn on a fresh
                             frame of the example episode at t = 20 s (the kit's camera and lights there, both looks the
                             base class): the frame's bytes and depth buffer by their SHA-256
optins.json    the picture opt-ins (older/picture_patches.py, bf-picture's `Look` methods of the same names):
                 off         with the patches installed, the kit draws exactly what it drew before: the example episode
                             with the base-class look for both fighters gives every frame of picture_ring/unit.json;
                             three kit examples with their own looks give the same frames and draw logs patched and
                             unpatched (checked here, the hashes kept)
                 on          the example episode with the base-class look for FLUTE and a stub look for TROMBONE that
                             uses every opt-in (its arithmetic written out in the Rust test's stub, exactly): each
                             frame's SHA-256 and its draw log's, how many times each opt-in changed something, and the
                             rope events after the stub's were added

The children run in fresh interpreters (`python -m tools.studio_export.older_vectors <mode> <out>`): kit.picture is
built at import for the looks it finds, so each mode needs a process of its own."""
import hashlib
import json
import math
import os
import subprocess
import sys

from . import common

OUT = common.ref('older', 'vectors')
EXAMPLE = 'flute_vs_trombone'
OFF_EXAMPLES = ('flute_vs_trombone', 'bassclarinet_vs_euphonium', 'trombone_vs_horn')
OFF_FRAMES = (0, 60, 108, 150, 450, 600, 900, 1200, 1350, 1500, 1520, 1600, 1700, 1800)
STUB_SIDE = 1                                    # the stub look draws the TROMBONE's side
DART_T = 20.0                                    # draw_dart's replays: the example's camera and lights at this time
CAP = 10                                         # recorded calls kept per helper, family and episode
KEEP_FIRST = 4
SPREAD = 37
THREADS = ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS')
# the episodes whose looks call a shared helper, and (module attribute path, helper, family) wrapped there
EPISODES = {
    'ballfight_bagpipes_vs_accordion': (('ST', 'mask_of', 'all'), ('ST', 'blend', 'into'),
                                        ('ST', 'annulus_polys', 'arc')),
    'ballfight_drums_vs_violin': (('ST', 'mask_of', 'all'), ('ST', 'annulus_polys', 'ring'),
                                  ('ST', 'dart_quads', 'dart'), ('ST', 'draw_dart', 'violin')),
    'ballfight_viola_vs_violin': (('ST.SV', 'dart_quads', 'dart'), ('ST.SV', 'draw_dart', 'violin')),
    'ballfight_turntables_vs_synth': (('ST', 'mask_of', 'polys'),),
    'ballfight_organ_vs_harp': (('ST', 'mask_of', 'polys'), ('ST', 'dart_quads', 'dart'), ('ST', 'draw_dart', 'harp')),
    'ballfight_ukulele_vs_acoustic': (('ST', 'mask_of', 'lines'), ('ST', 'blend', 'clip')),
    'ballfight_bassoon_vs_barisax': (('ST', 'blend', 'early'),),
}
DART_STYLES = {'violin': 'ballfight_drums_vs_violin', 'harp': 'ballfight_organ_vs_harp'}   # whose styles.py draws it


def sha(a):
    import numpy as np
    return hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()


def hx(x):
    import numpy as np
    if isinstance(x, (list, tuple, np.ndarray)):
        return [hx(v) for v in x]
    return float(x).hex()


def num(v):
    """how the kit passed a number: ['py', hex] for a Python float or int, ['f64', hex] for a numpy float64 scalar,
    ['f32', hex] for a numpy float32 scalar"""
    import numpy as np
    if isinstance(v, np.float32):
        return ['f32', float(v).hex()]
    if isinstance(v, np.floating):
        return ['f64', float(v).hex()]
    return ['py', float(v).hex()]


def unnum(v):
    import numpy as np
    kind, x = v
    x = float.fromhex(x)
    return {'py': x, 'f64': np.float64(x), 'f32': np.float32(x)}[kind]


def cam_of(cam):
    import numpy as np
    return dict(pos=hx(np.asarray(cam.pos, float)), M=hx(np.asarray(cam.M, float)), f=hx(cam.f), cx=hx(cam.cx),
                cy=hx(cam.cy), near=hx(cam.near), target=hx(np.asarray(cam.target, float)))


def kept(counter, key):
    n = counter.get(key, 0)
    counter[key] = n + 1
    k = counter.get(('kept',) + key, 0)
    if k >= CAP or not (n < KEEP_FIRST or n % SPREAD == 0):
        return False
    counter[('kept',) + key] = k + 1
    return True


# ---------------------------------------------------------------- helpers: one episode, wrapped (an m3parts child)
def _blend_case(fn, family, arr, m, col, k):
    """the family's blend on synthetic arrays of the recorded dtypes: the case (with the result's SHA-256)"""
    import numpy as np
    from .shared_vectors import synthetic, synthetic_mask
    a = synthetic(640 * 360 * 3, arr.dtype).reshape(640, 360, 3)
    mk = synthetic_mask(640 * 360, m.dtype).reshape(640, 360)
    out = fn(a, mk, col, k)
    res = a if out is None else out
    return dict(family=family, img=a.dtype.str, mask=mk.dtype.str, col=hx(np.asarray(col, float).ravel()),
                col_shape=list(np.shape(col)), alpha=num(k), out=res.dtype.str, sha256=sha(res))


def helpers_child(folder, episode):
    """in a fresh process, in the episode's scratch copy: its renderer draws a frame a second with the helpers
    wrapped; returns the recorded cases"""
    from .older import origin as O
    O._episode_env(folder)
    import numpy as np
    import render as RD
    cases, counter = [], {}

    def owner(path):
        mod = RD
        for part in path.split('.'):
            mod = getattr(mod, part)
        return mod

    def wrap(path, name, family):
        mod = owner(path)
        f = getattr(mod, name)
        where = f'{episode}:{path}.{name}'

        if name == 'mask_of':
            def g(polys, *a, **kw):
                m = f(polys, *a, **kw)
                width = kw.get('width', a[0] if a else None)
                if kept(counter, (name, family)):
                    cases.append(dict(fn='mask_of', family=family, where=where, width=width,
                                      polys=[hx(np.asarray([tuple(map(float, q)) for q in pts], float).reshape(-1, 2))
                                             for pts in polys], sha256=sha(m)))
                return m
        elif name == 'blend':
            def g(arr, m, col, k):
                if kept(counter, (name, family)):
                    cases.append(dict(fn='blend', where=where, **_blend_case(f, family, arr, m, col, k)))
                return f(arr, m, col, k)
        elif name == 'annulus_polys':
            def g(cam, cx, cz, r_in, r_out, y, *a, **kw):
                out = f(cam, cx, cz, r_in, r_out, y, *a, **kw)
                if kept(counter, (name, family)):
                    names = ('n', 'a0', 'a1')
                    extra = dict(zip(names, a))
                    extra.update(kw)
                    cases.append(dict(fn='annulus_polys', family=family, where=where, cam=cam_of(cam),
                                      args=dict(cx=hx(cx), cz=hx(cz), r_in=hx(r_in), r_out=hx(r_out), y=hx(y),
                                                n=int(extra.get('n', 36)), a0=hx(extra.get('a0', 0.0)),
                                                a1=hx(extra.get('a1', 2 * math.pi))),
                                      out=hx(np.asarray(out, float))))
                return out
        elif name == 'dart_quads':
            def g(x, z, ang, *a, **kw):
                out = f(x, z, ang, *a, **kw)
                if kept(counter, (name, family)):
                    names = ('y', 'L_', 'Wd')
                    extra = dict(y=0.48, L_=0.34, Wd=0.07)
                    extra.update(dict(zip(names, a)))
                    extra.update(kw)
                    cases.append(dict(fn='dart_quads', where=where, args=dict(x=hx(x), z=hx(z), ang=hx(ang),
                                      **{k: hx(v) for k, v in extra.items()}), head=hx(out[0]), body=hx(out[1])))
                return out
        elif name == 'draw_dart':
            def g(fr, cam, L, shade, x, z, ang, col):
                if kept(counter, (name, family)):
                    cases.append(dict(fn='draw_dart', family=family, where=where,
                                      args=dict(x=hx(x), z=hx(z), ang=hx(ang)), col=hx(np.asarray(col, float))))
                return f(fr, cam, L, shade, x, z, ang, col)
        else:
            raise SystemExit(f'older_vectors: no wrapper for {name}')
        setattr(mod, name, g)

    for path, name, family in EPISODES[episode]:
        wrap(path, name, family)
    for f in range(0, 1830, 30):
        RD.render(f)
    return cases


# ---------------------------------------------------------------- helpers: the replays on the kit's example episode
def _styles(episode):
    """the episode's styles.py from its scratch copy, under a module name of its own (its draw functions)"""
    import importlib.util
    from .older import origin as O
    folder = O.scratch(episode)
    for d in (common.REPO,) + O.SHARED:
        if d not in sys.path:
            sys.path.append(d)
    spec = importlib.util.spec_from_file_location(f'older_styles_{episode}', os.path.join(folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _dart_replays(darts, out, cases):
    """child: each recorded or edge dart drawn on a fresh frame of the example episode at DART_T; writes every case
    (the darts with their hashes) and the camera"""
    common.kit()
    from . import frames as F
    from ballfight.kit import fighters as FM
    FM.look = lambda name, P, k: FM.base.Look(P, k)
    P = F.picture('older_vectors_darts')
    from reelkit.raster import Frame
    from reelkit.lighting import shade
    import numpy as np
    styles = {fam: _styles(ep) for fam, ep in DART_STYLES.items()}
    cam = P.camera(DART_T)
    L = P.lights(DART_T, cam)
    for c in darts:
        fr = Frame((10, 8, 18))
        a = c['args']
        styles[c['family']].draw_dart(fr, cam, L, shade, float.fromhex(a['x']), float.fromhex(a['z']),
                                      float.fromhex(a['ang']), np.array([float.fromhex(v) for v in c['col']]))
        c.update(pil=sha(np.asarray(fr.pil)), depth=sha(np.asarray(fr.dep, np.float32)))
    with open(out, 'w', encoding='utf-8') as fh:
        json.dump(dict(cam=cam_of(cam), cases=cases), fh)


def _edge_cases(cases):
    """edge cases of the pure helpers, computed with the episodes' own functions (from their scratch copies)"""
    import numpy as np
    out = []
    st = {'all': _styles('ballfight_bagpipes_vs_accordion'), 'polys': _styles('ballfight_organ_vs_harp'),
          'lines': _styles('ballfight_ukulele_vs_acoustic')}
    sq = [(10.5, 20.25), (200.0, 30.0), (150.75, 400.5)]
    for fam, polys, width in (('all', [], None), ('all', [[(5.0, 5.0), (300.0, 600.0)]], None),
                              ('all', [sq, [(0.0, 0.0), (359.0, 639.0), (0.0, 639.0)]], None),
                              ('polys', [[(5.0, 5.0), (300.0, 600.0)], sq], None), ('polys', [], None),
                              ('lines', [sq], 3), ('lines', [sq], 0), ('lines', [[(1.0, 1.0)], sq], 1),
                              ('lines', [[(-20.0, 10.0), (400.0, 700.0)]], None)):
        f = st[fam].mask_of
        m = f(polys) if width is None else f(polys, width=width)
        out.append(dict(fn='mask_of', family=fam, where='edge', width=width,
                        polys=[hx(np.asarray(p, float).reshape(-1, 2)) for p in polys], sha256=sha(m)))
    blends = {'into': _styles('ballfight_bagpipes_vs_accordion').blend,
              'early': _styles('ballfight_bassoon_vs_barisax').blend, 'clip': st['lines'].blend}
    for fam, f in blends.items():
        for adt, mdt, col, k in ((np.float32, np.float32, (300.0, -5.0, 128.5), 0.5),
                                 (np.float64, np.float32, [10, 20, 30], 0.0),
                                 (np.float32, np.float64, np.array([1.5, 2.5, 3.5]), -0.25),
                                 (np.float32, np.float32, (200.0, 100.0, 50.0), np.float64(0.37)),
                                 (np.float64, np.float64, (0.0, 0.0, 0.0), float('nan'))):
            out.append(dict(fn='blend', where='edge', **_blend_case(f, fam, np.zeros(1, adt), np.zeros(1, mdt), col,
                                                                     k)))
    dq = _styles('ballfight_organ_vs_harp').dart_quads
    for x, z, ang in ((0.0, 0.0, 0.0), (1.5, -2.25, math.pi), (-3.0, 3.0, -1e-9), (2.0, 1.0, 7.5)):
        h, b = dq(x, z, ang)
        out.append(dict(fn='dart_quads', where='edge', args=dict(x=hx(x), z=hx(z), ang=hx(ang), y=hx(0.48),
                        L_=hx(0.34), Wd=hx(0.07)), head=hx(h), body=hx(b)))
    an = _styles('ballfight_bagpipes_vs_accordion').annulus_polys
    from reelkit.camera import Cam
    cam = Cam((0.0, 0.3, 0.0), 27.5, 0.0, 58.0, 36, 288)
    for cx, cz, r0, r1, y, n, a0, a1 in ((0.0, 0.0, 0.6, 0.9, 0.02, 36, 0.0, 2 * math.pi),
                                         (1.0, -1.5, 0.5, 0.5, 0.3, 1, 0.0, 1.0),
                                         (-2.0, 2.0, 1.2, 0.4, 0.0, 7, -0.5, 0.25)):
        q = an(cam, cx, cz, r0, r1, y, n=n, a0=a0, a1=a1)
        out.append(dict(fn='annulus_polys', family='arc', where='edge', cam=cam_of(cam),
                        args=dict(cx=hx(cx), cz=hx(cz), r_in=hx(r0), r_out=hx(r1), y=hx(y), n=n, a0=hx(a0), a1=hx(a1)),
                        out=hx(np.asarray(q, float))))
    ring = _styles('ballfight_drums_vs_violin').annulus_polys
    for cx, cz, r0, r1, y, n in ((0.5, 0.25, 0.7, 1.1, 0.02, 36), (0.0, 0.0, 0.2, 0.3, 0.1, 5)):
        q = ring(cam, cx, cz, r0, r1, y, n=n)
        out.append(dict(fn='annulus_polys', family='ring', where='edge', cam=cam_of(cam),
                        args=dict(cx=hx(cx), cz=hx(cz), r_in=hx(r0), r_out=hx(r1), y=hx(y), n=n, a0=hx(0.0),
                                  a1=hx(2 * math.pi)), out=hx(np.asarray(q, float))))
    for fam in ('violin', 'harp'):
        for x, z, ang, col in ((0.0, 0.0, 0.0, (255.0, 120.0, 30.0)), (3.4, -3.4, 2.0, (300.0, -20.0, 128.0)),
                               (9.0, 9.0, 1.0, (10.0, 20.0, 30.0))):
            out.append(dict(fn='draw_dart', family=fam, where='edge', args=dict(x=hx(x), z=hx(z), ang=hx(ang)),
                            col=hx(np.asarray(col, float))))
    return out


def helpers():
    from . import m3parts
    from .older import origin as O
    cases = []
    handles = []
    for episode in EPISODES:
        folder = O.scratch(episode)
        handles.append((episode, m3parts.start(helpers_child, folder, episode)))
        if len(handles) >= 3:
            ep, h = handles.pop(0)
            got = m3parts.wait(h)
            print(f'older_vectors: {ep}: {len(got)} calls kept')
            cases += got
    for ep, h in handles:
        got = m3parts.wait(h)
        print(f'older_vectors: {ep}: {len(got)} calls kept')
        cases += got
    O.check_kit_clean('older_vectors (the episodes)')
    folder = os.path.join(common.CACHE, 'older_vectors')
    os.makedirs(folder, exist_ok=True)
    recorded, done = os.path.join(folder, 'recorded.json'), os.path.join(folder, 'helpers.json')
    with open(recorded, 'w', encoding='utf-8') as fh:
        json.dump(cases, fh)
    child('darts', done, recorded)                # edge cases and the darts' replays, in a fresh interpreter
    with open(done, encoding='utf-8') as fh:
        rep = json.load(fh)
    cases = rep['cases']
    O.check_kit_clean('older_vectors (the replays)')
    for fn in ('mask_of', 'blend', 'annulus_polys', 'dart_quads', 'draw_dart'):
        n = sum(1 for c in cases if c['fn'] == fn)
        assert n >= 4, f'older_vectors: only {n} cases of {fn}'
        for fam in sorted({c.get('family', '') for c in cases if c['fn'] == fn}):
            rec = sum(1 for c in cases if c['fn'] == fn and c.get('family', '') == fam and c['where'] != 'edge')
            print(f'older_vectors: {fn} {fam}: {rec} recorded')
    return dict(about='the episode helpers several older looks call, on the arguments the episodes give them (a frame '
                      'a second of each episode, its own render.py in its scratch copy) and edge cases; see '
                      'tools/studio_export/older_vectors.py', tool='older_vectors', dart_t=hx(DART_T),
                dart_cam=rep['cam'], episodes=sorted(EPISODES), cases=cases)


# ---------------------------------------------------------------- the opt-ins: children in fresh interpreters
def child(mode, out, *args):
    env = dict(os.environ)
    for k in THREADS:
        env[k] = '1'
    subprocess.run([sys.executable, '-m', 'tools.studio_export.older_vectors', mode, out] + list(args),
                   cwd=common.REPO, check=True, env=env)


def _render_all(P, frames):
    import numpy as np
    rows = []
    for f in frames:
        P.ui.reset_drawn()
        img = np.ascontiguousarray(P.render(f))
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in P.ui.DRAWN]
        rows.append([f, sha(img), len(boxes), hashlib.sha256('\n'.join(boxes).encode()).hexdigest()])
    return rows


def _example(name, tag):
    if name == EXAMPLE:
        from . import frames as F
        return F.picture(tag)
    common.episode(name, tag)
    from ballfight.kit import picture as P
    return P


def stub_look(base, P):
    """a look that uses every opt-in, with arithmetic both languages compute alike (the Rust test's StubLook)"""
    import numpy as np
    W, H = P.W, P.H
    calls = {}

    def hit(name):
        calls[name] = calls.get(name, 0) + 1

    class Stub(base):
        def canvas_albedo(self, alb, C):
            chk = (np.floor(C.sxp * 2.0) + np.floor(C.szp * 2.0)) % 2 == 0
            out = alb.copy()
            out[chk] = out[chk] * 0.75
            hit('canvas_albedo')
            return out

        def canvas_paint(self, col, Ic, C):
            p = C.fs[self.k]['p']
            dx, dz = C.sxp - p[0], C.szp - p[1]
            m = dx * dx + dz * dz < 1.0
            if not m.any():
                return col
            out = col.copy()
            out[m] = out[m] * 0.5 + np.array([60.0, 20.0, 90.0]) * Ic[m]
            hit('canvas_paint')
            return out

        def ball_tint(self, kk, st, t, col):
            if kk == self.k:
                return col
            hit('ball_tint')
            return col * 0.5 + np.array([100.0, 100.0, 120.0]) * 0.5

        def ball_glass(self, kk, st, t, inner, geo):
            if kk != self.k:
                return inner
            stripe = (np.floor(geo['dx'] * 4.0 + geo['dy'] * 2.0) % 2 == 0)[..., None]
            hit('ball_glass')
            return np.where(stripe, inner * 0.6 + np.array([200.0, 40.0, 40.0]) * 0.4, inner)

        def ball_lit(self, kk, st, t, lit):
            if kk == self.k:
                return lit
            hit('ball_lit')
            return lit * 0.5 + 20

        def ball_after_sprite(self, kk, st, t, arr, s, rp):
            if kk != self.k:
                return
            y = int(s[1])
            xa, xb = max(0, int(s[0] - rp)), min(W, int(s[0] + rp))
            if 0 <= y < H and xb > xa:
                arr[y, xa:xb] = (255.0, 0.0, 255.0)
                hit('ball_after_sprite')

        def ball_late(self, kk, F, st):
            late = kk == self.k and F.f % 2 == 1
            if late:
                hit('ball_late')
            return late

        def rope_press(self, s, y, t, Pr, yy):
            p = self.P.fighter(self.k, t)['p']
            dx, dz = Pr[:, 0] - p[0], Pr[:, 1] - p[1]
            near = dx * dx + dz * dz < 2.25
            if not near.any():
                return yy
            hit('rope_press')
            return np.where(near, np.maximum(yy - 0.25, 0.05), yy)

        def rope_events(self):
            out = [(e[0] + 0.4, self.P.SIDE_OF[(e[3], e[4])], e[5], e[6], 3.2) for e in self.P.EV
                   if e[1] == 'rope' and e[2] == self.k]
            calls['rope_events'] = len(out)
            return out

        def after_post(self, img, F, drawn):
            for k, (s_, rp) in drawn.items():
                if k != self.k:
                    continue
                x0, y0 = int(s_[0]) - 4, int(s_[1]) - 4
                if 0 <= x0 and x0 + 9 <= W and 0 <= y0 and y0 + 9 <= H:
                    img[y0:y0 + 9, x0:x0 + 9] = 255 - img[y0:y0 + 9, x0:x0 + 9]
                    self.P.ui._note(x0, y0, x0 + 9, y0 + 9, 'label', 'stub after_post')
                    hit('after_post')
            return img

    return Stub, calls


def on_frames(P):
    """the frames of the opt-in test: a spread of the fight (pairs, so the late ball shows on odd frames), the first
    rope events of the stub's side and the stub's own (0.4 s later), the K.O. and the tail"""
    fs = [150, 151, 300, 301, 450, 451, 600, 601, 900, 901, 1200, 1201]
    ro = [e for e in P.EV if e[1] == 'rope' and e[2] == STUB_SIDE][:3]
    fs += [int(round((e[0] + d) * 30)) for e in ro for d in (0.05, 0.45)]
    fko = int(round(P.T_KO * 30))
    fs += [fko, fko + 12, 1700]
    return sorted(set(f for f in fs if 0 <= f < 1830))


def optins_child(mode, out, *args):
    """mode 'plain' / 'patched' <example>: its own looks, OFF_FRAMES; 'base': the base-class looks, unit.json's frames
    and on_frames; 'on': FLUTE's base look and the stub; 'darts' <cases json>: helpers' draw_dart replays"""
    from . import older
    if mode != 'plain':
        older.install()
    if mode == 'darts':
        with open(args[0], encoding='utf-8') as fh:
            cases = json.load(fh)
        cases += _edge_cases(cases)
        _dart_replays([c for c in cases if c['fn'] == 'draw_dart'], out, cases)
        return
    common.kit()
    from ballfight.kit import fighters as FM
    if mode in ('plain', 'patched'):
        P = _example(args[0], f'older_vectors_{mode}')
        res = dict(rows=_render_all(P, OFF_FRAMES))
    elif mode == 'base':
        FM.look = lambda name, P, k: FM.base.Look(P, k)
        P = _example(EXAMPLE, 'older_vectors_base')
        with open(common.ref('picture_ring', 'unit.json'), encoding='utf-8') as fh:
            unit = json.load(fh)
        want = [f for f, _ in unit['frames']]
        res = dict(unit=_render_all(P, want), on=_render_all(P, on_frames(P)))
    elif mode == 'on':
        holder = {}

        def look(name, P_, k):
            if k != STUB_SIDE:
                return FM.base.Look(P_, k)
            Stub, calls = stub_look(FM.base.Look, P_)
            holder['calls'] = calls
            return Stub(P_, k)
        FM.look = look
        P = _example(EXAMPLE, 'older_vectors_on')
        assert getattr(P, '_OLDER_PATCHED', False), 'the picture patches are not installed'
        n_rope = len(P.ROPE_EV)
        rows = _render_all(P, on_frames(P))
        res = dict(rows=rows, calls=dict(sorted(holder['calls'].items())), n_rope_ev=n_rope,
                   rope_ev=[[hx(r[0]), int(r[1]), hx(r[2]), hx(r[3]), hx(r[4])] for r in P.ROPE_EV],
                   optins=list(P.OLDER_OPTINS))
    else:
        raise SystemExit(f'older_vectors: no mode {mode!r}')
    with open(out, 'w', encoding='utf-8') as fh:
        json.dump(res, fh)


def optins():
    scratch = os.path.join(common.CACHE, 'older_vectors')
    os.makedirs(scratch, exist_ok=True)
    got = {}
    for mode, extra in [('base', ())] + [(m, (ex,)) for ex in OFF_EXAMPLES for m in ('plain', 'patched')] + \
                       [('on', ())]:
        path = os.path.join(scratch, f"{mode}_{'_'.join(extra) or 'x'}.json")
        child(mode, path, *extra)
        with open(path, encoding='utf-8') as fh:
            got[(mode,) + extra] = json.load(fh)
        print(f'older_vectors: {mode} {" ".join(extra)} done')
    with open(common.ref('picture_ring', 'unit.json'), encoding='utf-8') as fh:
        unit = json.load(fh)
    base = got[('base',)]
    want = {f: s for f, s in unit['frames']}
    wrong = [r[0] for r in base['unit'] if want[r[0]] != r[1]]
    if wrong:
        raise SystemExit(f'older_vectors: with the patches installed the base-class looks draw frames {wrong} '
                         f'differently from picture_ring/unit.json')
    off = {}
    for ex in OFF_EXAMPLES:
        a, b = got[('plain', ex)]['rows'], got[('patched', ex)]['rows']
        if a != b:
            bad = [x[0] for x, y in zip(a, b) if x != y]
            raise SystemExit(f'older_vectors: {ex}: the patched kit draws frames {bad} differently')
        off[ex] = b
    on = got[('on',)]
    calls = on['calls']
    for name in on['optins']:
        if not calls.get(name):
            raise SystemExit(f'older_vectors: the stub look never used opt-in {name} ({calls})')
    same = [r[0] for r, q in zip(on['rows'], base['on']) if r[1] == q[1]]
    if len(same) > len(on['rows']) // 4:
        raise SystemExit(f'older_vectors: the stub changes too few frames (unchanged: {same})')
    return dict(about='the picture opt-ins of older/picture_patches.py (bf-picture Look methods of the same names): '
                      'off, the kit unchanged with the patches installed; on, the example episode with the stub '
                      'look of tools/studio_export/older_vectors.py on the TROMBONE\'s side',
                tool='older_vectors', episode=EXAMPLE, stub_side=STUB_SIDE, optins=on['optins'],
                off=dict(base_frames=len(base['unit']), examples=off, frames=list(OFF_FRAMES)),
                on=dict(rows=on['rows'], base_rows=base['on'], calls=calls, n_rope_ev=on['n_rope_ev'],
                        rope_ev=on['rope_ev']))


def run():
    os.makedirs(OUT, exist_ok=True)
    doc = optins()
    common.write_json(os.path.join(OUT, 'optins.json'), doc)
    doc = helpers()
    common.write_json(os.path.join(OUT, 'helpers.json'), doc)
    for name in ('optins.json', 'helpers.json'):
        path = os.path.join(OUT, name)
        print(f'older_vectors: {common.rel(path)}  {os.path.getsize(path)} bytes')


EXPORTS = {'older_vectors': run}

if __name__ == '__main__':                       # a child: python -m tools.studio_export.older_vectors <mode> <out> ...
    optins_child(sys.argv[1], sys.argv[2], *sys.argv[3:])

"""The parts of the milestone 3 references the proving harness reads (Task 6), made in fresh interpreters.

m3refs.fighter_refs calls these after the fight half; episode_refs and theme_refs call the ones they share. The kit
builds `kit.picture` and `kit.music` once per process from the episode loaded first, so every part that draws or
plays runs in a fresh spawn process (`fresh`), and hands back plain data: the committed summaries, the paths of the
cache files it wrote (the main process pins them) and any committed file's bytes (the main process stages them).

    look_child(...)     one fight drawn with ONE real look (the fighter's; the kit's base Look for the other side),
                        every look hook spied as look_flute does: per frame the SHA-256 of the frame, of the hook
                        calls and of the draw log, and every scipy.spatial.ConvexHull call (count, vertices, point
                        sums; the points in full go to a cache file); the VS card alone on the fixed noise image at
                        16 times; the title hooks on the fixed base; the sprites and the model (left fight only)
    sound_child(...)    the soundtrack with the fighter's real part and the kit's base Sound for the other side:
                        the 16-bit master's SHA-256, the buses, every stem's and the master's RMS per 100 ms (dB,
                        3 decimals), the first-time notes (count, SHA-256 of their lines, the first eight), the
                        stems in five windows of 1 s (cache)
    full_sound_child()  the same with both real parts (a pair gate: the kit's audio.wav)
    voice_child(...)    pp_bridge.voice at 4 pitches x velocities x lengths, each from the state the one before left:
                        fingerprints (length, peak, RMS of every 10 ms block, the noise state after) committed, the
                        notes cached
    checks_child(...)   every check of kit/checks.py on the fight (both real looks and parts), the numbers of
                        checks.py's functions (reused as they are), the safe-zone boxes as one SHA-256 per frame
    frames_child(...)   a pair gate's golden frames with both real looks (episode_select) and the card frames
    bed_child(...)      a theme's bed alone (both fighters the kit's base Sound): master SHA-256, buses, RMS, stems
    hook_child(...)     a theme's optional bed hooks called directly on an empty mix (cache vectors)"""
import hashlib
import inspect
import multiprocessing as mp
import os
import sys
import time
import traceback

from . import common

THREAD_VARS = ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS')
SR = 48000
BLOCK = SR // 10                         # 100 ms: the RMS profile
VBLOCK = SR // 100                       # 10 ms: a voice note's fingerprint
STEM_WINDOWS = (0.0, 12.0, 30.0, 'T_KO - 1.0', 'T_KO + 3.0')   # 1 s each, cached
CARD_FRAMES = 16
WORKERS = 3
SAFE_STEP = 0.25


# ---------------------------------------------------------------- fresh processes
class _SerialPool:
    """what the kit uses of a `fork` pool (with ... as pool: pool.map / starmap / imap), mapped in this process, in
    order: the results are the pool's, since the kit only maps pure functions of their arguments over it"""

    def __init__(self, processes=None, *args, **kwargs):
        pass

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False

    def map(self, fn, jobs, chunksize=None):
        return [fn(j) for j in jobs]

    def starmap(self, fn, jobs, chunksize=None):
        return [fn(*j) for j in jobs]

    def imap(self, fn, jobs, chunksize=1):
        return (fn(j) for j in jobs)

    def close(self):
        pass

    def join(self):
        pass


class _SerialFork:
    """multiprocessing.get_context('fork') where Windows has no fork"""
    Pool = _SerialPool


def serial_fork():
    """in this process, multiprocessing.get_context('fork') gives a context whose Pool maps serially, in order; every
    other start method is the real one. Windows has no fork, and a look may build a sprite cache through a fork pool
    when it is not a pool worker itself (kit/fighters/trumpet.py `Look.atlas`): every part's fresh process (_child)
    and the checks' render workers install it, so no port wraps the parts. kit.checks' own fork pool is replaced by
    checks.py (`_ForkShim`) as before. Idempotent"""
    real = mp.get_context
    if getattr(real, 'serial_fork', False):
        return

    def get_context(method=None):
        return _SerialFork() if method == 'fork' else real(method)
    get_context.serial_fork = True
    mp.get_context = get_context


def build_atlases(P):
    """build every look's file-cached sprite atlas (a kit look's `atlas()`, TRUMPET's) in this process, before a pool
    of render workers starts: the workers then read the file this process saved instead of three of them writing it
    at once. The look's own method, so the atlas is the kit's"""
    for look in P.LOOKS:
        if callable(getattr(look, 'atlas', None)):
            look.atlas()


def _child(q, modname, fnname, args):
    for k in THREAD_VARS:
        os.environ[k] = '1'
    serial_fork()
    try:
        from . import __main__ as entry
        assert hash(entry.HASH_PROBE[0]) == entry.HASH_PROBE[1], 'this process does not hash strings as the ' \
                                                                 'reference does (PYTHONHASHSEED=0)'
        import importlib
        fn = getattr(importlib.import_module(modname), fnname)
        q.put(('ok', fn(*args)))
    except BaseException as ex:                       # noqa: BLE001  (handed to the parent, which raises it)
        q.put(('error', f'{type(ex).__name__}: {ex}\n{traceback.format_exc()}'))


def start(fn, *args):
    """fn(*args) in a fresh spawn process (not a pool worker: it may start a pool of its own). Returns a handle for
    wait()"""
    ctx = mp.get_context('spawn')
    q = ctx.Queue()
    p = ctx.Process(target=_child, args=(q, fn.__module__, fn.__name__, args))
    p.start()
    handle = (p, q, fn.__name__)
    _LIVE.append(handle)
    return handle


_LIVE = []                                # the parts started and not yet waited for, in start order
_DONE = {}                                # a live part's (status, value), read early while waiting for another
POLL = 0.5                                # s between looks at the parts while waiting


def _result(handle):
    """the part's (status, value) if it has one now, else None; a part that ended without one is an error"""
    p, q, name = handle
    if id(p) in _DONE:
        return _DONE[id(p)]
    import queue
    try:
        got = q.get_nowait()
    except queue.Empty:
        if p.is_alive():
            return None
        try:                                  # it may have put its result just before it ended
            got = q.get(timeout=POLL)
        except queue.Empty:
            got = ('error', f'the process ended (exit code {p.exitcode}) without handing back a result')
    _DONE[id(p)] = got
    return got


def stop_parts():
    """end every part still running (terminate, then kill one that does not go), so a failed export does not wait
    for them: a child blocked handing back a large result nobody reads would otherwise keep the interpreter from
    exiting. Returns the names of the parts it stopped"""
    stopped = []
    for p, q, name in list(_LIVE):
        if p.is_alive():
            p.terminate()
            p.join(5)
            if p.is_alive():
                p.kill()
                p.join(5)
            stopped.append(name)
        q.cancel_join_thread()
        _LIVE.remove((p, q, name))
        _DONE.pop(id(p), None)
    return stopped


def wait(handle):
    """the part's result. While waiting, every other running part is watched too: the first that fails (or ends
    without a result) stops all of them (stop_parts) and the export with the failed part's name and error; the
    SystemExit unwinds fighter_refs, so staged_on_disk puts left.bfrec back and nothing committed is written"""
    while True:
        for other in [h for h in _LIVE if h is not handle] + [handle]:
            got = _result(other)
            if got is not None and got[0] != 'ok':
                stopped = stop_parts()
                more = f" (stopped the other parts still running: {', '.join(stopped)})" if stopped else ''
                raise SystemExit(f'{other[2]} (in a fresh process) stopped{more}:\n{got[1]}')
        got = _result(handle)
        if got is not None:
            p = handle[0]
            p.join()
            if handle in _LIVE:
                _LIVE.remove(handle)
            _DONE.pop(id(p), None)
            return got[1]
        handle[0].join(POLL)


def fresh(fn, *args):
    return wait(start(fn, *args))


# ---------------------------------------------------------------- small helpers
def fresh_check(folder):
    """remove the kit's file caches of a config folder (check/: the run, the title sprites, the stems) before a
    part starts, so no part reads back what another part of another kind left there"""
    import shutil
    shutil.rmtree(os.path.join(folder, 'check'), ignore_errors=True)


def cfg_keys(cls):
    """the roster keys a kit look class reads from self.cfg: each chain of literal subscripts and `.get(literal)`
    from `self.cfg` (through a local name bound to such a chain), dotted ('view.2', 'abilities.low.speed'); only the
    longest chains are kept. A local name stands for the chain it was last bound to, in the function that binds it
    and from that statement on (source order); a nested function starts from its parent's names. So two methods'
    `ab = self.cfg['abilities'][...]` (OBOE's unlock1 and unlock2) are two names, and a rebinding replaces the old
    chain (or drops it when the new value is no chain)"""
    import ast
    import textwrap
    tree = ast.parse(textwrap.dedent(inspect.getsource(cls)))
    alias = {}
    found = set()

    def chain(node):
        if isinstance(node, ast.Attribute) and node.attr == 'cfg' and isinstance(node.value, ast.Name) \
                and node.value.id == 'self':
            return []
        if isinstance(node, ast.Name) and node.id in alias:
            return alias[node.id]
        if isinstance(node, ast.Subscript) and isinstance(node.slice, ast.Constant):
            base = chain(node.value)
            return None if base is None else base + [str(node.slice.value)]
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == 'get' \
                and node.args and isinstance(node.args[0], ast.Constant):
            base = chain(node.func.value)
            return None if base is None else base + [str(node.args[0].value)]
        return None

    class Reads(ast.NodeVisitor):
        """every chain read, with the names bound so far in the function being read (source order)"""

        def visit(self, node):
            if isinstance(node, ast.expr):
                c = chain(node)
                if c:
                    found.add('.'.join(c))
            return super().visit(node)

        def visit_FunctionDef(self, node):
            nonlocal alias
            outer, alias = alias, dict(alias)
            try:
                self.generic_visit(node)
            finally:
                alias = outer
        visit_AsyncFunctionDef = visit_FunctionDef

        def visit_Assign(self, node):
            self.visit(node.value)
            if len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
                c = chain(node.value)
                if c:
                    alias[node.targets[0].id] = c
                else:
                    alias.pop(node.targets[0].id, None)
            for t in node.targets:
                self.visit(t)
    Reads().visit(tree)
    return sorted(k for k in found if not any(o != k and o.startswith(k + '.') for o in found))


def sha_bytes(b):
    return hashlib.sha256(b).hexdigest()


def block_db3(x, block=BLOCK):
    """the RMS of each block of a (n, 2) buffer in dB over both channels (stems.py's formula), to 3 decimals"""
    import numpy as np
    return [round(float(20 * np.log10(np.sqrt((x[i:i + block] ** 2).mean()) + 1e-12)), 3)
            for i in range(0, len(x), block)]


def pcm_sha(m):
    import numpy as np
    return sha_bytes((np.clip(m, -1, 1) * 32767).astype('<i2').tobytes())


def first_at(t):
    from . import frames as F
    return F.first_at(t)


def card_frames(TT, nf, fps):
    """CARD_FRAMES frames spread evenly over the frames on which the VS card is drawn"""
    import numpy as np
    on = [f for f in range(nf) if TT.door_offset(f / fps) is not None and TT.door_offset(f / fps) < TT.OPEN_PX - 1]
    return sorted({on[int(round(float(j)))] for j in np.linspace(0, len(on) - 1, CARD_FRAMES)})


def noise_image():
    """the fixed backdrop of the card hashes: milestone 1's committed title/noise.png"""
    from . import frames as F
    mode, a = F.read_png(os.path.join(F.TITLE_OUT, 'noise.png'))
    assert mode == 'RGB' and a.shape == (F.H, F.W, 3)
    return a


def card_rows(P):
    """[frame, SHA-256 of title.draw(noise, frame / 30)] for the card frames"""
    noise = noise_image()
    rows = []
    for f in card_frames(P.TT, P.T.NF, P.T.FPS):
        img = P.TT.draw(noise.copy(), f / P.T.FPS)
        rows.append([f, sha_bytes(img.tobytes())])
    return rows


def load_music(cfg, loaded=False):
    """kit.music (and pp_bridge, the series' drums) for cfg in this fresh process (`loaded`: cfg.load() was
    called already, so a value set on the episode since stays)"""
    if not loaded:
        cfg.load()
    from ballfight.kit import music as M              # noqa: N812
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly as D
    return M, B, D


# ---------------------------------------------------------------- the look
def _hull_patch(calls):
    import numpy as np
    import scipy.spatial
    real = scipy.spatial.ConvexHull

    class Noted(real):                                # every call's points and vertices, in call order
        def __init__(self, points, *a, **kw):
            super().__init__(points, *a, **kw)
            calls.append((np.array(points, np.float64), np.array(self.vertices)))
    scipy.spatial.ConvexHull = Noted


HULL_COLUMN = 6                                       # a look row's compact hull calls (look.json format 1)
HULL_LINE = ("'<frame> <points> <v0>,<v1>,...': one line per scipy.spatial.ConvexHull call, in frame order then "
             "call order, with its number of input points and its vertices in scipy's order")


def hull_lines(rows):
    """the exact part of a fight's hull log (ruling M3-R29) from look rows that still hold their compact hull
    calls (column HULL_COLUMN, `[points, vertices, sum of x, sum of y]`): HULL_LINE. The sums are left out: the
    Studio's points agree within 1e-12 px, not to the bit, and the pinned hull cache holds every point"""
    return [f"{row[0]} {call[0]} {','.join(str(int(v)) for v in call[1])}" for row in rows for call in row[HULL_COLUMN]]


def hull_summary(rows, cache):
    """what look.json format 2 commits of a fight's hull log in place of the per-frame rows (ruling M3-R29, as
    sprites under M3-R18): the counts and the SHA-256 of hull_lines; `cache` is the pinned file with every call"""
    lines = hull_lines(rows)
    return dict(ruling='M3-R29', cache=cache, frames=len(rows), calls=len(lines),
                points=sum(int(call[0]) for row in rows for call in row[HULL_COLUMN]),
                sha256=sha_bytes('\n'.join(lines).encode()), line=HULL_LINE)


def without_hull(rows):
    """look rows without their hull column (look.json format 2): frame, frame SHA-256, hook calls, their SHA-256,
    boxes, their SHA-256, why"""
    return [row[:HULL_COLUMN] + row[HULL_COLUMN + 1:] for row in rows]


def look_select(P, k, events, which, n_min, times):
    """[(frame, [reason, ...])] in frame order: a spread over the video, the card and the count-in, each of the
    fighter's abilities 0.2 and 0.6 s after its first two uses, the first hit of each kind it lands and the first it
    takes, the first frame with its HP below 25, the K.O. and after (its shards when it loses), its solo when it
    wins, the reassembly, the glide, `times`; then every 15th frame until there are n_min"""
    from . import frames as F
    T, NF = P.T, P.T.NF
    picks = {}

    def add(f, why):
        f = int(f)
        if 0 <= f < NF:
            picks.setdefault(f, []).append(why)
    for f in range(0, NF, 45 if which == 'left' else 90):
        add(f, 'spread')
    for f in (13, 27, 43, 44, 107, 108, 109, NF - 2, NF - 1):
        add(f, 'the title card, the doors and the count-in')
    ab = P.CFG[k]['abilities']
    for slot in F.SLOTS:
        uses = [e for e in events if e[1] == 'ability' and e[2] == k and e[3] == ab[slot]['name']][:2]
        for e in uses:
            for d in (0.2, 0.6):
                add(first_at(e[0] + d), f"{d} s after {ab[slot]['name']} at {e[0]} s")
    hows = {}
    for e in events:
        if e[1] == 'hit' and e[2] == k and e[7] not in hows:
            hows[e[7]] = e
    for how, e in hows.items():
        add(first_at(e[0]) + 1, f"its first '{how}' hit at {e[0]} s")
    taken = next((e for e in events if e[1] == 'hit' and e[2] == 1 - k), None)
    if taken is not None:
        add(first_at(taken[0]) + 1, f"the first hit it takes ('{taken[7]}' at {taken[0]} s)")
    low = [f for f in range(first_at(T.T_FIGHT), first_at(P.T_KO)) if P.hp_at(k, f / P.FPS) < 25.0]
    if low:
        add(low[0], 'the first frame with its HP below 25')
    ko = first_at(P.T_KO)
    for d in (-1, 0, 1, 2, 5, 12, 30, 60):
        add(ko + d, f'{d} frames from the K.O.')
    add(first_at(P.T_HS + 0.3), 'after the hit-stop')
    if P.WIN == k:
        for d in (0.3, 1.0, 2.5):
            add(first_at(P.T_SOLO + d), f'{d} s into its victory solo')
    else:
        for d in (3, 8, 20):
            add(ko + d, f'its shards, {d} frames after the K.O.')
    add(first_at((T.T_REASSEMBLE + T.T_GLIDE) / 2), "the loser's pieces half-way home")
    for d in (-4, 0, 3):
        add(first_at(T.T_GLIDE) + d, f'{d} frames from the glide home')
    for t in times:
        add(first_at(t), f'{t} s (the spec\'s frames.times)')
    for f in range(22, NF, 15):
        if len(picks) >= n_min:
            break
        if f not in picks:
            add(f, 'filled up to the count the spec asks for')
    return sorted(picks.items())


def look_child(cfg, name, which, rec_path, n_min, times, sprites_out, hull_out, sprite_size=None):
    """one fight drawn with the fighter's real look and the base look for the other side (`sprite_size`: the size
    rule of the look's own sprites, sprites.own_size)"""
    import json
    import numpy as np
    from . import frames as F
    from . import look_flute as LF
    calls = []
    _hull_patch(calls)
    assert 'ballfight.kit.picture' not in sys.modules
    fresh_check(cfg.folder)
    ep, _ = cfg.load()
    from ballfight.kit import fighters as FM
    cls = FM.module(name).Look
    keys = cfg_keys(cls)
    real, bare = LF._spied(cls, 'RealSpy'), LF._spied(FM.base.Look, 'BareSpy')
    FM.look = lambda nm, P, k: (real if nm == name else bare)(P, k)
    from ballfight.kit import picture as P            # noqa: N812
    assert P.EP.seed == cfg.seed and tuple(P.NAMES) == cfg.fighters
    k = P.NAMES.index(name)
    build_log = [x for x in LF.LOG if '.rope' not in x]
    rec = F.check_recording(P, rec_path)
    rows, hulls = [], []
    for f, why in look_select(P, k, rec['events'], which, n_min, times):
        del LF.LOG[:]
        del calls[:]
        P.ui.reset_drawn()
        img = P.render(f)
        assert img.shape == (P.H, P.W, 3) and img.dtype == np.uint8
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in P.ui.DRAWN]
        hull = [[len(pts), [int(v) for v in vs], common.hexf(float(pts[:, 0].sum())),
                 common.hexf(float(pts[:, 1].sum()))] for pts, vs in calls]
        hulls.append(dict(frame=f, calls=[dict(points=[[common.hexf(float(x)), common.hexf(float(y))]
                                                       for x, y in pts], vertices=[int(v) for v in vs])
                                          for pts, vs in calls]))
        rows.append([f, LF.sha_array(img), len(LF.LOG), LF.sha_lines(LF.LOG), len(boxes), LF.sha_lines(boxes), hull,
                     '; '.join(why)])
    for j in sorted({0, len(rows) // 2, len(rows) - 1}):          # a frame is a pure function of its number
        assert LF.sha_array(P.render(rows[j][0])) == rows[j][1], rows[j][0]
    common.write_bytes(hull_out, common.dumps(dict(
        about=f'every scipy.spatial.ConvexHull call of each frame of {name}\'s {which} look fight, in call order: its '
              f'points (float.hex()) and vertices', frames=hulls)).encode('utf-8'))
    doc = dict(fighters=list(P.NAMES), k=k, seed=int(P.EP.seed), winner=P.NAMES[P.WIN],
               recording=dict(hash=rec['hash'], seed=rec['seed'], winner=rec['winner']),
               hooks_build=[len(build_log), LF.sha_lines(build_log)], hooks_build_head=build_log[:8],
               cards=card_rows(P), frames=rows)
    out = dict(doc=doc, cfg_keys=keys)
    if which != 'left':
        return out
    # ---- the title hooks (the look's own close-up or the model's; the card's pattern) on the fixed base
    L = P.LOOKS[k]
    r = int(P.TT.BALLS[k]['r'] * 0.8)
    if name in P.TT.CLOSEUP:
        close, how = P.TT.CLOSEUP[name](r), 'closeup'
    else:
        import sprites as SP
        yc, sw, roll = P.CFG[k]['view']
        close, how = SP.render_sprite(P.CFG[k]['pp'], r, float(yc), float(roll)), 'model'
    yy, xx = np.mgrid[0:P.H, 0:P.W].astype(float)
    base = np.stack([40.0 + xx * 0.5 + yy * 0.25 + 20.0 * c for c in range(3)], -1)
    pattern = name in P.TT.PATTERNS
    pat = P.TT.PATTERNS[name](base.copy()) if pattern else base.copy()
    at = [(0, 0), (180, 320), (359, 639), (100, 200), (201, 77), (17, 503)]
    title = dict(glass=[float(v) for v in L.glass], flare_scale=common.hexf(float(L.flare_scale)),
                 shards=list(L.shards),
                 closeup=dict(radius=r, drawn_by=how, size=int(close.shape[0]), sha256=LF.sha_array(close)),
                 pattern=dict(has=pattern, sha256=LF.sha_array(pat), samples=[[x, y, LF.hx(pat[y, x])] for x, y in at],
                              base='base[y, x, c] = 40.0 + x * 0.5 + y * 0.25 + 20.0 * c, float64, (640, 360, 3)',
                              sha256_of='the float64 values, little-endian, C order (the base itself when the card '
                                        'has no pattern for the fighter)'))
    # ---- the sprites (ruling M3-R18: the cache holds them, the committed file their digest)
    from . import sprites as SPR
    del LF.LOG[:]
    ready = SPR.prepare(P, rec_path)
    got = SPR.run_one(name, ready, out=sprites_out, sprite_size=sprite_size)
    with open(sprites_out[1], encoding='utf-8') as fh:
        sdoc = json.load(fh)
    order = [c['id'] for c in sdoc['cases']]
    title_case = next(c for c in sdoc['cases'] if c['args']['group'] == 'title')['args']
    sprites = dict(count=len(order), digest=sha_bytes('\n'.join(got['sha256'][c] for c in order).encode()),
                   digest_of='SHA-256 of the cases\' RGBA SHA-256 values, one a line, in the order of the cache\'s '
                             'sprites.json (sprites.build: picture, episode, title, model)',
                   k=k, rolls=sdoc['rolls'], radii=sdoc['radii'], drawn_by=sdoc['drawn_by'],
                   episode_keys=sdoc['episode_keys'], title=title_case,
                   groups={g: sum(c['args']['group'] == g for c in sdoc['cases'])
                           for g in ('picture', 'episode', 'title', 'model')})
    # ---- the model: its bytes, staged by the main process
    from . import models
    staged = {}
    models.model(name, P=P, write=lambda path, data: staged.update(path=path, data=data))
    out.update(title=title, sprites=sprites, model=staged)
    return out


# ---------------------------------------------------------------- the sound
def _windows(t_ko):
    return [(w, (t_ko - 1.0) if w == 'T_KO - 1.0' else (t_ko + 3.0) if w == 'T_KO + 3.0' else w)
            for w in STEM_WINDOWS]


def window_length(i0, na):
    """the samples a stem window starting at i0 holds in a soundtrack of `na`: a second (SR), fewer where the
    soundtrack ends inside it (the cache keeps x[i0:i0 + SR]), none past the end"""
    return max(0, min(SR, na - i0))


def stem_windows(t_ko, na, st):
    """(the windows' records, the cache members w<j>__<bus>) of the stems `st` of a soundtrack of `na` samples: each
    STEM_WINDOWS window with the samples it holds (window_length); a window wholly past the end holds none and is
    left out (no record, no member), so j counts the recorded windows only"""
    import numpy as np
    arrays, wins = {}, []
    for label, t in _windows(t_ko):
        i0 = int(round(t * SR))
        n = window_length(i0, na)
        if n == 0:
            continue
        j = len(wins)
        wins.append(dict(window=str(label), t=common.hexf(float(t)), i0=i0, n=n))
        for bus, x in st.items():
            arrays[f'w{j}__{bus}'] = x[i0:i0 + SR].astype(np.float32)
    return wins, arrays


def _soundtrack(M, B, D, stems_out):
    """score (notes logged) and master: the summary and the cache file of stem windows"""
    import numpy as np
    from . import voices as V
    start = V.noise_state()
    mix, log = V.record_score(M, B, D)
    end = V.noise_state()
    m, st = M.master(mix)
    assert m.shape == (M.NA, 2)
    lines = [f"{c['name']} {c['midi']} {common.hexf(c['dur'])} {common.hexf(c['cap'])} {common.hexf(c['vel'])}"
             for c in log.calls]
    wins, arrays = stem_windows(M.T_KO, M.NA, st)
    common.write_npz(stems_out, arrays, register=False)
    return dict(master_sha256=pcm_sha(m), buses=list(mix.buses), stems=sorted(st),
                rms=dict({bus: block_db3(x) for bus, x in st.items()}, master=block_db3(m)),
                notes=dict(count=len(lines), sha256=sha_bytes('\n'.join(lines).encode()), head=lines[:8],
                           line='name midi dur cap vel (floats as float.hex()), one first-time note a line'),
                noise=dict(start=start['state'], end=end['state']),
                windows=wins, windows_note='stem windows of 1 s (float32) in the cache file: member w<j>__<bus>')


def sound_child(cfg, name, role, stems_out, rec_path):
    """the soundtrack with the fighter's real part (as `role` when given) and the base Sound for the other side"""
    from . import frames as F
    fresh_check(cfg.folder)
    ep, _ = cfg.load()
    if role is not None:
        ep.T.ROLES[name] = role
    M, B, D = load_music(cfg, loaded=True)
    F.check_recording(M.P, rec_path)
    k = list(M.NAMES).index(name)
    if role is not None:
        assert M.SOUNDS[k].role == role, (M.SOUNDS[k].role, role)
    M.SOUNDS[1 - k] = M.FIGHTER_MODULES.base.Sound(M, 1 - k)
    out = _soundtrack(M, B, D, stems_out)
    out.update(k=k, fighters=list(M.NAMES), seed=int(M.EP.seed), role=M.SOUNDS[k].role,
               bed_calls=sorted(set(_bed_calls(M.FIGHTER_MODULES.module(name)))))
    return out


def _bed_calls(module):
    """the bed hooks a fighter's Sound calls without checking: `bed.<hook>(` not guarded by hasattr"""
    import re
    src = inspect.getsource(module.Sound)
    out = []
    for hook in ('boom', 'hit', 'press_roll', 'erupt', 'trailer_hit'):
        if re.search(rf'bed\.{hook}\(', src) and not re.search(rf"hasattr\([^)]*'{hook}'", src):
            out.append(hook)
    return out


def full_sound_child(cfg, stems_out, rec_path):
    """the soundtrack of the fight with both real parts (a pair gate: the kit's audio.wav, sample for sample)"""
    from . import frames as F
    fresh_check(cfg.folder)
    M, B, D = load_music(cfg)
    F.check_recording(M.P, rec_path)
    out = _soundtrack(M, B, D, stems_out)
    out.update(fighters=list(M.NAMES), seed=int(M.EP.seed))
    return out


# ---------------------------------------------------------------- the voice
def voice_child(cfg, name, pitches, vels, durs, notes_out):
    import numpy as np
    from . import voices as V
    fresh_check(cfg.folder)
    M, B, D = load_music(cfg)
    k = list(M.NAMES).index(name)
    pp = M.CFG[k]['pp']
    lo, hi = B.entrant(pp)[3]
    if pitches is None:
        pitches = sorted({int(round(lo + (hi - lo) * q)) for q in (0.0, 1 / 3, 2 / 3, 1.0)})
    fresh_state = np.random.default_rng(V.NOISE_SEED).bit_generator.state['state']
    start = V.noise_state()
    assert (int(start['state']), int(start['inc'])) == (fresh_state['state'], fresh_state['inc']), \
        'something drew from the shared noise before the first note'
    alto = sys.modules.get('altosax60_lowpoly')       # the ALTO SAX's part measures its level when it is made
    if alto is not None:                              # (alto_sax.py Sound.__init__), a note drawn from this module's
        alto._RN = np.random.default_rng(23)          # own generator: the notes start from a fresh one (ModuleNoise)
    rows, arrays = [], {}
    for midi in pitches:
        for vel in vels:
            for dur in durs:
                before = V.noise_state()['state']
                x = B.voice(pp, midi, vel, dur)
                assert x.dtype == np.float64 and x.ndim == 1
                after = V.noise_state()['state']
                cid = f'c{len(rows):03d}'
                arrays[cid] = x
                blocks = [round(float(np.sqrt((x[i:i + VBLOCK] ** 2).mean())), 9) for i in range(0, len(x), VBLOCK)]
                rows.append(dict(id=cid, midi=int(midi), vel=common.hexf(float(vel)), dur=common.hexf(float(dur)),
                                 noise_before=before, noise_after=after, samples=len(x),
                                 peak=common.hexf(float(np.abs(x).max())), rms=blocks, sha256=V.sha(x)))
    common.write_npz(notes_out, arrays, register=False)
    return dict(fighter=name, register=[int(lo), int(hi)], pitches=list(pitches), start=start, notes=rows)


# ---------------------------------------------------------------- the checks
def _checks_init(folder):
    """pool worker start: the config, then kit.picture, for checks.Frames (checks._draw reads checks._PICTURE)"""
    from . import checks as CK
    for k in THREAD_VARS:
        os.environ[k] = '1'
    serial_fork()
    E, _ = common.kit()
    E.load(folder)
    from ballfight.kit import picture
    CK._PICTURE = picture


def checks_child(cfg, rec_path):
    """every check of kit/checks.py on cfg's fight, through checks.py's own functions; compact for the budget"""
    import contextlib
    import io
    from . import checks as CK
    from . import frames as F
    fresh_check(cfg.folder)
    M, B, D = load_music(cfg)
    F.check_recording(M.P, rec_path)
    levels = M.A.section_levels                       # main() prints each span's level after writing the wav:
    M.A.section_levels = lambda path, spans: levels(path, [(a, b) for a, b in spans if int(b * SR) > int(a * SR)])
    try:
        with contextlib.redirect_stdout(io.StringIO()):   # an empty span (a solo the reassembly leaves no time for)
            wav = M.main()                                # is left out of that print, which nothing reads
    finally:
        M.A.section_levels = levels
    with open(wav, 'rb') as fh:
        wav_sha = sha_bytes(fh.read())
    from ballfight.kit import checks as KC
    from ballfight.kit import picture as P            # noqa: N812
    out = {}
    out['focus'] = CK.focus(KC)
    out['levels'] = CK.levels(KC, wav)
    out['band'] = CK.band(KC, M, M.EP.check)
    out['loudness'] = CK.loudness(KC, M, wav)
    P.render(0)
    build_atlases(P)
    with mp.get_context('spawn').Pool(WORKERS, initializer=_checks_init, initargs=(cfg.folder,)) as pool:
        frames = CK.Frames(pool)
        out['safezone'] = CK.safezone(KC, P, frames)
        out['flash'] = CK.flash(KC, P, frames)
        out['loop'] = CK.loop(KC, P, frames, wav)
    sz = out['safezone']
    sz['boxes'] = [[b['frame'], len(b['boxes']), sha_bytes('\n'.join(','.join(str(v) for v in box)
                                                                     for box in b['boxes']).encode())]
                   for b in sz['boxes']]
    sz['boxes_note'] = '[frame, boxes drawn, SHA-256 of the boxes, one "x0,y0,x1,y1,kind,info" a line]'
    return dict(episode_facts=dict(seed=int(M.EP.seed), names=list(M.NAMES), winner=int(P.WIN), t_ko=float(P.T_KO),
                                   t_fight=float(P.T.T_FIGHT), fps=P.T.FPS, frames=P.T.NF, audio_wav_sha256=wav_sha),
                frames_drawn=len(frames.got), checks=out)


def split_checks(doc):
    """(committed, rows): a checks_child result without its per-frame rows, which go to the cache (the safe-zone
    boxes of each frame, the flash areas, the loop steps, the levels windows); the committed part keeps every
    verdict, every headline number, the row counts and one SHA-256 over the frames' box hashes"""
    c = doc['checks']
    sz = c['safezone']
    rows = dict(safezone_boxes=sz.pop('boxes'), flash_area=c['flash'].pop('area'),
                loop_steps=c['loop']['picture'].pop('steps'), levels_rows=c['levels'].pop('rows'))
    sz['boxes_sha256'] = sha_bytes('\n'.join(b[2] for b in rows['safezone_boxes']).encode())
    sz.pop('boxes_note', None)
    sz['frames_rows'] = len(rows['safezone_boxes'])
    c['flash']['area_rows'] = len(rows['flash_area'])
    c['loop']['picture']['steps_rows'] = len(rows['loop_steps'])
    c['levels']['rows_count'] = len(rows['levels_rows'])
    rows['boxes_note'] = '[frame, boxes drawn, SHA-256 of the boxes, one "x0,y0,x1,y1,kind,info" a line]'
    return doc, rows


# ---------------------------------------------------------------- a pair gate's golden frames
def episode_select(P, events, weapon_hit):
    """(frames, absent): milestone 1's golden-frame rules (frames.select) for any pair, each rule skipped with its
    reason when the fight has no event for it. `weapon_hit` maps a fighter to the `how` of its weapon's hits (else
    its first hit that is not a wave, a shockwave or a slam)"""
    from . import frames as F
    T, TT, EV, names = P.T, P.TT, events, P.NAMES
    NF = T.NF
    picks, absent = [], []

    def add(f, rule, reason):
        if 0 <= f < NF:
            picks.append((int(f), rule, reason))

    def first_event(test):
        return next((e for e in EV if test(e)), None)
    f_vs = first_at(T.TITLE[1][0])
    add(0, 'frame_0', 'frame 0: the title card (the loop point)')
    add(13, 'frame_13', f'frame 13, before VS slams in on frame {f_vs}')
    add(27, 'frame_27', 'frame 27')
    closed = [f for f in range(first_at(T.T_DOORS[1])) if TT.door_offset(f / P.FPS) == 0.0]
    opening = F.doors_moving(TT, range(first_at(T.T_DOORS[1])))
    add(closed[-1], 'doors_last_closed', 'the last frame with the card shut')
    add(opening[len(opening) // 2], 'doors_mid', 'the doors half-way open')
    for tc, txt in T.COUNT:
        add(first_at(tc), 'count_' + txt.strip('!').lower(), f'the count-in\'s "{txt}"')
    for k in (0, 1):
        for slot in F.SLOTS:
            ab = P.CFG[k]['abilities'][slot]['name']
            e = first_event(lambda e: e[1] == 'ability' and e[2] == k and e[3] == ab)
            for d in (0.2, 0.6):
                rule = f'{names[k]}_{slot}_{d}s'
                if e is None:
                    absent.append(dict(rule=rule, why=f'{names[k]} never uses {ab} in this fight'))
                else:
                    add(first_at(e[0] + d), rule, f"{d} s after {names[k]}'s first {ab}")
        how = weapon_hit.get(names[k])
        e = first_event(lambda e: e[1] == 'hit' and e[2] == k and (
            e[7] == how if how else e[7] not in ('wave', 'shockwave', 'slam')))
        if e is None:
            absent.append(dict(rule=f'{names[k]}_weapon_hit', why=f'{names[k]} lands no weapon hit'))
        else:
            add(first_at(e[0]), f'{names[k]}_weapon_hit', f"{names[k]}'s first weapon hit ('{e[7]}')")
        low = [f for f in range(first_at(T.T_FIGHT), first_at(P.T_KO)) if P.hp_at(k, f / P.FPS) < 25.0]
        if low:
            add(low[0], f'{names[k]}_low_hp', f"{names[k]}'s HP first below 25")
        else:
            absent.append(dict(rule=f'{names[k]}_low_hp', why=f"{names[k]}'s HP never drops below 25 before the K.O."))
    add(first_at(P.T_KO), 'ko', 'the K.O. frame')
    for d in (0.2, 0.6, 1.5):
        add(first_at(P.T_KO + d), f'ko_{d}s', f'{d} s after the K.O.')
    add(first_at((P.T_SOLO + T.T_REASSEMBLE) / 2), 'solo_mid', "the middle of the winner's solo")
    if P.CAP_COMMENT:
        add(first_at(P.CAP_COMMENT[0][0] + 0.6), 'comment', '0.6 s into the COMMENT caption')
    add(first_at((T.T_REASSEMBLE + T.T_GLIDE) / 2), 'reassembly_mid', "the loser's pieces half-way home")
    add(first_at((T.T_GLIDE + T.T_REV) / 2), 'glide_mid', 'the glide to the corners')
    add(NF - 2, 'frame_last_but_one', 'the doors almost shut')
    add(NF - 1, 'frame_last', 'the last frame (the loop seam)')
    # frames.select's extras: states the rules above leave out
    if P.STORY['caps']:
        add(first_at(P.STORY['caps'][0][0] + 0.1), 'extra_caption_punch', 'the first ability caption as it lands')
    for k in (0, 1):
        e = first_event(lambda e: e[1] == 'level' and e[2] == k and e[3] == 2)
        if e is not None:
            add(first_at(e[0] + 0.1), f'extra_level_{names[k]}', f'0.1 s after {names[k]} reaches level 2')
    e = first_event(lambda e: e[1] == 'slam')
    if e is not None:
        add(first_at(e[0]), 'extra_slam', f'on the first slam (at {e[0]} s)')
    add(first_at(P.T_KO + 0.1), 'extra_ko_caption_punch', 'the K.O. caption as it lands')
    if P.CAP_WINS:
        add(first_at(P.CAP_WINS[0][0] + 0.22 + 0.1), 'extra_wins_second_line', "the WINS caption's second line")
    closing = F.doors_moving(TT, range(first_at(T.DUR - T.T_CLOSE), NF))
    if closing:
        add(closing[len(closing) // 2 - 1], 'extra_doors_closing', 'the doors sliding shut over the last bar')
    for f in range(45, NF, 90):                       # a spread, until there are MIN_GOLDEN
        if len({p[0] for p in picks}) >= MIN_GOLDEN:
            break
        if f not in {p[0] for p in picks}:
            add(f, 'spread', 'filled up to the count the template asks for')
    frames = []
    for f in sorted({p[0] for p in picks}):
        mine = [p for p in picks if p[0] == f]
        frames.append(dict(frame=f, rules=[p[1] for p in mine], reasons=[p[2] for p in mine]))
    return frames, absent


MIN_GOLDEN = 40


def frames_child(cfg, rec_path, weapon_hit):
    """a pair gate's golden frames and card frames, both real looks, every hull call noted"""
    import numpy as np
    from . import frames as F
    from . import look_flute as LF
    calls = []
    _hull_patch(calls)
    fresh_check(cfg.folder)
    cfg.load()
    from ballfight.kit import picture as P            # noqa: N812
    rec = F.check_recording(P, rec_path)
    frames, absent = episode_select(P, rec['events'], weapon_hit)
    rows = []
    for fr in frames:
        del calls[:]
        P.ui.reset_drawn()
        img = P.render(fr['frame'])
        assert img.dtype == np.uint8
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in P.ui.DRAWN]
        hull = [[len(pts), [int(v) for v in vs], common.hexf(float(pts[:, 0].sum())),
                 common.hexf(float(pts[:, 1].sum()))] for pts, vs in calls]
        rows.append(dict(frame=fr['frame'], sha256=LF.sha_array(img), boxes=[len(boxes), LF.sha_lines(boxes)],
                         hull=hull, rules=fr['rules']))
    return dict(recording=dict(hash=rec['hash'], seed=rec['seed'], winner=rec['winner']), frames=rows,
                absent=absent, cards=card_rows(P), fighters=list(P.NAMES))


# ---------------------------------------------------------------- a theme's bed alone and its hooks
def bed_child(cfg_folder, stems_out):
    """the bed alone (both fighters the kit's base Sound) on the config in `cfg_folder`"""
    fresh_check(cfg_folder)
    E, _ = common.kit()
    E.load(cfg_folder)
    from ballfight.kit import music as M              # noqa: N812
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly as D
    M.SOUNDS = [M.FIGHTER_MODULES.base.Sound(M, k) for k in (0, 1)]
    out = _soundtrack(M, B, D, stems_out)
    out.update(seed=int(M.EP.seed), names=list(M.NAMES), bpm=float(M.T.BPM), key=M.T.KEY,
               hooks=sorted(h for h in ('boom', 'hit', 'press_roll', 'erupt', 'trailer_hit')
                            if callable(getattr(M.bed, h, None))))
    return out


HOOK_CALLS = {                                        # each optional hook at three times with three gains
    'boom': [dict(t=t, g=g) for t, g in ((5.0, 0.5), (20.0, 0.8), (40.0, 1.0))],
    'hit': [dict(t=t, chord=None, g=g, ring=r) for t, g, r in ((5.0, 0.5, 0.4), (20.0, 0.8, 1.0), (40.0, 1.0, 2.0))],
    'press_roll': [dict(t0=t, t1=t + 1.5, g0=g, g1=g * 1.5) for t, g in ((5.0, 0.3), (20.0, 0.5), (40.0, 0.7))],
    'erupt': [dict(t=t) for t in (5.0, 20.0, 40.0)],
    'trailer_hit': [dict(t=t, taiko_g=g, seed=s, braam=b, braam_g=g) for t, g, s, b in
                    ((5.0, 0.5, 1, None), (20.0, 0.8, 2, 0.9), (40.0, 1.0, 3, 1.4))],
}


# a recorded argument's name in the kit's methods where the two differ: the kit's hit names the chord `ch`, and
# brass_trailer's trailer_hit names the braam's length `braam_dur` (bed.json keeps HOOK_CALLS' names: the harness
# reads `chord` and `braam`)
HOOK_KWARGS = dict(chord='ch', braam='braam_dur')
# the recorded arguments a hook's method may lack, each with why that is right: a hook's call is otherwise passed
# exactly, an argument the method does not take, or a parameter the call does not give, stops the export
HOOK_OMITTED = {
    ('hit', 'ring'): "the hit of dark_disco, heist and horror_trap takes no ring (a fixed tail); the Rust Bed::hit "
                     "takes one for every theme and theirs ignore it (dark_disco's `_ring`)",
}


def hook_kwargs(hook, args, fn):
    """the keywords one HOOK_CALLS call passes to the kit's `fn` (a bound Bed method): every argument under the
    method's own name (HOOK_KWARGS); an argument the method does not take only when HOOK_OMITTED lists it, and
    every parameter after `mix` given. Anything else stops the export, so no argument is silently dropped"""
    params = [n for n in inspect.signature(fn).parameters if n != 'mix']
    out = {}
    for a, v in args.items():
        name = HOOK_KWARGS.get(a, a)
        if name in params:
            out[name] = v
        elif (hook, a) not in HOOK_OMITTED:
            raise SystemExit(f"hook_child: {hook}'s call passes {a!r} (as {name!r}), which "
                             f"{getattr(fn, '__qualname__', hook)}{inspect.signature(fn)} does not take: map it in "
                             f"m3parts.HOOK_KWARGS, or list it in HOOK_OMITTED with why")
    missing = [n for n in params if n not in out]
    if missing:
        raise SystemExit(f"hook_child: {hook}'s call gives no {', '.join(missing)}, which "
                         f"{getattr(fn, '__qualname__', hook)}{inspect.signature(fn)} takes: add it to HOOK_CALLS")
    return out


def hook_child(cfg_folder, hooks, out_path):
    """each optional hook of the bed called directly on an empty mix (the arguments of HOOK_CALLS; a `chord` of None
    is the chord at that time): {hook: [{args, buses: {bus: sha256}}]} and the buses in a cache npz"""
    import numpy as np
    fresh_check(cfg_folder)
    E, _ = common.kit()
    E.load(cfg_folder)
    from ballfight.kit import music as M              # noqa: N812
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    out, arrays = {}, {}
    for hook in hooks:
        rows = []
        for j, args in enumerate(HOOK_CALLS[hook]):
            args = dict(args)
            if 'chord' in args and args['chord'] is None:
                args['chord'] = M.T.chord_at(args['t']) if hasattr(M.T, 'chord_at') else M.T.CHORDS[
                    int(args['t'] // M.T.BAR)]
            mix = M.A.Mix(M.T.DUR + 4.0)
            fn = getattr(M.bed, hook)
            fn(mix, **hook_kwargs(hook, args, fn))
            buses = {}
            for bus, x in mix.buses.items():
                arrays[f'{hook}{j}__{bus}'] = np.asarray(x, np.float64)
                buses[bus] = hashlib.sha256(np.ascontiguousarray(x, '<f8').tobytes()).hexdigest()
            rows.append(dict(args={a: (common.hexf(float(v)) if isinstance(v, float) else v) for a, v in args.items()},
                             buses=buses))
        out[hook] = rows
    common.write_npz(out_path, arrays, register=False)
    return out


# ---------------------------------------------------------------- stage dumps (fighter_stages)
def stages_child(cfg, name, rec_path, frames, out):
    """stages.py's dumps of `frames`, drawn with the fighter's real look and the base look for the other side"""
    from . import frames as F
    from . import stages as ST
    from . import look_flute as LF
    fresh_check(cfg.folder)
    cfg.load()
    from ballfight.kit import fighters as FM
    real, bare = LF._spied(FM.module(name).Look, 'RealSpy'), LF._spied(FM.base.Look, 'BareSpy')
    FM.look = lambda nm, P, k: (real if nm == name else bare)(P, k)
    from ballfight.kit import picture as P            # noqa: N812
    rec = F.check_recording(P, rec_path)
    ST.OUT = out
    for f in frames:
        ST.dump(P, f, rec)
    return out

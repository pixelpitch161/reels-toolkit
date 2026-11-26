"""title_optins: the VS title card's three opt-ins as the Python kit draws them, and the ring beam's hull calls.

    title_optins   studio/reference/picture_title_optins/title_optins.json
    hull_log       studio/reference/picture_title_optins/hull_beam.json

The opt-ins are a kit config's TITLE_LINES (a long name on two lines), TITLE_BALLS (the big balls moved) and MIDDLE
(another word than VS, turned; fitted by title._fit_middle). The Studio reads them from a fight file's `picture`
settings (`title_lines`, `title_balls`, `middle`). Each case is one kit config's card, drawn as title.draw(noise, t)
over the title vectors' fixed noise (frames.NOISE_SEED) on every frame the card is drawn (door_offset not None and
below OPEN_PX - 1), each pinned by the SHA-256 of its bytes. Each case runs in a fresh spawned process: title.py
keeps the layout, the panels and the sprites in module state.

The cards are the configs' own (names, colours, tempo, series tag, the three opt-ins) with two stand-ins, because the
fighters that use the opt-ins are not ported yet:

  - a panel pattern of title.PATTERNS that is not ported (BASS CLARINET, EUPHONIUM, HORN, BASSOON, BARI SAX) is
    taken out of PATTERNS, so the panel is the plain gradient: the opt-ins do not touch the panels;
  - those fighters' close-ups are title.CLOSEUP entries drawn by `standin_closeup` (a striped disc of integer
    arithmetic, the same function in tests/title_optins.rs), so no model of an unported fighter is needed. TROMBONE
    keeps its own panel pattern (_slide_tubes) and its model's close-up.

No look's title hooks are registered (the looks of the stood-in fighters are not ported; TROMBONE's look has none).

The kit's TITLE_BALLS are Python ints, a fight file's are floats: the bass clarinet example is drawn a second time
with every TITLE_BALLS value a float (74.0), and the two must be the same bytes on every frame (asserted here; the
file records it).

A MIDDLE word that cannot fit is the kit's ValueError: its message is recorded.

hull_log: the episode's renderer (kit.picture with both real looks) with scipy.spatial.ConvexHull wrapped to note
every call's points and vertices, on a spread of frames: the ring's spot beam is the one hull of those two looks. A
Rust test turns on bf_picture::hull's log, draws the same frames and compares call by call."""
import hashlib
import multiprocessing as mp
import os
import sys

import numpy as np

from . import common
from . import frames as F

OUT = common.ref('picture_title_optins', 'title_optins.json')
HULL_OUT = common.ref('picture_title_optins', 'hull_beam.json')
FPS = F.FPS
NOISE_SEED = F.NOISE_SEED
EXAMPLES = os.path.join('projects', 'ballfight', 'examples')

# name -> the config, the fighters stood in (pattern out of PATTERNS, close-up by standin_closeup), changes to it
CASES = {
    'bassclarinet_vs_euphonium': dict(config=os.path.join(EXAMPLES, 'bassclarinet_vs_euphonium', 'episode.py'),
                                      standins=('BASS CLARINET', 'EUPHONIUM'), why='TITLE_LINES (left), TITLE_BALLS'),
    'bassclarinet_vs_euphonium_floats': dict(config=os.path.join(EXAMPLES, 'bassclarinet_vs_euphonium', 'episode.py'),
                                             standins=('BASS CLARINET', 'EUPHONIUM'), floats=True,
                                             why='the same card with TITLE_BALLS as floats: the same bytes'),
    'bassclarinet_vs_bassoon': dict(config=os.path.join('projects', 'ballfight_bassclarinet_vs_bassoon', 'episode.py'),
                                    standins=('BASS CLARINET', 'BASSOON'), why='TITLE_LINES (left), no TITLE_BALLS'),
    'barisax_vs_bassclarinet': dict(config=os.path.join(EXAMPLES, 'barisax_vs_bassclarinet', 'episode.py'),
                                    standins=('BARI SAX', 'BASS CLARINET'), why='TITLE_LINES (right), TITLE_BALLS'),
    'horn_vs_trombone_rematch': dict(config=os.path.join('projects', 'ballfight_horn_vs_trombone_rematch', 'episode.py'),
                                     standins=('HORN',), why="MIDDLE = ('REMATCH', 'divider')"),
    'rematch_degrees': dict(config=os.path.join('projects', 'ballfight_horn_vs_trombone_rematch', 'episode.py'),
                            standins=('HORN',), middle=('ROUND 2', -12.5),
                            why='MIDDLE with an angle in degrees (a variant of the rematch, not a kit config)'),
    'rematch_upright': dict(config=os.path.join('projects', 'ballfight_horn_vs_trombone_rematch', 'episode.py'),
                            standins=('HORN',), middle=('ROUND 2', 0),
                            why="MIDDLE at 0 degrees: Python's falsy angle takes the upright paths (a variant)"),
}
TOO_LONG = ('THE ULTIMATE GRAND FINAL REMATCH', 'divider')    # a MIDDLE word that fits at no scale


def standin_closeup(k):
    """fn(r) -> an RGBA close-up sprite (2 r + 7 square): a disc of radius about 0.95 r with stripes of integer
    arithmetic, different per side; the same function in tests/title_optins.rs"""
    def closeup(r):
        n = 2 * r + 7
        yy, xx = np.mgrid[0:n, 0:n]
        c = n // 2
        inside = ((xx - c) ** 2 + (yy - c) ** 2) * 10 <= 9 * r * r
        rgb = np.stack([(xx * 7 + 40 * k) % 256, (yy * 5 + 90) % 256, ((xx + yy) * 3 + 20 * k) % 256], -1)
        out = np.zeros((n, n, 4), np.uint8)
        out[..., :3] = np.where(inside[..., None], rgb, 0).astype(np.uint8)
        out[..., 3] = inside.astype(np.uint8) * 255
        return out
    return closeup


def _config(name, conf, tag):
    """the case's config module, loaded from a private copy of its episode.py (caches land there), with the case's
    changes applied, and the loaded episode"""
    import importlib.util
    E, _ = common.kit()
    E.activate_paths()
    src = os.path.join(common.REPO, conf['config'])
    folder = os.path.join(common.CACHE, 'episodes', f'title_optins_{tag}', name)
    with open(src, 'rb') as fh:
        data = fh.read()
    common.write_bytes(os.path.join(folder, 'episode.py'), data)
    stale = os.path.join(folder, 'check', 'sprites_title.npz')       # title.big_sprite's cache, keyed by side
    if os.path.exists(stale):
        os.remove(stale)
    spec = importlib.util.spec_from_file_location(f'title_optins_{tag}', os.path.join(folder, 'episode.py'))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    if conf.get('floats'):
        cfg.TITLE_BALLS = [tuple(float(v) for v in b) for b in cfg.TITLE_BALLS]
    if 'middle' in conf:
        cfg.MIDDLE = conf['middle']
    ep = E.load(folder, cfg)
    return cfg, ep


def _render_module(T, names):
    """what title.py reads of kit.picture (its `render`): T, NAMES, CFG, COL, DARK"""
    import types
    from fighters import FIGHTERS
    RD = types.ModuleType('render')
    RD.T = T
    RD.NAMES = list(names)
    RD.CFG = [FIGHTERS[n] for n in RD.NAMES]
    RD.COL = [np.array(c['color'], float) for c in RD.CFG]
    RD.DARK = [np.array(c['dark'], float) for c in RD.CFG]
    sys.modules['render'] = RD
    return RD


def _hexf_or_int(x):
    return x if isinstance(x, (int, np.integer)) and not isinstance(x, bool) else common.hexf(float(x))


def _spec(sp):
    """a WORDS entry with its floats as float.hex() (ints stay ints)"""
    out = {}
    for k, v in sp.items():
        if isinstance(v, tuple) and k == 'dy':
            out[k] = [common.hexf(float(d)) for d in v]
        elif isinstance(v, tuple):
            out[k] = list(v)
        elif isinstance(v, (str, bool)):
            out[k] = v
        else:
            out[k] = _hexf_or_int(v)
    return out


def _drawn_text(drawn):
    """reelkit.ui.DRAWN as the text whose SHA-256 the file holds: one line `x0,y0,x1,y1,kind,info` a box"""
    return ''.join(f'{x0},{y0},{x1},{y1},{kind},{info}\n' for x0, y0, x1, y1, kind, info in drawn)


def card_frames(TT, T, noise):
    """frames.card_hashes, and with each frame its draw log (reelkit.ui.DRAWN): (frames, the SHA-256 of each frame,
    [(box count, SHA-256 of _drawn_text)] of each)"""
    from reelkit import ui
    frames, digests, logs = [], [], []
    for f in range(T.NF):
        off = TT.door_offset(f / FPS)
        if off is None or off >= TT.OPEN_PX - 1:
            continue
        ui.DRAWN.clear()
        src = noise.copy()
        out = TT.draw(src, f / FPS)
        assert np.array_equal(src, noise) and out.shape == (F.H, F.W, 3) and out.dtype == np.uint8, f
        frames.append(f)
        digests.append(hashlib.sha256(np.ascontiguousarray(out).tobytes()).hexdigest())
        logs.append([len(ui.DRAWN), hashlib.sha256(_drawn_text(ui.DRAWN).encode()).hexdigest()])
    return frames, digests, logs


def _case(name):
    """pool worker: one case's card hashes, its layout and its timeline"""
    conf = CASES[name]
    cfg, ep = _config(name, conf, name)
    T = ep.T
    assert (T.FPS, T.NF) == (FPS, F.NF), (T.FPS, T.NF)
    RD = _render_module(T, cfg.FIGHTERS)
    import title as TT
    if not T.SERIES:                                     # kit.picture: no episode tag on the card
        TT.ui_tag = lambda layer, alpha: None
    for k, n in enumerate(RD.NAMES):
        if n in conf['standins']:
            TT.PATTERNS.pop(n, None)
            TT.CLOSEUP[n] = standin_closeup(k)
    noise = np.random.default_rng(NOISE_SEED).integers(0, 256, size=(F.H, F.W, 3), dtype=np.uint8)
    frames, digests, logs = card_frames(TT, T, noise)
    from reelkit import ui
    cover = frames[len(frames) // 3]                    # a frame of the shut card with every word settled
    ui.DRAWN.clear()
    TT.draw(noise.copy(), cover / FPS)
    cover_log = [list(b) for b in ui.DRAWN]
    words = {('A' if n == 0 else 'VS' if n == 1 else 'B'): _spec(TT.WORDS[w])
             for n, (t0, w) in enumerate(T.TITLE)}
    balls = [[_hexf_or_int(b[q]) for q in ('cx', 'cy', 'r')] for b in TT.BALLS]
    return dict(
        case=name, fighters=list(RD.NAMES), bpm=cfg.BPM, series=T.SERIES or None,
        colours=[[float(c) for c in col] for col in RD.COL], darks=[[float(c) for c in d] for d in RD.DARK],
        views=[list(map(float, RD.CFG[k]['view'])) for k in (0, 1)],
        title_lines={n: list(v) for n, v in getattr(T, 'TITLE_LINES', {}).items()},
        title_balls=[[float(v) for v in b] for b in T.TITLE_BALLS] if getattr(T, 'TITLE_BALLS', None) else None,
        title_balls_python=[[type(v).__name__ for v in b] for b in T.TITLE_BALLS]
        if getattr(T, 'TITLE_BALLS', None) else None,
        middle=list(T.MIDDLE) if getattr(T, 'MIDDLE', None) else None,
        standins=list(conf['standins']), why=conf['why'],
        words=words, balls=balls, timeline_values=F.timeline_note(T),
        frames=[int(f) for f in frames], sha256=digests, drawn=logs,
        cover=dict(frame=int(cover), drawn=cover_log))


def _too_long(_):
    """pool worker: the kit's error for a MIDDLE word that fits at no scale"""
    conf = dict(CASES['horn_vs_trombone_rematch'], middle=TOO_LONG)
    cfg, ep = _config('too_long', conf, 'too_long')
    _render_module(ep.T, cfg.FIGHTERS)
    try:
        import title  # noqa: F401
    except ValueError as e:
        return dict(middle=list(TOO_LONG), fighters=list(cfg.FIGHTERS), error=str(e))
    raise AssertionError('the long MIDDLE word fitted')


def run():
    with mp.get_context('spawn').Pool(3, maxtasksperchild=1) as pool:
        rows = pool.map(_case, list(CASES), chunksize=1)
        err = pool.map(_too_long, [0])[0]
    by = {r['case']: r for r in rows}
    a, b = by['bassclarinet_vs_euphonium'], by['bassclarinet_vs_euphonium_floats']
    assert a['title_balls_python'] == [['int'] * 3] * 2 and b['title_balls_python'] == [['float'] * 3] * 2
    assert a['frames'] == b['frames'] and a['sha256'] == b['sha256'] and a['drawn'] == b['drawn'], \
        'TITLE_BALLS as floats drew other bytes'
    for r in rows:
        assert len(r['frames']) >= 16, (r['case'], len(r['frames']))
        print(f"title_optins: {r['case']}: {len(r['frames'])} card frames ({r['why']})")
    print(f"title_optins: MIDDLE {TOO_LONG[0]!r}: {err['error']}")
    common.write_json(OUT, dict(
        about='the VS title card with the kit configs\' opt-ins (TITLE_LINES, TITLE_BALLS, MIDDLE): every frame on '
              'which title.draw(noise, frame / 30) draws, pinned by the SHA-256 of its (640, 360, 3) uint8 bytes',
        tool='title_optins', fps=FPS, size=[F.W, F.H], mode='RGB',
        noise=dict(file='frames/flute_vs_trombone/title/noise.png', seed=NOISE_SEED, call=F.NOISE_CALL),
        standin='standin_closeup(k)(r): n = 2 r + 7; c = n // 2; inside = ((x - c)^2 + (y - c)^2) * 10 <= 9 r r; '
                'rgb = ((x * 7 + 40 k) % 256, (y * 5 + 90) % 256, ((x + y) * 3 + 20 k) % 256) inside, else 0; '
                'alpha 255 inside. The stood-in fighters\' PATTERNS entries are removed (plain panels).',
        note='Each case is a kit config (or a named variant) as `why` says, drawn in its own process with the '
             'stand-ins listed. words: title.WORDS by role (A, VS, B), floats as float.hex(); balls: title.BALLS. '
             'title_balls_python: the Python types of the config\'s TITLE_BALLS values. The _floats case is the '
             'bass clarinet example with every TITLE_BALLS value a float; its hashes equal the int case\'s. '
             'drawn: per frame, the count of reelkit.ui.DRAWN boxes and the SHA-256 of their lines '
             '"x0,y0,x1,y1,kind,info" (each ending in a newline); cover: one frame\'s boxes in full.',
        cases=rows, middle_error=err))
    size = os.path.getsize(OUT)
    print(f'title_optins: {sum(len(r["frames"]) for r in rows)} card hashes, {size / 1e3:.1f} KB -> {common.rel(OUT)}')


# ---------------------------------------------------------------- the ring beam's hull calls
HULL_FRAMES = list(range(0, F.NF, 183)) + [1500, 1501, 1502]


def run_hull():
    import scipy.spatial
    P = F.picture('hull_log')
    calls = []
    real = scipy.spatial.ConvexHull

    class Noted(real):                                  # every call's points and vertices, in call order
        def __init__(self, points, *a, **kw):
            super().__init__(points, *a, **kw)
            calls.append((np.array(points, np.float64), np.array(self.vertices)))

    scipy.spatial.ConvexHull = Noted
    rows = []
    try:
        for f in sorted(set(HULL_FRAMES)):
            calls.clear()
            img = P.render(f)
            rows.append(dict(frame=f, sha256=hashlib.sha256(np.ascontiguousarray(img).tobytes()).hexdigest(),
                             calls=[dict(points=[[common.hexf(float(x)), common.hexf(float(y))] for x, y in pts],
                                         vertices=[int(v) for v in vs]) for pts, vs in calls]))
    finally:
        scipy.spatial.ConvexHull = real
    n = sum(len(r['calls']) for r in rows)
    assert all(len(r['calls']) == 1 for r in rows), [len(r['calls']) for r in rows]
    common.write_json(HULL_OUT, dict(
        about='every scipy.spatial.ConvexHull call kit.picture.render(f) makes for the episode (both real looks): '
              'its points (float.hex()) and vertices, per frame, in call order; and the frame\'s SHA-256',
        tool='hull_log', episode=F.EPISODE, recording='fights/flute_vs_trombone/ep392.bfrec', frames=rows))
    print(f'hull_log: {len(rows)} frames, {n} hull calls -> {common.rel(HULL_OUT)}')


EXPORTS = {'title_optins': run, 'hull_log': run_hull}

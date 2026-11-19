"""look_flute: the example episode drawn with the FLUTE's real look and the kit's base-class look for the TROMBONE.

    look_flute          studio/reference/picture_looks/flute_look.json
    look_flute_stages   (no reference export) `look_flute_stages [fight] <frame>...`: the stage dumps of those frames,
                        drawn the same way, into studio/reference/.cache/stages_look_flute/<fight>/ (git-ignored; the
                        files the `stages` export writes, see stages.py). `fight` is a name of FIGHTS, `episode` when
                        it is left out

Both fighters are in every frame of the episode, so no golden frame shows one look alone. This export draws the
episode with ONE real look: `ballfight.kit.fighters.look` is replaced, for the drawing process only and before the
renderer is imported, so that FLUTE gets `kit/fighters/flute.py`'s Look and every other fighter the base class
(`kit/fighters/base.py`: the bare ball). Nothing in the kit is edited. The Studio's test
(`studio/crates/bf-content/tests/flute_look.rs`) draws the same frames with its `FluteLook` and `DefaultLook`.

Three fights are drawn (FIGHTS), each from a recording of the reference set, which the export first proves to be
the run the kit drew from (frames.check_recording):

    episode       the example episode itself (seed 392): the FLUTE loses, so its silver shards fly
    flute_wins    the same episode config with SEED = 3 and nothing else changed: the FLUTE wins, so its two victory
                  baskets stand beside it through its solo (the one layer of the look the episode never shows)
    flute_right   the config with SEED = 1 and the two sides reversed (FIGHTERS and DMG_SIDE, as the `scenarios`
                  export's `swapped` fights): the FLUTE fights from the right corner, as side 1, and wins

For each fight flute_look.json holds, under `fights`, for each frame chosen (`select`: a spread over the whole video,
plus the frames where each layer of the look is on screen: baskets landing and taken back, cobras rising, striking
and biting, a swat, FLUTTER TONGUE, SNAKE DANCE, PICCOLO's ring and its glass cracks, the K.O., the silver shards,
the title card; `select_other`: a thinner spread, each layer's first and middle frame, the K.O. and the victory
baskets from first to last):

    the SHA-256 of the finished frame's RGB bytes (kit.picture.render(frame), (640, 360, 3) uint8, C order);
    how many look hooks the frame called, and the SHA-256 of their names in call order, one a line:
    '<side>.<hook>' (with the ball, and the phase for `state`, where a hook is called per ball);
    how many boxes reelkit.ui.DRAWN holds after the frame, and the SHA-256 of them, one a line;
    which layers of the FLUTE's look the frame shows (frames.tags, the FLUTE's and the phase's only, and
    FLUTE:solo_baskets);
    why the frame was chosen

and the hook calls made while the renderer is built (`hooks_build`) and the look's state at a few times (`state`: its
baskets, its strikes, its cobras). At the top level, `title` holds the look's constants and its three title-card
hooks: the glass, the close-up sprite and the panel pattern on a fixed base picture, each by its SHA-256, with a few
values written out.

Each fight is drawn in its own spawned process (the kit's renderer is a module that builds everything at import, with
whatever looks and whatever run it finds then; another export of the same run may already hold it with both real
looks)."""
import hashlib
import importlib.util
import os
import sys

import numpy as np

from . import common
from . import frames as F

OUT = common.ref('picture_looks', 'flute_look.json')
STAGES_OUT = os.path.join(common.CACHE, 'stages_look_flute')
NAME = 'FLUTE'
FIGHTS = {                                               # name -> the SEED of the config, the recording of that run
    'episode': dict(seed=392, file='ep392.bfrec', swapped=False, winner='TROMBONE', min_frames=64),
    'flute_wins': dict(seed=3, file='seed3.bfrec', swapped=False, winner='FLUTE', min_frames=32),
    'flute_right': dict(seed=1, file='scenarios/swapped_s1.bfrec', swapped=True, winner='FLUTE', min_frames=32),
}
HOOKS = ('setup', 'state', 'ball', 'ball_sprite', 'hides_hp', 'shake', 'rope', 'spark', 'hit_glow', 'fx_setup',
         'fx_event', 'fx_after', 'ko_fx', 'fx_end', 'light', 'shadows', 'shadows_free', 'canvas', 'canvas_wave',
         'canvas_after', 'canvas_overlay', 'items_ball', 'items_free', 'draw_item', 'over_ring', 'draw_shard',
         'shimmer', 'shimmer_after', 'glows', 'fringe', 'fringe_size', 'after_grade')
SOLO_BASKETS = 'FLUTE:solo_baskets'
LOOK_TAGS = ('FLUTE:basket', 'FLUTE:cobra', 'FLUTE:strike', 'FLUTE:swatted', 'FLUTE:dance', 'FLUTE:haste',
             'FLUTE:wave', 'FLUTE:shards', SOLO_BASKETS)
PHASE_TAGS = ('card', 'card_closed', 'doors', 'rev', 'fight', 'hitstop', 'tail', 'solo', 'reassembly', 'glide',
              'glass_shards', 'marks', 'particles', 'flares', 'shimmer', 'burst', 'fringe')
LOG = []


def hx(x):
    if isinstance(x, (list, tuple, np.ndarray)):
        return [hx(v) for v in x]
    return float(x).hex()


def sha_lines(lines):
    return hashlib.sha256('\n'.join(lines).encode()).hexdigest()


def sha_array(a):
    return hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()


# ---------------------------------------------------------------- the renderer with one real look
def _spied(base_cls, label):
    """a subclass of `base_cls` that notes every hook call in LOG before making it"""
    def spy(name):
        def call(self, *a, **kw):
            tag = f'{self.k}.{name}'
            if name == 'state':
                tag += f'({a[0]},{a[3]})'
            elif name in ('ball', 'ball_sprite', 'hides_hp'):
                tag += f'({a[0]})'
            LOG.append(tag)
            return getattr(base_cls, name)(self, *a, **kw)
        return call
    return type(label, (base_cls,), {n: spy(n) for n in HOOKS})


def picture(tag, fight='episode'):
    """kit.picture for the fight FIGHTS[fight] (the example episode's config, with that SEED and, for a swapped
    fight, the two sides reversed) with FLUTE's real look and the base-class look for the other fighter, every hook
    call noted in LOG. Must be the first import of the renderer in this process. The config is loaded from a
    private copy under the cache folder `tag`, as common.episode does"""
    assert 'ballfight.kit.picture' not in sys.modules, 'the renderer was imported before its looks were replaced'
    conf = FIGHTS[fight]
    seed = conf['seed']
    E, _ = common.kit()
    src = os.path.join(common.REPO, 'projects', 'ballfight', 'examples', F.EPISODE, 'episode.py')
    folder = os.path.join(common.CACHE, 'episodes', tag, F.EPISODE)
    with open(src, 'rb') as fh:
        data = fh.read()
    common.write_bytes(os.path.join(folder, 'episode.py'), data)
    spec = importlib.util.spec_from_file_location('look_flute_config', os.path.join(folder, 'episode.py'))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    assert cfg.SEED == 392 and tuple(cfg.FIGHTERS) == ('FLUTE', 'TROMBONE'), (cfg.SEED, cfg.FIGHTERS)
    assert tuple(cfg.DMG_SIDE) == (2.0, 0.40), cfg.DMG_SIDE
    cfg.SEED = seed                                       # the one value changed, but for a swapped fight
    if conf['swapped']:                                   # (nothing else in the config is per side: scenarios.py)
        cfg.FIGHTERS = tuple(reversed(cfg.FIGHTERS))
        cfg.DMG_SIDE = tuple(reversed(cfg.DMG_SIDE))
    names = tuple(cfg.FIGHTERS)
    E.load(folder, cfg)                                   # (the fighter modules import the episode's scene)
    from ballfight.kit import fighters as FM
    from ballfight.kit.fighters import flute
    real, bare = _spied(flute.Look, 'FluteSpy'), _spied(FM.base.Look, 'BareSpy')
    FM.look = lambda name, P, k: (real if name == NAME else bare)(P, k)
    from ballfight.kit import picture as P                # noqa: N812  (the kit's own short name for it)
    assert P.EP.name == F.EPISODE and P.EP.seed == seed, (P.EP.name, P.EP.seed)
    assert tuple(P.NAMES) == names and (P.W, P.H, P.FPS, P.T.NF) == (F.W, F.H, F.FPS, F.NF)
    assert P.NAMES[P.WIN] == conf['winner'], f'{fight}: {P.NAMES[P.WIN]} wins'
    k = P.NAMES.index(NAME)
    assert isinstance(P.LOOKS[k], flute.Look) and not isinstance(P.LOOKS[1 - k], flute.Look)
    assert type(P.LOOKS[1 - k]).__mro__[1] is FM.base.Look, type(P.LOOKS[1 - k]).__mro__
    return P


def look_tags(P, f):
    """frames.tags(P, f), plus FLUTE:solo_baskets when one of its two victory baskets is on screen"""
    k = P.NAMES.index(NAME)
    t = f / F.FPS
    out = F.tags(P, f)
    cobs = P.LOOKS[k].cobras_at(t, [P.fighter(kk, t) for kk in (0, 1)])
    if any(bid >= 100 and bk > 0.02 for home, bk, cp, t_drop, strike, aim, bid in cobs):
        out = sorted(out + [SOLO_BASKETS])
    return out


# ---------------------------------------------------------------- which frames
def select(P, events, tags):
    """the episode: [(frame, [reason, ...])] in frame order. `tags[f]` is look_tags(P, f) for every frame"""
    T, k = P.T, P.NAMES.index(NAME)
    L = P.LOOKS[k]
    ab = P.CFG[k]['abilities']
    ko = F.first_at(P.T_KO)
    picks = {}

    def add(f, why):
        f = int(f)
        if 0 <= f < F.NF:
            picks.setdefault(f, []).append(why)

    for f in range(0, F.NF, 45):
        add(f, 'spread')
    for f in (13, 27, 43, 44, 107, 108, 109, 1828, 1829):
        add(f, 'the title card, the doors and the count-in')
    for tag in LOOK_TAGS:                                  # each layer of the look: first, middle and last frame
        on = [f for f in range(F.NF) if tag in tags[f]]
        if tag == SOLO_BASKETS:
            assert not on, 'the FLUTE loses the episode: no victory baskets'
            continue
        assert on, f'no frame shows {tag}'
        for f, what in ((on[0], 'first'), (on[len(on) // 2], 'middle'), (on[-1], 'last')):
            add(f, f'{tag} ({what} of {len(on)} frames)')
    bids = list(L.BASKETS)                                 # a basket lands (the pop, the wicker dust) and is taken back
    for bid in bids[:3] + bids[len(bids) // 2:len(bids) // 2 + 1] + bids[-2:]:
        b = L.BASKETS[bid]
        add(F.first_at(b['t_drop']) + 2, f'basket {bid} lands')
        add(F.first_at(b['t_drop'] + T.BEAT * 0.5), f'basket {bid}: its cobra half risen')
        if b['t_gone'] < P.T_KO:
            add(F.first_at(b['t_gone']) + 3, f'basket {bid} is taken back')
    hits = [e for e in events if e[1] == 'hit' and e[2] == k]
    for how, n in (('fang', 3), ('wave', 3)):
        mine = [e for e in hits if e[7] == how]
        for e in mine[:n] + mine[-1:]:
            add(int(e[0] * F.FPS) + 1, f"a '{how}' hit at {e[0]} s: its glow, burst and particles")
    for slot in F.SLOTS:
        for e in [e for e in events if e[1] == 'ability' and e[2] == k and e[3] == ab[slot]['name']][:2]:
            for d in (0.2, 0.6, 1.4):
                add(F.first_at(e[0] + d), f"{d} s after {ab[slot]['name']} at {e[0]} s")
    for e in [e for e in events if e[1] == 'swat' and e[3] == k][:3]:
        for d in (0, 2, 20):
            add(F.first_at(e[0]) + d, f'{d} frames after a swat at {e[0]} s')
    for d in (-1, 0, 1, 2, 5, 11, 12, 30, 60):
        add(ko + d, f'{d} frames from the K.O.')
    for f in (F.first_at(P.T_HS + 0.6) - 1, F.first_at(P.T_HS + 0.3)):
        add(f, 'the baskets fading after the hit-stop')
    add(F.first_at((T.T_REASSEMBLE + T.T_GLIDE) / 2), "the loser's pieces half-way home")
    for d in (-4, 0, 3):
        add(F.first_at(T.T_GLIDE) + d, f'{d} frames from the glide home')
    return sorted(picks.items())


def select_other(P, events, tags):
    """a fight the FLUTE wins: a thinner spread, each layer's first and middle frame, the K.O., and its victory
    baskets from first to last"""
    T = P.T
    ko = F.first_at(P.T_KO)
    picks = {}

    def add(f, why):
        f = int(f)
        if 0 <= f < F.NF:
            picks.setdefault(f, []).append(why)

    for f in range(0, F.NF, 90):
        add(f, 'spread')
    for tag in LOOK_TAGS:
        on = [f for f in range(F.NF) if tag in tags[f]]
        if on and tag != SOLO_BASKETS:
            for f, what in ((on[0], 'first'), (on[len(on) // 2], 'middle')):
                add(f, f'{tag} ({what} of {len(on)} frames)')
    for d in (-1, 0, 2, 12, 30):
        add(ko + d, f'{d} frames from the K.O.')
    on = [f for f in range(F.NF) if SOLO_BASKETS in tags[f]]
    assert len(on) >= 30, f'the victory baskets show on {len(on)} frames only'
    for j in np.linspace(0, len(on) - 1, 12):              # popping in, the solo, sinking away before the glide
        add(on[int(round(float(j)))], f'the victory baskets (frames {on[0]}..{on[-1]})')
    for f in (on[0] + 1, on[0] + 3, on[0] + 6, on[-1] - 5, on[-1] - 1):
        add(f, 'the victory baskets popping in and sinking away')
    for ts in P.SOLO_T[:4]:                                # a solo note: the cobras glow with it
        add(F.first_at(ts) + 1, f'a note of the solo at {ts:.3f} s')
    for d in (0.3, 0.6, 0.9):
        add(F.first_at(T.T_REASSEMBLE + d), f'{d} s into the reassembly')
    for d in (-1, 0):
        add(F.first_at(T.T_GLIDE) + d, f'{d} frames from the glide home')
    return sorted(picks.items())


# ---------------------------------------------------------------- the look's own numbers
def look_state(P, L, full):
    """the look's precomputed state and its cobras at a few times (fewer unless `full`), floats as float.hex()"""
    T = P.T
    out = dict(
        baskets=[[int(bid), hx(b['t_drop']), hx(b['home']), hx(b['t_gone'])] for bid, b in L.BASKETS.items()],
        n_strikes=sum(len(q) for q in L.STRIKES.values()),
        strikes=[[int(bid), len(q), hx(q[0]), hx(q[-1])] for bid, q in list(L.STRIKES.items())[:8 if full else 3]],
        dances=hx(L.DANCES), swat_down=hx(L.SWAT_DOWN),
        swats=[[int(bid), hx(ts)] for bid, ts in L.SWATS.items()])
    times = [T.T_FIGHT, 20.0, P.T_KO + 0.2]
    if full:
        times += [10.0, 30.0, 40.0, P.T_KO - 0.5, P.T_HS + 0.3]
    times += [ts + d for tl in list(L.SWATS.values())[:1] for ts in tl[:1] for d in (0.03, 0.2, 0.6)[:3 if full else 1]]
    times += [q[0][0] + d for q in list(L.STRIKES.values())[:1] for d in (0.02, 0.12)[:2 if full else 1]]
    times += [a + d for a, b in L.DANCES[:1] for d in (0.1, 1.0)[:2 if full else 1]]
    if P.WIN == L.k:                                       # its victory baskets
        times += [P.T_SOLO - 0.7, P.T_SOLO - 0.3, P.T_SOLO + 1.0, T.T_REASSEMBLE + 0.8, T.T_GLIDE - 0.05]
    rows = []
    for t in sorted(times):
        fs = [P.fighter(kk, t) for kk in (0, 1)]
        cobs = []
        for home, bk, cp, t_drop, strike, aim, bid in L.cobras_at(t, fs):
            tk = min(t, P.T_KO)
            cobs.append([int(bid), hx(home), hx(bk), hx(cp), hx(t_drop), None if strike is None else hx(strike),
                         hx(aim), hx(L.swat_level(bid, tk)),
                         None if L.swat_lean(bid, tk) is None else hx(L.swat_lean(bid, tk))])
        rows.append([hx(t), hx(L.dancing_k(t)), cobs])
    out['cobras'] = rows
    return out


def title_hooks(L):
    """the look's three title-card hooks: the glass, the close-up and the panel pattern on a fixed base"""
    r = int(56 * 0.8)                                      # the radius title.big_sprite asks its close-ups at
    close = L.title_closeup(r)
    assert close.dtype == np.uint8 and close.shape == (2 * r + 5, 2 * r + 5, 4), (close.dtype, close.shape)
    yy, xx = np.mgrid[0:F.H, 0:F.W].astype(float)
    base = np.stack([40.0 + xx * 0.5 + yy * 0.25 + 20.0 * c for c in range(3)], -1)
    pat = type(L).title_pattern(base.copy())
    assert pat.dtype == np.float64 and pat.shape == (F.H, F.W, 3), (pat.dtype, pat.shape)
    at = [(0, 0), (180, 320), (359, 639), (100, 200), (201, 77), (17, 503)]
    return dict(
        glass=[int(v) for v in L.glass], flare_scale=hx(L.flare_scale), shards=list(L.shards),
        closeup=dict(call='Look.title_closeup(44)', radius=r, size=int(close.shape[0]), sha256=sha_array(close)),
        pattern=dict(call='flute_keys(base)',
                     base='base[y, x, c] = 40.0 + x * 0.5 + y * 0.25 + 20.0 * c, float64, (640, 360, 3)',
                     sha256=sha_array(pat), sha256_of='the float64 values, little-endian, C order',
                     samples=[[x, y, hx(pat[y, x])] for x, y in at]))


# ---------------------------------------------------------------- the export
def _draw(fight):
    """pool worker: (the fight's part of flute_look.json, the look's title hooks)"""
    conf = FIGHTS[fight]
    P = picture(f'look_flute_{fight}', fight)
    build_log = [x for x in LOG if '.rope' not in x]      # (the kit calls rope() twice for an event that shakes them)
    rec = F.check_recording(P, common.ref('fights', F.EPISODE, conf['file']))
    k = P.NAMES.index(NAME)
    L = P.LOOKS[k]
    tags = [look_tags(P, f) for f in range(F.NF)]
    rows = []
    for f, why in (select if fight == 'episode' else select_other)(P, rec['events'], tags):
        del LOG[:]
        P.ui.reset_drawn()
        img = P.render(f)
        assert img.shape == (F.H, F.W, 3) and img.dtype == np.uint8, (f, img.shape, img.dtype)
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in P.ui.DRAWN]
        rows.append([f, sha_array(img), len(LOG), sha_lines(LOG), len(boxes), sha_lines(boxes),
                     ' '.join(t for t in tags[f] if t in LOOK_TAGS or t in PHASE_TAGS), '; '.join(why)])
    for j in sorted({0, len(rows) // 2, len(rows) - 1}, reverse=True):   # a frame is a pure function of its number
        assert sha_array(P.render(rows[j][0])) == rows[j][1], rows[j][0]
    del LOG[:]
    full_tag = 'FLUTE:strike' if fight == 'episode' else SOLO_BASKETS
    f_full = next(r[0] for r in rows if full_tag in r[6].split())
    P.render(f_full)
    hooks_full = list(LOG)
    note = F.recording_note(rec, 'before any frame was drawn the export read this file and found it equal to the '
                                 'run the kit drew from')
    note['file'] = common.rel(common.ref('fights', F.EPISODE, conf['file']))
    doc = dict(
        seed=P.EP.seed, fighters=list(P.NAMES), k=k, winner=P.NAMES[P.WIN], swapped=conf['swapped'], recording=note,
        hooks_build=[len(build_log), sha_lines(build_log)], hooks_build_head=build_log[:8],
        hooks_frame=dict(frame=f_full, calls=hooks_full),
        covered={t: sum(1 for r in rows if t in r[6].split()) for t in LOOK_TAGS},
        state=look_state(P, L, fight == 'episode'), frames=rows)
    return fight, doc, title_hooks(L)


def check(fight, doc):
    rows, covered = doc['frames'], doc['covered']
    need = FIGHTS[fight]['min_frames']
    assert len(rows) >= need, f'{fight}: {len(rows)} frames, at least {need} are asked for'
    assert [r[0] for r in rows] == sorted({r[0] for r in rows})
    tags = [r[6].split() for r in rows]
    assert any('card' in t for t in tags)
    assert doc['k'] == (1 if FIGHTS[fight]['swapped'] else 0), doc['k']
    if fight == 'episode':
        assert all(n >= 2 for t, n in covered.items() if t != SOLO_BASKETS) and covered[SOLO_BASKETS] == 0, covered
    else:
        assert covered[SOLO_BASKETS] >= 16 and covered['FLUTE:shards'] == 0, covered
        assert all(covered[t] >= 2 for t in ('FLUTE:basket', 'FLUTE:cobra', 'FLUTE:strike')), covered


def run():
    import multiprocessing as mp
    with mp.get_context('spawn').Pool(len(FIGHTS), maxtasksperchild=1) as pool:
        results = pool.map(_draw, list(FIGHTS), chunksize=1)
    fights = {name: doc for name, doc, _ in results}
    titles = [t for _, _, t in results]
    assert all(t == titles[0] for t in titles), 'the title hooks differ between the fights'
    for name, doc in fights.items():
        check(name, doc)
    common.write_json(OUT, dict(
        about="the example episode drawn by the Python kit with FLUTE's real look (kit/fighters/flute.py) and the "
              'base-class look (kit/fighters/base.py) for the other fighter: frame hashes, hook calls, draw logs',
        tool='look_flute', episode=F.EPISODE, look=NAME,
        other_look='kit/fighters/base.py Look', fps=F.FPS, size=[F.W, F.H], mode='RGB',
        call='kit.picture.render(frame)',
        fight_notes=dict(episode='the example episode (examples/flute_vs_trombone/episode.py, SEED 392): the FLUTE '
                                 'loses',
                         flute_wins='the same config with SEED = 3 and nothing else changed (the fight of '
                                    'fights/flute_vs_trombone/seed3.bfrec): the FLUTE wins, so its two victory '
                                    'baskets stand beside it through its solo',
                         flute_right='the config with SEED = 1 and FIGHTERS and DMG_SIDE reversed (the fight of '
                                     'fights/flute_vs_trombone/scenarios/swapped_s1.bfrec): the FLUTE is side 1, '
                                     'in the right corner, and wins'),
        hook_names=list(HOOKS),
        hook_note="a hook call is '<side>.<hook>'; state adds '(ball,phase)', ball, ball_sprite and hides_hp add "
                  "'(ball)'. sprite, title_closeup, title_pattern and focus_points are not hooks of this list. "
                  'hooks_build leaves out rope (the kit asks it twice for an event that shakes the ropes)',
        tag_notes=dict({t: F.TAG_MEANINGS[t] for t in LOOK_TAGS + PHASE_TAGS if t != SOLO_BASKETS},
                       **{SOLO_BASKETS: 'one of its two victory baskets is on screen (it won)'}),
        frame_columns=['frame', 'sha256 of the frame', 'hook calls', 'sha256 of the hook calls', 'boxes drawn',
                       'sha256 of the boxes', 'tags', 'why the frame was chosen'],
        title=titles[0], fights=fights))
    n = {name: len(doc['frames']) for name, doc in fights.items()}
    print(f"look_flute: {' + '.join(f'{v} frames ({k})' for k, v in n.items())}, "
          f'{os.path.getsize(OUT) / 1e3:.1f} KB -> {common.rel(OUT)}')
    for name, doc in fights.items():
        print(f'look_flute: {name}: frames showing each layer: '
              + ', '.join(f'{t} {v}' for t, v in doc['covered'].items()))


def run_stages(args=()):
    """`look_flute_stages [fight] <frame>...`: stage dumps of the renderer with the one real look"""
    from . import stages as ST
    args = list(args)
    fight = args.pop(0) if args and args[0] in FIGHTS else 'episode'
    frames = [int(a) for a in args] or [150, 600, 1500]
    P = picture(f'look_flute_stages_{fight}', fight)
    rec = F.check_recording(P, common.ref('fights', F.EPISODE, FIGHTS[fight]['file']))
    ST.OUT = os.path.join(STAGES_OUT, fight)
    for f in frames:
        ST.dump(P, f, rec)
    print(f'look_flute_stages: {fight}: ' + ', '.join(str(f) for f in frames) + f' -> {common.rel(ST.OUT)}')


EXPORTS = {'look_flute': run, 'look_flute_stages': run_stages}
TAKES_ARGS = ('look_flute_stages',)                      # run alone with its frames; `all` leaves it out

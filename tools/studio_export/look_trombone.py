"""look_trombone: the TROMBONE's look on its own, frame by frame, as the Python kit draws it.

    look_trombone   studio/reference/picture_looks/trombone_frames.json

The two fighters are in every frame, so no frame of the episode can prove one look alone. Here the kit's renderer
(kit.picture) is built with the TROMBONE's real look (`Look` of kit/fighters/trombone.py) and the BASE-CLASS look
(kit/fighters/base.py) for the FLUTE: `ballfight.kit.fighters.look` is replaced for the worker process only, before
the renderer is imported; nothing in the kit is edited. The Studio's test (crates/bf-content/tests/trombone_look.rs)
draws the same frames with its `TromboneLook` and `DefaultLook` and must give the same bytes.

Two fights are drawn, each in a fresh process (kit.picture builds everything at import, once a process):

    ep392   the episode's own fight: the TROMBONE wins (its solo on the slide, the glide home)
    seed3   the same episode fought with seed 3: the TROMBONE loses (its shards, its K.O. spray), and PEDAL TONE's
            slam lands (a `slam` hit)

For each fight the file holds: the hook calls made while the renderer was built (their count and hash); what the
look's `shake`, `spark` and `hit_glow` give for each kind of its events; and for every chosen frame (`select`: a
spread over the whole video plus the frames where the lance's thrust streak, a landed lance, SAD TROMBONE's ring and
slow, POWER SLIDE, every phase of PEDAL TONE's bell, a swat, the K.O., the shards or the solo are on screen) the
SHA-256 of the finished frame's bytes, the count and hash of the hook calls of that frame, the count and hash of
`reelkit.ui.DRAWN` (the draw log), and `hides_hp` for both balls. A few frames carry their hook calls in full, the
look's `focus_points` and its state fields.

Each fight's committed recording is first proven to be the run the renderer was built from (frames.check_recording).

    look_trombone_stages <fight> <frame>...     (no reference export: local dumps for finding a difference)

writes studio/reference/.cache/stages_look_trombone/<fight>/f<frame>.npz: the buffers after each stage of
kit.picture.render for the same two looks (the files the `stages` export writes for the episode's own looks)."""
import hashlib
import os
import sys

from . import common
from . import frames as F

EPISODE = F.EPISODE
FPS, NF = F.FPS, F.NF
OUT = common.ref('picture_looks', 'trombone_frames.json')
STAGES_OUT = os.path.join(common.CACHE, 'stages_look_trombone')
FIGHTER = 'TROMBONE'
FIGHTS = {                                        # name -> the seed the episode is fought with, and its recording
    'ep392': dict(seed=392, recording=common.ref('fights', EPISODE, 'ep392.bfrec'), spread=45),
    'seed3': dict(seed=3, recording=common.ref('fights', EPISODE, 'seed3.bfrec'), spread=150),
}
HOOKS = ('setup', 'state', 'ball', 'ball_sprite', 'hides_hp', 'shake', 'rope', 'spark', 'hit_glow', 'fx_setup',
         'fx_event', 'fx_after', 'ko_fx', 'fx_end', 'light', 'shadows', 'shadows_free', 'canvas', 'canvas_wave',
         'canvas_after', 'canvas_overlay', 'items_ball', 'items_free', 'draw_item', 'over_ring', 'draw_shard',
         'shimmer', 'shimmer_after', 'glows', 'fringe', 'fringe_size', 'after_grade')
COLUMNS = ['frame', 'sha256 of the frame (640 x 360 x 3 uint8, C order)', 'hook calls', 'sha256 of the hook calls',
           'draw-log boxes', 'sha256 of the draw log', 'hides_hp(0, t)', 'hides_hp(1, t)', 'why the frame was chosen']
MIN_FRAMES = 64
LOG = []


# ---------------------------------------------------------------- the renderer with the two looks
def _spy(cls):
    """`cls` with every hook noted in LOG as '<its side>.<hook>' (with the ball and the phase where a hook is called
    per ball): the Spy of unit_checks/ring_unit_vectors.py, over a real look"""
    def spy(name):
        def call(self, *a, **kw):
            tag = f'{self.k}.{name}'
            if name == 'state':
                tag += f'({a[0]},{a[3]})'
            elif name in ('ball', 'ball_sprite', 'hides_hp'):
                tag += f'({a[0]})'
            LOG.append(tag)
            return getattr(cls, name)(self, *a, **kw)
        return call
    return type('Spy' + cls.__name__, (cls,), {name: spy(name) for name in HOOKS})


def _load(name):
    """this process's kit.picture for the fight `name`: the TROMBONE's own look, the base-class look for the other
    fighter, every hook call noted. Returns (P, the hook calls of the build, the looks' classes)"""
    assert 'ballfight.kit.picture' not in sys.modules, 'kit.picture is built once a process: use a fresh one'
    ep = common.episode(EPISODE, f'look_trombone_{name}')
    assert ep.seed == 392 and tuple(ep.T.FIGHTERS) == ('FLUTE', FIGHTER), (ep.seed, ep.T.FIGHTERS)
    ep.seed = FIGHTS[name]['seed']                # story.load() reads it: the run the renderer is built from
    from ballfight.kit import fighters as FM
    real = FM.module(FIGHTER).Look
    assert real is not FM.base.Look and issubclass(real, FM.base.Look)
    # the TROMBONE's look has no hook for the title card: its panel is the card's own pattern, its close-up the model
    assert real.title_pattern is None and real.title_closeup is FM.base.Look.title_closeup
    assert tuple(real.glass) == tuple(FM.base.Look.glass)
    classes = {FIGHTER: _spy(real)}
    FM.look = lambda nm, P, k: classes.setdefault(nm, _spy(FM.base.Look))(P, k)
    from ballfight.kit import picture as P        # noqa: N812
    assert tuple(P.NAMES) == ('FLUTE', FIGHTER) and P.EP.seed == FIGHTS[name]['seed'] == P.RUN['seed']
    assert [type(L).__mro__[1] for L in P.LOOKS] == [FM.base.Look, real], [type(L).__mro__ for L in P.LOOKS]
    build = [x for x in LOG if '.rope' not in x]  # (the kit calls rope() twice for an event that shakes the ropes)
    del LOG[:]
    return P, build


def _sha(lines):
    return hashlib.sha256('\n'.join(lines).encode()).hexdigest()


def _hx(x):
    if hasattr(x, '__len__'):
        return [_hx(v) for v in x]
    return float(x).hex()


# ---------------------------------------------------------------- which frames
def select(P, name):
    """[(frame, [reasons])] in frame order: a spread over the whole video and the frames where each thing the
    TROMBONE's look draws is on screen, chosen by fixed rules from the run"""
    from ballfight.kit.fighters import base, trombone
    T, EV = P.T, P.EV
    k = P.NAMES.index(FIGHTER)
    picks = {}

    def add(f, why):
        f = int(f)
        if 0 <= f < NF and why not in picks.setdefault(f, []):
            picks[f].append(why)

    at = F.first_at
    for f in list(range(0, NF, FIGHTS[name]['spread'])) + [NF - 1]:
        add(f, 'spread')
    for f, why in ((0, 'title'), (43, 'title'), (44, 'doors'), (107, 'count-in'), (108, 'FIGHT!')):
        add(f, why)
    f_fight, f_hs = at(T.T_FIGHT), at(P.T_HS)
    # the lance: a thrust going out leaves a streak (brighter under POWER SLIDE)
    streak = {False: [], True: []}
    for f in range(f_fight, f_hs):
        t = f / FPS
        st, prev = P.fighter(k, t), P.fighter(k, max(0.0, t - 1.0 / FPS))
        if st['ext'] - prev['ext'] > 0.12 and abs(base.wrap(st['th'] - prev['th'])) < 0.3:
            streak[st['haste'] > 0].append(f)
    assert streak[False] and streak[True], 'no thrust streak with and without POWER SLIDE'
    for f in streak[False][:2] + streak[False][-1:]:
        add(f, 'thrust streak')
    for f in streak[True][:2] + streak[True][-1:]:
        add(f, 'POWER SLIDE streak')
    # its hits, its swats
    hits = {}
    for e in EV:
        if e[1] == 'hit' and e[2] == k:
            hits.setdefault(e[7], []).append(e)
    assert 'blade' in hits and 'shockwave' in hits, sorted(hits)
    for j, e in enumerate(hits['blade'][:3]):
        for d in ((0, 1, 3) if j == 0 else (0,)):
            add(at(e[0]) + d, 'blade hit')
    for e in hits['shockwave'][:2]:
        for d in (0, 2):
            add(at(e[0]) + d, 'shockwave hit')
    for e in hits.get('slam', [])[:1]:
        for d in (0, 1, 3):
            add(at(e[0]) + d, 'slam hit')
    for e in [e for e in EV if e[1] == 'swat' and e[2] == k][:2]:
        add(at(e[0]), 'swat')
    # SAD TROMBONE: the ring on the canvas and in the air, the slowed enemy's wobble
    runs = []
    for f in range(f_fight, f_hs):
        if any(w[0] == k and w[5] == 'shockwave' for w in P.RUN['WAVES'][P.sim_i(f / FPS)]):
            if runs and runs[-1][-1] == f - 1:
                runs[-1].append(f)
            else:
                runs.append([f])
    assert runs, 'no shockwave ring'
    for f in (runs[0][0], runs[0][len(runs[0]) // 2], runs[0][-1]):
        add(f, 'SAD TROMBONE ring')
    for r in runs[1:4]:
        add(r[len(r) // 2], 'SAD TROMBONE ring')
    slow = [f for f in range(f_fight, f_hs) if P.fighter(1 - k, f / FPS)['slow'] > 0]
    assert slow, 'the enemy is never slowed'
    for f in (slow[0], slow[len(slow) // 2]):
        add(f, 'slowed enemy')
    # PEDAL TONE: the bell drops in, hovers, slams, sits, flies off; the target, the stomp ring and its shimmer
    bells = []
    for rows in P.RUN['SLAMS']:
        for row in rows:
            if row[0] == k and tuple(row) not in bells:
                bells.append(tuple(row))
    assert bells, 'PEDAL TONE is never used'
    for _, cx, cz, t0, t_slam, rad in bells:
        for why, t in (('bell drops in', t0), ('bell drops in', t0 + 0.15), ('bell hovers', (t0 + t_slam) / 2),
                       ('bell slams', t_slam - 0.12), ('bell slams', t_slam - 0.06), ('bell stomps', t_slam),
                       ('bell sits', t_slam + 0.15), ('bell flies off', t_slam + 0.31),
                       ('bell flies off', t_slam + 0.45), ('bell flies off', t_slam + 0.59),
                       ('bell flies off', t_slam + 0.7), ('bell gone', t_slam + 0.76)):
            assert (trombone.bell_height(t, t0, t_slam) is None) == (why == 'bell gone'), (why, t)
            add(at(t), why)
        add(at(t_slam) + 1, 'bell stomps')
    # the K.O. and the tail
    ko = int(round(P.T_KO * FPS))
    for d in (-1, 0, 1, 2, 5, 11, 12, 30):
        add(ko + d, 'K.O.')
    if k == P.WIN:
        solo = T.SOLO[FIGHTER]
        add(at(P.T_HS + 0.2), 'slide closes after the K.O.')
        for j in sorted({0, 1, 2, 5, len(solo) - 1}):
            ts = P.T_SOLO + solo[j][0] * T.BEAT
            if ts < T.T_REASSEMBLE:
                add(at(ts), 'solo on the slide')
                add(at(ts) + 1, 'solo on the slide')
        add(at(T.T_REASSEMBLE - 0.15), 'slide closes for the glide')
        add(at(T.T_REASSEMBLE + 0.1), 'slide closes for the glide')
        for d in (-4, 0, 3):
            add(at(T.T_GLIDE) + d, 'glide home')
    else:
        for d in (0, 1, 3, 8, 15, 30, 60, 100):
            add(f_hs + d, 'shards')
        for d in (2, 6, 12, 20):
            add(at(T.T_REASSEMBLE) + d, 'shards fly back')
        for d in (-6, -3, -1, 0, 3):
            add(at(T.T_GLIDE) + d, 'reassembled')
    return [(f, picks[f]) for f in sorted(picks)]


# ---------------------------------------------------------------- one fight (a pool worker)
def _event_key(e):
    return f'hit:{e[7]}' if e[1] == 'hit' else e[1]


def _fight(name):
    """everything the file holds for one fight; runs in a process of its own"""
    import numpy as np
    from reelkit import ui
    P, build = _load(name)
    rec = F.check_recording(P, FIGHTS[name]['recording'])
    k = P.NAMES.index(FIGHTER)
    look = P.LOOKS[k]
    T = P.T
    # what the look says about each kind of its events (the same for every event of a kind: checked)
    table = {}
    for e in P.EV:
        if P.owner(e) != k:
            continue
        a = look.shake(e)
        row = dict(shake=None if a is None else _hx(a))
        if e[1] == 'hit':
            col, n, sp, life = look.spark(e)
            g = look.hit_glow(e)
            row['spark'] = [_hx(col), int(n), _hx(sp), _hx(life)]
            row['hit_glow'] = None if g is None else _hx(g)
        assert table.setdefault(_event_key(e), row) == row, (e, row)
    picks = select(P, name)
    assert name != 'ep392' or len(picks) >= MIN_FRAMES, len(picks)
    rows, full, focus, state = [], {}, [], []
    del LOG[:]
    first = {}
    for f, why in picks:
        for w in why:
            first.setdefault(w, f)
    in_full = set(first.values()) if name == 'seed3' else {first[w] for w in first if w not in ('spread', 'title')}
    for f, why in picks:
        t = f / FPS
        del LOG[:]
        ui.reset_drawn()
        img = P.render(f)
        calls = list(LOG)
        boxes = [f'{x0},{y0},{x1},{y1},{kind},{info}' for x0, y0, x1, y1, kind, info in ui.DRAWN]
        assert img.dtype == np.uint8 and img.shape == (F.H, F.W, 3)
        hides = [int(bool(look.hides_hp(kk, t))) for kk in (0, 1)]
        rows.append([f, hashlib.sha256(np.ascontiguousarray(img).tobytes()).hexdigest(), len(calls), _sha(calls),
                     len(boxes), _sha(boxes), hides[0], hides[1], '; '.join(why)])
        if f in in_full:
            st = P.fighter(k, t)
            state.append([f, _hx(st['thr_k']), _hx(st['glow']), _hx(st['ext'])])
            focus.append([f, [[label, _hx(w)] for w, label in look.focus_points(t, st)]])
        if f in (first.get('bell sits'), first.get('blade hit'), first.get('shards')):
            full[str(f)] = calls
    del LOG[:]
    kinds = sorted({w for _, why in picks for w in why})
    return dict(
        name=name, side=k, wins=bool(k == P.WIN),
        recording=dict(file=common.rel(FIGHTS[name]['recording']), hash=rec['hash'], seed=rec['seed'],
                       winner=rec['winner'], t_ko=common.hexf(rec['t_ko']), events=len(rec['events']),
                       frames=len(rec['arrays']['HP']),
                       checked='the renderer was built from the kit\'s own run of this seed, after this file was '
                               'read and found equal to it'),
        times=dict(t_fight=common.hexf(T.T_FIGHT), t_hs=common.hexf(P.T_HS), t_solo=common.hexf(P.T_SOLO),
                   t_reassemble=common.hexf(T.T_REASSEMBLE), t_glide=common.hexf(T.T_GLIDE)),
        build=dict(calls=len(build), sha256=_sha(build), head=build[:8]),
        built=dict(particles=int(len(P.PA.cols['t0'])), flares=int(len(P.FL.tl.t0)), bursts=int(len(P.BU._ev)),
                   marks=int(len(P.MK.tl.t0)), moments=int(len(P.MO.tl.t0)), shakes=int(len(P.SHK.tl.t0)),
                   parts=[q['kind'] for q in P.PARTS]),
        events=table, reasons=kinds, frames=rows, hooks_full=full, focus=focus, state=state)


# ---------------------------------------------------------------- the export
def run():
    import multiprocessing as mp
    with mp.get_context('spawn').Pool(len(FIGHTS), maxtasksperchild=1) as pool:
        fights = pool.map(_fight, list(FIGHTS), chunksize=1)
    need = {'thrust streak', 'POWER SLIDE streak', 'blade hit', 'shockwave hit', 'slam hit', 'swat',
            'SAD TROMBONE ring', 'slowed enemy', 'bell drops in', 'bell hovers', 'bell slams', 'bell stomps',
            'bell sits', 'bell flies off', 'bell gone', 'K.O.', 'solo on the slide', 'shards', 'shards fly back'}
    have = {w for fight in fights for w in fight['reasons']}
    assert need <= have, f'no frame for: {sorted(need - have)}'
    assert [fight['wins'] for fight in fights] == [True, False], 'ep392: the TROMBONE wins; seed3: it loses'
    assert fights[1]['built']['parts'][24:] == ['slidebit'] * 4 + ['tube'] * 4 + ['bellbit'] * 6
    doc = dict(
        about='the example episode drawn by the Python kit with the TROMBONE\'s own look and the base-class look '
              'for the FLUTE, for two fights: the frames\' hashes, the hook calls and the draw log',
        tool='look_trombone', episode=EPISODE, fighter=FIGHTER,
        looks={'FLUTE': 'kit/fighters/base.py Look', FIGHTER: 'kit/fighters/trombone.py Look'},
        title_hooks=dict(title_pattern=None, title_closeup=None, glass=[236, 232, 226],
                         note='the look has none of the three: the base class\'s values'),
        hooks=dict(note='a hook call is "<the look\'s side>.<hook>", with "(<ball>,<phase>)" for state and '
                        '"(<ball>)" for ball, ball_sprite and hides_hp; a list of calls is hashed as its lines '
                        'joined by a newline; the build\'s list leaves out rope (the kit asks twice an event)',
                   names=list(HOOKS)),
        draw_log='a box of reelkit.ui.DRAWN is "x0,y0,x1,y1,kind,info"; the list is hashed like the hook calls',
        events_note='per kind of the look\'s own events ("hit:<what hit>" for a hit): shake(e), and for a hit '
                    'spark(e) as [colour, n, speed, life] and hit_glow(e) as [radius, alpha]; floats as hex',
        columns=COLUMNS,
        focus_note='[frame, [[label, world point], ...]]: focus_points(t, fighter(k, t)) of the look',
        state_note='[frame, thr_k, glow, ext] of fighter(k, t) for the TROMBONE',
        fights=fights)
    common.write_json(OUT, doc)
    n = sum(len(fight['frames']) for fight in fights)
    print(f"look_trombone: {common.rel(OUT)}  {os.path.getsize(OUT)} bytes  {n} frames "
          f"({', '.join(str(len(fight['frames'])) for fight in fights)})")


def stages(args=()):
    """look_trombone_stages <fight> <frame>...: the stage dumps of those frames with the two looks"""
    usage = 'python -m tools.studio_export look_trombone_stages ep392 150 600 1500'
    if len(args) < 2 or args[0] not in FIGHTS:
        raise SystemExit(f"look_trombone_stages: name a fight ({', '.join(FIGHTS)}) and frames: {usage}")
    try:
        frames = [int(a) for a in args[1:]]
    except ValueError:
        frames = [-1]
    if any(not 0 <= f < NF for f in frames):
        raise SystemExit(f'look_trombone_stages: name the frames as numbers 0..{NF - 1}: {usage}')
    from . import stages as ST
    P, _ = _load(args[0])
    fight = F.check_recording(P, FIGHTS[args[0]]['recording'])
    ST.OUT = os.path.join(STAGES_OUT, args[0])
    for f in frames:
        ST.dump(P, f, fight)


EXPORTS = {'look_trombone': run, 'look_trombone_stages': stages}
TAKES_ARGS = ('look_trombone_stages',)

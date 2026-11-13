"""origin_selfproof: Gate O (older/origin.py) proven on two modules the kit already holds, against the episodes they
were ported from (plan m4, Task 2):

- ORGAN (projects/ballfight/kit/fighters/organ.py) against #35 v4 (projects/ballfight_organ_vs_harp, seed 237): the
  kit README's "frames pixel-identical in a matched harness, organ buses bit-identical";
- the brass_trailer mood (kit/moods/brass_trailer.py) against #34 v5's audio (projects/ballfight_trombone_vs_horn, seed
  140, the kit config examples/trombone_vs_horn): "the strings, taiko, braam and tremolo buses bit-identical";

and that it fails for an unfaithful module: the ORGAN module changed at run time by one pixel of one frame (after its
after_grade hook) and by one sample of its bus (after its play()) must each stop the gate naming that frame and pixel,
or that bus and sample.

    python -m tools.studio_export origin_selfproof     writes studio/reference/older/selfproof/{organ,brass_trailer}.json
                                                        and summary.json (the mutations' failure lines); about 3 min

The ORGAN gate's episode patches (`other`: #35's HARP drawn as the kit's bare ball; `layers`: #35's ORGAN layers off,
so the episode draws the ORGAN as the kit's base Look does) and its harness (#35's camera 8 px higher and its 0.16 s beat
ramp) are what a port of an older fighter writes for its own episode (studio/docs/porting-an-older-fighter.md, Step 0).
Each patch is one exact text replacement in the scratch copy of render.py, never in projects/."""
import os

from . import common

OUT = common.ref('older', 'selfproof')
R = 'render.py'

# ---------------------------------------------------------------- #35: its HARP drawn as the kit's bare ball
HARP_BARE = [
    (R, "        evt = LINE_T if k in LINE_K else (STR_T if k in TRAP_K else None)",
     "        evt = LINE_T if k in LINE_K else None"),                                   # no thump on its strings
    (R, "    if e[1] == 'hit' and e[7] in ('pipe', 'slam', 'wave'):\n"
        "        SHK.add(e[0], {'pipe': 1.1, 'slam': 2.5, 'wave': 2.0}[e[7]])",
     "    if e[1] == 'hit' and e[7] in ('pipe', 'slam'):\n"
     "        SHK.add(e[0], {'pipe': 1.1, 'slam': 2.5}[e[7]])"),                         # base.Look.shake: no wave hits
    (R, "        if _ab['kind'] == 'shield':", "        if _ab['kind'] == 'shield' and NAMES[_k] != 'HARP':"),
    (R, "['string'] * 7 + ['maple'] * 5 + ['gold'] * 4 if loser == 'HARP'", "[] if loser == 'HARP'"),
    (R, "    out = []\n    if not STR_LIST:\n        return out", "    out = []\n    return out"),   # strings_at
    (R, "    if NAMES[WIN] != 'HARP':\n        return []", "    return []"),                 # solo_strings
    (R, "    for sid, q in STRINGS.items():\n        a, b = q['a'], q['b']",
     "    for sid, q in []:\n        a, b = q['a'], q['b']"),                               # string flares + motes
    (R, "    while tb < T_KO:", "    while False:"),                                       # beat motes
    (R, "            elif how == 'string':", "            elif how == 'string' and False:"),
    (R, "            elif how == 'key':", "            elif how == 'key' and False:"),
    (R, "            elif how == 'wave':", "            elif how == 'wave' and False:"),
    (R, "            BU.add(e[0], S.to_world(e[4], e[5], R), (220, 230, 255), 0.6, bright=False)\n"
        "            PA.emit(e[0], S.to_world(e[4], e[5], R), 'motes' if e[2] == HRP else 'dust', n=5,\n"
        "                    col=None if e[2] == HRP else DUST_COL)",
     "            BU.add(e[0], S.to_world(e[4], e[5], R), (220, 230, 255) if e[2] == ORG else (255, 226, 150), 0.6,\n"
     "                   bright=False)\n"
     "            if e[2] == ORG:\n"
     "                PA.emit(e[0], S.to_world(e[4], e[5], R), 'dust', n=5, col=DUST_COL)"),   # base block_burst
    (R, "        if e[1] == 'ability' and e[3] == '47 STRINGS':", "        if False:"),
    (R, "'motes' if NAMES[LOSE] == 'HARP' else 'dust'", "'sparks' if NAMES[LOSE] == 'HARP' else 'dust'"),  # ko_fx
    (R, "    if T.T_FIGHT <= t < T_HS:\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:",
     "    if False:\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:"),   # base shimmer: none
    (R, "    pipes = pipes_at(t)\n    strs = strings_at(t)\n",
     "    keys = []\n    pipes = pipes_at(t)\n    strs = strings_at(t)\n"),                 # its darts
    (R, "(16, 3.2, 0.3) if how in BIG_HITS else ((9, 2.6, 0.24) if how == 'string' else (5, 1.8, 0.18))",
     "(16, 3.2, 0.3) if how in BIG_HITS else (5, 1.8, 0.18)"),                          # base.Look.spark
    (R, "            col, n, sp, life = np.array([255, 226, 170]), 12, 2.6, 0.3",
     "            col, n, sp, life = (np.array([255, 226, 170]) if e[2] == ORG else np.array([160.0, 220.0, 255.0])), "
     "12, 2.6, 0.3"),                                                                    # base block_spark
    (R, "        wav = RUN['WAVES'][i] if in_fight else []\n        for owner, cx, cz, r, rmax, kind, t0 in wav:",
     "        wav = []\n        for owner, cx, cz, r, rmax, kind, t0 in wav:"),          # base canvas_wave: none
    (R, "        if kind == 'wave' and NAMES[owner] == 'HARP':", "        if False:"),     # 47 STRINGS' snaps
    (R, "        if e[7] in BIG_HITS or e[7] == 'string':", "        if e[2] == ORG and e[7] in BIG_HITS:"),  # hit_glow
    (R, "            if e[3] == k and e[7] not in ('key', 'string'):", "            if e[3] == k and e[7] != 'key':"),
]

# ---------------------------------------------------------------- #35: its ORGAN's layers (all off = base.Look)
ORGAN_LAYERS = [
    dict(name='thump', patches=[(R, "        evt = LINE_T if k in LINE_K else None", "        evt = None")]),
    dict(name='shake', patches=[(R, "    if e[1] == 'hit' and e[7] in ('pipe', 'slam'):",
                                 "    if e[1] == 'hit' and e[7] in ():")]),
    dict(name='cathedral', patches=[(R, "        if _ab['kind'] == 'shield' and NAMES[_k] != 'HARP':",
                                     "        if False:")]),
    dict(name='flashbulbs', patches=[(R, "(e[1] == 'hit' and e[7] in ('pipe', 'slam', 'wave', 'shockwave'))",
                                      "(e[1] == 'hit' and e[7] in ('slam', 'wave', 'shockwave'))")]),
    dict(name='shards', patches=[(R, "['pipe'] * 7 + ['case'] * 6 + ['gold'] * 2)", "[])")]),
    dict(name='pipes', patches=[
        (R, "    out = []\n    tc = hz_clock(t)\n    if tc >= T.T_FIGHT and len(LINE_T):",
         "    return []\n    out = []\n    tc = hz_clock(t)\n    if tc >= T.T_FIGHT and len(LINE_T):"),
        (R, "    if NAMES[WIN] != 'ORGAN':\n        return []", "    return []")]),
    dict(name='effects', patches=[
        (R, "    for l in LINES:\n", "    for l in []:\n"),
        (R, "            if how == 'pipe':", "            if how == 'pipe' and False:"),
        (R, "            elif how == 'slam':", "            elif how == 'slam' and False:"),
        (R, "(220, 230, 255) if e[2] == ORG else (255, 226, 150)", "(255, 226, 150)"),
        (R, "            if e[2] == ORG:\n                PA.emit", "            if False:\n                PA.emit"),
        (R, "    for ts, owner, cx, cz in SLAM_EV:\n        c = S.to_world(cx, cz, 0.1)",
         "    for ts, owner, cx, cz in []:\n        c = S.to_world(cx, cz, 0.1)"),
        (R, "'sparks' if NAMES[LOSE] == 'HARP' else 'dust', n=30, spread=180, speed=(0.8, 2.2),\n"
            "            col=None if NAMES[LOSE] == 'HARP' else DUST_COL)",
         "'sparks', n=30, spread=180, speed=(0.8, 2.2),\n            col=None)"),
        (R, "    if NAMES[WIN] == 'ORGAN':\n        for j, (bt, m, d) in enumerate(T.SOLO['ORGAN']):",
         "    if False:\n        for j, (bt, m, d) in enumerate(T.SOLO['ORGAN']):")]),
    dict(name='toccata', patches=[
        (R, "    for ts, owner, cx, cz in SLAM_EV:\n        u = (tc - ts) / 0.6",
         "    for ts, owner, cx, cz in []:\n        u = (tc - ts) / 0.6"),
        (R, "        sl = RUN['SLAMS'][i] if in_fight else []", "        sl = []")]),
    dict(name='fringe', patches=[(R, "        if e[7] in ('pipe', 'slam', 'wave'):",
                                  "        if e[7] in ('slam', 'wave', 'shockwave'):")]),
    dict(name='sparks', patches=[
        (R, "BIG_HITS = ('pipe', 'slam', 'wave', 'shockwave')", "BIG_HITS = ('slam', 'wave', 'shockwave')"),
        (R, "np.array([255, 226, 170]) if e[2] == ORG else", "np.array([255, 226, 170]) if False else"),
        (R, "        if kind == 'hit' and e[7] in ('pipe', 'slam') and dt < 0.08:",
         "        if kind == 'hit' and e[7] in () and dt < 0.08:")]),
    dict(name='hit_glow', patches=[(R, "        if e[2] == ORG and e[7] in BIG_HITS:", "        if False:")]),
]

ORGAN = dict(name='ORGAN', origin=dict(
    episode='ballfight_organ_vs_harp',
    config=dict(MOOD='big_top',                 # #35's gothic horror is not a kit mood yet: the ORGAN's own buses only
                SHARD_SEED=271, FX_SEED=35),    # render.py's own seeds (default_rng(271), fx.Particles(seed=35))
    frames=24,
    other=HARP_BARE,
    layers=ORGAN_LAYERS,
    harness=[dict(name="#35's camera (SY 280: the ring 8 px higher; the kit's 288)", camera_sy=280),
             dict(name="#35's beat ramp (0.16 s; the kit's 0.12 s)", beat_pulse_pre=0.16)],
    buses=['organ', 'solo'],
))

BRASS_TRAILER = dict(name='brass_trailer', origin=dict(
    episode='ballfight_trombone_vs_horn',
    config='examples/trombone_vs_horn',         # the kit config the mood was ported with (#34's seed 140)
    silence=[                                   # the fighters' placements on the bed's buses (HORN, TROMBONE)
        ('audio.py', "            if e[3]:\n                mix.place('fx', A.thud(0.7, 140, 34), ts, 0.6)\n"
                     "                mix.place('taiko', taiko(1.0, True, 81), ts, 0.55)\n"
                     "                mix.place('braam', braam(26, 0.9, 1.0, 0.005), ts, 0.22)",
         "            if e[3]:\n                mix.place('fx', A.thud(0.7, 140, 34), ts, 0.6)"),           # FINAL FLICK
        ('audio.py', "            mix.place('taiko', taiko(1.0, True, 83), ts, 0.45)\n", ""),          # the flick rope
        ('audio.py', "            mix.place('taiko', taiko(1.0, True, 90), ts, 0.7)\n"
                     "            mix.place('braam', braam(26, 1.2, 1.0, 0.005), ts, 0.30)\n", ""),     # PEDAL TONE
    ],
    buses=['strings', 'taiko', 'braam', 'tremolo'],
))


def mutation_frame(doc):
    """a frame of the passing ORGAN gate where its layers draw (the first with a non-zero contribution)"""
    for row in doc['frames']:
        if row['sha256']['kit_on'] != row['sha256']['kit_off']:
            return row['frame']
    raise SystemExit('origin_selfproof: no frame where the ORGAN draws anything')


def expect_failure(fn, *args, want=()):
    """run a gate that must fail; returns its failure line, which must name every word of `want`"""
    try:
        fn(*args)
    except SystemExit as ex:
        line = str(ex)
        missing = [w for w in want if w not in line]
        if missing:
            raise SystemExit(f'origin_selfproof: the mutated gate failed, but its line does not name {missing}: {line}')
        return line
    raise SystemExit(f'origin_selfproof: the mutated gate PASSED ({args[-1]!r}): Gate O cannot see the change')


def run():
    from .older import origin as O
    import time
    t0 = time.time()
    organ = O.fighter_origin(ORGAN, out=os.path.join(OUT, 'organ.json'))
    trailer = O.mood_origin(BRASS_TRAILER, out=os.path.join(OUT, 'brass_trailer.json'))
    f = mutation_frame(organ)
    x, y = 180, 300
    pix = expect_failure(O.fighter_origin, ORGAN, False, ('pixel', f, x, y), want=(f'frame {f} ', '; 1 pixels differ'))
    import re
    at = re.search(r'pixel \(x=(\d+), y=(\d+)\)', pix)          # (a hit's chromatic fringe after the hook may move
    if not at or abs(int(at[1]) - x) > 3 or abs(int(at[2]) - y) > 3:   # the red channel by a pixel or two)
        raise SystemExit(f'origin_selfproof: the mutated gate names another pixel than ({x}, {y}): {pix}')
    i = int(round(30.0 * O.SR))
    smp = expect_failure(O.fighter_origin, ORGAN, False, ('sample', 'organ', i, 1e-3),
                         want=("bus 'organ'", f'sample {i} '))
    common.write_json(os.path.join(OUT, 'summary.json'), dict(
        about='Gate O on two modules the kit already holds (they must pass) and on the ORGAN module changed at run '
              'time (each change must fail, naming its frame and pixel or its bus and sample): plan m4 Task 2',
        organ=dict(file='older/selfproof/organ.json', frames=len(organ['frames']),
                   buses={b['bus']: b['peak'] for b in organ['buses']}, verdict='PASS'),
        brass_trailer=dict(file='older/selfproof/brass_trailer.json',
                           buses={b['bus']: b['peak'] for b in trailer['buses']}, verdict='PASS'),
        mutations=[dict(change=f'one pixel (x={x}, y={y}, red channel +-40) of frame {f}, after the ORGAN look\'s '
                               f'after_grade', verdict='FAIL', line=pix),
                   dict(change=f"sample {i} of the 'organ' bus + 1e-3, after the ORGAN sound's play()",
                        verdict='FAIL', line=smp)]))
    print(f'origin_selfproof: ORGAN PASS, brass_trailer PASS, both mutations FAIL ({time.time() - t0:.1f} s)')


EXPORTS = {'origin_selfproof': run}
ORDER = {'origin_selfproof': 95}

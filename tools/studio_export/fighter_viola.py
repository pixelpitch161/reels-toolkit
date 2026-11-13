"""fighter_viola: VIOLA's references (plan m4 Template F', Task 11).

Step 0 (Gate O, `python -m tools.studio_export origin fighters/viola`): the reference kit module
(older/fighters/viola.py) against its episode, projects/ballfight_viola_vs_violin (#42, seed 8219: today's roster
replays #42's fight, the VIOLIN winning at 50.31 s). The episode is switched layer by layer with exact text patches of
its scratch copy of render.py (never of projects/):

- `other`: #42's VIOLIN drawn as the kit's bare ball (no bow, no darts, no SCREECH ring, base.Look's sparks, glows,
  shakes, flashbulbs, effects, shards and K.O. spray), and #42's HUD holding its pills lit as the kit's _hold does (the
  kit's _hold knows no 'melody' or 'joke' kind: MELODY AT LAST and VIOLA JOKE light for 0.08 s there, for their length in
  #42; a series difference, reported, not one of the VIOLA's layers);
- `layers`: the VIOLA's layers, all off = the kit's base.Look for it.

Each patch replaces one whole block of render.py (one function, or one statement) by the same block with the listed
edits made: one patch per block keeps origin.json inside its 20 KB budget, and the edits below are the reviewable part.

Ruling M4-R10: the 'cellos' bus is not compared (Gate O's buses are 'viola' and 'laugh'; proofs() lists it as not
compared): the crush's cello note draws from the shared CELLO noise stream, whose position the stand-in bed cannot
reproduce; the exact proof is carried to the orchestral_drill theme gate (Task 30) and the viola_vs_violin pair gate
(Task 41). In Rust the crush's cello draws from the bed's CELLO noise stream, in the kit's order.

The harness is empty: #42's camera (SY 288) and beat ramp (0.12 s) are the kit's. Buses: 'viola' and 'laugh' (the
VIOLA's own); its placements on the bed's buses (fx, cellos, drums, bass808) cannot be compared alone.

    python -m tools.studio_export origin fighters/viola     Gate O: writes studio/reference/fighters/viola/origin.json"""
import os

from . import common

R = 'render.py'


def _config_lines():
    """the VIOLIN's line of the kit config of #42 (older/configs/viola_vs_violin, Task 5): the VIOLA's inner voice sits a
    fifth under it (timeline.py viola_inner reads melody_at; the kit reads the line through T.line_at)"""
    ns = {}
    path = os.path.join(common.REPO, 'tools', 'studio_export', 'older', 'configs', 'viola_vs_violin', 'episode.py')
    with open(path, encoding='utf-8') as fh:
        exec(compile(fh.read(), path, 'exec'), ns)
    return {k: list(v) for k, v in ns['LINES'].items()}


# ---------------------------------------------------------------- the patches: blocks of render.py and their edits
def _block(text, start, end, edits):
    """(patch, text after it): the block from `start` to the end of the first `end` after it, with `edits` made"""
    if text.count(start) != 1:
        raise SystemExit(f'fighter_viola: block start found {text.count(start)} times: {start!r}')
    i = text.index(start)
    j = text.index(end, i + len(start)) + len(end)
    old = text[i:j]
    new = old
    for x, y in edits:
        if new.count(x) != 1:
            raise SystemExit(f'fighter_viola: an edit matches {new.count(x)} times in block {start!r}: {x!r}')
        new = new.replace(x, y)
    return (R, old, new), text.replace(old, new)


def _patches(text, blocks):
    out = []
    for start, end, edits in blocks:
        p, text = _block(text, start, end, edits)
        out.append(p)
    return out, text


# Three blocks of render.py: [fighter() .. _hold()], [the static draw lists .. post_fx()], [render()].
B1 = ('def fighter(k, t):', '    return 0.08\n')
B2 = ('STAND_N = np.array(', '    img = BU.draw(img, cam, t)\n')
B3 = ('def render(f):', 'img = TT.draw(img, t)')

# #42's VIOLIN drawn as the kit's bare ball, and #42's HUD holding as the kit's _hold: (start, end, edits)
VIOLIN_BARE_EDITS = [
    B1 + ([
        ("e[7] in ('blade', 'wave', 'note'):", "e[7] in ('note',):"),                       # base.Look.shake: none
        ("    kinds = ['glass'] * n_glass + ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll'] + \\\n"
         "        (['bowbit'] * 3 if NAMES[LOSE] == 'VIOLIN' else [])",
         "    kinds = ['glass'] * n_glass + ([] if NAMES[LOSE] == 'VIOLIN' else\n"
         "                                  ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll'])"),   # shards
        ("('shield', 'haste', 'melody')", "('shield', 'haste')"),                              # the series HUD: the kit's
        ("    if ab['kind'] == 'joke':", "    if False:")],),                                  # _hold (no melody / joke)
    B2 + ([
        ("(e[1] == 'hit' and e[7] in ('wave', 'note') and e[8] == 2)",
         "(e[1] == 'hit' and ((CFG[e[2]]['weapon']['kind'] == 'blade' and e[7] in ('slam', 'wave', 'shockwave'))\n"
         "                                      or (e[7] == 'note' and e[8] == 2)))"),       # base flash_hits
        ("        if k in BLADE and not st['shattered']:", "        if False:"),               # the bow's shadow
        ("    for row in keys:", "    for row in []:"),                                    # the darts' shadows
        ("((8, 2.4, 0.22) if how == 'blade' else (5, 1.8, 0.18))", "(5, 1.8, 0.18)"),   # base.Look.spark
        ("e[7] in ('blade', 'note') and dt < 0.08", "e[7] in ('note',) and dt < 0.08"),   # base spark_flash: none
        ("            if how == 'blade':", "            if how == 'blade' and False:"),     # its effects: none
        ("            elif how == 'key':", "            elif how == 'key' and False:"),
        ("            elif how == 'wave':", "            elif how == 'wave' and False:"),
        ("        elif kind == 'ability' and e[3] == 'SCREECH':", "        elif kind == 'ability' and False:"),
        ("    PA.emit(T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD_COL), tuple(WOOD_COL * 1.3)])",
         "    if NAMES[LOSE] == 'VIOLIN':\n"                                                     # base.Look.ko_fx
         "        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))\n"
         "    else:\n"
         "        PA.emit(T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD_COL), tuple(WOOD_COL * 1.3)])"),
        ("in RUN['WAVES'][i]:\n            if kind == 'wave':", "in []:\n            if kind == 'wave':")],),   # shimmer
    B3 + ([
        ("    if T_KO <= t < T_KO + 0.9 and t >= T_HS:", "    if False:"),                   # darts dropping at the K.O.
        ("        wav = RUN['WAVES'][i] if in_fight else []", "        wav = []"),           # SCREECH's jagged ring
        ("        if k in BLADE and not st['shattered']:", "        if False:"),               # the bow (painter)
        ("    for row in keys:", "    for row in []:"),                                    # the darts (painter)
        ("        if e[7] in ('wave', 'blade', 'note'):", "        if e[7] in ('note',):")],),  # base hit_glow: none
]

# #42's VIOLA layers (all off = base.Look), named by what they turn off, one group per block (cut from the text `other`
# leaves, in order)
VIOLA_LAYER_EDITS = [
    ('halo, shake, shards, frozen enemy, padded sprite', [B1 + ([
        ("        if k in VIOLA:\n            st['glow']", "        if False:\n            st['glow']"),   # MELODY's halo
        ("e[7] in ('note',):", "e[7] in ():"),                                                # its shakes
        ("([] if NAMES[LOSE] == 'VIOLIN' else\n                                  ['string'] * 4 + ['wood'] * 7 + "
         "['bridge'] * 2 + ['scroll'])", "[]"),                                               # its shards
        ("        if name == 'VIOLA':\n", "        if False:\n"),                             # render_padded
        ("    frz = st.get('stun', 0.0)", "    frz = 0.0")],)]),                              # the frozen enemy
    ('flashbulbs, laugh, sparks, effects, shimmer, fringe', [B2 + ([
        ("\n                                      or (e[7] == 'note' and e[8] == 2)))", "))"),   # its flashbulbs
        ("    lk = laugh_k(t)\n", "    lk = 0.0\n"),                                       # the crowd's laugh
        ("how in ('wave', 'note') else", "how in ('wave',) else"),                            # its sparks
        ("e[7] in ('note',) and dt < 0.08", "e[7] in () and dt < 0.08"),
        ("('hit', 'block', 'wall', 'cleared', 'crushed')", "('hit', 'block', 'wall', 'cleared')"),
        ("            elif how == 'note':", "            elif how == 'note' and False:"),     # its effects
        ("        elif kind == 'note':", "        elif kind == 'note' and False:"),
        ("        elif kind == 'joke':", "        elif kind == 'joke' and False:"),
        ("    else:\n        PA.emit(T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), "
         "col=[tuple(WOOD_COL), tuple(WOOD_COL * 1.3)])",
         "    else:\n        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))"),   # base ko_fx
        ("    for t0, k, v, tl, x, z in JOKES:\n        if 0 <= t - tl",
         "    for t0, k, v, tl, x, z in []:\n        if 0 <= t - tl"),                         # the landing's shimmer
        ("if e[7] == 'wave' or (e[7] == 'note' and e[8] == 2):", "if e[7] == 'wave':")],)]),  # its fringe
    ('notes, pool, target, hit glow, rests', [B3 + ([
        ("    notes = notes_at(t)\n", "    notes = []\n"),                                 # painter, shadows, glows
        ("        for k in VIOLA:\n            st = fs[k]\n", "        for k in []:\n            st = fs[k]\n"),
        ("        for t0, k, v_, tl, x, z in JOKES:", "        for t0, k, v_, tl, x, z in []:"),   # target, ring
        ("        if e[7] in ('note',):", "        if e[7] in ():"),                          # its hit glows
        ("    if not (TT.door_offset(t) == 0.0):\n        draw_rests", "    if False:\n        draw_rests")],)]),
]


def _all_patches():
    path = os.path.join(common.REPO, 'projects', 'ballfight_viola_vs_violin', R)
    with open(path, encoding='utf-8') as fh:
        text = fh.read()
    other, text = _patches(text, VIOLIN_BARE_EDITS)
    layers = []
    for name, blocks in VIOLA_LAYER_EDITS:
        ps, text = _patches(text, blocks)
        layers.append(dict(name=name, patches=ps))
    return other, layers


VIOLIN_BARE, VIOLA_LAYERS = _all_patches()

SPEC = dict(name='VIOLA', opponent='FLUTE', origin=dict(
    episode='ballfight_viola_vs_violin',
    config=dict(MOOD='big_top',                 # #42's orchestral drill is not a kit mood yet: the VIOLA's own buses only
                SHARD_SEED=271, FX_SEED=42,     # render.py's own seeds (default_rng(271), fx.Particles(seed=42))
                LINES=_config_lines()),         # the VIOLIN's line (the VIOLA's inner voice reads it)
    frames=[160, 373, 427, 451, 463, 508, 644, 730, 742, 752, 915, 1077, 1186, 1237, 1251, 1263, 1321, 1402, 1494,
            1512, 1523, 1538, 1565, 1610] +    # origin.choose_frames' 24 (abilities, first hit, HP < 25, K.O., shards)
           [598, 750, 903, 1259],             # + flattened darts' sparks (19.91, 25.00, 30.08 s), VIOLA JOKE's
                                              #   note landing (41.94 s: its flashbulbs, crash, fringe, shimmer)
    other=VIOLIN_BARE,
    layers=VIOLA_LAYERS,
    harness=[],
    buses=['viola', 'laugh'],
))

EXPORTS = {}


# ---------------------------------------------------------------- two more proofs Gate O cannot make yet (controller)
# (a) EVERY bus the VIOLA's part places on, not only its own: on the shared buses (fx, cellos, drums, bass808, solo) the
#     episode's bed, the VIOLIN's part and the series' own sounds are removed. The episode keeps exactly the mix.place()
#     calls of its VIOLA part in audio.py main() (the VIOLA section, every call on 'viola' / 'laugh', and the 'solo'
#     calls that play viola(): VIOLA_PLACES lists them by line); the kit keeps exactly the calls made by the VIOLA's
#     module. Both are taken before the master, float64, against Gate O's -120 dBFS bar.
# (b) a fight the VIOLA wins (#42's seed 8219 is the VIOLIN's): seed 5 (the first seed from 1 where the VIOLA wins with
#     all three abilities and a K.O. in 44..53 s: 52.53 s), so its stinger, solo and the eighths over its solo are
#     proven: Gate O's frames at that seed (#42's sim.py patched to load it) and (a)'s buses.
#     python -c "from tools.studio_export import fighter_viola as F; F.proofs()"
WIN_SEED = 5
NOT_COMPARED = {'cellos': "ruling M4-R10: the crush's cello note draws from the shared CELLO noise stream, whose "
                          "position depends on every cello note the bed played before it (#42's bed plays many, the "
                          "stand-in bed none); proven exactly at the orchestral_drill theme gate (Task 30) and the "
                          "viola_vs_violin pair gate (Task 41)"}
AUDIO = 'audio.py'
VIOLA_SECTION = ('    # ------------------------------------------------ VIOLA: the rests',
                 '    # ------------------------------------------------ the K.O. moment')


def viola_places(text):
    """the line numbers of audio.py's main() whose mix.place() calls are the VIOLA's part"""
    lines = text.split('\n')
    a = next(i for i, ln in enumerate(lines) if ln.startswith(VIOLA_SECTION[0]))
    b = next(i for i, ln in enumerate(lines) if ln.startswith(VIOLA_SECTION[1]))
    m0 = next(i for i, ln in enumerate(lines) if ln.startswith('def main():'))
    m1 = next(i for i, ln in enumerate(lines) if i > m0 and ln.startswith('def '))
    out = []
    for i in range(m0, m1):
        ln = lines[i]
        if 'mix.place(' not in ln:
            continue
        if a < i < b or "mix.place('viola'" in ln or "mix.place('laugh'" in ln or \
                ("mix.place('solo'" in ln and 'viola(' in ln):
            out.append(i + 1)
    return out


def _seed_patch(seed):
    return [('sim.py', "    if seed is None:\n        seed = int(open(os.path.join(HERE, 'seed.txt')).read().split()[0])",
             "    if seed is None:\n        seed = %d" % int(seed))]


def episode_viola_buses_child(folder, out_npz):
    """the episode's audio.py main() with only the VIOLA's placements: every bus as placed (before its master)"""
    import sys
    from .older import origin as O
    O._episode_env(folder)
    import numpy as np
    import audio as AU
    with open(os.path.join(folder, AUDIO), encoding='utf-8') as fh:
        keep = set(viola_places(fh.read()))
    Mix = AU.A.Mix
    real = Mix.place
    seen = set()

    def place(self, *a, **kw):
        fr = sys._getframe(1)
        if os.path.basename(fr.f_code.co_filename) == AUDIO and fr.f_code.co_name == 'main' and fr.f_lineno in keep:
            seen.add(fr.f_lineno)
            return real(self, *a, **kw)
        self.bus(a[0] if a else kw['bus'])                  # (the bus exists, silent: main() tape-stops it by name)
    Mix.place = place
    got = {}
    real_master = AU.master

    def master(mix, *a, **kw):
        got.update({k: np.array(v, np.float64, copy=True) for k, v in mix.buses.items() if np.any(v)})
        return real_master(mix, *a, **kw)
    AU.master = master
    AU.main()
    common.write_npz(out_npz, got, register=False)
    return dict(buses=sorted(got), lines=sorted(seen), keep=sorted(keep))


def kit_viola_buses_child(cfg_folder, out_npz):
    """the kit's score with only the calls the VIOLA's module makes (the VIOLIN base.Sound, the bed's placements gone)"""
    import sys
    from .older import origin as O
    import numpy as np
    O._kit_load(cfg_folder, [])
    O._bare({'VIOLIN'})
    from ballfight.kit import music as M              # noqa: N812
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly  # noqa: F401
    M.SOUNDS = [M.SOUNDS[k] if M.NAMES[k] == 'VIOLA' else M.FIGHTER_MODULES.base.Sound(M, k) for k in (0, 1)]
    mod = type(M.SOUNDS[list(M.NAMES).index('VIOLA')]).__module__
    real = M.A.Mix.place

    def place(self, *a, **kw):
        if sys._getframe(1).f_globals.get('__name__') == mod:
            return real(self, *a, **kw)
        self.bus(a[0] if a else kw['bus'])                  # (the bus exists, silent)
    M.A.Mix.place = place
    mix = M.score()
    got = {k: np.array(v, np.float64, copy=True) for k, v in mix.buses.items() if np.any(v)}
    common.write_npz(out_npz, got, register=False)
    return dict(buses=sorted(got), module=mod)


def bus_proof(seed):
    """(a) at `seed`: every bus the VIOLA places on, the episode's VIOLA placements against the kit module's"""
    from .older import origin as O
    ep = SPEC['origin']['episode']
    folder = O.scratch(ep, _seed_patch(seed), 'viola_buses_%d' % seed)
    src = os.path.join(O.ROOT, 'kit', ep, 'viola', 'episode.py')
    with open(src, encoding='utf-8') as fh:
        text = fh.read()
    if text.count('SEED = 8219\n') != 1:
        raise SystemExit('bus_proof: %s is not the Gate O config of seed 8219 (run origin fighters/viola first)' % src)
    cfg = O.kit_config_folder(ep, 'viola_buses_%d' % seed, text.replace('SEED = 8219\n', 'SEED = %d\n' % seed))
    od = os.path.join(O.ROOT, 'out', ep, 'viola_buses_%d' % seed)
    e_npz, k_npz = os.path.join(od, 'e.npz'), os.path.join(od, 'k.npz')
    er, kr = O._run_parts([(episode_viola_buses_child, (folder, e_npz)), (kit_viola_buses_child, (cfg, k_npz))])
    O.check_kit_clean('projects/' + ep)
    eb, kb = O._load_npz(e_npz), O._load_npz(k_npz)
    facts = O._run_parts([(O.kit_facts_child, (cfg, [], 0))])[0]
    t0, t1 = facts['t_fight'], facts['t_reassemble']
    i0, i1 = int(round(t0 * O.SR)), int(round(t1 * O.SR))
    rows, failures = [], []
    if er['buses'] != kr['buses']:
        failures.append('the episode places the VIOLA on %s, the kit on %s' % (er['buses'], kr['buses']))
    for bus in sorted(set(er['buses']) & set(kr['buses'])):
        if bus in NOT_COMPARED:
            rows.append(dict(bus=bus, not_compared=NOT_COMPARED[bus]))
            continue
        summary, bad = O.compare_bus('VIOLA', bus, eb[bus], kb[bus], i0, i1, O.BAR_DB)
        rows.append(dict(bus=bus, peak=summary['peak'], whole=summary['whole']))
        if bad:
            failures.append(bad)
    return dict(seed=seed, winner=facts['names'][facts['winner']], t_ko=facts['t_ko'], window=[t0, t1],
                buses=rows, lines_kept=er['keep'], lines_unused=sorted(set(er['keep']) - set(er['lines'])),
                failures=failures)


def win_frames():
    """(b) Gate O's frames at WIN_SEED (the VIOLA wins: its solo layers); nothing written (origin.json is seed 8219's)"""
    from .older import origin as O
    o = dict(SPEC['origin'], seed=WIN_SEED, frames=24, buses=[],
             other=list(SPEC['origin']['other']) + _seed_patch(WIN_SEED))
    return O.fighter_origin(dict(SPEC, origin=o), out=False)


def proofs():
    import json
    out = dict(buses=[bus_proof(seed) for seed in (8219, WIN_SEED)])
    doc = win_frames()
    out['win_frames'] = dict(seed=doc['seed'], fight=doc['fight'], frames=[r['frame'] for r in doc['frames']],
                             whole_frames_equal=all(r['whole_frame_equal'] for r in doc['frames']),
                             state_keys=doc['state_keys'])
    print(json.dumps(out, indent=1))
    return out

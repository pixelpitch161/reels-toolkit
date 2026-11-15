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

Ruling M4-R10: the 'cellos' bus is not compared: the crush's cello note draws from the shared CELLO noise stream,
whose position the stand-in bed cannot reproduce; the exact proof is carried to the orchestral_drill theme gate (Task 30)
and the viola_vs_violin pair gate (Task 41). In Rust the crush's cello draws from the bed's CELLO noise stream, in the
kit's order. The shared buses fx, drums, bass808 and solo are compared through `silence` (the VIOLA's own placements
only, on both sides), and a second fight (seed 5, the VIOLA winning) proves its stinger, solo and solo eighths.

The harness is empty: #42's camera (SY 288) and beat ramp (0.12 s) are the kit's. Buses: 'viola' and 'laugh' (the
VIOLA's own); its placements on the bed's buses (fx, cellos, drums, bass808) cannot be compared alone.

    python -m tools.studio_export origin fighters/viola     Gate O: writes studio/reference/fighters/viola/origin.json"""
import os

from . import common
from . import m3refs

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


# ---------------------------------------------------------------- the shared buses (silence) and the second fight
# On fx, drums, bass808 and solo the VIOLA's part shares buses with #42's bed, the VIOLIN and the series' own sounds.
# `silence` makes the stems' scratch copy keep, on those buses, exactly the mix.place() calls of the VIOLA's part in
# audio.py main() (VIOLA_PLACES: the VIOLA section, and the 'solo' calls that play viola()); the kit side keeps the
# module Sound's own placements (silence_kit). 'cellos' is not compared (ruling M4-R10: the crush's cello note draws
# from the shared CELLO noise stream, whose position the stand-in bed cannot reproduce; the exact proof is carried to
# the orchestral_drill theme gate, Task 30, and the viola_vs_violin pair gate, Task 41).
AUDIO = 'audio.py'
SHARED = ('fx', 'drums', 'bass808', 'solo')
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
        if a < i < b or "mix.place('viola'" in ln or "mix.place('laugh'" in ln or                 ("mix.place('solo'" in ln and 'viola(' in ln):
            out.append(i + 1)
    return out


def _silence():
    path = os.path.join(common.REPO, 'projects', 'ballfight_viola_vs_violin', AUDIO)
    with open(path, encoding='utf-8') as fh:
        keep = tuple(viola_places(fh.read()))
    return [(AUDIO, "    mix = A.Mix(T.DUR + 4.0)\n",
             "    mix = A.Mix(T.DUR + 4.0); _place = mix.place; mix.place = lambda bus, *a, **kw: mix.bus(bus) if bus "
             f"in {SHARED!r} and sys._getframe(1).f_lineno not in {keep!r} else _place(bus, *a, **kw)\n")]


WIN_SEED = 5          # the first seed from 1 where the VIOLA wins with all three abilities and a K.O. in 44..53 s

SPEC = dict(name='VIOLA', opponent='FLUTE',
            fights=dict(handicap=(1.0, 2.0)),       # ruling M3-R34: at (1.0, 1.0) VIOLA won 197 of 200 (3 no K.O.);
                                                    #   at FLUTE's kit handicap (1.0, 2.0) 154 of 200 (77 %)
            scenarios=None,                         # (filled below: the template's + the Table O4 links)
            sprite_size=dict(add=7, pad=1.2, ceil=True),   # #42's render_padded: 2 * ceil(1.2 r) + 7
            origin=dict(
    episode='ballfight_viola_vs_violin',
    config=dict(MOOD='big_top',                 # #42's orchestral drill is not a kit mood yet: the VIOLA's own buses only
                SHARD_SEED=271, FX_SEED=42,     # render.py's own seeds (default_rng(271), fx.Particles(seed=42))
                LINES=_config_lines()),         # the VIOLIN's line (the VIOLA's inner voice reads it)
    other=VIOLIN_BARE,
    layers=VIOLA_LAYERS,
    harness=[],
    silence=_silence(),
    fights=[
        # #42's own fight (8219): the VIOLIN wins; the VIOLA's shards, K.O. spray, fall-apart
        dict(seed=8219, buses=['viola', 'laugh', 'fx', 'drums', 'bass808'],
             # origin.choose_frames' 24 (abilities, first hit, HP < 25, K.O., shards), then the flattened darts' sparks
             # (19.91, 25.00, 30.08 s) and VIOLA JOKE's note landing (41.94 s: its flashbulbs, crash, fringe, shimmer)
             frames=[160, 373, 427, 451, 463, 508, 644, 730, 742, 752, 915, 1077, 1186, 1237, 1251, 1263, 1321, 1402,
                     1494, 1512, 1523, 1538, 1565, 1610, 598, 750, 903, 1259]),
        # the VIOLA wins (seed 5): its stinger, its solo and the eighths over it
        dict(seed=WIN_SEED, frames=16, buses=['viola', 'laugh', 'fx', 'drums', 'bass808', 'solo']),
    ],
))

# Template F's scenarios plus Table O4's links: a rolling note flattens PIANO's keys (its projectiles), and the copy
# sweep, EUPHONIUM against VIOLA from both corners (IDENTITY CRISIS copies a generic ability and draws it through the
# viola's look: setup_as, ruling M3-R3)
SPEC['scenarios'] = m3refs.default_scenarios('VIOLA', 'FLUTE') + [
    dict(name='vs_piano', seeds=(1, 2, 3), opponent='PIANO'),
    dict(name='vs_euphonium', seeds=(1, 2, 3), opponent='EUPHONIUM'),
    dict(name='vs_euphonium_swapped', seeds=(1, 2, 3), opponent='EUPHONIUM', swapped=True),
]

EXPORTS = {'fighter_viola': lambda: m3refs.fighter_refs(SPEC)}


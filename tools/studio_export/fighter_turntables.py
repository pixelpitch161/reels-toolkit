"""fighter_turntables: TURNTABLES's milestone 4 references (Template F' of plan m4, Task 8).

Step 0 (Gate O): the reference kit module tools/studio_export/older/fighters/turntables.py against #31 v3
(projects/ballfight_turntables_vs_synth, seed 1339), run as

    python -m tools.studio_export origin fighters/turntables     writes studio/reference/fighters/turntables/origin.json

The episode is switched with exact text patches of its scratch copy (never projects/). Each patch is one region of
render.py with the edits listed below made inside it (OTHER_EDITS, LAYER_EDITS), so origin.json lists one patch per
region:

- `other` (always applied): the SYNTH drawn as the kit's bare ball (no cables, plugs, pulses, lashes, WHITE NOISE
  static ring or shimmer, no effects or shakes of its own; base.Look's sparks, flashbulbs, glows, fringes, shards and
  K.O. spray; its sprite without #31's pitch), and #31's own base details switched to the kit's series renderer (plan
  m4 Task 4's note): the sprites lit 1.06 + 0.3 glow (#31: 1.12 + 0.3 glow + LIFT), the ropes' ability / slam wobble
  0.15 s late and in the kit's event order, the shimmers before the glows, the K.O. glow 46 px at 0.9, FIGHT!'s punch
  0.25, every hit but a key flashing the HUD;
- `layers` (TURNTABLES's own; all off = the kit's base.Look): the jerk thump, the jerk ropes, the tipped and wiggling
  instrument, its effects, the tonearm and its shadow, the record floor and the solo rings, BASS DROP (canvas, falling,
  lying, shimmer), REWIND (ghosts, tape look), its hits (flashbulbs, sparks, glows, fringes).

The harness matches the kit to #31's series rules inside the window: its beat ramp (0.07 s), and (by
older/fighters/turntables.py:origin_harness) no ability list in the HUD (#31 predates it) and the decks' bus under #31's
name `decks` (the kit's is `turntables`). The config keeps only the champion's solo (the SYNTH loses: its solo is never
played; origin.json stays within 20 KB)."""
import os

R ='render.py'
EPISODE = 'ballfight_turntables_vs_synth'


def _render():
    """#31's render.py as delivered (read only; from this clone, wherever a check points common.REPO)"""
    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    with open(os.path.join(repo, 'projects', EPISODE, R), encoding='utf-8', newline='') as fh:
        return fh.read()


def _region(text, first, last, edits):
    """one patch of render.py: from the line holding `first` (once in the text) to the end of the first line holding
    `last` at or after it, with `edits` [(old, new)] made inside it in order, each matching exactly once there"""
    if text.count(first) != 1:
        raise SystemExit(f'fighter_turntables: {first[:60]!r} is not in render.py exactly once')
    i = text.rfind('\n', 0, text.index(first)) + 1
    j = text.index(last, i) + len(last)
    j = text.index('\n', j) if '\n' in text[j:] else len(text)
    old = new = text[i:j]
    for a, b in edits:
        if new.count(a) != 1:
            raise SystemExit(f'fighter_turntables: the edit {a[:60]!r} matches {new.count(a)} times in its region')
        new = new.replace(a, b)
    return (R, old, new)


# ---------------------------------------------------------------- #31: the SYNTH as the kit's bare ball, #31's base as the kit's
OTHER_EDITS = [
    # the ropes: the abilities' and slams' wobble 0.15 s late and in the kit's event order (kit picture.rope_points)
    ("WAVE_EV += [(e[0], 0.9) for e in EV if e[1] == 'slam']", "disp += 0.07 * k * math.exp(-dt / 0.3)", [
        ("WAVE_EV += [(e[0], 0.9) for e in EV if e[1] == 'slam']\n"
         "WAVE_EV += [(e[0], 0.35) for e in EV if e[1] == 'jerk']",
         "WAVE_EV += [(e[0], 0.9 if e[1] == 'slam' else 0.35) for e in EV if e[1] in ('slam', 'jerk')]"),
        ("        if 0.0 < dt < 1.2:\n"
         "            disp += 0.07 * k * math.exp(-dt / 0.3) * math.sin(2 * math.pi * 4.0 * dt) * np.sin(np.pi * u)",
         "        if 0.15 < dt < 1.2:\n"
         "            disp += 0.07 * k * math.exp(-(dt - 0.15) / 0.3) * math.sin(2 * math.pi * 4.0 * (dt - 0.15)) * "
         "np.sin(np.pi * u)")]),
    # the SYNTH's shatter: the 24 glass shards only (base.Look.shards)
    ("['key'] * 7 + ['bkey'] * 4", "['key'] * 7 + ['bkey'] * 4", [
        ("['key'] * 7 + ['bkey'] * 4 + ['knob'] * 4 + ['jack'] * 4)", "[])")]),
    # sprites: the SYNTH's without #31's pitch (the kit's sprites.render_sprite); both lit as the kit lights them
    ("        cache[key] = ST.render_sprite(nm, r", "lit = sub[..., :3].astype(np.float32) * (1.12", [
        ("        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))",
         "        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch)) if NAMES[k] == 'TURNTABLES' "
         "else __import__('sprites').render_sprite(nm, r, float(yk), float(roll))"),
        ("* (1.12 + 0.3 * glow) + LIFT.get(NAMES[k], 10)", "* (1.06 + 0.3 * glow)")]),
    # the kit's count-in punch; the SYNTH's flashbulbs (base: slam, wave, shockwave), effects (none) and K.O. spray
    ("punch = 1 + (0.25 if txt", "        PA.emit(T_KO, pos, 'sparks', n=20, col=CYAN_SPARK)", [
        ("punch = 1 + (0.25 if txt != 'FIGHT!' else 0.12) * max", "punch = 1 + 0.25 * max"),
        ("e[7] in ('slam', 'wave', 'scratch', 'lash')) or", "e[7] in ('slam', 'wave', 'shockwave', 'scratch')) or"),
        ("            elif how == 'plug':", "            elif how == 'plug' and False:"),
        ("            elif how == 'volt':", "            elif how == 'volt' and False:"),
        ("            elif how == 'lash':", "            elif how == 'lash' and False:"),
        ("            elif how == 'wave':", "            elif how == 'wave' and False:"),
        ("        elif kind == 'snap':", "        elif kind == 'snap' and False:"),
        ("        elif kind == 'leash':", "        elif kind == 'leash' and False:"),
        ("        elif kind == 'ability' and e[4] == 'wave':", "        elif kind == 'ability' and e[4] == 'wave' and False:"),
        ("        PA.emit(T_KO, pos, 'glitch', n=24, spread=180, speed=(0.8, 2.2))\n"
         "        PA.emit(T_KO, pos, 'sparks', n=20, col=CYAN_SPARK)",
         "        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))")]),
    # the SYNTH's sparks as base.Look.spark (no misses, no skipped pulses, no mid spray)
    ("events_between(t - 0.32, t, ('hit', 'plug_miss'))", "((9, 2.6, 0.24) if how in ('lash', 'plug')", [
        ("('hit', 'plug_miss')", "('hit',)"),
        ("            if how == 'volt':\n                continue\n", ""),
        ("((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))", "(5, 1.8, 0.18)")]),
    # the canvas, the frame and the post: the SYNTH's waves, cables, glows and fringes gone; the kit's shimmer order
    # (before the glows), K.O. glow (46 px, 0.9) and HUD flash (every hit but a key)
    ("        wav = RUN['WAVES'][i] if in_fight else []", "        if e[1] == 'snap' or e[7] in", [
        ("        wav = RUN['WAVES'][i] if in_fight else []", "        wav = []"),
        ("    jacks = draw_cables(arr, cam, t, fs, glows)\n", "    jacks = []\n"),
        ("    img = shimmers(img, cam, t)\n    img = grade(", "    img = grade("),
        ("    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n",
         "    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n    img = shimmers(img, cam, t)\n"),
        ("        if e[7] in ('slam', 'wave', 'scratch', 'lash'):", "        if e[7] in ('slam', 'scratch'):"),
        ("fr.glow(s_[0], s_[1], 40, (255, 240, 220), 0.7 *", "fr.glow(s_[0], s_[1], 46, (255, 240, 220), 0.9 *"),
        ("if e[3] == k and e[7] not in ('volt',):", "if e[3] == k and e[7] != 'key':"),
        ("in RUN['WAVES'][sim_i(t)]:\n            img = fx.shimmer_world", "in []:\n            img = fx.shimmer_world"),
        ("('hit', 'snap')", "('hit',)"),
        ("        if e[1] == 'snap' or e[7] in", "        if e[7] in")]),
]

# ---------------------------------------------------------------- #31: TURNTABLES's layers (all off = base.Look)
LAYER_EDITS = [
    ('thump', "        if k == DJ and len(JERK_T):", "        if k == DJ and len(JERK_T):", [
        ("if k == DJ and len(JERK_T):", "if False:")]),
    ('ropes', "for e in EV if e[1] in ('slam', 'jerk')]", "for e in EV if e[1] in ('slam', 'jerk')]", [
        ("('slam', 'jerk')", "('slam',)")]),
    ('instrument', "if NAMES[k] == 'TURNTABLES' else __import__", "    if k in REC and (st['haste']", [
        ("if NAMES[k] == 'TURNTABLES' else __import__", "if False else __import__"),
        ("    if k in REC and (st['haste']", "    if False and (st['haste']")]),
    ('effects', "            if how == 'scratch':", "            SHK.add(ts, 1.6)", [
        ("            if how == 'scratch':", "            if how == 'scratch' and False:"),
        ("            elif how == 'slam':", "            elif how == 'slam' and False:"),
        ("        elif kind == 'jerk':", "        elif kind == 'jerk' and False:"),
        ("        elif kind == 'slam':\n", "        elif kind == 'slam':\n            SHK.add(e[0], 1.6)\n"
                                         "        elif kind == 'slam':\n")]),            # (base.Look.shake: 1.6)
    ('tonearm', "    for kind, Q in ST.tonearm(PIVOT, tip, h):", "    draw_tonearm(fr, cam, L, arm, t)", [
        ("    for kind, Q in ST.tonearm(PIVOT, tip, h):", "    for kind, Q in []:"),
        ("    draw_tonearm(fr, cam, L, arm, t)", "    None")]),
    ('record', "    rings = solo_rings(t) if t >= T_SOLO - 0.01 else []", "alb, on_rec = ST.record_albedo(", [
        ("    rings = solo_rings(t) if t >= T_SOLO - 0.01 else []", "    rings = []"),
        ("        alb, on_rec = ST.record_albedo(alb, sxp, szp, phi, COL[DJ])", "        on_rec = np.zeros(len(sxp), bool)")]),
    ('bass_drop', "        sl = RUN['SLAMS'][i] if in_fight else []", "            u = (tc - e[0]) / 0.6", [
        ("        sl = RUN['SLAMS'][i] if in_fight else []", "        sl = []"),
        ("draw_drop(arr, cam, t, i, in_fight, falling=False)", "None"),
        ("draw_drop(arr, cam, t, i, in_fight, falling=True)", "None"),
        ("        if e[1] == 'slam':\n            u = (tc - e[0]) / 0.6", "        if False:\n            u = (tc - e[0]) / 0.6")]),
    ('rewind', "        if rk > 0 and win_ is not None and fs[k]['vis']:", "            img = ST.vhs(", [
        ("        if rk > 0 and win_ is not None and fs[k]['vis']:", "        if False:"),
        ("        if rk > 0:\n            img = ST.vhs(", "        if False:\n            img = ST.vhs(")]),
    ('hits', "e[7] in ('slam', 'wave', 'shockwave', 'scratch')) or", "        if e[7] in ('slam', 'wave', 'scratch'):", [
        ("e[7] in ('slam', 'wave', 'shockwave', 'scratch')) or", "e[7] in ('slam', 'wave', 'shockwave')) or"),   # flash
        ("(16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else", "(16, 3.2, 0.3) if how in ('slam', 'wave') else"),
        ("        if e[7] in ('slam', 'scratch'):", "        if e[7] in ():"),                                    # glow
        ("        if e[7] in ('slam', 'wave', 'scratch'):", "        if e[7] in ('slam', 'wave'):")]),             # fringe
]


def _patches():
    """(other, layers): each a region patch of render.py made of the edits listed (the layers on top of `other`, in
    order, as Gate O applies them)"""
    text = _render()
    other = []
    for first, last, edits in OTHER_EDITS:
        p = _region(text, first, last, edits)
        text = text.replace(p[1], p[2])
        other.append(p)
    layers = []
    for name, first, last, edits in LAYER_EDITS:
        p = _region(text, first, last, edits)
        text = text.replace(p[1], p[2])
        layers.append(dict(name=name, patches=[p]))
    return other, layers


SYNTH_BARE, TURNTABLES_LAYERS = _patches()
SOLO_TURNTABLES = [(0, 'stab', 0.5), (0, 'back', 1.0), (1, 'baby', 0.5), (1.5, 'baby', 0.5), (2, 'chirp', 0.25),
                   (2.25, 'chirp', 0.25), (2.5, 'trans', 0.5), (3, 'stab', 0.25), (3.25, 'stab', 0.25),
                   (3.5, 'flare', 0.5), (4, 'stab', 0.5), (4.5, 'tear', 0.5), (5, 'scrib', 0.5), (5.5, 'chirp', 0.25),
                   (5.75, 'chirp', 0.25), (6, 'trans', 1.0), (7, 'stab', 0.25), (7.5, 'stab', 0.25), (8, 'stab', 0.5),
                   (8, 'flare', 0.5), (8.5, 'baby', 0.25), (8.75, 'baby', 0.25), (9, 'tear', 0.5), (9.5, 'chirp', 0.25),
                   (9.75, 'chirp', 0.25), (10, 'scrib', 0.5), (10.5, 'trans', 0.5), (11, 'stab', 1.0),
                   (11, 'back', 1.0)]                    # timeline.py 116-123, as it is

# origin.choose_frames' 24 for this run, with two of its even spread (35.4 s, 35.9 s) traded for BASS DROP's record
# lying on the canvas under its stomp ring (42.0 s) and fading as the third REWIND starts (42.37 s: ghosts, tape look)
FRAMES = [110, 375, 385, 397, 429, 510, 645, 647, 659, 753, 915, 1185, 1235, 1247, 1260, 1271, 1320, 1401, 1452, 1455,
          1470, 1492, 1537, 1582]

SPEC = dict(
    name='TURNTABLES',
    opponent='FLUTE',
    left_in_cache=True,
    origin=dict(
        episode=EPISODE,
        config=dict(MOOD='big_top',               # #31's dark breakbeat is not a kit mood yet: the decks' own buses only
                    SHARD_SEED=271, FX_SEED=31,   # render.py's own seeds (default_rng(271), fx.Particles(seed=31))
                    SOLO={'TURNTABLES': SOLO_TURNTABLES}),   # the champion's (the SYNTH loses: its solo never plays)
        frames=FRAMES,
        other=SYNTH_BARE,
        layers=TURNTABLES_LAYERS,
        harness=[dict(name="#31's beat ramp (0.07 s)", beat_pulse_pre=0.07),
                 dict(name="#31's HUD (no ability list), its bus name decks",
                      call='ballfight.kit.fighters.turntables:origin_harness')],
        buses=['decks', 'solo'],
    ),
)

EXPORTS = {}

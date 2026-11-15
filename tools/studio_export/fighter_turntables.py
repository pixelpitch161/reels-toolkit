"""fighter_turntables: TURNTABLES's milestone 4 references (Template F' of plan m4, Task 8).

Step 0 (Gate O): the reference kit module tools/studio_export/older/fighters/turntables.py against #31 v3
(projects/ballfight_turntables_vs_synth, seed 1339; TURNTABLES wins), run as

    python -m tools.studio_export origin fighters/turntables     writes studio/reference/fighters/turntables/origin.json

The episode is switched with exact text patches of its scratch copy (never projects/):

- `other` (always applied): #31's renderer matched to the kit's where a bare ball shows it (the same texts as the SYNTH
  port's, tools/studio_export/fighter_synth.py: the sprite light 1.06 + 0.3 glow without LIFT, the rope wave's 0.15 s
  delay, the K.O. glow 46 px / 0.9, the heat shimmer right after the lens, FIGHT!'s punch 0.25; and here also the rope
  wave events in the kit's event order, because TURNTABLES's jerks shake the ropes), and the SYNTH drawn as the kit's
  bare ball (no cables, plugs, pulses, lashes, WHITE NOISE static ring or shimmer, no effects or shakes of its own;
  base.Look's sparks, flashbulbs, glows, fringes, HUD flash, shards and K.O. spray; its sprite without #31's pitch);
- `layers` (TURNTABLES's own; all off = the kit's base.Look): the record floor, the solo rings, BASS DROP (canvas,
  falling, lying, shimmer), the tonearm and its shadow, REWIND (ghosts, tape look), the jerk thump, the tipped and
  wiggling instrument, its effects, sparks, flashbulbs, glows, fringes and jerk ropes;
- `harness`: its beat ramp (0.07 s); by older/fighters/turntables.py:origin_harness the kit HUD without the ability
  list (#31 predates it), the decks' bus under #31's name `decks` (the kit's is `turntables`), and on the shared buses
  fx and bass (the framework's silence_kit) only the module's own placements.

`silence`: the stems' scratch copy of audio.py with every fx / bass placement removed but the decks' part's own (by
line), so the shared buses are compared too (the kit side keeps the module's own placements, `silence_kit`).
Two fights (`fights`): #31's own seed 1339 (TURNTABLES wins: its stinger, solo and solo rings) and seed 6 (it loses
at 52.7 s: its shards, its K.O. spray, its fall-apart and its reassembly)."""
from . import m3refs

R = 'render.py'
EPISODE = 'ballfight_turntables_vs_synth'

# ---------------------------------------------------------------- #31: the renderer the kit's, the SYNTH as the bare ball
OTHER = [
    # (base, as fighter_synth.py) the ball's sprite light: the kit's 1.06 + 0.3 glow, no lift
    (R, "        lit = sub[..., :3].astype(np.float32) * (1.12 + 0.3 * glow) + LIFT.get(NAMES[k], 10)",
     "        lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow)"),
    # (base, as fighter_synth.py) the ropes: the wave's shake starts 0.15 s after the ability, as the kit's
    (R, "        if 0.0 < dt < 1.2:\n"
        "            disp += 0.07 * k * math.exp(-dt / 0.3) * math.sin(2 * math.pi * 4.0 * dt) * np.sin(np.pi * u)",
     "        if 0.15 < dt < 1.2:\n"
     "            disp += 0.07 * k * math.exp(-(dt - 0.15) / 0.3) * math.sin(2 * math.pi * 4.0 * (dt - 0.15)) * "
     "np.sin(np.pi * u)"),
    # (base) the rope waves in the kit's order (abilities, then every other event in event order: slams and jerks)
    (R, "WAVE_EV += [(e[0], 0.9) for e in EV if e[1] == 'slam']\n"
        "WAVE_EV += [(e[0], 0.35) for e in EV if e[1] == 'jerk']",
     "WAVE_EV += [(e[0], 0.9 if e[1] == 'slam' else 0.35) for e in EV if e[1] in ('slam', 'jerk')]"),
    # (base, as fighter_synth.py) the heat shimmer right after the lens, as the kit's
    (R, "    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n",
     "    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n    img = shimmers(img, cam, t)\n"),
    (R, "    img = shimmers(img, cam, t)\n    img = grade(", "    img = grade("),
    # (base, as fighter_synth.py) the K.O. glow
    (R, "        fr.glow(s_[0], s_[1], 40, (255, 240, 220), 0.7 * (1 - ss(T_KO, T_HS + 0.25, t)))",
     "        fr.glow(s_[0], s_[1], 46, (255, 240, 220), 0.9 * (1 - ss(T_KO, T_HS + 0.25, t)))"),
    # (base, as fighter_synth.py) FIGHT!'s punch
    (R, "            punch = 1 + (0.25 if txt != 'FIGHT!' else 0.12) * max(0.0, 1 - u / 0.12) ** 2",
     "            punch = 1 + 0.25 * max(0.0, 1 - u / 0.12) ** 2"),
    # the SYNTH: its sprite as the kit's (no pitch); its HUD flash on every hit (base: all but a key)
    (R, "        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))",
     "        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch)) if NAMES[k] == 'TURNTABLES' "
     "else __import__('sprites').render_sprite(nm, r, float(yk), float(roll))"),
    (R, "            if e[3] == k and e[7] not in ('volt',):", "            if e[3] == k and e[7] != 'key':"),
    # its solo's groove rings (the SYNTH's when it wins: none of base.Look's)
    (R, "    rings = solo_rings(t) if t >= T_SOLO - 0.01 else []",
     "    rings = solo_rings(t) if t >= T_SOLO - 0.01 and NAMES[WIN] == 'TURNTABLES' else []"),
    # its shatter (glass only), cables, WHITE NOISE's static ring and shimmer
    (R, "['key'] * 7 + ['bkey'] * 4 + ['knob'] * 4 + ['jack'] * 4)", "[])"),
    (R, "    jacks = draw_cables(arr, cam, t, fs, glows)\n", "    jacks = []\n"),
    (R, "        wav = RUN['WAVES'][i] if in_fight else []", "        wav = []"),
    (R, "        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:\n            img = fx.shimmer_world",
     "        for owner, cx, cz, r, rmax, kind, t0 in []:\n            img = fx.shimmer_world"),
    # its effects (base.Look: none) and its K.O. spray (base.Look.ko_fx)
    (R, "            elif how == 'plug':", "            elif how == 'plug' and False:"),
    (R, "            elif how == 'volt':\n                PA.emit", "            elif how == 'volt' and False:\n                PA.emit"),
    (R, "            elif how == 'lash':", "            elif how == 'lash' and False:"),
    (R, "            elif how == 'wave':", "            elif how == 'wave' and False:"),
    (R, "        elif kind == 'snap':", "        elif kind == 'snap' and False:"),
    (R, "        elif kind == 'leash':", "        elif kind == 'leash' and False:"),
    (R, "        elif kind == 'ability' and e[4] == 'wave':", "        elif kind == 'ability' and e[4] == 'wave' and False:"),
    (R, "    if NAMES[LOSE] == 'SYNTH':\n"
        "        PA.emit(T_KO, pos, 'glitch', n=24, spread=180, speed=(0.8, 2.2))\n"
        "        PA.emit(T_KO, pos, 'sparks', n=20, col=CYAN_SPARK)\n",
     "    if NAMES[LOSE] == 'SYNTH':\n"
     "        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))\n"),
    # its sparks (base.Look.spark), flashbulbs, glows, fringes
    (R, "events_between(t - 0.32, t, ('hit', 'plug_miss'))", "events_between(t - 0.32, t, ('hit',))"),
    (R, "            if how == 'volt':\n                continue\n", ""),
    (R, "(16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else \\\n"
        "                ((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))",
     "(16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else \\\n                (5, 1.8, 0.18)"),
    (R, "e[7] in ('slam', 'wave', 'scratch', 'lash')) or e[1] == 'ability'",
     "e[7] in ('slam', 'wave', 'shockwave', 'scratch')) or e[1] == 'ability'"),
    (R, "        if e[7] in ('slam', 'wave', 'scratch', 'lash'):\n            s_ = cam.wp",
     "        if e[7] in ('slam', 'scratch'):\n            s_ = cam.wp"),
    (R, "events_between(t - 0.12, t, ('hit', 'snap'))", "events_between(t - 0.12, t, ('hit',))"),
    (R, "        if e[1] == 'snap' or e[7] in ('slam', 'wave', 'scratch'):",
     "        if e[7] in ('slam', 'wave', 'scratch'):"),
]

# ---------------------------------------------------------------- #31: TURNTABLES's layers (all off = base.Look)
LAYERS = [
    dict(name='record', patches=[(R, "        alb, on_rec = ST.record_albedo(alb, sxp, szp, phi, COL[DJ])",
                                  "        on_rec = np.zeros(len(sxp), bool)")]),
    dict(name='solo_rings', patches=[(R, "and NAMES[WIN] == 'TURNTABLES' else []", "and False else []")]),
    dict(name='bass_drop', patches=[
        (R, "        sl = RUN['SLAMS'][i] if in_fight else []", "        sl = []"),
        (R, "draw_drop(arr, cam, t, i, in_fight, falling=False)", "None"),
        (R, "draw_drop(arr, cam, t, i, in_fight, falling=True)", "None"),
        (R, "        if e[1] == 'slam':\n            u = (tc - e[0]) / 0.6", "        if False:\n            u = (tc - e[0]) / 0.6")]),
    dict(name='shatter', patches=[                                    # when it loses: base.Look's glass and spray
        (R, "['vinyl'] * 8 + ['label'] * 3 + ['arm'] * 2 + ['plinth'] * 3 if loser == 'TURNTABLES'",
         "[] if loser == 'TURNTABLES'"),
        (R, "    else:\n        PA.emit(T_KO, pos, 'shards', n=24, spread=180, speed=(0.8, 2.6))",
         "    else:\n        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))")]),
    dict(name='tonearm', patches=[(R, "    draw_tonearm(fr, cam, L, arm, t)\n", ""),
                                  (R, "    for kind, Q in ST.tonearm(PIVOT, tip, h):", "    for kind, Q in []:")]),
    dict(name='rewind', patches=[
        (R, "        if rk > 0 and win_ is not None and fs[k]['vis']:", "        if False:"),
        (R, "        if rk > 0:\n            img = ST.vhs", "        if False:\n            img = ST.vhs")]),
    dict(name='thump', patches=[(R, "        if k == DJ and len(JERK_T):", "        if False:")]),
    dict(name='instrument', patches=[
        (R, "    if k in REC and (st['haste']", "    if False and (st['haste']"),
        (R, "if NAMES[k] == 'TURNTABLES' else __import__", "if False else __import__")]),
    dict(name='effects', patches=[
        (R, "            if how == 'scratch':", "            if how == 'scratch' and False:"),
        (R, "            elif how == 'slam':", "            elif how == 'slam' and False:"),
        (R, "        elif kind == 'jerk':", "        elif kind == 'jerk' and False:"),
        (R, "        elif kind == 'slam':\n            ts, owner, cx, cz = e[0], e[2], e[3], e[4]\n",
         "        elif kind == 'slam':\n            SHK.add(e[0], 1.6)\n"
         "        elif kind == 'slam':\n            ts, owner, cx, cz = e[0], e[2], e[3], e[4]\n")]),   # base.Look.shake
    dict(name='sparks', patches=[(R, "(16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else",
                                  "(16, 3.2, 0.3) if how in ('slam', 'wave') else")]),
    dict(name='flashbulbs', patches=[(R, "e[7] in ('slam', 'wave', 'shockwave', 'scratch')) or",
                                      "e[7] in ('slam', 'wave', 'shockwave')) or")]),
    dict(name='hit_glow', patches=[(R, "        if e[7] in ('slam', 'scratch'):\n            s_ = cam.wp",
                                    "        if e[7] in ():\n            s_ = cam.wp")]),
    dict(name='fringe', patches=[(R, "        if e[7] in ('slam', 'wave', 'scratch'):\n            x, z",
                                  "        if e[7] in ('slam', 'wave'):\n            x, z")]),
    dict(name='ropes', patches=[(R, "for e in EV if e[1] in ('slam', 'jerk')]", "for e in EV if e[1] in ('slam',)]")]),
]

# ---------------------------------------------------------------- #31 audio.py: the shared buses with only the decks' part on
# them: every fx / bass placement dropped but those of the decks' own lines (455 the jerk thump; 474-475 a rope slam's
# thud and tick; 531 REWIND's whirr; 533 BASS DROP's whoosh; 544, 545, 547 its sub drop, thud and crash). One line, so
# audio.py's line numbers stay as delivered; a dropped placement still makes its bus (main() tape-stops 'bass').
KEEP_LINES = (455, 474, 475, 531, 533, 544, 545, 547)
SILENCE = [
    ('audio.py', "    mix = A.Mix(T.DUR + 4.0)\n",
     "    mix = A.Mix(T.DUR + 4.0); _place = mix.place; mix.place = lambda bus, *a, **kw: mix.bus(bus) if bus in "
     f"('fx', 'bass') and sys._getframe(1).f_lineno not in {KEEP_LINES!r} else _place(bus, *a, **kw)\n"),
]
SHARED_BUSES = ['fx', 'bass']


# origin.choose_frames' 24 for this run, with two of its even spread (35.4 s, 35.9 s) traded for BASS DROP's record
# lying on the canvas under its stomp ring (42.0 s) and fading as the third REWIND starts (42.37 s: ghosts, tape look)
FRAMES = [110, 375, 385, 397, 429, 510, 645, 647, 659, 753, 915, 1185, 1235, 1247, 1260, 1271, 1320, 1401, 1452, 1455,
          1470, 1492, 1537, 1582]

# Template F' (plan m4 Review Focus 1, Table O4: a rewinding ball is intangible to every kind): TURNTABLES against a
# few kit fighters, three seeds each side (FLUTE's are the template's own); EUPHONIUM's are the copy sweep. The whole
# 19-fighter sweep made the folder too big for its budget; ruling M4-R16 (the eye test) keeps a few
KIT = ('EUPHONIUM', 'PIANO', 'ORGAN', 'BASS DRUM')
VERSUS = [dict(name=f"kit_{o.lower().replace(' ', '_')}{'_swapped' if sw else ''}", seeds=(1, 2, 3), opponent=o,
               swapped=sw) for o in KIT for sw in (False, True)]

SPEC = dict(
    name='TURNTABLES',
    opponent='FLUTE',
    fights=dict(handicap=(0.8, 1.0)),   # M3-R34: at neutral TURNTABLES wins 96.8 %; FLUTE at x2 19.8 %; 0.8 ~ 77 %
    scenarios=m3refs.default_scenarios('TURNTABLES', 'FLUTE') + VERSUS,
    sprite_size=dict(add=7, clip=(12, 30)),     # its own sprites: #31's render_sprite, 2 r + 7 at r in 12..30
    left_in_cache=True,
    origin=dict(
        episode=EPISODE,
        config=dict(MOOD='big_top',               # #31's dark breakbeat is not a kit mood yet: its buses as the part's
                    SHARD_SEED=271, FX_SEED=31),  # render.py's own seeds (default_rng(271), fx.Particles(seed=31))
        silence=SILENCE,
        fights=[dict(seed=1339, frames=FRAMES, buses=['decks', 'solo', 'fx', 'bass']),     # #31's: TURNTABLES wins
                dict(seed=6, frames=14, buses=['decks', 'fx', 'bass'])],                   # it loses (K.O. 52.7 s)
        other=OTHER,
        layers=LAYERS,
        harness=[dict(name="#31's beat ramp (0.07 s)", beat_pulse_pre=0.07),
                 dict(name="#31's HUD (no ability list); its bus name decks",
                      call='ballfight.kit.fighters.turntables:origin_harness')],
    ),
)

EXPORTS = {'fighter_turntables': lambda: m3refs.fighter_refs(SPEC)}

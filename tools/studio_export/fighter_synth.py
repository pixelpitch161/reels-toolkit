"""fighter_synth: SYNTH against the Python kit (plan m4 Task 10, Template F'), with its reference kit module
tools/studio_export/older/fighters/synth.py proven against #31 v3 (projects/ballfight_turntables_vs_synth) by Gate O:

    python -m tools.studio_export origin fighters/synth      Gate O: writes studio/reference/fighters/synth/origin.json

Gate O's episode patches (each one exact text replacement in a scratch copy of #31's render.py, never in projects/):

- `other` (always applied): #31's TURNTABLES drawn as the kit's bare ball (its record floor and gloss, tonearm, REWIND
  ghosts and tape look, BASS DROP's record, its hits' effects, sparks, glows, fringes and flashbulbs, its shards, its
  K.O. spray, its solo's rings, its sprite's pitch and wiggle, its jerk glow and rope shakes all as base.Look gives
  them), and #31's renderer matched to the kit's where a bare ball shows it (the sprite light 1.06 + 0.3 glow, the rope
  wave's 0.15 s delay, the K.O. glow 46 px / 0.9, the heat shimmer right after the lens, FIGHT!'s punch 0.25);
- `layers`: SYNTH's layers, each group off = the kit's base.Look for it;
- `harness`: the kit matched to #31's own series rules inside the window (its beat ramp 0.07 s; its HUD without the
  ability list, which the series added on Oct 25, after #31)."""
from . import m3refs

R = 'render.py'

# ---------------------------------------------------------------- #31: TURNTABLES bare, and the renderer the kit's
OTHER = [
    # the sprites: TURNTABLES (the bare ball) is the kit's Pixel Pitch model, not tipped
    (R, "    if key not in cache:\n        cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))\n"
        "    return cache[key]",
     "    if key not in cache:\n        if NAMES[k] == 'SYNTH':\n"
     "            cache[key] = ST.render_sprite(nm, r, float(yk), float(roll), float(pitch))\n"
     "        else:\n            import sprites as SP\n"
     "            cache[key] = SP.render_sprite(nm, r, float(yk), float(roll))\n    return cache[key]"),
    # (base) the ball's sprite light: the kit's 1.06 + 0.3 glow, no lift
    (R, "        lit = sub[..., :3].astype(np.float32) * (1.12 + 0.3 * glow) + LIFT.get(NAMES[k], 10)",
     "        lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow)"),
    # TURNTABLES: the deck wiggling while it scratches / solos
    (R, "    if k in REC and (st['haste'] > 0 or (k == WIN and solo_env(t) > 0.2 and t > T_SOLO)):\n"
        "        yaw += 12 * math.sin(2 * math.pi * t / (T.BEAT / 2))\n", ""),
    # TURNTABLES: its thump on every jerk of the floor
    (R, "        if k == DJ and len(JERK_T):", "        if False:"),
    # TURNTABLES: the record floor, its gloss, its tonearm's shadow
    (R, "        alb, on_rec = ST.record_albedo(alb, sxp, szp, phi, COL[DJ])\n", ""),
    (R, "        Ic[on_rec] *= 1.12", "        pass"),
    (R, "    tip, h, on = arm\n    for kind, Q in ST.tonearm(PIVOT, tip, h):\n        if kind in ('arm', 'shell', 'weight'):\n"
        "            polys.append(proj_point(Q, spot))\n", ""),
    # TURNTABLES: its solo's groove rings (only the SYNTH's, when it wins)
    (R, "    rings = solo_rings(t) if t >= T_SOLO - 0.01 else []",
     "    rings = solo_rings(t) if t >= T_SOLO - 0.01 and NAMES[WIN] == 'SYNTH' else []"),
    # TURNTABLES: BASS DROP's darkening target and impact ring on the canvas, its record, its shimmer
    (R, "        sl = RUN['SLAMS'][i] if in_fight else []", "        sl = []"),
    (R, "    draw_drop(arr, cam, t, i, in_fight, falling=False)", "    pass"),
    (R, "    draw_drop(arr, cam, t, i, in_fight, falling=True)\n", ""),
    (R, "    for e in EV:\n        if e[1] == 'slam':\n            u = (tc - e[0]) / 0.6",
     "    for e in []:\n        if e[1] == 'slam':\n            u = (tc - e[0]) / 0.6"),
    # (base) the ropes: the wave's shake starts 0.15 s after the ability, as the kit's; no shake on the floor's jerks
    (R, "        if 0.0 < dt < 1.2:\n"
        "            disp += 0.07 * k * math.exp(-dt / 0.3) * math.sin(2 * math.pi * 4.0 * dt) * np.sin(np.pi * u)",
     "        if 0.15 < dt < 1.2:\n"
     "            disp += 0.07 * k * math.exp(-(dt - 0.15) / 0.3) * math.sin(2 * math.pi * 4.0 * (dt - 0.15)) * "
     "np.sin(np.pi * u)"),
    (R, "WAVE_EV += [(e[0], 0.35) for e in EV if e[1] == 'jerk']", "WAVE_EV += []"),
    # TURNTABLES: REWIND's ghosts, its tonearm, its tape look and glyph
    (R, "        if rk > 0 and win_ is not None and fs[k]['vis']:", "        if False:"),
    (R, "    draw_tonearm(fr, cam, L, arm, t)\n", ""),
    (R, "        if rk > 0:\n            img = ST.vhs(", "        if False:\n            img = ST.vhs("),
    # TURNTABLES: its shards (glass only)
    (R, "['glass'] * 24 + (['vinyl'] * 8 + ['label'] * 3 + ['arm'] * 2 + ['plinth'] * 3 if loser == 'TURNTABLES' else",
     "['glass'] * 24 + ([] if loser == 'TURNTABLES' else"),
    # the crowd's flashbulbs: TURNTABLES' hits as base.Look's
    (R, "(e[1] == 'hit' and e[7] in ('slam', 'wave', 'scratch', 'lash'))",
     "(e[1] == 'hit' and e[7] in (('slam', 'wave', 'scratch', 'lash') if e[2] == SY else ('slam', 'wave', 'shockwave')))"),
    # TURNTABLES: no effects of its own (its hits, jerks, BASS DROP: base.Look's shake 1.6 only); base K.O. spray
    (R, "            if how == 'scratch':                                       # a rope slam",
     "            if att != SY:\n                pass\n            elif how == 'scratch':"),
    (R, "        elif kind == 'jerk':", "        elif kind == 'jerk' and False:"),
    (R, "            FL.add(ts, S.to_world(cx, cz, 1.2), COL[owner], radius=2.6, peak=1.2, decay=0.5)\n"
        "            PA.emit(ts, c, 'shards', n=24, spread=180, speed=(1.2, 3.6), scale=1.3)\n"
        "            MK.add(ts, 'crack', cx, cz, size=1.4, n=8, life=7.0, seed=int(ts * 10))\n"
        "            MK.add(ts, 'scorch', cx, cz, size=1.1, col=(60, 70, 30), life=7.0, seed=int(ts * 10) + 1, "
        "strength=0.6)\n", ""),
    (R, "        PA.emit(T_KO, pos, 'shards', n=24, spread=180, speed=(0.8, 2.6))",
     "        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))"),
    # the spark sprays: TURNTABLES' hits as base.Look's
    (R, "            n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else \\\n"
        "                ((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))",
     "            n, sp, life = ((16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else "
     "((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))) if att == SY else \\\n"
     "                ((16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave', 'cannon') else (5, 1.8, 0.18))"),
    # (base) the heat shimmer right after the lens, as the kit's
    (R, "    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n",
     "    img = dof(img, coc_map(depth, zf, 26.0, sharp=0.17))\n    img = shimmers(img, cam, t)\n"),
    (R, "    img = shimmers(img, cam, t)\n    img = grade(", "    img = grade("),
    # the hit glows: TURNTABLES' hits as base.Look's (none)
    (R, "        if e[7] in ('slam', 'wave', 'scratch', 'lash'):\n            s_ = cam.wp(",
     "        if e[2] == SY and e[7] in ('slam', 'wave', 'scratch', 'lash'):\n            s_ = cam.wp("),
    # (base) the K.O. glow
    (R, "        fr.glow(s_[0], s_[1], 40, (255, 240, 220), 0.7 * (1 - ss(T_KO, T_HS + 0.25, t)))",
     "        fr.glow(s_[0], s_[1], 46, (255, 240, 220), 0.9 * (1 - ss(T_KO, T_HS + 0.25, t)))"),
    # the fringes: TURNTABLES' hits as base.Look's
    (R, "        if e[1] == 'snap' or e[7] in ('slam', 'wave', 'scratch'):",
     "        if ((e[1] == 'snap' or e[7] in ('slam', 'wave', 'scratch')) if e[2] == SY else "
     "(e[1] == 'hit' and e[7] in ('slam', 'wave', 'shockwave'))):"),
    # (base) FIGHT!'s punch
    (R, "            punch = 1 + (0.25 if txt != 'FIGHT!' else 0.12) * max(0.0, 1 - u / 0.12) ** 2",
     "            punch = 1 + 0.25 * max(0.0, 1 - u / 0.12) ** 2"),
]

# ---------------------------------------------------------------- #31: the SYNTH's layers (all off = the kit's base.Look)
LAYERS = [
    dict(name='sprite', patches=[(R, "        if NAMES[k] == 'SYNTH':\n            cache[key] = ST.render_sprite(",
                                  "        if False:\n            cache[key] = ST.render_sprite(")]),
    dict(name='cables', patches=[(R, "    jacks = draw_cables(arr, cam, t, fs, glows)\n", "    jacks = []\n")]),
    dict(name='effects', patches=[
        (R, "            if att != SY:\n                pass\n", "            if True:\n                pass\n"),
        (R, "        elif kind == 'snap':", "        elif kind == 'snap' and False:"),
        (R, "        elif kind == 'leash':", "        elif kind == 'leash' and False:"),
        (R, "        elif kind == 'ability' and e[4] == 'wave':", "        elif kind == 'ability' and e[4] == 'wave' and False:"),
        (R, "    if NAMES[LOSE] == 'SYNTH':\n        PA.emit(T_KO, pos, 'glitch'", "    if False:\n        PA.emit(T_KO, pos, 'glitch'")]),
    dict(name='sparks', patches=[
        (R, "            if how == 'volt':\n                continue\n", ""),
        (R, "((16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else "
            "((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))) if att == SY else \\\n                ", ""),
        (R, "events_between(t - 0.32, t, ('hit', 'plug_miss'))", "events_between(t - 0.32, t, ('hit',))")]),
    dict(name='flashbulbs', patches=[(R, "('slam', 'wave', 'scratch', 'lash') if e[2] == SY else",
                                      "('slam', 'wave', 'shockwave') if e[2] == SY else")]),
    dict(name='hit_glow', patches=[(R, "        if e[2] == SY and e[7] in ('slam', 'wave', 'scratch', 'lash'):",
                                    "        if False:")]),
    dict(name='fringe', patches=[(R, "((e[1] == 'snap' or e[7] in ('slam', 'wave', 'scratch')) if e[2] == SY else",
                                  "((e[1] == 'hit' and e[7] in ('slam', 'wave', 'shockwave')) if e[2] == SY else")]),
    dict(name='white_noise', patches=[
        (R, "        wav = RUN['WAVES'][i] if in_fight else []", "        wav = []"),
        (R, "        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][sim_i(t)]:\n            img = fx.shimmer_world",
         "        for owner, cx, cz, r, rmax, kind, t0 in []:\n            img = fx.shimmer_world")]),
    dict(name='solo_rings', patches=[(R, "and NAMES[WIN] == 'SYNTH' else []", "and False else []")]),
    dict(name='shards', patches=[(R, "['key'] * 7 + ['bkey'] * 4 + ['knob'] * 4 + ['jack'] * 4)", "[])")]),
    dict(name='quiet_pulses', patches=[(R, "            if e[3] == k and e[7] not in ('volt',):",
                                        "            if e[3] == k and e[7] not in ():")]),
]


def harness(stage, module):
    """#31's HUD (stage 'picture'): the fight HUD without the ability list (the series' Oct 25 rule came after #31)"""
    if stage != 'picture':
        return
    real = module.fight_hud

    def fight_hud(img, left, right, *a, abilities=None, **kw):
        return real(img, left, right, *a, **kw)
    module.fight_hud = fight_hud


SPEC = dict(name='SYNTH', opponent='FLUTE', origin=dict(
    episode='ballfight_turntables_vs_synth',
    config=dict(MOOD='big_top',                 # #31's dark breakbeat is not a kit mood yet: the SYNTH's own bus only
                SHARD_SEED=271, FX_SEED=31),    # render.py's own seeds (default_rng(271), fx.Particles(seed=31))
    frames=24,
    other=OTHER,
    layers=LAYERS,
    harness=[dict(name="#31's beat ramp (0.07 s; the kit's 0.12 s)", beat_pulse_pre=0.07),
             dict(name="#31's HUD (no ability list: the series rule of Oct 25 came after #31)",
                  call='tools.studio_export.fighter_synth:harness')],
    buses=['synth'],
))

EXPORTS = {'fighter_synth': lambda: m3refs.fighter_refs(SPEC)}

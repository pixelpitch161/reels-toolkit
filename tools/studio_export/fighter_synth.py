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

# ---------------------------------------------------------------- Gate O, more of it (the controller's asks after the
# first review): SYNTH's placements on every bus it plays on, and a second fight of #31's config that SYNTH wins
#
#   python -m tools.studio_export fighter_synth_origin search   the first seed from 1 of #31's config that fits the
#                                                              kit's beat sheet (search.one) with SYNTH winning
#   python -m tools.studio_export fighter_synth_origin run      writes fighters/synth/origin_extra.json
#
# Buses: SYNTH's contribution to each of synth, fx, bass, solo: the episode's (audio.py as delivered minus audio.py
# with SYNTH's placements removed, SILENCE) against the kit's (the module's Sound minus base.Sound), sample for sample in
# float64, over the window and the whole buffer. The bed's and TURNTABLES' placements are on both sides of each
# difference and cancel (a Gate O for a fighter cannot silence the kit's bed, and the kit's shared fx differ from #31's).
# Second fight: SECOND_SEED, the episode switched to it by one patch of render.py (its sim.load() reads seed.txt);
# frames and state through older.origin.fighter_origin, buses as above.
A_ = 'audio.py'
SILENCE = [
    (A_, "    for m in (65, 72, 77, 80):\n        mix.place('synth', arp_lv(m, 3, 1.0), t3, 0.16, pan=0.2)\n"
         "    mix.place('synth', supersaw_sweep(0.45, 65, 89), t3, 0.14, pan=0.2)\n", ""),            # its title blast
    (A_, "        for m in (60, 64, 67):\n            mix.place('synth', arp_note(m, 0.9, 0.18), tc, 0.20, pan=0.2)\n",
     ""),                                                                                          # its count-in
    (A_, "    for slot, evs in sorted(by_tick.items()):", "    for slot, evs in []:"),             # the pulses' arp
    (A_, "        if kind == 'hit' and e[2] == KS:", "        if kind == 'hit' and False:"),     # plug, cut, noise hit
    (A_, "        elif kind == 'plug_fire' and e[2] == KS:", "        elif kind == 'plug_fire' and False:"),
    (A_, "        elif kind == 'snap':", "        elif kind == 'snap' and False:"),
    (A_, "        elif kind == 'leash':", "        elif kind == 'leash' and False:"),
    (A_, "            elif name == 'LFO':", "            elif name == 'LFO' and False:"),
    (A_, "            elif name == 'SAWTOOTH':", "            elif name == 'SAWTOOTH' and False:"),
    (A_, "            elif name == 'WHITE NOISE':", "            elif name == 'WHITE NOISE' and False:"),
    (A_, "            mix.place('synth', x, RD.T_HS + 0.07 * j, 0.20 * (1 - j / 10), pan=(j - 3.5) / 8)",
     "            pass"),                                                                         # it falls apart
    (A_, "            mix.place('solo', arp_note(m, 1.0, 0.4), tw, 0.20)", "            pass"),  # its stinger
    (A_, "            mix.place('solo', arp_note(m, 0.95, max(0.1, d * T.BEAT * 0.9)), solo0 + bt * T.BEAT, 0.34)",
     "            pass"),                                                                         # its solo
    (A_, "            mix.place('synth', arp_note(m, 0.7), tj, 0.18)", "            pass"),      # its reassembly run
]
EXTRA_BUSES = ('synth', 'fx', 'bass', 'solo')
SECOND_SEED = 415                      # `fighter_synth_origin search`: the first fit with SYNTH winning (415 seeds)


def seed_patch(seed):
    return (R, "RUN = sim.load()", f"RUN = sim.load({int(seed)})")


def _search_child(cfg_folder, harness, a, b):
    from .older import origin as O
    O._kit_load(cfg_folder, harness)
    from ballfight.kit import search
    for seed in range(a, b):
        st = search.one(seed)
        if st['fits'] and st['winner'] == 1:
            return dict(seed=seed, t_ko=st['t_ko'], winner_hp=st['winner_hp'], caps=st['caps'], searched=seed - a + 1)
    return None


def _kit_buses_all_child(cfg_folder, harness, name, out_npz):
    """the kit's score before the master, every bus: SYNTH by its module (name) or both by base.Sound (None)"""
    import numpy as np
    from . import common
    from .older import origin as O
    ep, _ = O._kit_load(cfg_folder, harness)
    O._bare({n for n in ep.T.FIGHTERS if n != name})
    from ballfight.kit import music as M              # noqa: N812
    O._harness(harness, 'music', M)
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly  # noqa: F401
    M.SOUNDS = [M.SOUNDS[k] if M.NAMES[k] == name else M.FIGHTER_MODULES.base.Sound(M, k) for k in (0, 1)]
    mix = M.score()
    common.write_npz(out_npz, {k: np.array(v, np.float64, copy=True) for k, v in mix.buses.items()}, register=False)
    return sorted(mix.buses)


def _config_for(o, seed, tag):
    from .older import origin as O
    facts = O._run_parts([(O.episode_facts_child, (O.scratch(o['episode']), seed))])[0]
    folder, text, timeline = O._config(dict(o, seed=seed), facts, tag)
    return folder, text, timeline, facts


def _bus_contributions(seed):
    """SYNTH's contribution to each of EXTRA_BUSES on the episode's and the kit's run of `seed`"""
    import os
    import numpy as np
    from .older import origin as O
    o = O.origin_spec(SPEC['origin'])
    ep = o['episode']
    cfg_dir, cfg_text, timeline, facts = _config_for(o, seed, f'synth_buses_{seed}')
    kf = O._run_parts([(O.kit_facts_child, (cfg_dir, o['harness'], 1))])[0]
    if kf['raw_line'] != facts['line']:
        raise SystemExit(f'fighter_synth_origin: seed {seed}: the kit {kf["raw_line"]!r}, the episode {facts["line"]!r}')
    full = [] if seed == 1339 else [seed_patch(seed)]
    e_full = O.scratch(ep, full, f'buses_{seed}') if full else O.scratch(ep)
    e_sil = O.scratch(ep, full + SILENCE, f'buses_{seed}_silent')
    od = os.path.join(O.ROOT, 'out', ep, f'synth_buses_{seed}')
    paths = {w: os.path.join(od, f'{w}.npz') for w in ('e_full', 'e_sil', 'k_mod', 'k_base')}
    cfg_mod = O.kit_config_folder(ep, f'synth_buses_{seed}_mod', cfg_text)
    cfg_base = O.kit_config_folder(ep, f'synth_buses_{seed}_base', cfg_text)
    O._run_parts([(O.episode_stems_child, (e_full, paths['e_full'], None)),
                  (O.episode_stems_child, (e_sil, paths['e_sil'], None)),
                  (_kit_buses_all_child, (cfg_mod, o['harness'], 'SYNTH', paths['k_mod'])),
                  (_kit_buses_all_child, (cfg_base, o['harness'], None, paths['k_base']))])
    O.check_kit_clean(f'projects/{ep}/audio.py')
    got = {w: O._load_npz(p) for w, p in paths.items()}
    n = len(next(iter(got['e_full'].values())))
    i0 = int(round(kf['t_fight'] * O.SR))
    i1 = int(round(min(kf['t_reassemble'], float(timeline['T_REASSEMBLE'])) * O.SR))

    def bus(w, b):
        return got[w][b] if b in got[w] else np.zeros((n, 2))
    rows, failures = [], []
    for b in EXTRA_BUSES:
        ec, kc = bus('e_full', b) - bus('e_sil', b), bus('k_mod', b) - bus('k_base', b)
        summary, bad = O.compare_bus('SYNTH', b, ec, kc, i0, i1, o['bar_db'])
        mx = float(np.abs(ec[i0:i1]).max())
        summary['contribution_peak'] = f'{O.db(mx):.1f} dBFS' if mx else 'silent'
        rows.append(summary)
        if bad:
            failures.append(f'seed {seed}: ' + bad.replace("bus '", "contribution to bus '"))
    return dict(seed=seed, fight=kf['raw_line'], winner=kf['winner'], window=[i0, i1], buses=rows), failures


def origin_extra(args):
    import json
    from . import common
    from .older import install, origin as O
    install()
    o = O.origin_spec(SPEC['origin'])
    if args == ['search']:
        cfg_dir, *_ = _config_for(o, 1339, 'synth_search')
        hit = O._run_parts([(_search_child, (cfg_dir, o['harness'], 1, 4000))])[0]
        print(f'fighter_synth_origin: first fit with SYNTH winning: {hit}')
        return
    if args != ['run'] or SECOND_SEED is None:
        raise SystemExit('fighter_synth_origin: search | run (run needs SECOND_SEED)')
    failures, out = [], dict(about="Gate O, more of it (plan m4 Task 10, the Gate O review): SYNTH's contribution to "
                                   "each bus it plays on (the episode's audio.py minus its SILENCE patches, the kit's "
                                   "module minus base.Sound) on #31's seed and on a second seed of its config that "
                                   "SYNTH wins (the first fit of the kit's search with SYNTH winning); that fight's "
                                   "frames and state through older.origin.fighter_origin")
    out['silence'] = O._patch_doc(SILENCE)
    for seed in (1339, SECOND_SEED):
        doc, bad = _bus_contributions(seed)
        out[f'buses_{seed}'] = doc
        failures += bad
    spec2 = dict(SPEC, origin=dict(SPEC['origin'], seed=SECOND_SEED, other=OTHER + [seed_patch(SECOND_SEED)],
                                   buses=[]))
    d2 = O.fighter_origin(spec2, out=False)
    out['second_fight'] = dict(
        seed=SECOND_SEED, fight=d2['fight'], window=d2['window'], state_keys=d2['state_keys'],
        seed_patch=O._patch_doc([seed_patch(SECOND_SEED)]),
        frames=[[r['frame'], r['contribution_sha256'][:16], r['whole_frame_equal'],
                 r['sha256']['kit_on'] != r['sha256']['kit_off']] for r in d2['frames']],
        frame_columns=['frame', 'contribution sha256 (16 hex)', 'whole frames equal', 'SYNTH draws'])
    if failures:
        raise SystemExit(failures[0] + (f' (and {len(failures) - 1} more)' if len(failures) > 1 else ''))
    common.write_json(common.ref('fighters', 'synth', 'origin_extra.json'), out)
    print('fighter_synth_origin: PASS', json.dumps({k: [(b['bus'], b['peak'], b['whole'], b['contribution_peak'])
                                                        for b in v['buses']] for k, v in out.items()
                                                    if k.startswith('buses_')}))


EXPORTS = {'fighter_synth': lambda: m3refs.fighter_refs(SPEC), 'fighter_synth_origin': origin_extra}
TAKES_ARGS = ('fighter_synth_origin',)

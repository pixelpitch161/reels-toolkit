"""fighter_bari_sax: BARI SAX's references (plan m4 Task 9, Template F').

Step 0 (Gate O): the reference kit module tools/studio_export/older/fighters/bari_sax.py against #39 v4 Bassoon vs
Bari Sax (projects/ballfight_bassoon_vs_barisax, seed 77: the kit's run of the episode's own config replays #39's fight,
plan m4 Task 5). Every patch below is one exact text replacement in the episode's scratch copy (never in projects/):

- `other` (always applied): #39's BASSOON drawn and played as the kit's bare ball (base.Look, base.Sound) without its
  TIPTOE hops, stomps, BOCAL HOOK and MOUNTAIN KING, its hits' sparks, glows, fringes, shakes and flashbulbs as
  base.Look gives them, its shards and K.O. spray as base.Look's; the crane's leap test kept on the BASSOON's hops (it
  is the BARI SAX's layer: the crane goes under the balls when a hop sails over the swing); and one naming: #39's
  ball height `h` is reported in fighter() under the kit's state names (lift = h; paint_h = R when it is off the canvas: #39 painted every
  ball at its resting height; air_bias = 0.6 above 0.02: #39's airborne bias), so Gate O's state check compares them.
- `layers` (applied on top: all of them off = the BARI SAX as base.Look draws a bare ball): its pose on the cable, the
  harness, the swing's smear and trail, the crane, the ball over a front rope, the pressed ropes and their spring-back,
  the canvas rings and CABLE SNAP's target, the shimmer, GROWL's fronts, its shards, its effects, shakes, sparks, hit
  glows, fringes, flashbulbs, rope wobble strength and its HUD holds.
- `harness`: #39's beat ramp (0.10 s; the kit's 0.12 s) and #39's bus name ('bari'; the kit's 'bari sax', M4-R9).

    python -m tools.studio_export origin fighters/bari_sax     Gate O: writes studio/reference/fighters/bari_sax/origin.json"""
from . import m3refs

R_ = 'render.py'
A_ = 'audio.py'

# ---------------------------------------------------------------- #39: its BASSOON drawn as the kit's bare ball
BASSOON_BARE = [
    (R_, "HOPPER = [k for k in (0, 1) if CFG[k]['weapon']['kind'] == 'tiptoe']",
     "HOPPER = []"),                                                                   # no hops, crook, arcs, bobs
    (R_, "    crane_first = any(fs[k_].get('leap') for k_ in HOPPER)",
     "    crane_first = any(hop_at(k_, t)[0] is not None and leap_k(k_, hop_at(k_, t)[0]) > 1.0 for k_ in (0, 1)\n"
     "                      if CFG[k_]['weapon']['kind'] == 'tiptoe')"),                # (the bari's crane: its leap test)
    (R_, "        if in_fight:\n            img = ST.swirl(", "        if False:\n            img = ST.swirl("),
    (R_, "    stomps = live_stomps(t)\n", "    stomps = []\n"),
    (R_, "    for k_, x_, z_, a_, age in live_slashes(t):", "    for k_, x_, z_, a_, age in []:"),
    (R_, "    mk = mk_state(t) if in_fight else None", "    mk = None"),
    (R_, "    for t_, k, x, z, hit, back, rad, kind in STOMPS:          # TIPTOE",
     "    for t_, k, x, z, hit, back, rad, kind in []:          # TIPTOE"),
    (R_, "            if how == 'stomp' and e[8]:                            # a backstab",
     "            if how == 'stomp' and e[8] and False:                            # a backstab"),
    (R_, "            elif how == 'stomp':\n                PA.emit(", "            elif how == 'stomp' and False:\n                PA.emit("),
    (R_, "            elif how == 'hook':                                    # BOCAL HOOK lands",
     "            elif how == 'hook' and False:                                    # BOCAL HOOK lands"),
    (R_, "    for t0, t1, k in MKS:                                         # MOUNTAIN KING",
     "    for t0, t1, k in []:                                         # MOUNTAIN KING"),
    (R_, "    if e[1] == 'hit' and e[7] in ('stomp', 'hook', 'sling', 'shockwave', 'quake') and e[0] <= T_KO:",
     "    if e[1] == 'hit' and e[7] in ('sling', 'shockwave', 'quake') and e[0] <= T_KO:"),     # base.Look.shake
    (R_, "            if how == 'stomp':\n                n, sp, life = (12, 2.8, 0.26)",
     "            if NAMES[att] == 'BASSOON':\n"
     "                n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave', 'cannon') else (5, 1.8, 0.18)\n"
     "            elif how == 'stomp':\n                n, sp, life = (12, 2.8, 0.26)"),              # base.Look.spark
    (R_, "        if e[7] in ('quake', 'sling', 'hook', 'shockwave', 'rattle', 'wreck', 'crash') or (e[7] == 'stomp' and e[8] == 1):",
     "        if e[7] in ('quake', 'sling', 'shockwave', 'rattle', 'wreck', 'crash'):"),            # hit glows
    (R_, "        if e[7] in ('quake', 'sling', 'hook', 'shockwave', 'wreck', 'crash') or (e[7] == 'stomp' and e[8] == 1):",
     "        if e[7] in ('quake', 'sling', 'shockwave', 'wreck', 'crash'):"),                      # fringes
    (R_, "e[7] in ('quake', 'sling', 'hook', 'wreck', 'crash')) or", "e[7] in ('quake', 'sling', 'wreck', 'crash')) or"),
    (R_, "    kinds = ['glass'] * 24 + ['wood'] * 6 + ['key'] * 8 + (['reed'] * 3 if NAMES[LOSE] == 'BASSOON' else ['bell'] * 3)",
     "    kinds = ['glass'] * 24 + ([] if NAMES[LOSE] == 'BASSOON' else ['wood'] * 6 + ['key'] * 8 + ['bell'] * 3)"),
    (R_, "    PA.emit(T_KO, pos, 'chips' if NAMES[LOSE] == 'BASSOON' else 'brass', n=26, spread=180, speed=(0.8, 2.2),\n"
         "            col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])",
     "    if NAMES[LOSE] == 'BASSOON':\n"
     "        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))\n"
     "    else:\n"
     "        PA.emit(T_KO, pos, 'brass', n=26, spread=180, speed=(0.8, 2.2),\n"
     "                col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])"),                     # base.Look.ko_fx
    # the kit's state names for #39's height (Gate O's state check), on the fight and the champion's tail
    (R_, "def fighter(k, t):\n",
     "def _kit_keys(st):\n"
     "    st['lift'] = st['h']\n"
     "    if st['h'] != 0:\n"
     "        st['paint_h'] = R\n"
     "    if st['h'] > 0.02:\n"
     "        st['air_bias'] = 0.6\n"
     "    return st\n\n\n"
     "def fighter(k, t):\n"),
    (R_, "        if t >= T_KO and k == LOSE:\n            st['glow'] = 1.0\n        return st\n",
     "        if t >= T_KO and k == LOSE:\n            st['glow'] = 1.0\n        return _kit_keys(st)\n"),
    (R_, "            st['h'] = 0.35 * (1 - e) * e * 4\n        return st\n",
     "            st['h'] = 0.35 * (1 - e) * e * 4\n        return _kit_keys(st)\n"),
]

# ---------------------------------------------------------------- #39: its BARI SAX's layers (all off = base.Look)
BARI_LAYERS = [
    dict(name='pose', patches=[
        (R_, "        ap = air_pose(k, min(t, T_KO - 1e-4))", "        ap = None"),
        (R_, "H_KO = [(air_pose(k, T_KO - 1e-4) or (None, 0.0))[1] for k in (0, 1)]", "H_KO = [0.0, 0.0]")]),
    dict(name='harness', patches=[
        (R_, "    if k in WRECKER and st['scale'] > 0.5:", "    if False:")]),
    dict(name='swing_fx', patches=[
        (R_, "\n        if obj in WRECKER and T.T_FIGHT <= t < T_HS:            # v2: smear + trail",
         "\n        if False:            # v2: smear + trail"),
        (R_, "\n            if obj in WRECKER and T.T_FIGHT <= t < T_HS:            # v2: smear + trail",
         "\n            if False:            # v2: smear + trail")]),
    dict(name='crane', patches=[
        (R_, "    crane_first = any(hop_at(k_, t)[0] is not None", "    crane_first = False and any(hop_at(k_, t)[0] is not None"),
        (R_, "    for k_ in (WRECKER if not crane_first else ()):", "    for k_ in ():")]),
    dict(name='late', patches=[
        (R_, "    late = [k_ for k_ in WRECKER if in_fight", "    late = [k_ for k_ in () if in_fight")]),
    dict(name='ropes', patches=[
        (R_, "    for c, hb in bari_over(t):                                    # v3",
         "    for c, hb in []:                                    # v3"),
        (R_, "ROPE_EV = sorted(ROPE_EV + _rope_wobbles())", "ROPE_EV = ROPE_EV")]),
    dict(name='canvas', patches=[
        (R_, "        wav = RUN['WAVES'][i] if in_fight else []", "        wav = []"),
        (R_, "        for t0_, k_s, spot_, ts_, t1_, rad_ in SNAPS:", "        for t0_, k_s, spot_, ts_, t1_, rad_ in []:")]),
    dict(name='shimmer', patches=[
        (R_, "    if T.T_FIGHT <= t < T_HS:\n        i = sim_i(min(t, T_KO))\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:",
         "    if False:\n        i = sim_i(min(t, T_KO))\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:")]),
    dict(name='fronts', patches=[
        (R_, "    if in_fight:\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:\n            arr = ST.draw_honk_front(",
         "    if False:\n        for owner, cx, cz, r, rmax, kind, t0 in RUN['WAVES'][i]:\n            arr = ST.draw_honk_front(")]),
    dict(name='shards', patches=[
        (R_, "else ['wood'] * 6 + ['key'] * 8 + ['bell'] * 3)", "else [])")]),
    dict(name='effects', patches=[
        (R_, "            elif how == 'shockwave':                               # GROWL lands: grit",
         "            elif how == 'shockwave' and False:                               # GROWL lands: grit"),
        (R_, "            elif how == 'wreck':                                   # v2: WRECKING BALL smash",
         "            elif how == 'wreck' and False:                                   # v2: WRECKING BALL smash"),
        (R_, "            elif how == 'crash':                                   # CABLE SNAP lands on its target",
         "            elif how == 'crash' and False:                                   # CABLE SNAP lands on its target"),
        (R_, "        elif kind == 'swing':                                      # v2: the swing's bottom",
         "        elif kind == 'swing' and False:                                      # v2: the swing's bottom"),
        (R_, "        elif kind == 'cable_snap':                                 # CABLE SNAP: the cable parts",
         "        elif kind == 'cable_snap' and False:                                 # CABLE SNAP: the cable parts"),
        (R_, "        elif kind == 'crater':                                     # the crash",
         "        elif kind == 'crater' and False:                                     # the crash"),
        (R_, "        elif kind == 'ability' and e[4] == 'shockwave':            # GROWL leaves the bari",
         "        elif kind == 'ability' and e[4] == 'shockwave' and False:            # GROWL leaves the bari"),
        (R_, "    else:\n        PA.emit(T_KO, pos, 'brass', n=26, spread=180, speed=(0.8, 2.2),\n"
             "                col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])",
         "    else:\n        PA.emit(T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))")]),
    dict(name='shake', patches=[
        (R_, "    if e[1] == 'hit' and e[7] in ('sling', 'shockwave', 'quake') and e[0] <= T_KO:",
         "    if e[1] == 'hit' and e[7] in () and e[0] <= T_KO:"),
        (R_, "for t_, k_, x_, z_, sp_ in SMASHES:                    # v2", "for t_, k_, x_, z_, sp_ in []:                    # v2"),
        (R_, "for t_, k_, x_, z_, r_, hit_ in CRATERS:               # CABLE SNAP", "for t_, k_, x_, z_, r_, hit_ in []:               # CABLE SNAP")]),
    dict(name='sparks', patches=[
        (R_, "            if NAMES[att] == 'BASSOON':\n                n, sp, life = (16, 3.2, 0.3) if how in",
         "            if True:\n                n, sp, life = (16, 3.2, 0.3) if how in")]),
    dict(name='hit_glow', patches=[
        (R_, "        if e[7] in ('quake', 'sling', 'shockwave', 'rattle', 'wreck', 'crash'):", "        if False:")]),
    dict(name='fringe', patches=[
        (R_, "        if e[7] in ('quake', 'sling', 'shockwave', 'wreck', 'crash'):\n            s_ = cam.wp",
         "        if e[7] in ('slam', 'wave', 'shockwave'):\n            s_ = cam.wp")]),
    dict(name='flashbulbs', patches=[
        (R_, "e[7] in ('quake', 'sling', 'wreck', 'crash')) or", "e[7] in ('slam', 'wave', 'shockwave')) or")]),
    dict(name='rope_wave', patches=[
        (R_, "WAVE_EV = [(e[0], 1.0 if e[4] == 'quake' else 0.5) for e in EV if e[1] == 'ability' and e[4] in ('shockwave', 'quake')]",
         "WAVE_EV = [(e[0], {'wave': 1.0, 'shockwave': 0.6}[e[4]]) for e in EV if e[1] == 'ability' and e[4] in ('wave', 'shockwave')]")]),
    dict(name='hud', patches=[
        (R_, "    if ab['kind'] in ('shield', 'haste', 'offbeat', 'quake'):", "    if ab['kind'] in ('shield', 'haste', 'quake'):"),
        (R_, "    if ab['kind'] == 'snap':                         # v2: CABLE SNAP, until the crash",
         "    if False:                         # v2: CABLE SNAP, until the crash")]),
]

SPEC = dict(
    name='BARI SAX',
    opponent='FLUTE',
    origin=dict(
        episode='ballfight_bassoon_vs_barisax',
        config=dict(MOOD='big_top',                 # #39's ska punk is not a kit mood yet: the BARI SAX's own bus only
                    SHARD_SEED=371, FX_SEED=39),    # render.py's own seeds (default_rng(371), fx.Particles(seed=39))
        frames=[89, 107, 108, 121, 189, 283, 295, 358, 413, 458, 462, 465, 495, 538, 550, 592, 631,
                740, 904, 1068, 1177, 1277, 1283, 1295, 1299, 1313, 1321, 1395, 1525, 1543, 1554, 1569, 1596, 1641],
        # choose_frames' 24 (abilities, first hit, HP < 25, the K.O., its shards) and 10 more: the first crane drop
        # (89), a rope springing back (107, 462), the block flying back up (121, 189), dropped on
        # the ropes and back in (458, 465), SKANK's HUD hold (592), the cable parting (1299), the crater (1321)
        other=BASSOON_BARE,
        layers=BARI_LAYERS,
        harness=[dict(name="#39's beat ramp (0.10 s; the kit's 0.12 s)", beat_pulse_pre=0.10),
                 dict(name="#39's bus name: the kit's 'bari sax' is #39's 'bari' (ruling M4-R9)",
                      call='ballfight.kit.fighters.bari_sax:origin_harness')],
        buses=['bari'],
    ),
)

EXPORTS = {'fighter_bari_sax': lambda: m3refs.fighter_refs(SPEC)}

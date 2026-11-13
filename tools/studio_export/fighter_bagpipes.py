"""fighter_bagpipes: BAGPIPES's references (plan m4 Template F', Task 7).

BAGPIPES is an older fighter: its look and sound exist only in #30 v3 (projects/ballfight_bagpipes_vs_accordion), so
its reference is the kit module tools/studio_export/older/fighters/bagpipes.py, proven against #30 by Gate O first:

    python -m tools.studio_export origin fighters/bagpipes   Gate O: writes studio/reference/fighters/bagpipes/origin.json

SPEC.origin is Gate O's: #30's fight (seed 1442: the kit's run of the episode's own config gives the replay guard's
line of #30's sim), its renderer switched layer by layer with exact text patches of the episode's scratch copy (never
of projects/), and the kit matched to the episode's own series rules inside the window (`harness`):

- `other` (always applied to the episode): its ACCORDION drawn as the kit's bare ball, and the parts of #28's renderer
  (which #30 copied) that the kit's renderer has since changed for every fighter. ACCORDION's own event kinds are
  renamed in the renderer's event list (bite, snap, clamp, clamp_snap, its 'bite' and 'clamp' hits, SQUEEZEBOX's
  'shield'), so none of #30's layers for them draws (its jaws and their light and glow, the bellows wall and its
  shimmer, its shakes, flashbulbs, hit glows, fringes, effects and rope wobble); the fight and its story are the run's,
  untouched. Then base.Look's spark spray for it, base.Look's block burst (no puffs), no SQUEEZEBOX collar; and the
  kit's effects clock held through the hit-stop (picture.fx_clock: the flares' light, the crowd's tint, the marks, the
  particles), the K.O. flare at the ball's height and the speed lines' 0.06 s look-back;
- `layers`: BAGPIPES's, each group named; all of them applied = the episode draws BAGPIPES as the kit's base.Look
  (`swell`: its size, the bag's tartan, the shiver, its glow and the flight's trail all follow fighter()'s state);
- `harness`: #30's beat ramp (0.07 s; the kit's 0.12 s) and #30's HUD without the ability list (a series rule from
  after #30; the kit's fight_hud without `abilities` draws exactly as #30's).

    python -m tools.studio_export fighter_bagpipes        writes studio/reference/fighters/bagpipes/ (after Gate O)"""
from . import m3refs

RD, AU = 'render.py', 'audio.py'

# ---------------------------------------------------------------- #30: its ACCORDION bare, and the kit's renderer
ACCORDION_BARE = [
    (RD, "EV = RUN['events']\n",
     "def _bare(e):\n"
     "    if e[1] in ('bite', 'snap', 'clamp', 'clamp_snap'):\n"
     "        return (e[0], 'x_' + e[1]) + tuple(e[2:])\n"
     "    if e[1] == 'hit' and e[7] in ('bite', 'clamp'):\n"
     "        return tuple(e[:7]) + ('x_' + e[7],) + tuple(e[8:])\n"
     "    if e[1] == 'ability' and e[4] == 'shield':\n"
     "        return tuple(e[:4]) + ('x_shield',) + tuple(e[5:])\n"
     "    return e\n\n\n"
     "def fx_clock(t):\n"
     "    if t < T_KO:\n        return t\n    if t < T_HS:\n        return T_KO\n    return t - HITSTOP\n\n\n"
     "EV = [_bare(e) for e in RUN['events']]\n"),
    (RD, "FL.apply(L, t)", "FL.apply(L, fx_clock(t))"),
    (RD, "FL.tint(heads, t)", "FL.tint(heads, fx_clock(t))"),
    (RD, "MK.apply(alb, sxp, szp, t,", "MK.apply(alb, sxp, szp, fx_clock(t),"),
    (RD, "cam, t, light=lambda P: 1.0 + 0.6 * FL.tint(P, t))",
     "cam, fx_clock(t), light=lambda P: 1.0 + 0.6 * FL.tint(P, fx_clock(t)))"),
    (RD, "pk[1], 1.2), COL", "pk[1]), COL"),
    (RD, "t - 0.1))['p']", "t - 0.06))['p']"),
    (RD, "            col = COL[att] * 0.6 + 100\n"
         "            n, sp, life = (16, 3.2, 0.3) if how in ('flight', 'bodyslam', 'clamp', 'wave', 'shockwave') else \\\n"
         "                ((8, 2.2, 0.22) if how == 'bite' else (3, 1.4, 0.16))\n",
     "            col = COL[att] * 0.6 + 100\n"
     "            bp = NAMES[att] == 'BAGPIPES'\n"
     "            n, sp, life = ((16, 3.2, 0.3) if how in ('flight', 'bodyslam', 'clamp', 'wave', 'shockwave') else \\\n"
     "                ((8, 2.2, 0.22) if how == 'bite' else (3, 1.4, 0.16))) if bp else \\\n"
     "                ((16, 3.2, 0.3) if how in ('slam', 'wave', 'shockwave', 'cannon') else (5, 1.8, 0.18))\n"),
    (RD, "(255, 214, 250), 0.6, bright=False)\n            PA.emit(t0, W_(e[4], e[5]), 'puffs', n=4)\n",
     "(255, 226, 150), 0.6, bright=False)\n"),
    (RD, "if st['shield'] > 0:", "if False:"),
    # when it loses: base.Look's shards (glass only) and K.O. spray (sparks)
    (RD, "['button'] * 6 + ['pleat'] * 6 + ['key'] * 4)", "[])"),
    (RD, "'puffs', n=24, spread=180, speed=(1.2, 3.0), scale=1.3)", "'sparks', n=30, spread=180, speed=(0.8, 2.2))"),
    # the episode's own bug (ruling M4-R11): main() overwrites its stinger time `tw` with the title card's loop
    # (`for tw in (t1, t2, t3)`); BAGPIPES's stinger lands on "(WINNER) WINS" at q16(T_KO + 1.3), as the kit's
    (AU, "        mix.place('solo', chanter(81, 1.2, 1.0, grace=True), tw, 0.36)\n"
         "        mix.place('solo', chanter(69, 1.2, 0.8), tw, 0.18)\n",
     "        mix.place('solo', chanter(81, 1.2, 1.0, grace=True), q16(T_KO + 1.3), 0.36)\n"
     "        mix.place('solo', chanter(69, 1.2, 0.8), q16(T_KO + 1.3), 0.18)\n"),
]

# ---------------------------------------------------------------- #30: its BAGPIPES's layers (all off = base.Look)
BAGPIPES_LAYERS = [
    dict(name='swell', patches=[(RD, "if k in INFL:", "if False:")]),
    dict(name='shake', patches=[(RD, "'clamp', 'wave', 'shockwave'):\n        SHK",
                                 "'clamp', 'wave', 'shockwave') and False:\n        SHK")]),
    dict(name='flashbulbs', patches=[(RD, "'clamp', 'wave', 'shockwave')) or",
                                      "'clamp', 'wave', 'shockwave') and e[7] in ('slam', 'wave', 'shockwave')) or")]),
    dict(name='sparks', patches=[(RD, "bp = NAMES[att] == 'BAGPIPES'", "bp = False"),
                                 (RD, "'clamp') and dt < 0.08", "'clamp') and False and dt < 0.08")]),
    dict(name='hit_glow', patches=[(RD, "'shockwave', 'bite'):", "'shockwave', 'bite') and False:")]),
    dict(name='fringe', patches=[(RD, "t - 0.1, t, ('hit',)):              # chromatic fringing on the big hits (local)\n"
                                      "        if e[7] in ('flight', 'bodyslam', 'clamp', 'wave'):\n"
                                      "            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]\n"
                                      "            img = fx.fringe(img, s_[0], s_[1], 30 if e[7] in ('clamp', 'wave') else "
                                      "20, 2.6 if e[7] in ('clamp', 'wave') else 1.8,\n"
                                      "                            k=1 - (t - e[0]) / 0.1)\n",
                                  "t - 0.12, t, ('hit',)):\n"
                                  "        if e[7] in ('slam', 'wave', 'shockwave'):\n"
                                  "            s_ = cam.wp(S.to_world(e[5], e[6], R)[None])[0]\n"
                                  "            img = fx.fringe(img, s_[0], s_[1], 26, 2.5, k=1 - (t - e[0]) / 0.12)\n")]),
    dict(name='effects', patches=[
        (RD, "        if t0 > T_KO + 0.01:\n            break\n",
         "        if t0 > T_KO + 0.01:\n            break\n"
         "        if kind in ('hit', 'release', 'ability') and NAMES[e[2]] == 'BAGPIPES':\n            continue\n"),
        (RD, "if c['t_land'] > T_KO + 1:\n"
             "            continue\n"
             "        for j in range(sim_i(c['t_rel']), sim_i(c['t_land']), 2):\n"
             "            k = c['k']\n"
             "            p, v = RUN['P'][j, k], RUN['V'][j, k]\n"
             "            tj = T.T_FIGHT + j / FPS\n"
             "            back = p - v / (np.linalg.norm(v) + 1e-9) * float(RUN['RAD'][j, k]) * 0.9\n"
             "            PA.emit(tj, W_(back[0], back[1], R * 0.8), 'puffs', n=1, vel=S.to_world(*(-0.4 * v), 0.0) - "
             "S.to_world(0, 0, 0))\n"
             "    for t0, who, name in STORY['caps']:\n"
             "        MO.add(t0, who, COL[who])\n"
             "    pk = P_KO[LOSE]\n"
             "    BU.add(T_KO, W_(pk[0], pk[1]), COL[LOSE], 2.0)\n"
             "    FL.add(T_KO, W_(pk[0], pk[1]), COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)\n"
             "    if NAMES[LOSE] == 'BAGPIPES':\n"
             "        PA.emit(T_KO, W_(pk[0], pk[1]), 'shreds', n=36, spread=180, speed=(1.5, 4.0))\n",
         "if True:                                 # the flight's puff trail off\n"
         "            continue\n"
         "        for j in range(sim_i(c['t_rel']), sim_i(c['t_land']), 2):\n"
         "            k = c['k']\n"
         "            p, v = RUN['P'][j, k], RUN['V'][j, k]\n"
         "            tj = T.T_FIGHT + j / FPS\n"
         "            back = p - v / (np.linalg.norm(v) + 1e-9) * float(RUN['RAD'][j, k]) * 0.9\n"
         "            PA.emit(tj, W_(back[0], back[1], R * 0.8), 'puffs', n=1, vel=S.to_world(*(-0.4 * v), 0.0) - "
         "S.to_world(0, 0, 0))\n"
         "    for t0, who, name in STORY['caps']:\n"
         "        MO.add(t0, who, COL[who])\n"
         "    pk = P_KO[LOSE]\n"
         "    BU.add(T_KO, W_(pk[0], pk[1]), COL[LOSE], 2.0)\n"
         "    FL.add(T_KO, W_(pk[0], pk[1]), COL[LOSE] * 0.9, radius=2.4, peak=1.1, decay=0.6)\n"
         "    if NAMES[LOSE] == 'BAGPIPES':\n"
         "        PA.emit(T_KO, W_(pk[0], pk[1]), 'sparks', n=30, spread=180, speed=(0.8, 2.2))\n")]),
    dict(name='canvas', patches=[(RD, "wav = RUN['WAVES'][i] if in_fight else []", "wav = []")]),
    dict(name='shimmer', patches=[(RD, "    if in_fight:  ", "    if False:  ")]),
    dict(name='shards', patches=[(RD, "['pipe'] * 5 + ['tartan'] * 7 + ['reed'] * 2 if", "[] if")]),
]


def no_ability_list(stage, module):
    """(Gate O harness) #30's HUD: the fight HUD without the ability list (BALLFIGHT series rule of Oct 25, after #30;
    reelkit.ui.fight_hud without `abilities` draws exactly as #30's HUD)"""
    if stage == 'picture':
        real = module.fight_hud

        def fight_hud(img, left, right, *a, **kw):
            kw.pop('abilities', None)
            return real(img, left, right, *a, **kw)
        module.fight_hud = fight_hud


GATE_O_BUSES = {'bagpipes': 'chanter', 'bagpipes stomp': 'stomp', 'bagpipes choir': 'choir'}


def episode_bus_names(stage, module):
    """(Gate O harness, ruling M4-R9) the module's own buses under #30's names: 'bagpipes' was #30's 'chanter', its
    'bagpipes stomp' / 'bagpipes choir' #30's bed buses 'stomp' / 'choir' (the kit's score, before the master)"""
    if stage == 'music':
        real = module.score

        def score():
            mix = real()
            for own, ep in GATE_O_BUSES.items():
                if own in mix.buses:
                    if ep in mix.buses:
                        raise SystemExit(f'Gate O: the kit score already has a bus {ep!r} (renaming {own!r})')
                    mix.buses[ep] = mix.buses.pop(own)
            return mix
        module.score = score


SHARED = ('fx', 'stomp', 'choir', 'solo')
KEEP = (404, 424, 427, 428, 466, 477, 480, 482, 509, 510, 525)   # #30 audio.py: BAGPIPES's own placements on them

ORIGIN = dict(
    episode='ballfight_bagpipes_vs_accordion',
    config=dict(MOOD='big_top',                       # #30's sea shanty is not a kit mood yet: BAGPIPES's own buses only
                FX_SEED=30,                           # render.py's fx.Particles(seed=30) (its shards' 271 is the kit's)
                LOOP=['Am', 'G', 'Am', 'Em'],         # timeline.py 81: the 4-bar loop from FIGHT!
                LINES={'BAGPIPES': [69, 69, 72, 74, 76, 74, 72, 71, 67, 67, 71, 74, 79, 78, 74, 71,   # timeline.py
                                    69, 72, 76, 81, 79, 76, 74, 72, 71, 71, 67, 71, 76, 74, 71, 68]}),  # MELODY
    frames=24,
    other=ACCORDION_BARE,
    layers=BAGPIPES_LAYERS,
    harness=[dict(name="#30's beat ramp (0.07 s; the kit's 0.12 s)", beat_pulse_pre=0.07),
             dict(name="#30's HUD: no ability list", call='tools.studio_export.fighter_bagpipes:no_ability_list'),
             dict(name="#30's bus names (M4-R9)", call='tools.studio_export.fighter_bagpipes:episode_bus_names')],
    buses=['chanter', 'drone', 'fx', 'stomp', 'choir', 'solo'],
    # the shared buses: the stems' copy keeps only BAGPIPES's own placements on fx, stomp, choir and solo (#30 audio.py
    # lines, the line count unchanged); the kit side keeps only the module Sound's own (silence_kit)
    silence=[(AU, "    mix = A.Mix(T.DUR + 4.0)\n",
              "    mix = A.Mix(T.DUR + 4.0); _place = mix.place; mix.place = lambda bus, *a, **kw: mix.bus(bus) if bus in "
              f"{SHARED!r} and sys._getframe(1).f_lineno not in {KEEP!r} else _place(bus, *a, **kw)\n")],
    # seed 1442 (#30's own: ACCORDION wins, BAGPIPES shatters); seed 628 (the first fit of the config BAGPIPES wins,
    # by the kit's search: its stinger, solo and the champion's glow)
    fights=[dict(seed=1442, buses=['chanter', 'drone', 'fx', 'stomp', 'choir']),
            dict(seed=628, frames=12)],
)

# Table O4 (a per-ball radius and mass) and Review Focus 1: BAGPIPES against every kit fighter, three seeds with
# BAGPIPES left and three swapped; and the copy sweep (Template F'): EUPHONIUM against BAGPIPES, three seeds each side,
# at the early levels and low turn so IDENTITY CRISIS copies soon (WAR CRY, HIGHLAND FLING, BAG BURST are shockwave,
# haste and wave, all copyable), every copied kind drawn by BAGPIPES's look through setup_as
KIT = ('ACOUSTIC', 'ALTO SAX', 'BASS CLARINET', 'BASS DRUM', 'BASSOON', 'CELLO', 'CLARINET', 'CONTRABASS',
       'ELECTRIC', 'EUPHONIUM', 'HORN', 'OBOE', 'ORGAN', 'PIANO', 'SOUSAPHONE', 'TENOR SAX', 'TROMBONE', 'TRUMPET',
       'TUBA')                                   # (FLUTE: the template's own scenarios)
LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)
SCENARIOS = (m3refs.default_scenarios('BAGPIPES', 'FLUTE')
             + [dict(name=f'with_{m3refs.slug(o)}', seeds=(1, 2, 3), opponent=o) for o in KIT]
             + [dict(name=f'with_{m3refs.slug(o)}_swapped', seeds=(1, 2, 3), opponent=o, swapped=True) for o in KIT]
             + [dict(name='copy_sweep', seeds=(1, 2, 3), opponent='EUPHONIUM', overrides=dict(LOW)),
                dict(name='copy_sweep_swapped', seeds=(1, 2, 3), opponent='EUPHONIUM', overrides=dict(LOW),
                     swapped=True)])

SPEC = dict(
    name='BAGPIPES',
    opponent='FLUTE',
    scenarios=SCENARIOS,
    origin=ORIGIN,
    # pp_bridge's BAGPIPES note is the chanter plus the scene's three drones held 9.77 s (its first event): three notes
    # (under G5 with the high-G grace, G5 with the high A, A5 with none) keep voice.json inside the folder's budget
    voice_notes=dict(pitches=(67, 79, 81), velocities=(0.9,), durations=(0.25,)),
)

EXPORTS = {'fighter_bagpipes': lambda: m3refs.fighter_refs(SPEC)}

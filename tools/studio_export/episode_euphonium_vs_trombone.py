"""episode_euphonium_vs_trombone: the kit episode ballfight_euphonium_vs_trombone (#48, under second_line in E harmonic
minor) as a pair gate (Template P, plan Task 52).

    python -m tools.studio_export episode_euphonium_vs_trombone        writes studio/reference/episodes/euphonium_vs_trombone/
    python -m tools.studio_export cache episodes/euphonium_vs_trombone regenerates the cache and checks the pins

The kit's recording of the config (seed 4604) is the theme folder's pinned cache copy
(themes/second_line/euphonium_vs_trombone.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it
out of the committed folder if those bytes ever change); the fight file is studio/fights/euphonium_vs_trombone.toml.
The config sets the pair rule swat_shells (Table S5: a live lance swats a falling shell, fight.py 1243-1271). What the
pair adds (plan Task 52):
- `swat_shell`: TROMBONE's live lance knocks EUPHONIUM's shell out of the air on its way down; the shell is marked
  done with `landed = 3` and its time `ts` (no `shell` event, no damage, no growth) and the look draws it tumbling
  away in sparks (the config's own fight swats three, at 30.5, 33.5 and 38.9 s);
- the copies: EUPHONIUM's IDENTITY CRISIS (`copy`) takes SAD TROMBONE (the ring `slow`, `hit/shockwave`), POWER
  SLIDE (a haste) and PEDAL TONE (the bell's `slam`, `hit/slam`), drawn by TROMBONE's look in amber and played in
  the euphonium's voice as the wah-wah (the config's own fight copies SAD TROMBONE at 28.5 s and POWER SLIDE at
  38.4 s after FIGHT!; the golden frames, soundtrack and checks prove them);
- both weapons and the other abilities: the lance (`thrust`, `hit/blade`), BELL MORTAR (`mortar`, `shell`,
  `hit/shell`), NOT A TUBA (`barrage`), MARCHING ORDERS and POWER SLIDE (hastes: their `ability` events).
In the plain and swapped fights IDENTITY CRISIS, NOT A TUBA and PEDAL TONE first come 25 s or more after FIGHT!, so
the scenarios add Template F's early levels and early low turn in both corners (as Task 46)."""
from . import m3refs

EARLY = dict(LV_DMG=m3refs.LV)                     # m3refs.default_scenarios' early_levels
EARLY_LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)    # ... and its early_low

SPEC = dict(
    config='ballfight_euphonium_vs_trombone',
    links=('thrust', 'hit/blade', 'swat_shell', 'mortar', 'shell', 'hit/shell', 'slow', 'hit/shockwave', 'slam',
           'hit/slam', 'ability/POWER SLIDE', 'ability/MARCHING ORDERS', 'barrage', 'copy'),
    weapon_hit={'EUPHONIUM': 'shell', 'TROMBONE': 'blade'},   # the `how` of each weapon's hits (fight.py 2753, 769)
    scenarios=[dict(name='plain', seeds=[1, 2, 3]), dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_levels', seeds=[1, 2, 3], overrides=EARLY),
               dict(name='early_levels_swapped', seeds=[1, 2, 3], swapped=True, overrides=EARLY),
               dict(name='early_low', seeds=[1, 2, 3], overrides=EARLY_LOW),
               dict(name='early_low_swapped', seeds=[1, 2, 3], swapped=True, overrides=EARLY_LOW)],
    recording_in_cache=True,                                    # ruling M3-R44
)

EXPORTS = {'episode_euphonium_vs_trombone': lambda: m3refs.episode_refs(SPEC)}

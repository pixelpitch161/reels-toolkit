"""episode_electric_vs_acoustic: the kit episode ballfight_electric_vs_acoustic (#44, under stadium_rock) as a pair
gate (Template P, plan Task 51).

    python -m tools.studio_export episode_electric_vs_acoustic        writes studio/reference/episodes/electric_vs_acoustic/
    python -m tools.studio_export cache episodes/electric_vs_acoustic regenerates the cache and checks the pins

The kit's recording of the config (seed 418) is the theme folder's pinned cache copy
(themes/stadium_rock/electric_vs_acoustic.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it
out of the committed folder if those bytes ever change); the fight file is studio/fights/electric_vs_acoustic.toml.
The config sets no pair rule. What the pair adds (plan Task 51):
- plugged and unplugged with the lead envelope: the theme's option `plugged` is the config's PLUGGED (ELECTRIC), and
  the band follows whoever leads; the soundtrack gate proves the master, the ACOUSTIC's chord SOLO included (the
  ACOUSTIC wins the config's fight and strums the chords of its solo, M3-T64);
- the smash against the rake: ELECTRIC's low ability smashes itself (`smash`, its splinters `splinter`,
  `splinter_end`, a landed wave `hit/smash`, a landed splinter `hit/splinter`) and lies in pieces, out of reach of the
  rake and its whip and sweep, until it snaps back (`rebuild`). The low ability first comes 35 to 51 s after FIGHT!
  in the plain fights, so a third scenario group, early_low (Template F's: levels and the low ability early), puts
  it inside the coverage window. Both weapons and the other abilities are covered too: the bolts of the riff
  (`riff`, `bolt`, `hit/bolt`), DISTORTION's `fork`, TAPPING's `tapping`, the rake (`rake`, `hit/rake`),
  FINGERPICK's `whip` (`hit/whip`) and BIG STRUM's `sweep` (`hit/sweep`)."""
from . import m3refs

EARLY_LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)       # m3refs.default_scenarios' early_low

SPEC = dict(
    config='ballfight_electric_vs_acoustic',
    links=('riff', 'bolt', 'hit/bolt', 'fork', 'tapping', 'rake', 'hit/rake', 'whip', 'hit/whip', 'sweep', 'hit/sweep',
           'smash', 'splinter', 'splinter_end', 'hit/smash', 'hit/splinter', 'rebuild'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], swapped=False, overrides=EARLY_LOW)],
    weapon_hit={'ELECTRIC': 'bolt', 'ACOUSTIC': 'rake'},   # the `how` of each weapon's hits (fight.py 3917, 624)
    recording_in_cache=True,                                # ruling M3-R44
)

EXPORTS = {'episode_electric_vs_acoustic': lambda: m3refs.episode_refs(SPEC)}

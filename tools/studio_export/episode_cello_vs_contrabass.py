"""episode_cello_vs_contrabass: the kit episode ballfight_cello_vs_contrabass (#47, under dark_tango) as a pair gate
(Template P, plan Task 50).

    python -m tools.studio_export episode_cello_vs_contrabass        writes studio/reference/episodes/cello_vs_contrabass/
    python -m tools.studio_export cache episodes/cello_vs_contrabass regenerates the cache and checks the pins

The kit's recording of the config (seed 2455) is the theme folder's pinned cache copy
(themes/dark_tango/cello_vs_contrabass.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it out
of the committed folder if those bytes ever change); the fight file is studio/fights/cello_vs_contrabass.toml. The
config sets no pair rule. What the pair adds (plan Task 50, Table S5 "a walking bass slips the grab, a slap breaks
it"):
- a walker slips the grab: THE EMBRACE's rush reaching a CONTRABASS that is walking ends as `grab` with how 2
  (fight.py 4147); the early_low fights (Template F's: levels and the low ability early) have one in each of their
  three seeds about 6 s after FIGHT!, and the prefix gate holds them to the kit's to the K.O.;
- a slap breaks it: SLAP BASS fired while held lets go of the grab (`release` with why 'slap', then `slap` with
  broke 1, fight.py 4211); the plain fights take three more seeds (4 to 6) so that a break (plain s6) falls inside
  the coverage window, where it is the first `release`;
- the tango stops from the cello's rope slams: the CELLO part's `stops()` are its rope slams, and the dark tango band
  stops on each; the soundtrack and stems gates prove the master and every bus of the config's own fight.
Both fighters' kinds and abilities are covered too: the grapple (`rush`, `grab`, `hold`, `throw`, `ropeslam`),
VIBRATO, PRELUDE and SWAN SONG, the walk (`step`, `step_land`), SLAP BASS and THE ELEPHANT (`stampede`,
`stampede_stomp`, `trample`, `stampede_end`). PRELUDE first comes 20.3 s after FIGHT! in the plain fights and THE
ELEPHANT and SWAN SONG over 33 s, so the early_low group puts them inside the window."""
from . import m3refs

EARLY_LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)       # m3refs.default_scenarios' early_low

SPEC = dict(
    config='ballfight_cello_vs_contrabass',
    links=('rush', 'grab', 'hit/grab', 'hold', 'throw', 'hit/throw', 'ropeslam', 'hit/ropeslam', 'release',
           'vibrato', 'hit/vibrato', 'prelude', 'prelude_note', 'hit/prelude', 'swan', 'swan_land', 'hit/swan',
           'step', 'step_land', 'hit/step', 'slap', 'hit/slap', 'stampede', 'stampede_stomp', 'hit/stampede',
           'trample', 'hit/trample', 'stampede_end'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3, 4, 5, 6], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], swapped=False, overrides=EARLY_LOW)],
    weapon_hit={'CELLO': 'grab', 'CONTRABASS': 'step'},   # the `how` of each weapon's hits (fight.py 4154, 4037)
    recording_in_cache=True,                              # ruling M3-R44
)

EXPORTS = {'episode_cello_vs_contrabass': lambda: m3refs.episode_refs(SPEC)}

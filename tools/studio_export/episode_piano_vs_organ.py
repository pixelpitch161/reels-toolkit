"""episode_piano_vs_organ: the kit episode ballfight_piano_vs_organ (#45, under prog_duel) as a pair gate (Template P,
plan Task 56).

    python -m tools.studio_export episode_piano_vs_organ        writes studio/reference/episodes/piano_vs_organ/
    python -m tools.studio_export cache episodes/piano_vs_organ regenerates the cache and checks the pins

The kit's recording of the config (seed 638) is the theme folder's pinned cache copy
(themes/prog_duel/piano_vs_organ.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it out of
the committed folder if those bytes ever change); the fight file is studio/fights/piano_vs_organ.toml. The config sets
no pair rule. What the pair adds (plan Task 56):
- keys against the pipe lines and the shield: PIANO ROLL's rows fire keys (`roll`, a landed key `hit/key`; SUSTAIN's
  hanging rows `sustain`; 88 KEYS running off the spool `runoff`) at the ORGAN, whose weapon raises rows of pipes from
  the floor (`line`, a landed row `hit/pipe`) and whose CATHEDRAL shields it: a key on the shield is `block` (the
  ORGAN is the only one with a shield here, and every PIANO hit is a key, so each `block` is a key on CATHEDRAL);
  TOCCATA's `slam` (`hit/slam`), PULL ALL STOPS and GLISSANDO are covered by their ability items;
- the roll from the config's own line: the rule `roll_rows` is made from the PIANO's LINE of the config (one row per
  8th, a rest is no row: `rest` in the fight file's music.lines); the recording gate compares the rules, roll_rows
  included, and the fight to the K.O.
The low abilities (88 KEYS, TOCCATA) first come about 40 s after FIGHT! in the plain fights and PULL ALL STOPS and
GLISSANDO after 20 s, so a third scenario group, early_low (Template F's: levels and the low ability early), puts
them inside the coverage window."""
from . import m3refs

EARLY_LOW = dict(LV_DMG=m3refs.LV, LOW_HP=92.0)       # m3refs.default_scenarios' early_low

SPEC = dict(
    config='ballfight_piano_vs_organ',
    links=('roll', 'hit/key', 'sustain', 'runoff', 'line', 'hit/pipe', 'block', 'slam', 'hit/slam',
           'ability/SUSTAIN', 'ability/GLISSANDO', 'ability/88 KEYS', 'ability/CATHEDRAL', 'ability/PULL ALL STOPS',
           'ability/TOCCATA'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], swapped=False, overrides=EARLY_LOW)],
    weapon_hit={'PIANO': 'key', 'ORGAN': 'pipe'},   # the `how` of each weapon's hits (fight.py 5178, 1411)
    recording_in_cache=True,                         # ruling M3-R44
)

EXPORTS = {'episode_piano_vs_organ': lambda: m3refs.episode_refs(SPEC)}

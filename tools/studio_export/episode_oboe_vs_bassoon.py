"""episode_oboe_vs_bassoon: the kit episode ballfight_oboe_vs_bassoon (#52, under heist) as a pair gate (Template P,
plan Task 55).

    python -m tools.studio_export episode_oboe_vs_bassoon        writes studio/reference/episodes/oboe_vs_bassoon/
    python -m tools.studio_export cache episodes/oboe_vs_bassoon regenerates the cache and checks the pins

The kit's recording of the config (seed 954) is the theme folder's copy (themes/heist/oboe_vs_bassoon.bfrec: pointed
at, never copied; recording_in_cache keeps it out of the committed folder if those bytes ever change, ruling M3-R44);
the fight file is studio/fights/oboe_vs_bassoon.toml. The config sets no pair rule; its own links (plan Task 55,
Table S5 "a hook spins a hopper, which follows up") are the two fighters' kinds meeting: OBOE's TUNING A beam
(`tune`) knocks the hopper OUT OF TUNE (`detune`) while BASSOON hops (`hop`, a landed stomp as `hit/stomp`); OBOE's
DOUBLE REED snaps at it (`snip_open`, `snip`, a landed one as `hit/snip`), and BOCAL HOOK lands on the oboe
(`hook_open`, `hook`, a landed one as `hit/hook`: it spins the oboe round, and the bassoon's next hop follows straight
behind it). The early_low scenario (the fighter template's: abilities below 92 HP) brings BOCAL HOOK and MOUNTAIN
KING (`mountain`) inside the agreed window. heist has no `boom` (its hooks are [Hit]): neither fighter's part calls
it, which the soundtrack proves."""
from . import m3refs

SPEC = dict(
    config='ballfight_oboe_vs_bassoon',
    links=('tune', 'detune', 'hop', 'hit/stomp', 'snip_open', 'snip', 'hit/snip', 'hook_open', 'hook', 'hit/hook',
           'mountain'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit={'OBOE': 'sting', 'BASSOON': 'stomp'},  # the `how` of each weapon's hits (fight.py 2494, 3358)
    recording_in_cache=True,                           # ruling M3-R44
)

EXPORTS = {'episode_oboe_vs_bassoon': lambda: m3refs.episode_refs(SPEC)}

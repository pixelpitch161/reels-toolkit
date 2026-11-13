"""episode_bassclarinet_vs_bassoon: the kit episode ballfight_bassclarinet_vs_bassoon (#55, under horror_trap) as a
pair gate (Template P, plan Task 48).

    python -m tools.studio_export episode_bassclarinet_vs_bassoon        writes studio/reference/episodes/bassclarinet_vs_bassoon/
    python -m tools.studio_export cache episodes/bassclarinet_vs_bassoon regenerates the cache and checks the pins

The kit's recording of the config (seed 4) is the theme folder's pinned cache copy
(themes/horror_trap/bassclarinet_vs_bassoon.bfrec, in the cache since plan m4 Task 1: pointed at, never copied;
recording_in_cache keeps it out of the committed folder if those bytes ever change, ruling M3-R44);
the fight file is studio/fights/bassclarinet_vs_bassoon.toml. The config sets no pair rule; its own links
(plan Task 48, Table S5 "a hook spins a hopper, which follows up") are the two fighters' kinds meeting: BASS
CLARINET's endpin anchor (`anchor`, `pull`, a landed whirl as `hit/anchor`) against BASSOON's hops (`hop`, a landed
stomp as `hit/stomp`), and BOCAL HOOK on the hopper's enemy (`hook_open`, `hook`, a landed one as `hit/hook`: it spins
the bass clarinet round, and the bassoon's next hop follows straight behind it). The early_low scenario (the
fighter template's: LIGHTS OUT below 92 HP) adds LIGHTS OUT and the eruption on the hopper (`lights_out`, `erupt`,
`hit/erupt`). The bed's `erupt`, which the BASS CLARINET part calls at the eruption, is proven by the soundtrack: the
config's own fight has LIGHTS OUT at 38.6 s."""
from . import m3refs

SPEC = dict(
    config='ballfight_bassclarinet_vs_bassoon',
    links=('anchor', 'pull', 'hit/anchor', 'hop', 'hit/stomp', 'hook_open', 'hook', 'hit/hook', 'lights_out', 'erupt',
           'hit/erupt'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3], swapped=False),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit={'BASS CLARINET': 'anchor', 'BASSOON': 'stomp'},  # the `how` of each weapon's hits (fight.py 2649, 3358)
    recording_in_cache=True,                                      # ruling M3-R44
)

EXPORTS = {'episode_bassclarinet_vs_bassoon': lambda: m3refs.episode_refs(SPEC)}

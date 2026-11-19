"""episode_clarinet_vs_oboe: the kit episode examples/clarinet_vs_oboe (BALLFIGHT #11, under baroque_storm) as a pair
gate (Template P, plan Task 42).

    python -m tools.studio_export episode_clarinet_vs_oboe        writes studio/reference/episodes/clarinet_vs_oboe/
    python -m tools.studio_export cache episodes/clarinet_vs_oboe regenerates the cache and checks the pins

The kit's recording of the config (seed 654) is the theme folder's pinned cache copy
(themes/baroque_storm/clarinet_vs_oboe.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/clarinet_vs_oboe.toml. The config sets no pair rule; its own links (plan Task 42) are the two
fighters' kinds meeting: CLARINET's SQUEAK stuns the tuner (`stun`, `hit/stun`) while OBOE's TUNING A beam
(`tune`) knocks the jumper OUT OF TUNE (`detune`), and OBOE's DOUBLE REED opens and snaps at a fighter that
teleports (`jump`, `snip_open`, `snip`, a landed one as `hit/snip`)."""
from . import m3refs

SPEC = dict(
    config='examples/clarinet_vs_oboe',
    links=('stun', 'hit/stun', 'tune', 'detune', 'jump', 'snip_open', 'snip', 'hit/snip'),
    weapon_hit=dict(CLARINET='cut', OBOE='sting'),       # the `how` of each weapon's hits (fight.py 2390, 2494)
)

EXPORTS = {'episode_clarinet_vs_oboe': lambda: m3refs.episode_refs(SPEC)}

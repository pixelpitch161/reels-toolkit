"""episode_flute_vs_clarinet: the kit episode ballfight_flute_vs_clarinet (#49, under bossa_noir) as a pair gate
(Template P, plan Task 53).

    python -m tools.studio_export episode_flute_vs_clarinet        writes studio/reference/episodes/flute_vs_clarinet/
    python -m tools.studio_export cache episodes/flute_vs_clarinet regenerates the cache and checks the pins

The kit's recording of the config (seed 3864) is the theme folder's pinned cache copy
(themes/bossa_noir/flute_vs_clarinet.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/flute_vs_clarinet.toml. The config sets no pair rule; its own links (plan Task 53) are the two
fighters' kinds meeting: FLUTE's SNAKE CHARMER drops baskets (`basket`) whose cobras lunge (`strike`) and bite
(`hit/fang`), while CLARINET's TWELFTH JUMP vanishes and lands past the enemy and its baskets (`jump`, a cut on the
way as `hit/cut`), and its SQUEAK stuns the cobra charmer (`stun`, `hit/stun`): only CLARINET stuns here, so every
`stun` is FLUTE stunned."""
from . import m3refs

SPEC = dict(
    config='ballfight_flute_vs_clarinet',
    links=('basket', 'strike', 'hit/fang', 'jump', 'hit/cut', 'stun', 'hit/stun'),
    weapon_hit=dict(FLUTE='fang', CLARINET='cut'),          # the `how` of each weapon's hits (fight.py 1205, 2390)
)

EXPORTS = {'episode_flute_vs_clarinet': lambda: m3refs.episode_refs(SPEC)}

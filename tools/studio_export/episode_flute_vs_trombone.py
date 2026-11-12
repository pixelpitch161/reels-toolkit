"""episode_flute_vs_trombone: milestone 1's example episode as a pair gate (Template P), the proof of episode_refs.

    python -m tools.studio_export episode_flute_vs_trombone        writes studio/reference/episodes/flute_vs_trombone/
    python -m tools.studio_export cache episodes/flute_vs_trombone regenerates the cache and checks the pins

The kit's recording of the config is milestone 1's fights/flute_vs_trombone/ep392.bfrec (pointed at, not copied);
the fight file is studio/fights/flute_vs_trombone.toml. The pair's own link (plan Table S5) is the series' rule
`swat_cobras`: the lance swats a cobra (`swat`)."""
from . import m3refs

SPEC = dict(
    config='examples/flute_vs_trombone',
    links=('swat',),
    weapon_hit=dict(FLUTE='fang', TROMBONE='blade'),        # frames.py's WEAPON_HIT for the striker and the blade
)

EXPORTS = {'episode_flute_vs_trombone': lambda: m3refs.episode_refs(SPEC)}

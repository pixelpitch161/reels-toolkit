"""episode_altosax_vs_tenorsax: the kit episode ballfight_altosax_vs_tenorsax (#51, under dark_disco) as a pair gate
(Template P, plan Task 47).

    python -m tools.studio_export episode_altosax_vs_tenorsax        writes studio/reference/episodes/altosax_vs_tenorsax/
    python -m tools.studio_export cache episodes/altosax_vs_tenorsax regenerates the cache and checks the pins

The kit's recording of the config (seed 621) is the theme folder's pinned cache copy
(themes/dark_disco/altosax_vs_tenorsax.bfrec, ruling M3-R44: pointed at, never copied; recording_in_cache keeps it
out of the committed folder if those bytes ever change); the fight file is studio/fights/altosax_vs_tenorsax.toml.
The config sets no pair rule; its own links (plan Task 47, spotlight against the crowd) are the two fighters' kinds
meeting: ALTO SAX's SPOTLIGHT SOLO sweeps a light across the crowd pleaser's path (`spot`, `spot_end`) and burns it
(`burn`, a landed one as `hit/burn`), while TENOR SAX's crowd poses (`pose`) and throws from the stands at the
light-wielder (`throw`, `splat`, a landed one as `hit/throw`), WALK THE BAR struts its spotlit runway into the alto
(`runway`, `strut_bump`, `hit/strut`) and SUBTONE hushes the crowd (`hush`). The theme has no `boom`; neither part
calls it, so the soundtrack gate proves the pair plays under it."""
from . import m3refs

SPEC = dict(
    config='ballfight_altosax_vs_tenorsax',
    links=('spot', 'burn', 'hit/burn', 'spot_end', 'pose', 'throw', 'splat', 'hit/throw', 'runway', 'strut_bump',
           'hit/strut', 'hush'),
    weapon_hit={'ALTO SAX': 'burn', 'TENOR SAX': 'throw'},   # the `how` of each weapon's hits (fight.py 4415, 4631)
    recording_in_cache=True,                                  # ruling M3-R44
)

EXPORTS = {'episode_altosax_vs_tenorsax': lambda: m3refs.episode_refs(SPEC)}

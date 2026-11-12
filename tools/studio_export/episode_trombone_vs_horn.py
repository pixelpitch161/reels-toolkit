"""episode_trombone_vs_horn: the kit episode examples/trombone_vs_horn (BALLFIGHT #8, under brass_trailer) as a pair
gate (Template P, plan Task 43).

    python -m tools.studio_export episode_trombone_vs_horn        writes studio/reference/episodes/trombone_vs_horn/
    python -m tools.studio_export cache episodes/trombone_vs_horn regenerates the cache and checks the pins

The kit's recording of the config (seed 140) is the theme folder's pinned cache copy
(themes/brass_trailer/trombone_vs_horn.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/trombone_vs_horn.toml. The config sets no pair rule; its own links (plan Task 43) are the two
fighters' kinds meeting: HORN's STOPPED HORN shield (`ability/STOPPED HORN`) turns TROMBONE's lance away
(`block`), and HORN's FINAL FLICK (below its low HP: the `early_low` scenarios raise the low HP so it fires early)
winds (`flick_wind`), flicks the enemy (`flick`, a landed one as `hit/flick`) and hurts it again when it hits the
ropes (`hit/flick_rope`)."""
from . import m3refs

SPEC = dict(
    config='examples/trombone_vs_horn',
    links=('ability/STOPPED HORN', 'block', 'ability/FINAL FLICK', 'flick_wind', 'flick', 'hit/flick',
           'hit/flick_rope'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3]),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit=dict(TROMBONE='blade', HORN='echo'),      # the `how` of each weapon's hits (fight.py 766/337, 2273)
)

EXPORTS = {'episode_trombone_vs_horn': lambda: m3refs.episode_refs(SPEC)}

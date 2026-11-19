"""episode_horn_vs_trombone_rematch: the kit episode ballfight_horn_vs_trombone_rematch (#54, THE REMATCH, under
brass_trailer's sequel) as a pair gate (Template P, plan Task 44).

    python -m tools.studio_export episode_horn_vs_trombone_rematch        writes studio/reference/episodes/horn_vs_trombone_rematch/
    python -m tools.studio_export cache episodes/horn_vs_trombone_rematch regenerates the cache and checks the pins

The kit's recording of the config (seed 630) is the theme folder's pinned cache copy
(themes/brass_trailer/horn_vs_trombone_rematch.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/horn_vs_trombone_rematch.toml. What this config adds beyond the template: the sequel bed
(`TRAILER = dict(sequel=True)`, the fight file's `music.theme_options.sequel`: its pad, braams and subs every second
bar, the taiko ensemble, a darker hall) and the card's `MIDDLE` (REMATCH along the divider). The config sets no pair
rule; its own links are the two fighters' kinds meeting, HORN now on the left: HORN's STOPPED HORN shield
(`ability/STOPPED HORN`) turns TROMBONE's lance away (`block`), and HORN's FINAL FLICK (below its low HP: the
`early_low` scenarios raise the low HP so it fires early) winds (`flick_wind`), flicks the enemy (`flick`, a landed
one as `hit/flick`) and hurts it again when it hits the ropes (`hit/flick_rope`)."""
from . import m3refs

SPEC = dict(
    config='ballfight_horn_vs_trombone_rematch',
    links=('ability/STOPPED HORN', 'block', 'ability/FINAL FLICK', 'flick_wind', 'flick', 'hit/flick',
           'hit/flick_rope'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3]),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit=dict(HORN='echo', TROMBONE='blade'),      # the `how` of each weapon's hits (fight.py 766/337, 2273)
)

EXPORTS = {'episode_horn_vs_trombone_rematch': lambda: m3refs.episode_refs(SPEC)}

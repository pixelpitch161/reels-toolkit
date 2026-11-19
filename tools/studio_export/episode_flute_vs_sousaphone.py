"""episode_flute_vs_sousaphone: the kit episode ballfight_flute_vs_sousaphone (#53, under dark_drumline) as a pair
gate (Template P, plan Task 54).

    python -m tools.studio_export episode_flute_vs_sousaphone        writes studio/reference/episodes/flute_vs_sousaphone/
    python -m tools.studio_export cache episodes/flute_vs_sousaphone regenerates the cache and checks the pins

The kit's recording of the config (seed 146) is the theme folder's pinned cache copy
(themes/dark_drumline/flute_vs_sousaphone.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/flute_vs_sousaphone.toml. The config sets no pair rule and no theme option (the drumline as built).
What it adds beyond the template (plan Task 54, Table S5): the march tramples the enemy's baskets. FLUTE's SNAKE
CHARMER drops baskets (`basket`) whose cobras lunge (`strike`) and bite (`hit/fang`); SOUSAPHONE's DRILL SET marches
a straight path (`drill`, its footfalls `drill_step`), trampling the flute on the way (`drill_trample`, a landed one
as `hit/trample`) and its baskets (`trample_basket`, fight.py `_trample_baskets`), then halts and blasts its cone bell
(`blast`, `hit/blast`). COMPANY FRONT's wall of ghost sousaphones (`company_front`, its steps `front_step`, a hit as
`hit/front`) tramples the baskets on its line too (the wall's own basket loop): at damage scale 0.7 it comes late,
so the `early_low` scenarios (levels early, the low HP raised) bring it inside the window, as Tasks 44 and 46 did."""
from . import m3refs

SPEC = dict(
    config='ballfight_flute_vs_sousaphone',
    links=('basket', 'strike', 'hit/fang', 'drill', 'drill_step', 'drill_trample', 'hit/trample', 'trample_basket',
           'blast', 'hit/blast', 'company_front', 'front_step', 'hit/front'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3]),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit=dict(FLUTE='fang', SOUSAPHONE='blast'),      # the `how` of each weapon's hits (fight.py 1205, 4925)
)

EXPORTS = {'episode_flute_vs_sousaphone': lambda: m3refs.episode_refs(SPEC)}

"""episode_trumpet_vs_trombone: the kit episode ballfight_trumpet_vs_trombone (#50, under the big-band mambo in G
harmonic minor) as a pair gate (Template P, plan Task 58).

    python -m tools.studio_export episode_trumpet_vs_trombone        writes studio/reference/episodes/trumpet_vs_trombone/
    python -m tools.studio_export cache episodes/trumpet_vs_trombone regenerates the cache and checks the pins

The kit's recording of the config (seed 1001) is the theme folder's pinned cache copy
(themes/mambo/trumpet_vs_trombone.bfrec, ruling M3-R44: pointed at, never copied); the fight file is
studio/fights/trumpet_vs_trombone.toml. The config sets no pair rule; what it adds beyond the template is the lance
against the combo and the TROMBONE's `boom` over the mambo. The TRUMPET's VALVE COMBO winds up (`combo`) and fires
its pistons (`punch`, a landed one as `hit/piston`); the TROMBONE's lance thrusts (`thrust`) and lands
(`hit/blade`). Both lows fire below their low HP (the `early_low` scenarios raise it so they fire early): the
TRUMPET's HIGH C charges (`ability/HIGH C`) and blasts (`beam`, a landed one as `hit/beam`); the TROMBONE's PEDAL
TONE (`ability/PEDAL TONE`) stomps its bell (`slam`), the event its part plays on the mambo's `bed.boom`
(kit/fighters/trombone.py 551-553; the config's own fight has both lows, so the soundtrack gate hears it)."""
from . import m3refs

SPEC = dict(
    config='ballfight_trumpet_vs_trombone',
    links=('combo', 'punch', 'hit/piston', 'thrust', 'hit/blade', 'ability/HIGH C', 'beam', 'hit/beam',
           'ability/PEDAL TONE', 'slam'),
    scenarios=[dict(name='plain', seeds=[1, 2, 3]),
               dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    weapon_hit=dict(TRUMPET='piston', TROMBONE='blade'),  # the `how` of each weapon's hits (fight.py 961, 769)
    recording_in_cache=True,                              # ruling M3-R44 (the theme's cached copy is pointed at)
)

EXPORTS = {'episode_trumpet_vs_trombone': lambda: m3refs.episode_refs(SPEC)}

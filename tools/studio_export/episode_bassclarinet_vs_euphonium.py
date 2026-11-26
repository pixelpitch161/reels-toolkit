"""episode_bassclarinet_vs_euphonium: the kit episode examples/bassclarinet_vs_euphonium (BALLFIGHT #12, under big_top
in C minor) as a pair gate (Template P, plan Task 46).

    python -m tools.studio_export episode_bassclarinet_vs_euphonium   writes the folder
    python -m tools.studio_export cache episodes/bassclarinet_vs_euphonium   regenerates the cache, checks the pins

big_top was ported in milestone 1, so no theme folder holds this config's recording: the gate exports the kit's
recording of the config (seed 1269) itself as run.bfrec, kept in the pinned cache with its committed SHA-256 (ruling
M3-R44: recording_in_cache=True). The fight file is studio/fights/bassclarinet_vs_euphonium.toml. The config sets no
pair rule; its own links (plan Task 46) are BASS CLARINET's ENDPIN ANCHOR (the peg `anchor`, the whirl's smash
`hit/anchor`, the release `pull`), MULTIPHONICS (`twin`), DOUBLE REED's snip (`snip_open`, `snip`) and LIGHTS OUT
(`lights_out`, `erupt`) against EUPHONIUM's BELL MORTAR (`mortar`, the shells `shell` and `hit/shell`, aimed round
the whirl while the bass clarinet is anchored), NOT A TUBA (`barrage`) and IDENTITY CRISIS (`copy`: the copies of
snip, twin, LIGHTS OUT and the whirl itself, drawn by the bass clarinet's look in amber)."""
from . import m3refs

SPEC = dict(
    config='examples/bassclarinet_vs_euphonium',
    links=('anchor', 'pull', 'hit/anchor', 'twin', 'snip_open', 'snip', 'lights_out', 'erupt', 'mortar', 'shell',
           'hit/shell', 'barrage', 'copy'),
    # the `how` of each weapon's hits (fight.py 2649, 2753)
    weapon_hit={'BASS CLARINET': 'anchor', 'EUPHONIUM': 'shell'},
    # plain and swapped, seeds 1 to 3, as the template; at dmg_scale 0.68 the level-ups come late, so LIGHTS OUT,
    # NOT A TUBA and IDENTITY CRISIS are reached within 20 s of FIGHT! only with the early levels (LV2 at 2 dealt, LV3
    # at 5) and the early low turn (LOW_HP 92) of Template F's scenarios, in both corners
    scenarios=[dict(name='plain', seeds=[1, 2, 3]), dict(name='swapped', seeds=[1, 2, 3], swapped=True),
               dict(name='early_levels', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV)),
               dict(name='early_levels_swapped', seeds=[1, 2, 3], swapped=True, overrides=dict(LV_DMG=m3refs.LV)),
               dict(name='early_low', seeds=[1, 2, 3], overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0)),
               dict(name='early_low_swapped', seeds=[1, 2, 3], swapped=True,
                    overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0))],
    recording_in_cache=True,  # ruling M3-R44: the pair's own kit recording in the cache
)

EXPORTS = {'episode_bassclarinet_vs_euphonium': lambda: m3refs.episode_refs(SPEC)}

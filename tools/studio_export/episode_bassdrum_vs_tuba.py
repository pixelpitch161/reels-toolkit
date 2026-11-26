"""episode_bassdrum_vs_tuba: the kit episode ballfight_bassdrum_vs_tuba (#56, under dark_drumline) as a pair gate
(Template P, plan Task 49).

    python -m tools.studio_export episode_bassdrum_vs_tuba        writes studio/reference/episodes/bassdrum_vs_tuba/
    python -m tools.studio_export cache episodes/bassdrum_vs_tuba regenerates the cache and checks the pins

The kit's recording of the config (seed 132) is the theme folder's pinned cache copy
(themes/dark_drumline/bassdrum_vs_tuba.bfrec: pointed at, never copied; ruling M3-R44); the fight file is
studio/fights/bassdrum_vs_tuba.toml. The config sets the pair rule `swallow_rollers` (RULES, fight.py 1100): the
TUBA's bell can gulp the BASS DRUM mid-roll, and the gulp ends the roll (`_end_roll(k, 'held')`, fight.py 5398, the
event `roll_off k 'held'`). Its links (plan Task 49, Table S5) are the two fighters' kinds meeting: STEAMROLLER's
rolls (`roll`, a landed roll as `hit/roll`) against OOM-PAH SWALLOW (`gulp`, `hit/gulp`, `pah`, a spat drum as
`hit/spit`), and the gulp that ends a roll (`roll_off`). The theme options (DRUMLINE = dict(bass_drums=False,
after_ko=True)), the MIX (`solo`) and BASS DRUM's hymn doubled in the bed's pit (timp and marimba, bass_drum.py 566
to 582, at 1812 OVERTURE) are proven by the soundtrack and its stems."""
from . import m3refs

SPEC = dict(
    config='ballfight_bassdrum_vs_tuba',
    links=('roll', 'hit/roll', 'gulp', 'hit/gulp', 'roll_off', 'pah', 'hit/spit'),
    weapon_hit={'BASS DRUM': 'roll', 'TUBA': 'gulp'},  # the `how` of each weapon's hits (fight.py 1107, 5423)
    recording_in_cache=True,                            # ruling M3-R44
)

EXPORTS = {'episode_bassdrum_vs_tuba': lambda: m3refs.episode_refs(SPEC)}

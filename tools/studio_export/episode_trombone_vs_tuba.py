"""episode_trombone_vs_tuba: the kit episode examples/trombone_vs_tuba (BALLFIGHT #10, under big_top in F minor) as a
pair gate (Template P, plan Task 45).

    python -m tools.studio_export episode_trombone_vs_tuba        writes studio/reference/episodes/trombone_vs_tuba/
    python -m tools.studio_export cache episodes/trombone_vs_tuba regenerates the cache and checks the pins

big_top was ported in milestone 1, so no theme folder holds this config's recording: the gate exports the kit's
recording of the config (seed 358) itself as run.bfrec, kept in the pinned cache with its committed SHA-256 (ruling
M3-R44: recording_in_cache=True). The fight file is studio/fights/trombone_vs_tuba.toml. The config sets no pair
rule (swallow_rollers is BASS DRUM's against TUBA only); its own links (plan Task 45) are the lance against a
swallower: TROMBONE's lance thrusts (`thrust`) and lands (`hit/blade`) or meets BRASS WALL (`block`), while TUBA's
bell inhales (`oom`), swallows the trombone (`gulp`, `hit/gulp`), spits it out (`pah`, `hit/spit`) and fires it
into the ropes as a cannonball (`hit/cannon`)."""
from . import m3refs

SPEC = dict(
    config='examples/trombone_vs_tuba',
    links=('thrust', 'hit/blade', 'block', 'oom', 'gulp', 'hit/gulp', 'pah', 'hit/spit', 'hit/cannon'),
    weapon_hit=dict(TROMBONE='blade', TUBA='gulp'),      # the `how` of each weapon's hits (fight.py 769, 1107)
    recording_in_cache=True,                             # ruling M3-R44: the pair's own kit recording in the cache
)

EXPORTS = {'episode_trombone_vs_tuba': lambda: m3refs.episode_refs(SPEC)}

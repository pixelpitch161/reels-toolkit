"""fighter_cello: CELLO's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's base config (the big top of the
milestone 1 example). One departure: the reference handicap (ruling M3-R34). At the neutral (1.0, 1.0) the kit gave
CELLO none of 60 fights from the left corner and none of 60 from the right (FLUTE won all 120), so no `left` fight
(CELLO winning with all three abilities) could be chosen. A trial with the kit gave CELLO 42 of 60 from the left
corner and 24 of 60 from the right at (2.0, 0.8) for (CELLO, FLUTE), so the stronger side wins a real share; that is
the handicap of every fight of this folder.
    python -m tools.studio_export fighter_cello          writes studio/reference/fighters/cello/ and its cache
    python -m tools.studio_export cache fighters/cello   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='CELLO',
    opponent='FLUTE',
    fights=dict(handicap=(2.0, 0.8)),            # the reference handicap, CELLO left (ruling M3-R34)
    scenarios=m3refs.default_scenarios('CELLO', 'FLUTE') + [
        # SWAN SONG taking off while THE EMBRACE holds FLUTE lets go (`release`, 'swan'): in no default scenario
        # fight within 20 s of FIGHT!; these early_low seeds have it 4.0, 7.2 and 3.6 s in
        dict(name='swan_release', seeds=(29, 74, 144), overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0)),
    ],
    left_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)

EXPORTS = {'fighter_cello': lambda: m3refs.fighter_refs(SPEC)}

"""fighter_electric: ELECTRIC's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the big top config of the milestone 1 example. One
departure: the reference handicap (ruling M3-R34). At the neutral (1.0, 1.0) ELECTRIC wins almost nothing: a trial
of 40 seeds a corner with the kit gave it 1 of 39 decided fights from the left corner and 1 of 38 from the right.
Trials of 60 to 80 seeds a corner gave ELECTRIC about 59 % at (1.5, 1.0) (ELECTRIC's hits x 1.5), about 59 % at 1.6,
about 72 % at 1.7 (62 of 80 from the left, 53 of 80 from the right) and about 95 % at 2.0, so (1.7, 1.0) gives the
stronger side 60 to 85 %. Every other key is the template's default.

    python -m tools.studio_export fighter_electric          writes studio/reference/fighters/electric/ and its cache
    python -m tools.studio_export cache fighters/electric   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(name='ELECTRIC', opponent='FLUTE', fights=dict(handicap=(1.7, 1.0)), left_in_cache=True)

EXPORTS = {'fighter_electric': lambda: m3refs.fighter_refs(SPEC)}

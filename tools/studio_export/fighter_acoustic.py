"""fighter_acoustic: ACOUSTIC's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the big top config of the milestone 1 example. At the
neutral handicap ACOUSTIC never wins: a trial of 80 seeds a corner with the kit gave it 0 of 63 decided fights from
the left corner and 0 of 70 from the right, so no left fight exists and the statistics would prove little. Ruling
M3-R34 asks for a reference handicap that gives the stronger side 60 to 85 %: at (1.7, 1.0) (ACOUSTIC's hits x 1.7)
a trial of 60 seeds a corner gave ACOUSTIC 44 of 59 decided fights from the left and 39 of 58 from the right (about
71 %); at 1.5 it was about 48 %, at 2.0 about 89 %. Every other key is the template's default.

    python -m tools.studio_export fighter_acoustic          writes studio/reference/fighters/acoustic/ and its cache
    python -m tools.studio_export cache fighters/acoustic   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(name='ACOUSTIC', opponent='FLUTE', fights=dict(handicap=(1.7, 1.0)))

EXPORTS = {'fighter_acoustic': lambda: m3refs.fighter_refs(SPEC)}

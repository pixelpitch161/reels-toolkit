"""fighter_oboe: OBOE's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F; everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example at the neutral handicap, with the series' pair rules.

One fixed seed: at the neutral handicap OBOE wins 99.9 % of the 2,000 statistics fights, and FLUTE wins none of
seeds 1 to 500 from the left corner, so the scan finds no `right` fight (FLUTE left and winning: OBOE's shards
fly). Seed 1277 is the lowest seed at which FLUTE wins from the left (seeds 1 to 1276 were all tried; the next is
2283), so `right` is fixed to it. The handicap stays neutral.

    python -m tools.studio_export fighter_oboe          writes studio/reference/fighters/oboe/ and its cache
    python -m tools.studio_export cache fighters/oboe   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='OBOE',
    opponent='FLUTE',
    fights=dict(right=1277),                     # the lowest seed FLUTE wins from the left (none in 1..500)
)

EXPORTS = {'fighter_oboe': lambda: m3refs.fighter_refs(SPEC)}

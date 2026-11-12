"""fighter_clarinet: CLARINET's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F. Everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example.

One departure from the template (ruling M3-R33): the handicap is the kit's own for this pair
(`ballfight_flute_vs_clarinet`'s DMG_SIDE: 5.3 for FLUTE, 0.36 for CLARINET), not (1.0, 1.0). At (1.0, 1.0) CLARINET
won all 2,000 statistics fights and its RHAPSODY (below 25 HP) fired in none it won, so no `left` fight (CLARINET
winning with all three abilities) and no `right` fight (CLARINET losing) could be chosen.

    python -m tools.studio_export fighter_clarinet          writes studio/reference/fighters/clarinet/ and its cache
    python -m tools.studio_export cache fighters/clarinet   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='CLARINET',
    opponent='FLUTE',
    fights=dict(handicap=(0.36, 5.3)),           # the kit's frozen handicap for this pair, CLARINET left
)

EXPORTS = {'fighter_clarinet': lambda: m3refs.fighter_refs(SPEC)}

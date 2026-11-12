"""fighter_tenor_sax: TENOR SAX's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, and everything is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example at the neutral handicap. A trial of 150 seeds a corner
with the kit gave TENOR SAX 124 wins from the left corner and 118 from the right (about four in five), inside the
band ruling M3-R34 asks for, so no reference handicap is needed and both look fights are the framework's own picks.

    python -m tools.studio_export fighter_tenor_sax          writes studio/reference/fighters/tenor_sax/ and its cache
    python -m tools.studio_export cache fighters/tenor_sax   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(name='TENOR SAX', opponent='FLUTE', left_in_cache=True)

EXPORTS = {'fighter_tenor_sax': lambda: m3refs.fighter_refs(SPEC)}

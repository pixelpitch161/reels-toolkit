"""fighter_alto_sax: ALTO SAX's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part does not branch on its role, so only its kit role is exported.

One departure from the template: the handicap is (0.5, 0.9), a reference handicap (ruling M3-R34). At (1.0, 1.0)
ALTO SAX won all of 120 trial fights, with a median K.O. at 25.6 s: no `right` fight (ALTO SAX loses, its shards
fly) could be chosen, and no fight lasted the 44 s the checks need. At (0.5, 0.9) ALTO SAX wins 81.2 % of the 2,000
statistics fights and the median K.O. is at 43.6 s, so the natural `left` and `right` fights exist. (Trial scans of
120 seeds each: (1.0, 2.0) 66 wins, (1.0, 1.8) 97, both with the median K.O. near 25.6 s; (0.6, 1.1) 84 at 37.9 s;
(0.5, 1.0) 75 at 43.6 s; (0.5, 0.9) 94 at 44.2 s; (0.45, 0.8) 108 at 47.1 s.)

    python -m tools.studio_export fighter_alto_sax          writes studio/reference/fighters/alto_sax/ and its cache
    python -m tools.studio_export cache fighters/alto_sax   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='ALTO SAX',
    opponent='FLUTE',
    fights=dict(handicap=(0.5, 0.9)),            # a reference handicap (see above)
)

EXPORTS = {'fighter_alto_sax': lambda: m3refs.fighter_refs(SPEC)}

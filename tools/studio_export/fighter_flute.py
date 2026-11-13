"""fighter_flute: FLUTE's milestone 3 references (Template F), the framework's first user with fighter_trombone.

The opponent is the other milestone 1 fighter, TROMBONE (Template F fights every fighter against FLUTE, and FLUTE
against TROMBONE). Everything else is the template's default (m3refs.py's docstring): the big top config of the
milestone 1 example, with the series' pair rule `swat_cobras`.

One departure from the template: the handicap is the kit's own for this pair (the example's DMG_SIDE: 2.0 for
FLUTE, 0.40 for TROMBONE), not (1.0, 1.0). At (1.0, 1.0) FLUTE won none of 60 trial fights, so no `left` fight
could be chosen; with the example's handicap the framework's fights are milestone 1's own recordings wherever the
seeds are the same (fight.json `same_as`), which is the framework's proof.

    python -m tools.studio_export fighter_flute          writes studio/reference/fighters/flute/ and its cache
    python -m tools.studio_export cache fighters/flute   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='FLUTE',
    opponent='TROMBONE',
    fights=dict(handicap=(2.0, 0.40)),           # the kit's frozen handicap for this pair, FLUTE left
    left_in_cache=False,                         # its left fight is milestone 1's committed recording
)

EXPORTS = {'fighter_flute': lambda: m3refs.fighter_refs(SPEC)}

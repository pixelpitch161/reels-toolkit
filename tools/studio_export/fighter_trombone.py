"""fighter_trombone: TROMBONE's milestone 3 references (Template F), the framework's first user with fighter_flute.

The opponent is FLUTE, as for every fighter of Template F. Everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example, with the series' pair rule `swat_cobras`. TROMBONE is
one of the two fighters, so the template's extra "plain against TROMBONE" scenario is left out (a mirror match).

One departure from the template: the handicap is the kit's own for this pair (the example's DMG_SIDE: 2.0 for
FLUTE, 0.40 for TROMBONE), not (1.0, 1.0). At (1.0, 1.0) FLUTE won none of 60 trial fights, so FLUTE's `left`
fight could not be chosen; with the example's handicap the framework's fights are milestone 1's own recordings
wherever the seeds are the same (fight.json `same_as`), which is the framework's proof.

    python -m tools.studio_export fighter_trombone          writes studio/reference/fighters/trombone/ and its cache
    python -m tools.studio_export cache fighters/trombone   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='TROMBONE',
    opponent='FLUTE',
    fights=dict(handicap=(0.40, 2.0)),           # the kit's frozen handicap for this pair, TROMBONE left
)

EXPORTS = {'fighter_trombone': lambda: m3refs.fighter_refs(SPEC)}

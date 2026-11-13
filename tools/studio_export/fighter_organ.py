"""fighter_organ: ORGAN's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part branches on its role (`bass`: the pedal under every pipe row and the climb of the last bar).
The config gives ORGAN no role, so the `left` and `right` soundtracks are its `melody` part already; the one extra
role exported is `bass`.

Two departures from the template:

1. The handicap is (1.0, 2.0): ORGAN neutral, FLUTE at its kit handicap (the example's DMG_SIDE for FLUTE). At
   (1.0, 1.0) ORGAN won all 2,000 statistics fights, so no `right` fight (ORGAN loses, its shards fly) could be
   chosen (rulings M3-R16, M3-R28). At (1.0, 2.0) ORGAN wins 76.5 % of the 2,000.
2. The `left` fight is seed 271, fixed (the framework checks its K.O. is after 44 s).

    python -m tools.studio_export fighter_organ          writes studio/reference/fighters/organ/ and its cache
    python -m tools.studio_export cache fighters/organ   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='ORGAN',
    opponent='FLUTE',
    fights=dict(handicap=(1.0, 2.0),             # ORGAN neutral, FLUTE at its kit handicap (see above)
                left=271),                       # K.O. at 47.0 s, all three abilities
    roles=('bass',),                             # `melody` is the left and right soundtracks' own role
    left_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)

EXPORTS = {'fighter_organ': lambda: m3refs.fighter_refs(SPEC)}

"""fighter_horn: HORN's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part does not branch on its role, so only its kit role is exported.

Two departures from the template:

1. The handicap is (1.0, 2.0), not (1.0, 1.0) (ruling M3-R34). At (1.0, 1.0) HORN won every decided fight of 200
seeds (197 of 197, median K.O. 37.8 s), so no natural `right` fight existed. Turning HORN down instead leaves most
fights without a K.O. (0.6: 54 % of 200 undecided), so FLUTE is turned up: at (1.0, 2.0) HORN won 140 of 200
(70 %), every fight decided, median K.O. 34.6 s. ORGAN's reference handicap is the same (M3-R28).

2. `left_in_cache=True` (ruling M3-R36): the left recording is 229,396 bytes (ECHO_G holds 24 floats a frame), and
   with it the folder would be 368,658 of 300,000 bytes; without it about 139 KB.

Its record keys ECHO_G, ECHO_N and FLICK are in the kit's own `story.REC_KEYS`; ECHO_R (the live rings, a list a
frame) is ragged (`fights.RAGGED_DEPTH`).

    python -m tools.studio_export fighter_horn          writes studio/reference/fighters/horn/ and its cache
    python -m tools.studio_export cache fighters/horn   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='HORN',
    opponent='FLUTE',
    fights=dict(handicap=(1.0, 2.0)),            # the reference handicap (departure 1)
    left_in_cache=True,                          # the left recording cannot fit the folder (departure 2)
)

EXPORTS = {'fighter_horn': lambda: m3refs.fighter_refs(SPEC)}

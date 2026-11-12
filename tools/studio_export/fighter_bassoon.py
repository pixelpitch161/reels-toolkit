"""fighter_bassoon: BASSOON's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part does not branch on its role, so only its kit role is exported.

One departure from the template:

The handicap is (0.41, 0.6), not (1.0, 1.0) (ruling M3-R34). At (1.0, 1.0) BASSOON won 99.6 % of the 2,000
statistics fights with a median K.O. at 23 s, so no natural `right` fight existed and no winning fight lasted to
44 s, which the checks need of the `left` fight. Both sides are turned down so the fights are long enough (median
K.O. 49 s), and BASSOON a little more so FLUTE wins a real share (BASSOON 77.8 % of the 2,000). The kit's own pairs
turn BASSOON down too (0.85 against OBOE, 0.70 against BASS CLARINET).

Its record keys HOOK and HOP are `.npy` members through the framework's `recording.KIT_RECT_KEYS`.

    python -m tools.studio_export fighter_bassoon          writes studio/reference/fighters/bassoon/ and its cache
    python -m tools.studio_export cache fighters/bassoon   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='BASSOON',
    opponent='FLUTE',
    fights=dict(handicap=(0.41, 0.6)),           # the reference handicap (the departure)
)

EXPORTS = {'fighter_bassoon': lambda: m3refs.fighter_refs(SPEC)}

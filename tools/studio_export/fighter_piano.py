"""fighter_piano: PIANO's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the big top config of the milestone 1 example. That
config writes no line for PIANO, so the roll's rows (`kit/story.py` `roll_rows`, the rule `roll_rows` of every fight
here) are the kit's sketch line (`default_line`) with the harmony a denser row adds. The two plain fights against
TROMBONE are the template's "lance against its kinds": the lance parries the roll's keys (`parry` events).

Two departures from the template's defaults:

- The handicap is (0.7, 1.0), not (1.0, 1.0) (ruling M3-R34). With no handicap PIANO wins 98.4 % of 2,000 fights
  against FLUTE, at a median K.O. of 35.5 s: the statistics then prove little, and a fight that ends after 44 s
  (which the checks part needs) is rare. At (0.7, 1.0) PIANO wins about 74 % and the median K.O. is about 47 s, so
  the natural `left` and `right` fights exist and no seed is fixed.
- `left_in_cache` (ruling M3-R36): a roll's recording holds its shots in `K` (72 slots of 7 floats a frame, a dozen
  or more of them live in most frames), which alone is more than a fighter's whole 300 KB folder. The left
  recording is the pinned cache file `left.bfrec`; fight.json names it with its hash.

    python -m tools.studio_export fighter_piano          writes studio/reference/fighters/piano/ and its cache
    python -m tools.studio_export cache fighters/piano   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='PIANO',
    opponent='FLUTE',
    fights=dict(handicap=(0.7, 1.0)),
    left_in_cache=True,
)

EXPORTS = {'fighter_piano': lambda: m3refs.fighter_refs(SPEC)}

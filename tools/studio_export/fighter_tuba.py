"""fighter_tuba: TUBA's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, and every key is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example at the neutral handicap. A trial of 60 seeds a corner with
the kit gave the TUBA 21 of 47 decided fights from the left corner and 19 of 54 from the right, so both sides win a
real share and no reference handicap is needed (ruling M3-R34).

The opt-in rule `swallow_rollers` (a bell gulps a rolling drum) is a pair rule of TUBA against BASS DRUM only; no
fight of this folder has it (it is proven at that pair's gate).

    python -m tools.studio_export fighter_tuba          writes studio/reference/fighters/tuba/ and its cache
    python -m tools.studio_export cache fighters/tuba   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(name='TUBA', opponent='FLUTE', left_in_cache=True)

EXPORTS = {'fighter_tuba': lambda: m3refs.fighter_refs(SPEC)}

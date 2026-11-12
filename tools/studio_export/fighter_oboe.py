"""fighter_oboe: OBOE's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F; everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example, with the series' pair rules.

One departure from the template: a reference handicap (ruling M3-R34). At (1.0, 1.0) OBOE wins 99.9 % of the 2,000
statistics fights and FLUTE wins none of seeds 1 to 500 from the left corner, so the statistics say little and no
natural `right` fight exists. There is no kit episode of OBOE against FLUTE to take a handicap from, so this one
was chosen by trial over 100 seeds a corner with the kit: (0.8, 1.3) gives OBOE about 91 %, (0.7, 1.5) about 82 %,
(0.6, 1.5) about 64 %. With (0.6, 1.5), OBOE's hits x 0.6 and FLUTE's x 1.5, both sides win often, and the `left`
and `right` fights are the framework's own picks.

One scenario more than the template's: `vs_trombone_low`, the template's `early_low` rules against TROMBONE. With
this handicap the TROMBONE first drops below 25 HP 32 s into a plain fight, so its PEDAL TONE and the `slam` would
have their coverage rows long after any fight's agreed window; under `early_low` they come in the first seconds.

    python -m tools.studio_export fighter_oboe          writes studio/reference/fighters/oboe/ and its cache
    python -m tools.studio_export cache fighters/oboe   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='OBOE',
    opponent='FLUTE',
    fights=dict(handicap=(0.6, 1.5)),            # the reference handicap, OBOE left (ruling M3-R34)
    scenarios=m3refs.default_scenarios('OBOE', 'FLUTE') + [
        dict(name='vs_trombone_low', seeds=(1, 2), opponent='TROMBONE',
             overrides=dict(LV_DMG=m3refs.LV, LOW_HP=92.0)),
    ],
)

EXPORTS = {'fighter_oboe': lambda: m3refs.fighter_refs(SPEC)}

"""fighter_euphonium: EUPHONIUM's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Three departures from the template:

1. The handicap is (0.5, 1.0), a reference handicap (ruling M3-R34). At (1.0, 1.0) EUPHONIUM won all of 120 trial
   fights from either corner, so no `right` fight (EUPHONIUM loses, its shards fly) could be chosen. Trial scans of
   120 seeds a corner, EUPHONIUM left / right: (0.7, 1.0) 115 / 115 wins; (0.6, 1.0) 105 / 107; (0.6, 1.2) 72 / 77;
   (0.5, 1.0) 78 / 81; (0.5, 1.5) 10 / 5; (0.4, 2.0) 0 / 0.
2. The copy sweep (ruling M3-R3, the plan's Task 26): IDENTITY CRISIS runs another fighter's kind on the
   euphonium's ball and draws it through that fighter's look, so besides the template's scenarios there is one
   scenario group against every other ported fighter (`copy_<slug>`, the euphonium left, at the folder's handicap),
   with seeds where the euphonium copies each copyable kind that fighter has (`fight.py` 2802 COPYABLE: arc, stun,
   snip, twin, lights_out, shield, slam, shockwave, wave, haste) and, for BASS CLARINET, its signature (the whirl).
   A fighter with no copyable kind (ELECTRIC, CELLO, ALTO SAX, TENOR SAX, BASS DRUM) gets one seed where the copy is
   its signature (an event only). The `prefix` gate proves the fights, `scenarios_draw` draws each with both real
   looks, and the test file's `copy_sweep` checks the table of copied kinds. Most groups use the early levels and
   the early low turn (LV_DMG = (2, 5), LOW_HP 92) so the copy comes soon after FIGHT!; the seeds were found with
   the kit (seeds 1 to 40 under three override sets; TROMBONE's PEDAL TONE only at seed 53 of 200 without them).
3. The left recording is in the pinned cache (ruling M3-R36, `left_in_cache`): its 134 KB took the folder to
   361 KB, with everything else at about 227 KB.

    python -m tools.studio_export fighter_euphonium          writes studio/reference/fighters/euphonium/ and its cache
    python -m tools.studio_export cache fighters/euphonium   regenerates the cache and checks the pins"""
from . import m3refs

LV = m3refs.LV
EARLY = dict(LV_DMG=LV)                          # the levels come early
LOW = dict(LV_DMG=LV, LOW_HP=92.0)               # ... and the low turn too

# (opponent, overrides, seeds): the copy sweep; the comment names the kinds each seed copies first
COPY_SWEEP = (
    ('FLUTE', LOW, (13, 19)),                    # haste 13, wave 19
    ('TROMBONE', EARLY, (5, 28)),                # haste 5, shockwave 28
    ('TROMBONE', {}, (53,)),                     # slam 53
    ('PIANO', LOW, (12,)),                       # arc
    ('TRUMPET', EARLY, (5, 15)),                 # haste 5, shockwave 15
    ('ORGAN', LOW, (7, 9, 13)),                  # shield 7, slam 9, haste 13
    ('CLARINET', LOW, (2, 4)),                   # haste 2, stun 4
    ('OBOE', LOW, (3, 17)),                      # wave 3 (ILL WIND), snip 17
    ('SOUSAPHONE', LOW, (1,)),                   # haste
    ('TENOR SAX', LOW, (2,)),                    # its signature
    ('ALTO SAX', LOW, (3,)),                     # its signature
    ('BASS CLARINET', LOW, (1, 24, 25, 32)),     # its whirl 1, lights_out 24, twin 25, snip 32
    ('TUBA', LOW, (3, 8, 14)),                   # shield 3, slam 8, haste 14
    ('CONTRABASS', LOW, (4,)),                   # haste
    ('BASSOON', LOW, (3,)),                      # haste
    ('HORN', LOW, (2, 3)),                       # haste 2, shield 3
    ('ACOUSTIC', LOW, (8,)),                     # haste
    ('ELECTRIC', LOW, (8,)),                     # its signature
    ('CELLO', LOW, (21,)),                       # its signature
    ('BASS DRUM', LOW, (16,)),                   # its signature
)


def _group(opp, over, seeds, n):
    tag = 'plain' if not over else ('low' if 'LOW_HP' in over else 'lv')
    return dict(name=f'copy_{m3refs.slug(opp)}_{tag}' if n > 1 else f'copy_{m3refs.slug(opp)}', seeds=seeds,
                overrides=dict(over), opponent=opp)


def _sweep():
    count = {}
    for opp, _, _ in COPY_SWEEP:
        count[opp] = count.get(opp, 0) + 1
    return [_group(opp, over, seeds, count[opp]) for opp, over, seeds in COPY_SWEEP]


SPEC = dict(
    name='EUPHONIUM',
    opponent='FLUTE',
    fights=dict(handicap=(0.5, 1.0)),            # a reference handicap (see above)
    scenarios=m3refs.default_scenarios('EUPHONIUM', 'FLUTE') + _sweep(),
    left_in_cache=True,                          # ruling M3-R36 (see above)
)

EXPORTS = {'fighter_euphonium': lambda: m3refs.fighter_refs(SPEC)}

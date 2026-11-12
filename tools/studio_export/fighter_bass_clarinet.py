"""fighter_bass_clarinet: BASS CLARINET's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F; everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example, at the neutral handicap (BASS CLARINET wins about 73 % of
the kit's fights there, so both sides win a real share and no reference handicap is needed: ruling M3-R34).

Its part branches on its role (`bass`: the line at the bass, the low band and colour, the pump of the last bar; any
other role an octave up, resting in the last bar). The config gives BASS CLARINET no role, so the `left` and `right`
soundtracks are the octave-up part already; the roles exported beside them are `bass` and `counter` (the role the
kit's own episode gives it: the same branch as the config's, by another name).

    python -m tools.studio_export fighter_bass_clarinet          writes studio/reference/fighters/bass_clarinet/
    python -m tools.studio_export cache fighters/bass_clarinet   regenerates the cache and checks the pins"""
from . import m3refs

SPEC = dict(
    name='BASS CLARINET',
    opponent='FLUTE',
    roles=('bass', 'counter'),
    left_in_cache=True,  # storage plan B (M3-R43): its kit recording is the pinned cache file
)

EXPORTS = {'fighter_bass_clarinet': lambda: m3refs.fighter_refs(SPEC)}

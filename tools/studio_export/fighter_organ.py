"""fighter_organ: ORGAN's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F, on the template's default config (m3refs.py's
docstring). Its part branches on its role (`bass`: the pedal under every pipe row and the climb of the last bar).
The config gives ORGAN no role, so the `left` and `right` soundtracks are its `melody` part already; the one extra
role exported is `bass`.

Four departures from the template:

1. The handicap is (1.0, 2.0): ORGAN neutral, FLUTE at its kit handicap (the example's DMG_SIDE for FLUTE). At
   (1.0, 1.0) ORGAN won all 2,000 statistics fights, so no `right` fight (ORGAN loses, its shards fly) could be
   chosen: the same problem ruling M3-R16 settled for FLUTE and TROMBONE. Of 40 trial fights at (1.0, 2.0) ORGAN
   won 32 and FLUTE 8.
2. `fight.py`'s `_to_ropes` returns a numpy float whenever a rope cuts a row short, so the kit's `line` event then
   carries `np.float64(L)`, which a recording cannot tell from the float `L` (the format has one float type): the
   framework's replay guard on a recording read back (`m3refs.record`) would differ from the kit's by that word
   alone. `kit_floats` patches `_to_ropes` at run time, in this process only, to return the same number as a plain
   float. No value changes: every position, HP and event field is bit for bit what the unpatched kit computes.

3. `m3refs.fighter_parts` hands the picture and sound processes the path of `left.bfrec`, which the framework
   only writes when the folder's run finishes (FLUTE and TROMBONE point at milestone 1's recordings, which are
   there already). `left_on_disk` writes the staged bytes there for the parts, and takes the file away again if
   the run stops; the framework's `finish` then writes the same bytes.
4. The `left` fight is seed 271, not the first statistics fight ORGAN wins with all three abilities: the kit's
   `music.main()` (which the checks run) prints the level of the span from 44 s to the K.O. and raises when the
   K.O. comes before 44 s, as it does in most fights at this handicap (median 34 s). Seed 271 is the first of the
   2,000 that ORGAN wins between 44.3 and 50 s with CATHEDRAL, PULL ALL STOPS and TOCCATA all fired.

    python -m tools.studio_export fighter_organ          writes studio/reference/fighters/organ/ and its cache
    python -m tools.studio_export cache fighters/organ   regenerates the cache and checks the pins"""
import os

from . import common, m3refs

SPEC = dict(
    name='ORGAN',
    opponent='FLUTE',
    fights=dict(handicap=(1.0, 2.0),             # ORGAN neutral, FLUTE at its kit handicap (see above)
                left=271),                       # K.O. at 47.0 s, all three abilities (see above)
    roles=('bass',),                             # `melody` is the left and right soundtracks' own role
)


def kit_floats():
    """the kit's `Fight._to_ropes` returning a plain float (the same value), in this process"""
    E, _ = common.kit()
    E.activate_paths()
    import fight
    if getattr(fight.Fight._to_ropes, 'plain_float', False):
        return
    kit_to_ropes = fight.Fight._to_ropes

    def _to_ropes(self, p, u):
        return float(kit_to_ropes(self, p, u))
    _to_ropes.plain_float = True
    fight.Fight._to_ropes = _to_ropes


def left_on_disk():
    """m3refs.fighter_parts with the staged left.bfrec written where its child processes read it"""
    if getattr(m3refs.fighter_parts, 'left_on_disk', False):
        return
    parts = m3refs.fighter_parts

    def fighter_parts(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath):
        path = os.path.abspath(os.path.join(common.REPO, *f'{m3refs.REF}/{left_file}'.split('/')))
        staged = {os.path.normcase(os.path.abspath(p)): v
                  for p, v in m3refs.run_state().stage(folder).commits.items()}.get(os.path.normcase(path))
        made = staged is not None and not os.path.isfile(path)
        if made:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'wb') as f:
                f.write(staged[0])
        try:
            return parts(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath)
        except BaseException:
            if made:
                os.remove(path)
            raise
    fighter_parts.left_on_disk = True
    m3refs.fighter_parts = fighter_parts


def refs():
    kit_floats()
    left_on_disk()
    return m3refs.fighter_refs(SPEC)


EXPORTS = {'fighter_organ': refs}

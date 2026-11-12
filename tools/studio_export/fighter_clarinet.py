"""fighter_clarinet: CLARINET's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F. Everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example.

One departure from the template: the handicap is the kit's own for this pair (`ballfight_flute_vs_clarinet`'s
DMG_SIDE: 5.3 for FLUTE, 0.36 for CLARINET), not (1.0, 1.0). At (1.0, 1.0) CLARINET won all 2,000 statistics
fights and its RHAPSODY (below 25 HP) fired in none it won, so no `left` fight (CLARINET winning with all three
abilities) and no `right` fight (CLARINET losing) could be chosen. Ruling M3-R16 settled the same problem for
FLUTE and TROMBONE the same way.

    python -m tools.studio_export fighter_clarinet          writes studio/reference/fighters/clarinet/ and its cache
    python -m tools.studio_export cache fighters/clarinet   regenerates the cache and checks the pins

A stand-in for a gap in the framework (`_left_on_disk` below; the controller's to close in m3refs.py, then this goes):
m3refs keeps the folder's committed files in the run's stage until finish(), but fighter_parts hands the picture,
sound and check processes the path of the committed `left.bfrec`, which for a fighter new to the set is not on disk
yet (FLUTE's and TROMBONE's folders point at milestone 1's recordings, so the framework's first users never met
this). Until m3refs does it itself, this module puts the staged bytes at that path while the parts run: the same
bytes finish() writes there, removed again (or put back as they were) if the run stops, so a stopped run still
leaves the committed state as it was."""
import os

from . import m3refs

SPEC = dict(
    name='CLARINET',
    opponent='FLUTE',
    fights=dict(handicap=(0.36, 5.3)),           # the kit's frozen handicap for this pair, CLARINET left
)


def _left_on_disk(fn, undo):
    """fighter_parts with the staged left.bfrec readable at its committed path while it runs (the module docstring);
    `undo` gets what puts the path back as it was"""
    def parts(spec, folder, *args, **kw):
        path = m3refs.ref_dir(folder, 'left.bfrec')
        staged = m3refs.run_state().stage(folder).commits.get(path)
        before = open(path, 'rb').read() if os.path.isfile(path) else None
        if staged is not None and staged[0] != before:
            undo.append((path, before, not os.path.isdir(os.path.dirname(path))))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'wb') as f:
                f.write(staged[0])
        return fn(spec, folder, *args, **kw)
    return parts


def _refs():
    real, undo = m3refs.fighter_parts, []
    m3refs.fighter_parts = _left_on_disk(real, undo)
    try:
        return m3refs.fighter_refs(SPEC)
    except BaseException:                         # the run stopped before finish(): the committed state as it was
        for path, before, made in undo:
            if before is None:
                os.remove(path)
                if made and not os.listdir(os.path.dirname(path)):
                    os.rmdir(os.path.dirname(path))
            else:
                with open(path, 'wb') as f:
                    f.write(before)
        raise
    finally:
        m3refs.fighter_parts = real


EXPORTS = {'fighter_clarinet': _refs}

"""fighter_piano: PIANO's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F; everything but the left seed is the template's default
(m3refs.py's docstring): the big top config of the milestone 1 example with no handicap. That config writes no line
for PIANO, so the roll's rows (`kit/story.py` `roll_rows`, the rule `roll_rows` of every fight here) are the kit's
sketch line (`default_line`) with the harmony a denser row adds. The two plain fights against TROMBONE are the
template's "lance against its kinds": the lance parries the roll's keys (`parry` events).

PIANO is the first fighter whose fight holds what milestone 1's two fighters never had: a rule that is a dict keyed
by the fighter's index (`roll_rows`), events with a tuple inside (`roll` and `runoff` carry their pitches), a ragged
record key of its own (`HOLD`), and a first export with no recording in the set to point at. The export framework
(`common`, `fights`, `frames`, `m3refs`: files a port does not edit) stops on each of them. Until the framework
handles them itself, this module adapts it at run time, for its own run only (`_run` installs the adapters and
takes them out again; a part process gets them through `_child`), without touching the framework's files:

- `_rows_as_pairs` (`common.plain`): a rules dict's `roll_rows` is listed as `[index, rows]` pairs in index order,
  exactly what `recording.write_bfrec` writes into a recording's rules (ruling M3-R19). `common.plain` refuses a
  dict key that is not a text, and every rules dict the framework turns into JSON goes through it (`fight.json`'s
  rules, `fights.check`, `frames.check_recording`).
- `_with_hold` (`fights.as_kit_run`): a recording read back becomes the kit's run again with `HOLD`'s entries and
  the events' pitches as tuples (the framework's table of ragged keys has only milestone 1's).
- `_check_with_pitches` (`frames.check_recording`): its last test compares events with `==`, for which a list of
  pitches is not the kit's tuple.
- `_parts_with_left` (`m3refs.fighter_parts`): the staged `left.bfrec` is put on disk before the part processes
  read it (the framework writes committed files only at the end of a run).
- `_no_handicap` (`m3refs.config_text`): at the template's neutral handicap the config says `DMG_SIDE = None`, not
  `(1.0, 1.0)`. The fights are the same (a damage times 1.0), but the kit records the rule `dmg_side` for the tuple
  and none for `None`, and a Studio fight file with handicap `[1.0, 1.0]` records none (`bf_sim::setup_from`): the
  harness's `left_fight` gate compares the two rule sets whole.

    python -m tools.studio_export fighter_piano          writes studio/reference/fighters/piano/ and its cache
    python -m tools.studio_export cache fighters/piano   regenerates the cache and checks the pins
    python -m tools.studio_export fighter_piano_stages [left | right] <frame>...
                                                         `fighter_stages piano ...` with the adapters (stage dumps)"""
from . import m3refs

SPEC = dict(
    name='PIANO',
    opponent='FLUTE',
    # the left fight: the lowest seed of the statistics fights where PIANO wins from the left with all three
    # abilities AND the K.O. comes at or after 44 s (seed 9, the lowest without that, ends at 37.4 s). The kit's
    # soundtrack step (kit/music.py main(), which the checks run) measures the section from 44 s to the K.O. and
    # stops on an empty one, so a fight that ends sooner cannot be checked
    fights=dict(left=41),
)

PITCHES_AT = dict(roll=6, runoff=5)               # the element of these events that is a tuple of pitches
_ORIGINAL = {}                                    # (module name, attribute) -> what was there before _install()


def _rows_as_pairs(plain):
    """common.plain, with a rules dict's `roll_rows` ({fighter index: [row, ...]}) listed as [index, [[note, ...],
    ...]] pairs in index order first"""
    def plain_(x, where='value'):
        if isinstance(x, dict) and isinstance(x.get('roll_rows'), dict):
            x = dict(x, roll_rows=[[int(k), [[int(m) for m in row] for row in rows]]
                                   for k, rows in sorted(x['roll_rows'].items())])
        return plain(x, where)
    return plain_


def _with_hold(as_kit_run):
    """fights.as_kit_run for a recording of a roll: `HOLD` (per frame a list of (id, t_stop, t_release) tuples, as
    `WAVES` and `SLAMS` are lists of tuples) and the tuple of pitches inside its `roll` and `runoff` events"""
    def as_kit_run_(rec):
        arrays = dict(rec['arrays'])
        hold = arrays.pop('HOLD', None)
        r = as_kit_run(dict(rec, arrays=arrays))
        if hold is not None:
            r['HOLD'] = [[tuple(h) for h in frame] for frame in hold]

        def pitches(e):
            i = PITCHES_AT.get(e[1])
            return e if i is None else e[:i] + (tuple(e[i]),) + e[i + 1:]
        r['events'] = [pitches(e) for e in r['events']]
        return r
    return as_kit_run_


def _check_with_pitches(check_recording):
    """frames.check_recording for a roll's fight: the framework's function is given a view of the renderer module
    `P` whose `EV` lists each event's pitches (as the recording read back does); everything else is `P`'s own"""
    def check_recording_(P, path=None):
        class View:
            def __getattr__(self, name):
                return getattr(P, name)

        def listed(e):
            i = PITCHES_AT.get(e[1])
            return e if i is None else e[:i] + (list(e[i]),) + e[i + 1:]
        view = View()
        view.EV = [listed(tuple(e)) for e in P.EV]
        return check_recording(view, path)
    return check_recording_


def _parts_with_left(fighter_parts):
    """m3refs.fighter_parts, with the staged `left.bfrec` on disk first: the bytes finish() will write there"""
    def fighter_parts_(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath):
        import os
        from . import common
        path = os.path.abspath(os.path.join(common.REPO, *f'{m3refs.REF}/{left_file}'.split('/')))
        staged = m3refs.run_state().stage(folder).commits.get(path)
        if staged is not None and (not os.path.isfile(path) or open(path, 'rb').read() != staged[0]):
            common.write_bytes(path, staged[0])
        return fighter_parts(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath)
    return fighter_parts_


def _no_handicap(config_text):
    """m3refs.config_text, with `DMG_SIDE = None` where the handicap is (1.0, 1.0): the kit's config of a fight with
    no handicap"""
    def config_text_(base, fighters, handicap, dmg_scale, rules, seed=None):
        text = config_text(base, fighters, handicap, dmg_scale, rules, seed)
        if tuple(float(x) for x in handicap) == (1.0, 1.0):
            line = 'DMG_SIDE = (1.0, 1.0)\n'
            assert text.count(line) == 1, 'the config text names its handicap once'
            text = text.replace(line, 'DMG_SIDE = None\n')
        return text
    return config_text_


def _install():
    """put the adapters in (see the module's docstring); a second call changes nothing"""
    from . import common, fights, frames
    for mod, attr, wrap in ((common, 'plain', _rows_as_pairs), (fights, 'as_kit_run', _with_hold),
                            (frames, 'check_recording', _check_with_pitches),
                            (m3refs, 'fighter_parts', _parts_with_left), (m3refs, 'config_text', _no_handicap)):
        key = (mod.__name__, attr)
        if key not in _ORIGINAL:
            _ORIGINAL[key] = (mod, getattr(mod, attr))
            setattr(mod, attr, wrap(getattr(mod, attr)))


def _uninstall():
    """take the adapters out again: another folder's export in the same run sees the framework as it is"""
    for (_, attr), (mod, original) in list(_ORIGINAL.items()):
        setattr(mod, attr, original)
    _ORIGINAL.clear()


def _child(q, modname, fnname, args):
    """m3parts' part process, started through this module so that the new process has the adapters too. The
    thread counts are set first, as m3parts._child sets them, before anything imports numpy"""
    import os
    from . import m3parts
    for k in m3parts.THREAD_VARS:
        os.environ[k] = '1'
    _install()
    m3parts._child(q, modname, fnname, args)


def _adapted(fn, *args):
    """fn(*args) with the adapters in, here and in the part processes it starts"""
    from . import m3parts
    _install()
    child, m3parts._child = m3parts._child, _child
    try:
        return fn(*args)
    finally:
        m3parts._child = child
        _uninstall()


def _run():
    return _adapted(m3refs.fighter_refs, SPEC)


def _stages(args):
    """`fighter_stages piano [left | right] <frame>...` (the stage dumps the harness's `stages` test reads), with
    the adapters: `python -m tools.studio_export fighter_piano_stages left 300 900 1500`"""
    from . import fighter_stages
    return _adapted(fighter_stages.run, ['piano', *args])


EXPORTS = {'fighter_piano': _run, 'fighter_piano_stages': _stages}
TAKES_ARGS = ('fighter_piano_stages',)

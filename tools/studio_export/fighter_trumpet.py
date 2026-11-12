"""fighter_trumpet: TRUMPET's milestone 3 references (Template F).

The opponent is FLUTE, as for every fighter of Template F; everything else is the template's default (m3refs.py's
docstring): the big top config of the milestone 1 example, neutral handicap. That config has no LINES['TRUMPET'], so
the soundtracks here are the part without a line (the valve fingerings).

One thing is TRUMPET's own. Its look keeps an atlas of 288 sprites (kit/fighters/trumpet.py `Look.atlas`), which it
builds with `multiprocessing.get_context('fork').Pool(2)` unless it runs inside a pool worker. Windows has no fork,
and the framework's parts run in fresh processes that are not pool workers. So while this export runs, the parts
that draw (the two looks, the checks, the stage dumps) are started through the wrappers below: each installs, in
its own fresh process, a stand-in for the `fork` context whose pool maps in the process itself, and then calls
m3parts' own function unchanged. The sprites are a pure function of their arguments, so the atlas is the kit's.
The checks' render workers are pool workers, where the kit builds the atlas serially itself or reads the file the
parent saved: so that three workers never write that file at once, the checks' process builds the atlas (through the
look's own `atlas()`) before it starts a pool.

A second thing is the framework's, worked round here until it is fixed there: the picture, sound and check parts
read the `left` recording from the folder's `left.bfrec` on disk, but an export or remake run only stages that file
until the run finishes, so the first export of a fighter whose `left` fight is new finds no file (FLUTE's and
TROMBONE's pointed at milestone 1's recordings). `fighter_parts` below writes the staged bytes to their place first:
the same bytes the run's finish writes. If the run then stops, the folder holds that one file and nothing else.

A third: sprites.py checks every sprite's size as 2 r + 5 when a look draws its own (FLUTE's flute) and 2 r + 7
when the model does. The TRUMPET's own sprite is its model at a radius held to 12..19 (`Look.sprite`), 2 clip(r) + 7
pixels. `sprite_sizes` below makes that the size the check asks for of the TRUMPET's ball sprites, and leaves every
other check of sprites.check_cases as it is.

And one more file than the template's, `sound_line.json` (with the cache files `stems_line.npz` and
`sound_line_rms.json`): the `left`
soundtrack again with a line for the TRUMPET, the base config's FLUTE line an octave down, so that the part's other
half is proven too (with a line the punches play it instead of the valve fingerings; plan, Task 8). The test
`the_line_takes_the_punches` in bf-harness/tests/fighter_trumpet.rs reads it (and `the_line_stems`, ignored, the
cached RMS profiles `sound_line_rms.json` and the stem windows).

    python -m tools.studio_export fighter_trumpet                        writes studio/reference/fighters/trumpet/
    python -m tools.studio_export cache fighters/trumpet                 regenerates the cache and checks the pins
    python -m tools.studio_export fighter_trumpet_stages [left | right] <frame>...   fighter_stages for TRUMPET"""
from . import m3refs

SPEC = dict(
    name='TRUMPET',
    opponent='FLUTE',
)

WRAPPED = ('look_child', 'checks_child', 'stages_child')


def serial_fork(atlas_first=False):
    """in this process, `multiprocessing.get_context('fork')` gives a context whose Pool maps serially; with
    `atlas_first`, asking for any other context first builds the atlas of every look that has one"""
    import multiprocessing as mp
    import sys
    real = mp.get_context

    class Pool:
        def __init__(self, n=None):
            pass

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

        def map(self, fn, jobs, chunksize=1):
            return [fn(j) for j in jobs]

    class Context:
        pass
    Context.Pool = Pool

    def get_context(method=None):
        if method == 'fork':
            return Context()
        picture = sys.modules.get('ballfight.kit.picture')
        if atlas_first and picture is not None:
            for look in picture.LOOKS:
                if hasattr(look, 'atlas'):
                    look.atlas()
        return real(method)
    mp.get_context = get_context


def sprite_sizes():
    """in this process, sprites.check_cases asks of the TRUMPET's own ball sprites (the `picture` and `episode`
    groups) the size its look draws them at, 2 * clip(radius, 12, 19) + 7, instead of 2 * radius + 5"""
    import copy
    from . import sprites
    check = sprites.check_cases

    def check_cases(P, k, cases, episode_keys):
        if P.NAMES[k] != SPEC['name']:
            return check(P, k, cases, episode_keys)
        seen = copy.deepcopy(cases)
        for c in seen:
            a = c['args']
            if a['group'] in ('picture', 'episode'):
                own = 2 * min(19, max(12, a['radius'])) + 7
                # what the shared check expects (2 r + 5) exactly when the size is the look's own
                a['size'] = 2 * a['radius'] + 5 if a['size'] == own else -1
        return check(P, k, seen, episode_keys)
    sprites.check_cases = check_cases


def look_child(*args):
    from . import m3parts
    serial_fork()
    sprite_sizes()
    return m3parts.look_child(*args)


def checks_child(*args):
    from . import m3parts
    serial_fork(atlas_first=True)
    return m3parts.checks_child(*args)


def stages_child(*args):
    from . import m3parts
    serial_fork()
    return m3parts.stages_child(*args)


def line_child(cfg, name, stems_out, rec_path):
    """m3parts.sound_child with a line for the fighter: the base config's FLUTE line an octave down"""
    from . import frames as F
    from . import m3parts
    m3parts.fresh_check(cfg.folder)
    ep, _ = cfg.load()
    line = [n - 12 for n in ep.T.LINES['FLUTE']]
    assert name not in ep.T.LINES and all(58 <= n <= 86 for n in line)
    ep.T.LINES[name] = line
    M, B, D = m3parts.load_music(cfg, loaded=True)
    F.check_recording(M.P, rec_path)
    k = list(M.NAMES).index(name)
    assert M.T.LINES[name] == line
    M.SOUNDS[1 - k] = M.FIGHTER_MODULES.base.Sound(M, 1 - k)
    out = m3parts._soundtrack(M, B, D, stems_out)
    out.update(k=k, fighters=list(M.NAMES), seed=int(M.EP.seed), line=line)
    return out


def line_part(spec, folder, export, module, main, left_seed, left_file):
    """sound_line.json and its cache file, staged like the template's parts"""
    import os
    from . import common, m3parts
    name = spec['name']
    stems = m3refs.cache_dir(folder, 'stems_line.npz')
    left_path = os.path.join(common.REPO, *f'{m3refs.REF}/{left_file}'.split('/'))
    doc = m3parts.fresh(line_child, main.with_seed(left_seed).part('line'), name, stems, left_path)
    m3refs.pin(stems, export)
    m3refs.commit_json(folder, 'sound_line.json', dict(
        about=f"{name}'s part with a line (LINES['{name}']: the punches play it instead of the valve fingerings): "
              f"the soundtrack of the `left` fight as sound.json's `left`, with the base config's FLUTE line an "
              f"octave down as its line. The RMS profiles are in the cache (the folder's budget)",
        tool=module, format=1, fighter=name, line=doc['line'],
        left=dict({k: v for k, v in doc.items() if k not in ('rms', 'line')}, cache=f'{folder}/stems_line.npz',
                  rms_cache=f'{folder}/sound_line_rms.json')), module)
    m3refs.cache_json(folder, 'sound_line_rms.json', dict(
        about="the line soundtrack's RMS per 100 ms of every stem and the master (dB, 3 decimals)", rms=doc['rms']),
        export)


def wrapped(fn):
    """fn() with m3parts' drawing parts started through this module's wrappers, and the staged `left` recording on
    disk before the parts read it"""
    import os
    import sys
    from . import common, m3parts
    here = sys.modules[__name__]
    kept = {n: getattr(m3parts, n) for n in WRAPPED}
    parts = m3refs.fighter_parts

    def fighter_parts(spec, folder, *rest):
        path = os.path.abspath(m3refs.ref_dir(folder, 'left.bfrec'))
        staged = m3refs.run_state().stage(folder).commits.get(path)
        if staged is not None:
            have = open(path, 'rb').read() if os.path.isfile(path) else None
            if have != staged[0]:
                common.write_bytes(path, staged[0])
        out = parts(spec, folder, *rest)
        export, module, main, _swapped, left_seed, _right_seed, left_file, _rpath = rest
        line_part(spec, folder, export, module, main, left_seed, left_file)
        return out
    for n in WRAPPED:
        setattr(m3parts, n, getattr(here, n))
    m3refs.fighter_parts = fighter_parts
    try:
        return fn()
    finally:
        m3refs.fighter_parts = parts
        for n, f in kept.items():
            setattr(m3parts, n, f)


def stages(args):
    from . import fighter_stages
    return wrapped(lambda: fighter_stages.run(['trumpet'] + list(args)))


EXPORTS = {'fighter_trumpet': lambda: wrapped(lambda: m3refs.fighter_refs(SPEC)),
           'fighter_trumpet_stages': stages}
TAKES_ARGS = ('fighter_trumpet_stages',)

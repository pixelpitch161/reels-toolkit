"""Checks of the milestone 3 storage framework (tools/studio_export/m3refs.py and the sharded manifest.py).

Run from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
    export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
    .studio-tools/py313/Scripts/python.exe -m tools.studio_export.unit_checks.m3refs_checks

Most checks point the tool at a scratch copy of the folder layout (a temporary folder with studio/reference/ in it;
git is stubbed), so nothing in the real reference set is touched. The last one uses the real kit: the framework's
fight config, given the milestone 1 example's own settings, records the committed milestone 1 fights byte for byte
(it writes only to a temporary folder and to the git-ignored cache). Prints one line per check; exits 1 on the
first failure."""
import contextlib
import io
import os
import shutil
import sys
import tempfile

from tools.studio_export import __main__ as entry
from tools.studio_export import common, m3refs, manifest

KIT_SHA = 'f' * 40


@contextlib.contextmanager
def scratch():
    """common.REPO / REFERENCE / CACHE redirected to a fresh temporary folder, git stubbed; restored afterwards"""
    saved = (common.REPO, common.REFERENCE, common.CACHE, common.git, m3refs.set_run(None))
    root = tempfile.mkdtemp(prefix='m3refs_checks_')
    try:
        common.REPO = root
        common.REFERENCE = os.path.join(root, 'studio', 'reference')
        common.CACHE = os.path.join(common.REFERENCE, '.cache')
        os.makedirs(common.CACHE)

        def git(*args):
            if args[:1] == ('rev-parse',):
                return KIT_SHA
            if args[:1] == ('status',):
                return ''
            raise AssertionError(f'git {args} in a scratch check')
        common.git = git
        manifest._save([])                                # an empty top-level manifest, as a fresh set has
        yield root
    finally:
        common.REPO, common.REFERENCE, common.CACHE, common.git, run = saved
        m3refs.set_run(run)
        shutil.rmtree(root, ignore_errors=True)


def put(rel, data=b'x\n'):
    path = os.path.join(common.REPO, *rel.split('/'))
    common.write_bytes(path, data)
    return path


def exits(fn, *args):
    """the SystemExit text fn(*args) stops with (AssertionError if it does not stop)"""
    try:
        fn(*args)
    except SystemExit as ex:
        return str(ex)
    raise AssertionError(f'{fn.__name__} did not stop')


def entries_of(shard_rel):
    return [e['path'] for e in manifest.load(os.path.join(common.REPO, *shard_rel.split('/')))]


# ---------------------------------------------------------------- the checks
def shard_takes_a_fighter_file():
    with scratch():
        p = put('studio/reference/fighters/flute/fight.json', b'{}\n')
        q = put('studio/reference/fights/other/x.json', b'{}\n')
        manifest.add(p, tool='check')
        manifest.add(q, tool='check')
        assert entries_of('studio/reference/fighters/flute/manifest.json') == [
            'studio/reference/fighters/flute/fight.json'], 'not in the fighter shard'
        assert entries_of('studio/reference/manifest.json') == ['studio/reference/fights/other/x.json'], \
            'the top-level manifest holds the fighter file, or lost the other one'
        n, problems, unlisted = manifest.verify()
        assert (n, problems, unlisted) == (2, [], []), (n, problems, unlisted)
    return 'a file under fighters/flute/ lands in that shard, not in the top-level manifest; verify reads both'


def verify_finds_a_changed_byte_in_a_shard():
    with scratch():
        p = put('studio/reference/themes/heist/theme.json', b'{"a": 1}\n')
        manifest.add(p, tool='check')
        put('studio/reference/themes/heist/theme.json', b'{"a": 2}\n')
        _, problems, _ = manifest.verify()
        assert problems == ['changed  studio/reference/themes/heist/theme.json'], problems
        os.remove(p)
        _, problems, _ = manifest.verify()
        assert problems == ['missing  studio/reference/themes/heist/theme.json'], problems
    with scratch():                                       # an entry in the wrong shard is reported too
        p = put('studio/reference/episodes/a_vs_b/sound.json', b'{}\n')
        e = manifest.add(p, tool='check')
        manifest.remove([p])
        manifest._save([e])                                # (by hand: into the top-level manifest)
        _, problems, _ = manifest.verify()
        assert len(problems) == 1 and problems[0].startswith('misplaced studio/reference/episodes/a_vs_b/sound.json'), \
            problems
    return 'verify finds a changed byte, a missing file and a misplaced entry in a shard'


MODEL = 'studio/crates/bf-content/data/models/FLUTE.json'
STATE = {}


def fake_export():
    """a stand-in for fighter_flute in the scratch set: one pinned cache file, one committed file in the folder, the
    fighter's model; STATE says what they hold"""
    folder = 'fighters/flute'
    m3refs.cache_bytes(folder, 'right.bfrec', STATE['cache'], 'fighter_flute')
    m3refs.commit_bytes(folder, 'fight.json', STATE['commit'], 'tools.studio_export.fighter_flute')
    m3refs.commit_path(folder, os.path.join(common.REPO, *MODEL.split('/')), STATE['model'],
                       'tools.studio_export.fighter_flute')
    m3refs.finish(folder)


FOUND = {'fighter_flute': (50, fake_export, 'm3refs_checks')}


def made(cache=b'abc', commit=b'{"v": 1}\n', model=b'{"m": 1}\n'):
    """the scratch folder made once by an export run with these contents"""
    STATE.update(cache=cache, commit=commit, model=model)
    m3refs.set_run(m3refs.Run('export'))
    fake_export()
    m3refs.set_run(None)


def read(rel):
    return open(os.path.join(common.REPO, *rel.split('/')), 'rb').read()


def check_pins_names_the_command():
    with scratch():
        made()
        path = os.path.join(m3refs.cache_root(), 'fighters', 'flute', 'right.bfrec')
        assert m3refs.check_pins('fighters/flute') == 1
        pins = m3refs.read_pins('fighters/flute')
        assert list(pins) == ['fighters/flute/right.bfrec'] and pins['fighters/flute/right.bfrec']['bytes'] == 3
        assert entries_of('studio/reference/fighters/flute/manifest.json') == [
            MODEL, 'studio/reference/fighters/flute/fight.json', 'studio/reference/fighters/flute/pins.json'], \
            'the folder\'s committed files, its model and pins.json are all in its own shard'
        assert entries_of('studio/reference/manifest.json') == [], 'the top-level manifest is untouched'
        os.remove(path)
        text = exits(m3refs.check_pins, 'fighters/flute')
        assert 'missing  studio/reference/.cache/m3/fighters/flute/right.bfrec' in text, text
        assert 'run: python -m tools.studio_export cache fighters/flute' in text, text
        common.write_bytes(path, b'abd')
        text = exits(m3refs.check_pins, 'fighters/flute')
        assert 'changed  studio/reference/.cache/m3/fighters/flute/right.bfrec' in text, text
        assert 'run: python -m tools.studio_export cache fighters/flute' in text, text
    return 'check_pins stops naming the file and `cache fighters/flute` on a missing and on a changed cache file; ' \
           'the model\'s entry is in its fighter\'s shard'


def a_pin_never_moves_on_its_own():
    with scratch():
        made()
        before = {r: read(r) for r in (MODEL, 'studio/reference/fighters/flute/fight.json',
                                       'studio/reference/fighters/flute/pins.json')}
        STATE.update(cache=b'abd', commit=b'{"v": 2}\n')
        text = exits(m3refs.run_folder, 'fighters/flute', 'cache', FOUND)
        assert 'differs from its pin' in text and 'remake fighters/flute' in text, text
        m3refs.set_run(m3refs.Run('export'))              # an export run stops too, and writes nothing committed
        text = exits(fake_export)
        assert 'differs from its pin' in text, text
        assert {r: read(r) for r in before} == before, 'a stopped run changed a committed file'
        m3refs.set_run(None)
    with scratch():
        made()
        pins = m3refs.read_pins('fighters/flute')
        pins['fighters/flute/old.bfrec'] = dict(path='fighters/flute/old.bfrec', sha256='0' * 64, bytes=1,
                                                export='fighter_flute')
        m3refs.write_pins('fighters/flute', pins)
        text = exits(m3refs.run_folder, 'fighters/flute', 'cache', FOUND)
        assert 'pinned but no longer made: fighters/flute/old.bfrec' in text, text
        m3refs.set_run(m3refs.Run('cache'))              # a cache run refuses a file it has no pin for
        text = exits(m3refs.cache_bytes, 'fighters/flute', 'new.json', b'3\n', 'fighter_flute')
        assert 'not pinned' in text and 'remake fighters/flute' in text, text
        m3refs.set_run(None)
    return 'a changed regenerated file stops a cache and an export run, which write nothing committed; a stale or ' \
           'unpinned file stops a cache run'


def remake_makes_a_folder_again_whole():
    with scratch():
        made()
        STATE.update(cache=b'new cache bytes', commit=b'{"v": 2}\n', model=b'{"m": 2}\n')
        exits(m3refs.run_folder, 'fighters/flute', 'cache', FOUND)          # the spec changed: cache refuses
        pins = m3refs.read_pins('fighters/flute')
        pins['fighters/flute/old.bfrec'] = dict(path='fighters/flute/old.bfrec', sha256='0' * 64, bytes=1,
                                                export='fighter_flute')
        m3refs.write_pins('fighters/flute', pins)                             # and a pin it no longer makes
        assert m3refs.run_folder('fighters/flute', 'remake', FOUND) == 1   # one typed command
        assert read('studio/reference/fighters/flute/fight.json') == b'{"v": 2}\n' and read(MODEL) == b'{"m": 2}\n'
        assert list(m3refs.read_pins('fighters/flute')) == ['fighters/flute/right.bfrec']
        assert m3refs.run_folder('fighters/flute', 'cache', FOUND) == 1    # and the folder reproduces itself
        n, problems, _ = manifest.verify()
        assert problems == [] and n == 3, (n, problems)
        assert 'name your own folder' in exits(m3refs.remake_cmd, ['all'], FOUND)
    return 'after a spec change (a committed file, a pinned file, a stale pin) `remake <folder>` makes the folder ' \
           'again in one run; then `cache` passes and verify is clean'


def a_cache_run_writes_no_committed_file():
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        m3refs.commit_bytes('fighters/oboe', 'fight.json', b'{}\n', 'tools.studio_export.fighter_oboe')
        p = m3refs.ref_dir('fighters/oboe', 'fight.json')
        assert not os.path.exists(p), 'an export run writes committed files only in finish()'
        m3refs.finish('fighters/oboe')
        assert open(p, 'rb').read() == b'{}\n'
        m3refs.set_run(m3refs.Run('cache'))
        m3refs.commit_bytes('fighters/oboe', 'fight.json', b'{}\n', 'tools.studio_export.fighter_oboe')
        text = exits(m3refs.commit_bytes, 'fighters/oboe', 'fight.json', b'{"a": 1}\n',
                     'tools.studio_export.fighter_oboe')
        assert 'would change' in text and 'remake fighters/oboe' in text, text
        assert open(p, 'rb').read() == b'{}\n'
        m3refs.set_run(None)
        try:
            m3refs.commit_path('fighters/oboe', os.path.join(common.REPO, *MODEL.split('/')), b'x', 't')
            raise AssertionError('FLUTE\'s model accepted into the oboe\'s folder')
        except ValueError:
            pass
    return 'committed writes wait for finish(); a cache run stops (writing nothing) when one would change; a ' \
           'model belongs to its own fighter\'s folder only'


def a_port_never_points_at_another_port():
    with scratch():
        for rel in ('studio/reference/fights/x/a.bfrec', 'studio/reference/fighters/tuba/left.bfrec',
                    'studio/reference/fighters/oboe/left.bfrec'):
            manifest.add(put(rel, rel.encode()), tool='check')
        sha = manifest.sha256
        assert m3refs.existing_copy(sha(os.path.join(common.REPO, 'studio/reference/fights/x/a.bfrec')),
                                    'fighters/oboe') == 'studio/reference/fights/x/a.bfrec'
        assert m3refs.existing_copy(sha(os.path.join(common.REPO, 'studio/reference/fighters/tuba/left.bfrec')),
                                    'fighters/oboe') is None, 'found in another port\'s shard'
        assert m3refs.existing_copy(sha(os.path.join(common.REPO, 'studio/reference/fighters/oboe/left.bfrec')),
                                    'fighters/oboe') == 'studio/reference/fighters/oboe/left.bfrec'
        # a remake that moves the left recording to the cache (delete_committed) no longer counts it as a copy:
        # what the remake writes in same_as is what a cache run after it finds
        m3refs.set_run(m3refs.Run('remake'))
        m3refs.delete_committed('fighters/oboe', 'left.bfrec')
        assert m3refs.existing_copy(sha(os.path.join(common.REPO, 'studio/reference/fighters/oboe/left.bfrec')),
                                    'fighters/oboe') is None, 'a file the run is deleting counted as a copy'
        m3refs.set_run(m3refs.Run('cache'))
        assert m3refs.existing_copy(sha(os.path.join(common.REPO, 'studio/reference/fighters/oboe/left.bfrec')),
                                    'fighters/oboe') == 'studio/reference/fighters/oboe/left.bfrec'
    return 'existing_copy looks in the top-level manifest and the folder\'s own shard, never another port\'s, and ' \
           'never at a file the run is deleting'


def budget_reports_an_overfull_fighter():
    files = [('fighters/flute/look.json', 210_000), ('fighters/flute/fight.json', 10_000),
             ('fighters/tuba/look.json', 211_000), ('fighters/tuba/fight.json', 10_000),
             ('fighters/euphonium/look.json', 290_000),
             ('themes/heist/bed.json', 50_000), ('themes/heist/theme.json', 70_000),
             ('themes/mambo/bed.json', 120_001),
             ('episodes/trombone_vs_tuba/checks.json', 255_000), ('episodes/oboe_vs_bassoon/checks.json', 100_001),
             ('older/vectors.json', 400_000),
             ('fights/flute_vs_trombone/ep392.bfrec', 146_000_000)]
    rows = {r['folder']: r for r in m3refs.budget_rows(files)}
    assert rows['fighters/flute']['bytes'] == 220_000 and rows['fighters/flute']['limit'] == 220_000
    assert rows['fighters/tuba']['bytes'] == 221_000 > rows['fighters/tuba']['limit'], rows['fighters/tuba']
    assert rows['fighters/euphonium']['limit'] == 300_000, 'merged at 227 KB, before the milestone 4 budget'
    assert rows['themes/heist']['limit'] == 120_000 >= rows['themes/heist']['bytes']
    assert rows['themes/mambo']['bytes'] > rows['themes/mambo']['limit'] == 120_000
    assert rows['episodes/trombone_vs_tuba']['limit'] == 260_000
    assert rows['episodes/oboe_vs_bassoon']['bytes'] > rows['episodes/oboe_vs_bassoon']['limit'] == 100_000
    assert rows['older'] == dict(folder='older', bytes=400_000, limit=400_000, files=1), rows['older']
    assert 'fights/flute_vs_trombone/ep392.bfrec' not in rows, 'milestone 1\'s fights may commit recordings'
    total = rows['studio/reference']
    assert total['limit'] == 147_000_000 and total['bytes'] == sum(b for _, b in files) > total['limit']
    over = sorted(r['folder'] for r in rows.values() if r['bytes'] > r['limit'])
    assert over == ['episodes/oboe_vs_bassoon', 'fighters/tuba', 'studio/reference', 'themes/mambo'], over
    names = [r['folder'] for r in m3refs.budget_rows(files)]
    assert names[:-1] == sorted(names[:-1]) and names[-1] == 'studio/reference', names
    return 'budget: a 221 KB fighter folder is over, 220 KB is not (EUPHONIUM keeps 300 KB); themes 120 KB, ' \
           'older/ 400 KB, episode and total limits as plan m4 section 4'


def _budget_of(files):
    """(stdout, the SystemExit text or None) of the `budget` command over `files` instead of git's list"""
    saved = m3refs.tracked_files
    m3refs.tracked_files = lambda: sorted(files)
    out = io.StringIO()
    try:
        with contextlib.redirect_stdout(out):
            m3refs.budget_cmd([])
        return out.getvalue(), None
    except SystemExit as ex:
        return out.getvalue(), str(ex)
    finally:
        m3refs.tracked_files = saved


def budget_refuses_a_committed_recording():
    """plan m4 Task 1: a .bfrec committed in a fighter, theme or episode folder (anywhere outside milestone 1's
    fights/, m3refs.COMMITTED_RECORDINGS) fails `budget` naming the file, however small the folder"""
    assert m3refs.COMMITTED_RECORDINGS == ('fights',), m3refs.COMMITTED_RECORDINGS
    base = [('fights/flute_vs_trombone/ep392.bfrec', 146_000), ('themes/heist/theme.json', 1_000)]
    out, err = _budget_of(base)
    assert err is None and 'OVER' not in out, (out, err)
    for rel in ('fighters/tuba/left.bfrec', 'themes/heist/oboe_vs_bassoon.bfrec', 'episodes/oboe_vs_bassoon/run.bfrec',
                'shared/x.bfrec'):
        out, err = _budget_of(base + [(rel, 10)])
        assert err is not None and rel in err, (rel, err)
        assert 'committed recordings outside fights/' in out and rel in out, out
        row = next(r for r in m3refs.budget_rows(base + [(rel, 10)]) if r['folder'] == rel)
        assert row['limit'] == 0, row
    return 'a committed .bfrec outside fights/ (a fighter, theme, episode or other folder) fails budget by name; ' \
           'fights/ may commit them'


def budget_limits_origin_json():
    """plan m4 section 4: Gate O's origin.json in a fighter or theme folder is at most 32 KB"""
    files = [('fighters/viola/origin.json', 32_000), ('themes/surf_rock/origin.json', 32_001),
             ('episodes/viola_vs_violin/origin.json', 32_001)]
    rows = {r['folder']: r for r in m3refs.budget_rows(files)}
    assert rows['fighters/viola/origin.json']['limit'] == 32_000 >= rows['fighters/viola/origin.json']['bytes']
    assert rows['themes/surf_rock/origin.json']['bytes'] > rows['themes/surf_rock/origin.json']['limit']
    assert 'episodes/viola_vs_violin/origin.json' not in rows, 'only fighter and theme folders hold one'
    out, err = _budget_of(files)
    assert err is not None and 'themes/surf_rock/origin.json (32,001 > 32,000)' in err, err
    assert 'fighters/viola/origin.json' not in err, err
    return 'an origin.json of 32,001 bytes fails budget by name, 32,000 passes'


def the_tripwire_warns():
    """plan m4 section 4: past 145.5 MB `budget` prints the warning and still passes; it fails only past 147 MB"""
    assert m3refs.TRIPWIRE == 145_500_000
    assert m3refs.tripwire_warning(145_500_000) is None
    want = 'warning: the reference set is 145.60 MB, past the 145.5 MB tripwire (plan m4 Section 4)'
    assert m3refs.tripwire_warning(145_600_000) == want, m3refs.tripwire_warning(145_600_000)
    out, err = _budget_of([('fights/flute_vs_trombone/ep392.bfrec', 145_600_000)])
    assert err is None and want in out, (out, err)
    out, err = _budget_of([('fights/flute_vs_trombone/ep392.bfrec', 145_400_000)])
    assert err is None and 'tripwire' not in out, (out, err)
    out, err = _budget_of([('fights/flute_vs_trombone/ep392.bfrec', 147_000_001)])
    assert err is not None and 'studio/reference' in err and 'tripwire' in out, (out, err)
    return '145.6 MB prints the tripwire warning and passes; 145.4 MB is silent; 147 MB + 1 byte fails'


def recordings_go_to_the_cache_by_default():
    """plan m4 Task 1: a fighter, theme or pair SPEC with no storage key keeps its kit recording in the pinned cache
    (left_in_cache, recordings_in_cache, recording_in_cache default to True in code), so no new spec can forget it;
    FLUTE and TROMBONE say False (their left fights are milestone 1's committed recordings, pointed at)"""
    import inspect
    from tools.studio_export import fighter_flute, fighter_trombone
    assert m3refs.normalize(dict(name='TUBA', opponent='FLUTE'))['left_in_cache'] is True
    assert m3refs.normalize(dict(name='TUBA', opponent='FLUTE', fights=dict(stats=10)))['left_in_cache'] is True
    for spec in (fighter_flute.SPEC, fighter_trombone.SPEC):
        assert spec.get('left_in_cache') is False and 'left_in_cache' not in m3refs.normalize(spec), spec
    assert m3refs.theme_spec(dict(name='x'))['recordings_in_cache'] is True
    assert m3refs.episode_spec(dict(config='examples/x'))['recording_in_cache'] is True
    assert m3refs.episode_spec(dict(config='examples/x', recording_in_cache=False))['recording_in_cache'] is False
    assert 'True or False' in exits(m3refs.episode_spec, dict(config='x', recording_in_cache=1))
    assert 'unknown keys' in exits(m3refs.episode_spec, dict(config='x', recordings_in_cache=True))
    src = inspect.getsource(m3refs.episode_refs)
    assert 'spec = episode_spec(spec)' in src and "spec['recording_in_cache']" in src
    assert 'episode_spec(_spec_of(folder))' in inspect.getsource(m3refs.episode_drift)
    src = inspect.getsource(m3refs.fighter_refs)
    assert 'spec = normalize(spec)' in src and "spec.get('left_in_cache')" in src
    assert 'theme_spec(spec)' in inspect.getsource(m3refs.theme_refs)
    return 'a fighter, theme and pair SPEC with no storage key put the recording in the pinned cache; FLUTE and ' \
           'TROMBONE say False'


def the_command_line_parses():
    with scratch():
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            entry.main(['cache', 'all'])                  # nothing pinned in the scratch set: nothing to do
        assert out.getvalue().startswith('[cache: ') and '/' not in out.getvalue(), out.getvalue()
        assert 'unknown option --force' in exits(entry.main, ['cache', 'fighters/flute', '--force'])
        assert 'remake <folder>' in exits(entry.main, ['cache', 'fighters/flute', '--repin'])
        assert 'name your own folder' in exits(entry.main, ['remake', 'all'])
        assert 'all stands alone' in exits(entry.main, ['cache', 'all', 'fighters/flute'])
        assert 'not a reference folder' in exits(entry.main, ['cache', 'flute'])
    found = entry.exports()
    takes = getattr(sys.modules[found['cache'][2]], 'TAKES_ARGS', ())
    assert {'cache', 'remake', 'budget'} <= set(takes), 'all must leave them out'
    return '`cache all` and `remake <folder>` parse; a bad option or folder, `--repin` and `remake all` are ' \
           'refused; all leaves cache, remake and budget out'


def the_framework_records_milestone_1s_fights():
    """real kit: a Config with the example's own settings gives the committed milestone 1 recordings, bytes and all"""
    m3refs.set_run(None)
    cfg = m3refs.Config('m3refs_checks', 'flute_vs_trombone', ('FLUTE', 'TROMBONE'), (2.0, 0.40), 1.0,
                        m3refs.pair_rules('FLUTE', 'TROMBONE'))
    tmp = os.path.join(common.CACHE, 'm3refs_checks')                # (inside the repo: read_bfrec names paths)
    os.makedirs(tmp, exist_ok=True)
    try:
        for seed in (1, 2, 3):
            out = os.path.join(tmp, f'seed{seed}.bfrec')
            m3refs.record(cfg, seed, {}, out)
            want = common.ref('fights', 'flute_vs_trombone', f'seed{seed}.bfrec')
            assert open(out, 'rb').read() == open(want, 'rb').read(), f'seed {seed}: not the milestone 1 file'
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return 'with the example\'s own settings the framework records fights/flute_vs_trombone/seed{1,2,3}.bfrec byte ' \
           'for byte'


# ---------------------------------------------------------------- the framework fix after ORGAN (Task 9)
# SHA-256 of FLUTE's left hull lines (m3parts.hull_lines of fighters/flute/look.json's rows); bf-harness's
# `hull_lines_are_the_exports` checks the same number from the same rows, so both sides write the lines alike
FLUTE_LEFT_HULL_SHA = '8ba8b6f534bbd307eac7d709cc34cf967ae4925c1a13e68f27fd3bfee0b3cadb'


def look_parts_of(folder):
    """the two look_child results a committed format 1 look.json was made from, rebuilt from it (the right fight's
    rows are in the cache, which this check does not need: the left rows stand in for them)"""
    import json
    with open(m3refs.ref_dir(folder, 'look.json'), encoding='utf-8') as fh:
        look = json.load(fh)
    left = {k: v for k, v in look['left'].items() if k != 'file'}
    right = dict(k=1, frames=left['frames'][:look['right']['frames']], seed=look['right']['seed'],
                 recording=look['right']['recording'], cards=look['cards']['right'])
    look_l = dict(doc=left, cfg_keys=look['cfg_keys'], title=look['title'],
                  sprites={k: v for k, v in look['sprites'].items() if k != 'cache'})
    return look, look_l, dict(doc=right)


def hull_rows_go_to_the_cache():
    """ruling M3-R29: FLUTE's look.json is rebuilt byte for byte in format 1; any other folder gets format 2"""
    from tools.studio_export import m3parts as MP
    look, look_l, look_r = look_parts_of('fighters/flute')
    assert look['format'] == 1 and m3refs.INLINE_HULL_ROWS == ('fighters/flute', 'fighters/trombone')
    again, _ = m3refs.look_docs('fighters/flute', 'FLUTE', 'TROMBONE', look['tool'], look['left']['file'], look_l,
                                look_r)
    with open(m3refs.ref_dir('fighters/flute', 'look.json'), 'rb') as fh:
        assert common.dumps(again, True).encode('utf-8') == fh.read(), 'format 1 is not FLUTE\'s look.json'
    rows = look['left']['frames']
    lines = MP.hull_lines(rows)
    assert lines[0].startswith(f'{rows[0][0]} 33 19,0,31,32,1,2,'), lines[0]
    assert len(lines) == 79, len(lines)
    doc, right = m3refs.look_docs('fighters/piano', 'FLUTE', 'TROMBONE', look['tool'], 'fighters/piano/left.bfrec',
                                  look_l, look_r)
    assert doc['format'] == 2 and len(doc['frame_columns']) == 7 and 'hull' not in ' '.join(doc['frame_columns'])
    hull = doc['left']['hull']
    assert hull == dict(ruling='M3-R29', cache='fighters/piano/hull_left.json', frames=79, calls=79, points=79 * 33,
                        sha256=FLUTE_LEFT_HULL_SHA, line=MP.HULL_LINE), hull
    assert right['hull']['cache'] == 'fighters/piano/hull_right.json'
    assert right['hull']['frames'] == len(right['frames'])
    for r in doc['left']['frames'] + right['frames']:
        assert len(r) == 7 and isinstance(r[6], str), r                 # frame .. boxes' SHA-256, why
    assert [r[:6] for r in doc['left']['frames']] == [r[:6] for r in rows]
    assert look_l['doc']['frames'] is rows and len(rows[0]) == 8, 'look_docs changed its input'
    big = len(common.dumps(look, True)) - len(common.dumps(doc, True))
    return f'FLUTE\'s look.json comes back byte for byte in format 1; format 2 drops each row\'s hull calls ({big} ' \
           f'bytes for FLUTE\'s 79) for their counts and one SHA-256'


def the_left_file_is_on_disk_for_the_parts():
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        folder = 'fighters/oboe'
        path = m3refs.commit_bytes(folder, 'left.bfrec', b'new\n', 'tools.studio_export.fighter_oboe')
        assert not os.path.exists(path)
        with m3refs.staged_on_disk(folder, path):
            assert open(path, 'rb').read() == b'new\n', 'the parts cannot read the staged left.bfrec'
        assert open(path, 'rb').read() == b'new\n'
        os.remove(path)
        try:                                              # a run that stops takes the file away again
            with m3refs.staged_on_disk(folder, path):
                raise SystemExit('a part stopped')
        except SystemExit:
            pass
        assert not os.path.exists(path), 'a stopped run left a committed file behind'
        common.write_bytes(path, b'old\n')                  # ... or puts the old bytes back (a remake)
        try:
            with m3refs.staged_on_disk(folder, path):
                assert open(path, 'rb').read() == b'new\n'
                raise SystemExit('a part stopped')
        except SystemExit:
            pass
        assert open(path, 'rb').read() == b'old\n'
        other = put('studio/reference/fights/x/seed3.bfrec', b'm1\n')     # a pointer: nothing staged there
        with m3refs.staged_on_disk(folder, other):
            assert open(other, 'rb').read() == b'm1\n'
        m3refs.set_run(None)
    import inspect
    src = inspect.getsource(m3refs.fighter_refs)
    assert src.index('with staged_on_disk(') < src.index('fighter_parts(spec') < src.index('finish(folder)'), \
        'fighter_refs does not run the parts with the left file on disk'
    return 'a staged left.bfrec is on disk for the parts (and finish), put back as it was if the run stops'


def the_replay_guard_compares_values():
    """real kit: seed 1 of the example with every float of every event turned into a numpy float64 (what ORGAN's
    `_to_ropes` leaves in a `line` event) records the milestone 1 file byte for byte and passes the guard"""
    import numpy as np
    from tools.studio_export import fights
    m3refs.set_run(None)
    cfg = m3refs.Config('m3refs_checks', 'flute_vs_trombone', ('FLUTE', 'TROMBONE'), (2.0, 0.40), 1.0,
                        m3refs.pair_rules('FLUTE', 'TROMBONE'))
    _, story = cfg.load()
    kit_run_seed = story.run_seed

    def numpy_floats(seed, **kw):
        r = kit_run_seed(seed, **kw)
        r['events'] = [tuple(np.float64(v) if type(v) is float else v for v in e) for e in r['events']]
        return r
    plain = kit_run_seed(1)
    wrapped = numpy_floats(1)
    assert 'np.float64(' in repr(wrapped['events']) and fights.replay_hash(wrapped) != fights.replay_hash(plain)
    assert fights.replay_hash_of_values(wrapped) == fights.replay_hash_of_values(plain) == fights.replay_hash(plain)
    tmp = os.path.join(common.CACHE, 'm3refs_checks')
    os.makedirs(tmp, exist_ok=True)
    story.run_seed = numpy_floats
    try:
        out = os.path.join(tmp, 'seed1.bfrec')
        m3refs.record(cfg, 1, {}, out)
        want = common.ref('fights', 'flute_vs_trombone', 'seed1.bfrec')
        assert open(out, 'rb').read() == open(want, 'rb').read(), 'the numpy floats changed the recorded bytes'
    finally:
        story.run_seed = kit_run_seed
        shutil.rmtree(tmp, ignore_errors=True)
    x = np.float32(0.1)
    assert fights.plain_scalars((x, np.int64(3), np.bool_(True), [np.float64(-0.0)])) == (float(x), 3, True, [-0.0])
    assert repr(fights.plain_scalars((np.float64(-0.0),))) == '(-0.0,)'
    try:
        fights.plain_scalars([np.complex128(1j)])
        raise AssertionError('a numpy complex was taken')
    except TypeError:
        pass
    return 'numpy floats in the kit\'s events pass the replay guard (values compared, bits unchanged) and record ' \
           'milestone 1\'s seed1.bfrec byte for byte; a run without them hashes as before'


def the_checks_fight_ends_after_44_s():
    sr = 48000
    assert not m3refs.checks_can_run(44.0) and not m3refs.checks_can_run(30.5) and not m3refs.checks_can_run(None)
    assert m3refs.checks_can_run(44.0 + 1.0 / sr) and m3refs.checks_can_run(47.0)
    abilities = ['A', 'B', 'C']
    items = ['ability/A', 'ability/B', 'ability/C']
    got = [(dict(seed=1, winner=0, t_ko=30.0), items), (dict(seed=2, winner=1, t_ko=50.0), items),
           (dict(seed=3, winner=0, t_ko=44.00001), items), (dict(seed=4, winner=0, t_ko=48.0), items[:2]),
           (dict(seed=5, winner=0, t_ko=47.0), items)]
    assert m3refs.choose_left('fighter_x', 'X', abilities, got, None) == (5, [1, 3])
    assert m3refs.choose_left('fighter_x', 'X', abilities, got, 2) == (2, [])
    assert m3refs.early_note([]) == '' and 'seeds 1, 3' in m3refs.early_note([1, 3])
    text = exits(m3refs.choose_left, 'fighter_x', 'X', abilities, got[:4], None)
    assert 'after 44 s' in text and 'music.main()' in text and 'fights.left' in text, text
    text = exits(m3refs.choose_left, 'fighter_x', 'X', abilities, got[1:2], None)
    assert 'all three abilities' in text and '44' not in text, text
    text = m3refs.checks_too_early('fighter_x', 12, 30.0)
    assert 'fights.left = 12' in text and 'before 44 s' in text, text
    import inspect
    src = inspect.getsource(m3refs.fighter_refs)
    assert src.index('checks_can_run(rec_left') < src.index('fighter_parts(spec'), 'a fixed left seed is not checked'
    return 'the left fight is the lowest with a K.O. after 44 s (seeds passed over are named); a fixed early one ' \
           'or none at all stops with what the kit\'s checks need'


class _Look:
    def __init__(self, own):
        self.own = own

    def sprite(self, r, yaw, roll):
        return object() if self.own else None

    def sprite_rolls(self, k, roll0):
        return []


class _P:
    NAMES = ('TRUMPET',)
    CFG = [dict(view=(0, 0, 0))]

    def __init__(self, own=True):
        self.LOOKS = [_Look(own)]


def a_port_declares_its_sprite_size():
    from tools.studio_export import fighter_flute, fighter_trombone, sprites
    rule = dict(add=7, clip=(12, 19))
    assert [sprites.own_size(None, r) for r in (12, 30)] == [29, 65]
    assert [sprites.own_size(rule, r) for r in (8, 12, 16, 30)] == [31, 31, 39, 45]
    assert sprites.own_size(dict(add=3, clip=None), 10) == 23

    def cases(size_of):
        out = [dict(id=f'c{i}', args=dict(group='picture', k=0, radius=r, yaw=y, roll=0, size=size_of(r)))
               for i, (r, y) in enumerate((r, y) for r in sprites.RADII for y in sprites.YAWS)]
        return out + [dict(id='t', args=dict(group='title', k=0, radius=44, drawn_by='closeup', size=99))]
    trumpet = cases(lambda r: 2 * min(19, max(12, r)) + 7)
    sprites.check_cases(_P(), 0, trumpet, set(), rule)
    refused = False
    try:
        sprites.check_cases(_P(), 0, trumpet, set())            # without the rule: FLUTE's 2 r + 5
    except AssertionError:
        refused = True
    assert refused, 'the TRUMPET sizes passed as 2 r + 5'
    sprites.check_cases(_P(), 0, cases(lambda r: 2 * r + 5), set())
    sprites.check_cases(_P(own=False), 0, cases(lambda r: 2 * r + 7), set(), rule)   # the model's: rule unused
    for mod in (fighter_flute, fighter_trombone):
        assert 'sprite_size' not in m3refs.normalize(mod.SPEC), 'fight.json\'s spec would change'
    got = m3refs.normalize(dict(name='TRUMPET', opponent='FLUTE', sprite_size=dict(add=7, clip=[12, 19])))
    assert got['sprite_size'] == dict(add=7, clip=(12, 19)), got
    for bad in (dict(add=7.0), dict(add=7, clip=(19, 12)), dict(add=7, radius=3), 7):
        exits(m3refs.normalize, dict(name='TRUMPET', opponent='FLUTE', sprite_size=bad))
    import inspect
    assert "spec.get('sprite_size')" in inspect.getsource(m3refs.fighter_parts)
    return 'a port declares its look\'s sprite size (TRUMPET: 2 clip(r, 12, 19) + 7) and the check asks exactly ' \
           'that; FLUTE\'s and TROMBONE\'s specs, and so their fight.json, are unchanged'


def _fork_stand_in():
    """(in a part's fresh process) a fork pool maps here, in order; spawn is still the real one"""
    import multiprocessing as mp
    with mp.get_context('fork').Pool(2) as pool:
        got = pool.map(abs, [-3, 2, -1], chunksize=8)
    assert got == [3, 2, 1] and type(mp.get_context('spawn')).__name__ == 'SpawnContext'
    return got


def _trumpet_atlas(cfg):
    """(in a part's fresh process) the TRUMPET look's atlas built by the kit through the fork stand-in, against the
    kit's own serial branch (what its render workers do) and the file it saves"""
    import numpy as np
    P = m3refs.fighter_picture(cfg)
    from ballfight.kit.fighters import trumpet as KT
    look = P.LOOKS[list(P.NAMES).index('TRUMPET')]
    KT._ATLAS.clear()
    path = os.path.join(P.EP.check, 'sprites_trumpet_aim.npz')
    if os.path.exists(path):
        os.remove(path)
    atlas = dict(look.atlas())
    yk = int(round(look.cfg['view'][0] / 6.0)) * 6 % 360
    jobs = [(look.cfg['pp'], r, yk, rl) for r in KT.SPR_RADII for rl in range(-180, 180, KT.ROLL_STEP)]
    serial = dict(map(KT._sprite_job, jobs))
    assert list(atlas) == list(serial) and all(np.array_equal(atlas[k], serial[k]) for k in serial)
    z = np.load(path)
    assert z.files == [f'{r}_{rl}' for r, rl in serial]
    assert all(np.array_equal(z[f'{r}_{rl}'], v) for (r, rl), v in serial.items())
    return len(atlas)


def windows_has_a_fork_stand_in():
    """every part's fresh process (m3parts._child) maps a fork pool serially; the TRUMPET's atlas is the kit's"""
    import inspect
    from tools.studio_export import m3parts as MP
    assert MP.fresh(_fork_stand_in) == [3, 2, 1]
    cfg = m3refs.Config('m3refs_checks_trumpet', 'flute_vs_trombone', ('TRUMPET', 'FLUTE'), (1.0, 1.0), 1.0, {})
    n = MP.fresh(_trumpet_atlas, cfg.with_seed(1))
    assert 'serial_fork()' in inspect.getsource(MP._child) and 'serial_fork()' in inspect.getsource(MP._checks_init)
    assert 'build_atlases(P)' in inspect.getsource(MP.checks_child)
    return f'a part\'s process maps a fork pool in order (spawn untouched); the TRUMPET\'s {n}-sprite atlas through ' \
           f'it equals the kit\'s own serial build, in the file it saves too'


class _Rebinds:
    """made up: two methods bind `ab` to two entries, one rebinds it to something else, a nested function reads
    its parent's name, and a read before a binding is not the alias"""

    def one(self):
        ab = self.cfg['abilities']['unlock1']
        return ab['half'] + ab['reach']

    def two(self):
        x = ab['ghost']                              # noqa: F821  (not bound here yet: no key)
        ab = self.cfg['abilities']['unlock2']
        y = ab['len']
        ab = 3
        return x + y + ab['not_a_key']

    def three(self):
        w = self.cfg.get('weapon')

        def inner():
            return w['hold']
        return inner()


def _kit_cfg_keys(names):
    """(in a part's fresh process) the kit look classes' cfg_keys"""
    from tools.studio_export import m3parts as MP
    cfg = m3refs.Config('m3refs_checks_keys', 'flute_vs_trombone', ('FLUTE', 'TROMBONE'), (2.0, 0.40), 1.0,
                        m3refs.pair_rules('FLUTE', 'TROMBONE'))
    cfg.load()
    from ballfight.kit import fighters as FM
    return {n: MP.cfg_keys(FM.module(n).Look) for n in names}


def cfg_keys_follow_each_function():
    import json
    from tools.studio_export import m3parts as MP
    got = MP.cfg_keys(_Rebinds)
    assert got == ['abilities.unlock1.half', 'abilities.unlock1.reach', 'abilities.unlock2.len', 'weapon.hold'], got
    kit = MP.fresh(_kit_cfg_keys, ('OBOE', 'FLUTE', 'TROMBONE'))
    oboe = kit['OBOE']
    assert {'abilities.unlock1.half', 'abilities.unlock1.reach', 'abilities.unlock2.len', 'abilities.unlock2.width',
            'weapon.hold'} <= set(oboe) and 'abilities.unlock2.half' not in oboe, oboe
    for name in ('FLUTE', 'TROMBONE'):
        with open(m3refs.ref_dir(f'fighters/{name.lower()}', 'look.json'), encoding='utf-8') as fh:
            listed = json.load(fh)['cfg_keys']
        assert kit[name] == listed, (name, kit[name], listed)
    return f"an alias holds in its own function from its binding on; OBOE's keys are {', '.join(oboe)}; FLUTE's " \
           f"and TROMBONE's are look.json's as committed"


def a_padded_sprite_is_declared_too():
    from tools.studio_export import sprites
    pad = dict(add=7, pad=1.22)                                    # SOUSAPHONE (kit/fighters/sousaphone.py PAD)
    assert [sprites.own_size(pad, r) for r in (12, 16, 22, 30)] == [2 * int(round(r * 1.22)) + 7
                                                                     for r in (12, 16, 22, 30)] == [37, 47, 61, 81]
    assert sprites.own_size(dict(add=7, clip=(12, 19), pad=2.0), 30) == 2 * 38 + 7

    def cases(size_of):
        out = [dict(id=f'c{i}', args=dict(group='picture', k=0, radius=r, yaw=y, roll=0, size=size_of(r)))
               for i, (r, y) in enumerate((r, y) for r in sprites.RADII for y in sprites.YAWS)]
        return out + [dict(id='t', args=dict(group='title', k=0, radius=44, drawn_by='closeup', size=99))]
    padded = cases(lambda r: 2 * int(round(r * 1.22)) + 7)
    sprites.check_cases(_P(), 0, padded, set(), pad)
    for rule in (None, dict(add=7), dict(add=7, pad=1.25)):          # FLUTE's, unpadded, CONTRABASS's pad
        refused = False
        try:
            sprites.check_cases(_P(), 0, padded, set(), rule)
        except AssertionError:
            refused = True
        assert refused, rule
    got = m3refs.normalize(dict(name='SOUSAPHONE', opponent='FLUTE', sprite_size=dict(add=7, pad=1.22)))
    assert got['sprite_size'] == dict(add=7, clip=None, pad=1.22), got
    got = m3refs.normalize(dict(name='TRUMPET', opponent='FLUTE', sprite_size=dict(add=7, clip=(12, 19))))
    assert got['sprite_size'] == dict(add=7, clip=(12, 19)), 'a rule without pad is written as before'
    for bad in (dict(add=7, pad=1), dict(add=7, pad=-1.2), dict(add=7, pad='1.22')):
        exits(m3refs.normalize, dict(name='SOUSAPHONE', opponent='FLUTE', sprite_size=bad))
    return 'a padded sprite\'s size is declared (SOUSAPHONE: dict(add=7, pad=1.22), 2 int(round(r 1.22)) + 7) and ' \
           'checked exactly: another pad or none is refused'


def _part_fails():
    raise ValueError('a part that stops')


def _part_sleeps():
    import time
    time.sleep(300)


def _part_big():
    return b'x' * (64 * 1000 * 1000)                       # blocks handing it back until someone reads it


def _part_dies():
    os._exit(3)


def a_failed_part_ends_the_export_promptly():
    import time
    from tools.studio_export import m3parts as MP
    t0 = time.time()
    slow, big, bad = MP.start(_part_sleeps), MP.start(_part_big), MP.start(_part_fails)
    text = exits(MP.wait, slow)                            # waiting for the slow one: the failed one ends it all
    took = time.time() - t0
    assert '_part_fails (in a fresh process) stopped' in text and 'a part that stops' in text, text
    assert '_part_sleeps' in text.split('\n')[0], text
    assert not any(h[0].is_alive() for h in (slow, big, bad)) and MP._LIVE == [], 'a part still runs'
    assert took < 60, f'{took:.0f} s'
    text = exits(MP.wait, MP.start(_part_dies))
    assert 'without handing back a result' in text and 'exit code 3' in text, text
    a, b = MP.start(_part_big), MP.start(abs, -4)
    assert MP.wait(b) == 4 and len(MP.wait(a)) == 64 * 1000 * 1000 and MP._LIVE == []
    with scratch():                                         # and the staged left.bfrec goes as staged_on_disk says
        m3refs.set_run(m3refs.Run('export'))
        path = m3refs.commit_bytes('fighters/oboe', 'left.bfrec', b'new\n', 'tools.studio_export.fighter_oboe')

        def run():
            with m3refs.staged_on_disk('fighters/oboe', path):
                assert os.path.isfile(path)
                MP.wait(MP.start(_part_fails))
        assert 'a part that stops' in exits(run) and not os.path.exists(path)
        m3refs.set_run(None)
    return f'a failed part stops the others (one sleeping, one blocked on a 64 MB result) and the export in ' \
           f'{took:.0f} s, naming it; a part that dies says so; the staged left.bfrec is taken away'


def _piano_recording(seed):
    """(in a part's fresh process) PIANO's left fight recorded by the framework alone (no adapter): roll_rows,
    HOLD and the pitches inside its events go through write, read back, check and the replay guard; then the kit's
    renderer for it accepts the file (frames.check_recording)"""
    from tools.studio_export import frames as F
    cfg = m3refs.Config('m3refs_checks_piano', 'flute_vs_trombone', ('PIANO', 'FLUTE'), (1.0, 1.0), 1.0, {})
    tmp = os.path.join(common.CACHE, 'm3refs_checks')
    os.makedirs(tmp, exist_ok=True)
    out = os.path.join(tmp, f'piano_s{seed}.bfrec')
    rec = m3refs.record(cfg, seed, {}, out)
    kinds = {e[1] for e in rec['events']}
    held = sum(len(f) for f in rec['arrays']['HOLD'])
    rows = rec['rules']['roll_rows']
    P = m3refs.fighter_picture(cfg.with_seed(seed))
    F.check_recording(P, out)
    return sorted(kinds & {'roll', 'runoff'}), held, rows[0][0], len(rows)


def a_rolls_fight_goes_through_the_framework_alone():
    """real kit: PIANO's left fight (seed 41) without any of PIANO's adapters"""
    from tools.studio_export import fights, m3parts as MP
    assert common.plain({'roll_rows': {1: [(60, 64)], 0: [[48]]}, 'bpm': 132.0}) == \
        {'roll_rows': [[0, [[48]]], [1, [[60, 64]]]], 'bpm': 132.0}
    assert 'not a string' in str(_raises(TypeError, common.plain, {'a': 1, 2: 3}))
    rec = dict(seed=1, winner=0, t_ko=50.0, names=['A', 'B'], corner=[0, 1], f0=0,
               events=[[1.0, 'roll', 0, 1, 2.0, 3, [60, 64, 67]], [2.0, 'hit', 0, 1, 5.0]],
               arrays=dict(HOLD=[[[1, 2.0, 3.0]], []]))
    run = fights.as_kit_run(rec)
    assert run['events'][0] == (1.0, 'roll', 0, 1, 2.0, 3, (60, 64, 67)) and run['HOLD'] == [[(1, 2.0, 3.0)], []]
    assert 'RAGGED_DEPTH' in exits(fights.as_kit_run, dict(rec, arrays=dict(NEW=[[1]])))
    kinds, held, first, n = MP.fresh(_piano_recording, 41)
    assert kinds == ['roll', 'runoff'] and held > 0 and first == 0, (kinds, held, first)
    shutil.rmtree(os.path.join(common.CACHE, 'm3refs_checks'), ignore_errors=True)
    return f'PIANO\'s seed 41 ({", ".join(kinds)} events, {held} HOLD entries, roll_rows for {n} side(s)) records, ' \
           f'reads back, passes the replay guard and the renderer\'s check with the framework alone'


def _raises(kind, fn, *args):
    try:
        fn(*args)
    except kind as ex:
        return ex
    raise AssertionError(f'{fn.__name__} did not raise {kind.__name__}')


def _neutral_both_ways(fighters, seeds):
    """(in a part's fresh process) the same fights with DMG_SIDE = (1.0, 1.0) (config_text) and DMG_SIDE = None"""
    from tools.studio_export import fights
    tag = 'm3refs_checks_neutral_' + '_'.join(m3refs.slug(f) for f in fighters)
    text = m3refs.config_text('flute_vs_trombone', fighters, (1.0, 1.0), 1.0, {})
    assert text.count('DMG_SIDE = (1.0, 1.0)\n') == 1
    folders = (m3refs.config_folder(tag, 'flute_vs_trombone', text),
               m3refs.config_folder(tag + '_none', 'flute_vs_trombone',
                                    text.replace('DMG_SIDE = (1.0, 1.0)\n', 'DMG_SIDE = None\n')))
    runs, rules = {}, {}
    for which, folder in zip(('tuple', 'none'), folders):
        _, story = m3refs.load_config(folder)
        rules[which] = dict(vars(story.rules()))
        runs[which] = [story.run_seed(s) for s in seeds]
    assert rules['tuple'].pop('dmg_side') == (1.0, 1.0) and 'dmg_side' not in rules['none'], rules['none']
    assert rules['tuple'] == rules['none'], 'another rule differs'
    for a, b in zip(runs['tuple'], runs['none']):
        assert sorted(a) == sorted(b) and fights.replay_hash_of_values(a) == fights.replay_hash_of_values(b), a['seed']
        common.same(common.plain(a['events']), common.plain(b['events']), f"seed {a['seed']} events")
    return len(seeds)


def a_neutral_handicap_is_the_same_fight_either_way():
    """real kit: DMG_SIDE = (1.0, 1.0) and DMG_SIDE = None give bit-identical fights (rules apart from dmg_side)"""
    from tools.studio_export import m3parts as MP
    n = MP.fresh(_neutral_both_ways, ('PIANO', 'FLUTE'), tuple(range(1, 21)) + (41,))
    m = MP.fresh(_neutral_both_ways, ('FLUTE', 'TROMBONE'), tuple(range(1, 11)))
    return f'with DMG_SIDE (1.0, 1.0) and None the kit\'s fights are bit-identical ({n} PIANO and {m} FLUTE seeds: ' \
           f'events, every per-frame value, the replay guard); only the rule dmg_side differs'


def a_big_left_recording_goes_to_the_cache():
    """ruling M3-R36: SPEC.left_in_cache; fight.json then names the pinned cache file and its SHA-256"""
    import inspect
    from tools.studio_export import fighter_flute
    assert 'left_in_cache' not in m3refs.normalize(fighter_flute.SPEC), 'fight.json\'s spec would change'
    assert m3refs.normalize(dict(name='PIANO', opponent='FLUTE', left_in_cache=True))['left_in_cache'] is True
    assert 'left_in_cache' not in m3refs.normalize(dict(name='PIANO', opponent='FLUTE', left_in_cache=False))
    exits(m3refs.normalize, dict(name='PIANO', opponent='FLUTE', left_in_cache='yes'))
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        path = m3refs.cache_bytes('fighters/piano', 'left.bfrec', b'a roll\n', 'fighter_piano')
        left_file = m3refs.LEFT_IN_CACHE + m3refs.cache_rel(path)
        assert os.path.join(common.REFERENCE, *left_file.split('/')) == path, 'a part joins it to REF as before'
        ref = m3refs.left_ref(left_file)
        assert ref == dict(cache='fighters/piano/left.bfrec', sha256=manifest.sha256(path)), ref
        assert m3refs.left_path_of(ref) == path
        assert m3refs.left_ref('fights/x/seed3.bfrec') == dict(file='fights/x/seed3.bfrec')
        assert m3refs.left_path_of(dict(file='fights/x/seed3.bfrec')) == os.path.join(common.REFERENCE, 'fights', 'x',
                                                                                      'seed3.bfrec')
        m3refs.finish('fighters/piano')
        assert list(m3refs.read_pins('fighters/piano')) == ['fighters/piano/left.bfrec'], 'not pinned'
        assert not os.path.exists(m3refs.ref_dir('fighters/piano', 'left.bfrec')), 'a committed copy was written'
        m3refs.set_run(None)
    src = inspect.getsource(m3refs.fighter_refs)
    assert src.index("spec.get('left_in_cache')") < src.index("commit_bytes(folder, 'left.bfrec'")
    return 'SPEC.left_in_cache puts the left recording in the pinned cache (fight.json: cache and sha256), never ' \
           'a committed copy; specs without it are unchanged'


def a_left_file_finish_wrote_is_kept():
    """review fix: when finish() has applied the stage and check_pins then stops, staged_on_disk does not put the
    old left.bfrec back under the new fight.json"""
    with scratch():
        folder = 'fighters/oboe'
        old_left = put('studio/reference/fighters/oboe/left.bfrec', b'old\n')
        m3refs.set_run(m3refs.Run('export'))
        m3refs.commit_bytes(folder, 'left.bfrec', b'new\n', 'tools.studio_export.fighter_oboe')
        m3refs.commit_bytes(folder, 'fight.json', b'{"left": "new"}\n', 'tools.studio_export.fighter_oboe')
        kept = m3refs.check_pins

        def check_pins(f):
            raise SystemExit('check_pins stopped')
        m3refs.check_pins = check_pins
        try:
            try:
                with m3refs.staged_on_disk(folder, old_left):
                    m3refs.finish(folder)
            except SystemExit as ex:
                assert 'check_pins stopped' in str(ex), ex
        finally:
            m3refs.check_pins = kept
            m3refs.set_run(None)
        assert read('studio/reference/fighters/oboe/left.bfrec') == b'new\n', 'the old left.bfrec came back'
        assert read('studio/reference/fighters/oboe/fight.json') == b'{"left": "new"}\n'
    return 'a stop after finish() applied the stage leaves the new left.bfrec with the new fight.json'


def a_nan_rectangular_key_is_an_array():
    """BASSOON's HOOK (rectangular, NaN where no hook is out) is a .npy member through recording.rect_keys, NaN and
    all; through the kit's own list alone it would be JSON, which refuses the NaN; a ragged key stays JSON"""
    import types
    import numpy as np
    from tools.studio_export import recording
    _, story = common.kit()
    assert 'HOOK' not in story.REC_KEYS and 'HOP' not in story.REC_KEYS, 'the kit lists them now: drop KIT_RECT_KEYS'
    keys = recording.rect_keys(story)
    assert keys[:len(story.REC_KEYS)] == tuple(story.REC_KEYS) and {'HOOK', 'HOP'} <= set(keys)
    assert recording.rect_keys(types.SimpleNamespace(REC_KEYS=('P', 'HOOK'))) == ('P', 'HOOK', 'HOP')
    nan = float('nan')
    hook = [[[nan, nan, nan], [nan, nan, nan]], [[1.0, 0.5, 12.75], [nan, nan, nan]]]
    kind, data = recording._array('HOOK', hook, 2, keys)
    assert kind == 'npy', kind
    back = np.load(io.BytesIO(data))
    assert back.dtype == np.dtype('<f8') and back.shape == (2, 2, 3)
    assert np.array_equal(back, np.array(hook), equal_nan=True) and np.isnan(back[0]).all()
    try:
        recording._array('HOOK', hook, 2, tuple(story.REC_KEYS))
        raise AssertionError('HOOK went to JSON with its NaN')
    except ValueError as ex:
        assert 'nan' in str(ex), ex
    assert recording._array('HOOK', [[[1.0, 2.0, 3.0]], [[1.0, 2.0]]], 2, keys)[0] == 'json'
    return 'HOOK with NaN is a (frames, 2, 3) <f8 .npy member through rect_keys; the kit\'s list alone would put ' \
           'it in JSON, which refuses the NaN'


MOODS = ('baroque_storm', 'big_top', 'bossa_noir', 'brass_trailer', 'dark_disco', 'dark_drumline', 'dark_tango',
         'heist', 'horror_trap', 'hot_jazz', 'mambo', 'prog_duel', 'second_line', 'stadium_rock')


def _mood_source(mood):
    with open(os.path.join(common.REPO, 'projects', 'ballfight', 'kit', 'moods', f'{mood}.py'), encoding='utf-8') as f:
        return f.read()


def every_option_the_moods_read_is_listed():
    """m3refs.THEME_OPTIONS holds every config name a kit mood reads with getattr(T, '<NAME>', ...), under that
    mood, and every key a dict option's mood reads from it"""
    import re
    moods = sorted(n[:-3] for n in os.listdir(os.path.join(common.REPO, 'projects', 'ballfight', 'kit', 'moods'))
                   if n.endswith('.py') and n != '__init__.py')
    assert tuple(moods) == MOODS, moods
    read = {}
    for mood in moods:
        src = _mood_source(mood)
        for name in re.findall(r"getattr\((?:self\.)?(?:M\.)?T, '([A-Z_]+)'", src):
            read.setdefault(name, set()).add(mood)
    assert set(read) == set(m3refs.THEME_OPTIONS), (sorted(read), sorted(m3refs.THEME_OPTIONS))
    for name, (mood, names) in m3refs.THEME_OPTIONS.items():
        assert read[name] == {mood}, (name, read[name])
        if isinstance(names, dict):
            src = _mood_source(mood)
            var = re.findall(rf"(\w+) = getattr\(self\.T, '{name}', \{{\}}\)", src)
            keys = set(re.findall(rf"getattr\(self\.T, '{name}', \{{\}}\)\.get\('(\w+)'", src))
            for v in var:
                keys |= set(re.findall(rf"\b{v}\.get\('(\w+)'", src))
            assert keys == set(names), (name, keys, names)
    return f"the 14 mood files read {', '.join(sorted(read))} besides the music; THEME_OPTIONS lists each with its " \
           f"mood and keys"


def theme_options_are_recorded_per_config():
    """read_config records a config's theme options under the fight file's names (brass_trailer's rematch,
    dark_drumline's bassdrum_vs_tuba, stadium_rock's electric_vs_acoustic); a config without any reads as before (no
    key: the merged themes' theme.json keep their bytes); a key or value the mood cannot take stops the export"""
    import json
    import types
    got = {c: m3refs.read_config(c).get('theme_options') for c in (
        'ballfight_horn_vs_trombone_rematch', 'ballfight_bassdrum_vs_tuba', 'ballfight_electric_vs_acoustic',
        'examples/trombone_vs_horn', 'ballfight_flute_vs_sousaphone')}
    assert got == {'ballfight_horn_vs_trombone_rematch': {'sequel': True},
                   'ballfight_bassdrum_vs_tuba': {'bass_drums': False, 'after_ko': True},
                   'ballfight_electric_vs_acoustic': {'plugged': 'ELECTRIC'},
                   'examples/trombone_vs_horn': None, 'ballfight_flute_vs_sousaphone': None}, got
    for theme in ('big_top', 'baroque_storm', 'bossa_noir', 'dark_disco', 'heist', 'horror_trap'):
        with open(os.path.join(common.REFERENCE, 'themes', theme, 'theme.json'), encoding='utf-8') as f:
            for c in json.load(f)['configs']:
                facts = m3refs.read_config(c['config'])
                assert 'theme_options' not in facts and 'theme_options' not in c, (theme, c['config'])
                # MUSIC_EXTRAS (roles, mix, solos, title_hits), recorded after these folders were first made, are
                # in theme.json since their re-make: every value reads exactly as committed
                assert set(facts) == set(c) - {'seed', 'recording', 'winner', 't_ko', 'bed_alone'}, (theme, c['config'])
                assert {k: c[k] for k in facts} == json.loads(json.dumps(facts)), (theme, c['config'])
    fighters = ['ELECTRIC', 'ACOUSTIC']

    def opts(mood, names):
        return m3refs.theme_options_of(types.SimpleNamespace(**names), mood, fighters, 'a config')
    assert opts('stadium_rock', dict(TRAILER=dict(sequel=True))) == {}, 'another mood\'s name: ignored, as by the kit'
    assert opts('dark_drumline', dict(DRUMLINE=dict(after_ko=True))) == {'after_ko': True}
    assert 'does not read' in exits(opts, 'dark_drumline', dict(DRUMLINE=dict(bass_drum=False)))
    assert 'True or False' in exits(opts, 'brass_trailer', dict(TRAILER=dict(sequel=1)))
    assert 'not a dict' in exits(opts, 'brass_trailer', dict(TRAILER=True))
    assert 'not one of its fighters' in exits(opts, 'stadium_rock', dict(PLUGGED='FLUTE'))
    return 'TRAILER, DRUMLINE and PLUGGED reach theme.json as sequel, bass_drums/after_ko and plugged; the merged ' \
           'themes\' configs read exactly as committed'


def every_hook_call_reaches_its_method():
    """m3parts.hook_kwargs on every hook of the 14 kit moods: each recorded argument under the method's own name
    (trailer_hit's braam as braam_dur, hit's chord as ch), nothing dropped but HOOK_OMITTED's (`ring` for the
    moods whose hit has none), every parameter given; anything else stops the export"""
    import importlib
    from tools.studio_export import m3parts as MP
    common.kit()
    omitted = set()
    n = 0
    for mood in MOODS:
        Bed = importlib.import_module(f'ballfight.kit.moods.{mood}').Bed
        bed = Bed.__new__(Bed)
        for hook in MP.HOOK_CALLS:
            if not callable(getattr(bed, hook, None)):
                continue
            for args in MP.HOOK_CALLS[hook]:
                kw = MP.hook_kwargs(hook, dict(args), getattr(bed, hook))
                n += 1
                dropped = {a for a in args if MP.HOOK_KWARGS.get(a, a) not in kw}
                assert all((hook, a) in MP.HOOK_OMITTED for a in dropped), (mood, hook, dropped)
                omitted |= {(mood, hook, a) for a in dropped}
                if hook == 'trailer_hit':
                    assert kw['braam_dur'] is args['braam'] and 'braam' not in kw, kw
    assert omitted == {('dark_disco', 'hit', 'ring'), ('heist', 'hit', 'ring'), ('horror_trap', 'hit', 'ring')}, \
        omitted

    def hit(mix, t, ch, g=1.0):
        pass

    def two(mix, t, g, h):
        pass
    assert 'does not take' in exits(MP.hook_kwargs, 'boom', dict(t=1.0, g=0.5, extra=2), two)
    assert 'gives no h' in exits(MP.hook_kwargs, 'boom', dict(t=1.0, g=0.5), two)
    assert MP.hook_kwargs('hit', dict(t=1.0, chord='Em', g=0.5, ring=0.4), hit) == dict(t=1.0, ch='Em', g=0.5)
    return f'{n} calls of the 14 moods\' hooks pass every argument (braam -> braam_dur, chord -> ch); only ring is ' \
           f'dropped, for the hit of dark_disco, heist and horror_trap'


def a_stem_window_past_the_end_is_cut():
    """m3parts.window_length: a stem window records the samples the soundtrack has there (the cache keeps
    x[i0:i0 + SR]): a whole second inside, fewer where the soundtrack ends inside it, none past the end"""
    import numpy as np
    from tools.studio_export import m3parts as MP
    SR = MP.SR
    na = 61 * SR
    x = np.zeros((na, 2))
    for i0 in (0, na - SR, na - SR + 1, int(round(60.7 * SR)), na, na + 5):
        assert MP.window_length(i0, na) == len(x[i0:i0 + SR]), i0
    assert MP.window_length(int(round(60.7 * SR)), na) == 14400 and MP.window_length(na - SR, na) == SR
    st = dict(master=np.arange(na * 2, dtype=float).reshape(na, 2), drums=np.ones((na, 2)))
    wins, arrays = MP.stem_windows(57.7, na, st)                 # T_KO + 3.0 = 60.7 s: cut to 14,400 samples
    assert [w['n'] for w in wins] == [SR, SR, SR, SR, 14400], wins
    assert sorted(arrays) == sorted(f'w{j}__{b}' for j in range(5) for b in st)
    assert all(arrays[f'w{j}__{b}'].shape == (w['n'], 2) for j, w in enumerate(wins) for b in st)
    assert np.array_equal(arrays['w4__master'], st['master'][wins[4]['i0']:].astype(np.float32))
    wins, arrays = MP.stem_windows(58.5, na, st)                 # T_KO + 3.0 = 61.5 s: past the end, left out
    assert [w['window'] for w in wins] == ['0.0', '12.0', '30.0', 'T_KO - 1.0'], wins
    assert sorted(arrays) == sorted(f'w{j}__{b}' for j in range(4) for b in st), sorted(arrays)
    return 'a window is SR samples long inside the soundtrack, as long as its cached slice at the end, and left ' \
           'out (no record, no member) when it starts past the end'


def a_theme_recording_goes_to_the_cache():
    """ruling M3-R43, end to end in a scratch folder through m3refs.place_recording (what theme_refs and episode_refs
    call): a theme's recording with recordings_in_cache is the pinned cache file, its entry {cache, sha256}, and the
    committed copy an earlier export made is gone after finish(); a pair gate points at its theme's cached copy
    (cached_copy), or with recording_in_cache pins its own run.bfrec; without the flags the file is committed as
    before"""
    import hashlib
    import inspect
    import json
    from tools.studio_export import theme_big_top
    assert m3refs.theme_spec(theme_big_top.SPEC)['recordings_in_cache'] is True   # (plan m4 Task 1: the default;
    # big_top's recording is milestone 1's committed fight, which place_recording points at before the cache)
    assert m3refs.theme_spec(dict(name='x', recordings_in_cache=True))['recordings_in_cache'] is True
    assert 'True or False' in exits(m3refs.theme_spec, dict(name='x', recordings_in_cache='yes'))
    theme, pair = 'themes/brass_trailer', 'episodes/trombone_vs_horn'
    tool, data = 'tools.studio_export.theme_brass_trailer', b'a kit fight\n'
    sha = hashlib.sha256(data).hexdigest()
    with scratch():
        # an earlier export committed the recording in the theme folder
        m3refs.set_run(m3refs.Run('export'))
        assert m3refs.place_recording(theme, 'trombone_vs_horn.bfrec', data, False, 'theme_brass_trailer',
                                      tool) == f'{theme}/trombone_vs_horn.bfrec'
        m3refs.commit_json(theme, 'theme.json', dict(recording=f'{theme}/trombone_vs_horn.bfrec'), tool)
        m3refs.finish(theme)
        assert read(f'studio/reference/{theme}/trombone_vs_horn.bfrec') == data
        # the remake with recordings_in_cache: the same bytes move to the cache
        m3refs.set_run(m3refs.Run('export'))
        entry = m3refs.place_recording(theme, 'trombone_vs_horn.bfrec', data, True, 'theme_brass_trailer', tool)
        assert entry == dict(cache=f'{theme}/trombone_vs_horn.bfrec', sha256=sha), entry
        m3refs.commit_json(theme, 'theme.json', dict(recording=entry), tool)
        m3refs.finish(theme)
        assert not os.path.exists(m3refs.ref_dir(theme, 'trombone_vs_horn.bfrec')), 'the committed copy is still there'
        assert f'studio/reference/{theme}/trombone_vs_horn.bfrec' not in entries_of(f'studio/reference/{theme}/'
                                                                                    'manifest.json')
        pins = m3refs.read_pins(theme)
        assert pins[f'{theme}/trombone_vs_horn.bfrec']['sha256'] == sha and len(pins) == 1, pins
        assert json.loads(read(f'studio/reference/{theme}/theme.json'))['recording'] == entry
        assert read(f'studio/reference/.cache/m3/{theme}/trombone_vs_horn.bfrec') == data
        # a pair on the theme's config points at the theme's cached copy: nothing of its own
        m3refs.set_run(m3refs.Run('export'))
        got = m3refs.place_recording(pair, 'run.bfrec', data, False, 'episode_trombone_vs_horn', tool,
                                     also=(theme,), cached_from=theme)
        assert got == entry, got
        assert m3refs.cached_copy('0' * 64, theme) is None and m3refs.cached_copy(sha, 'themes/nobody') is None
        # a pair whose recording is no theme's, with recording_in_cache: its own pinned run.bfrec
        other = b'another kit fight\n'
        own = m3refs.place_recording(pair, 'run.bfrec', other, True, 'episode_trombone_vs_horn', tool, also=(theme,),
                                     cached_from=theme)
        assert own['cache'] == f'{pair}/run.bfrec', own
        m3refs.commit_json(pair, 'episode.json', dict(recording=own), tool)
        m3refs.finish(pair)
        assert list(m3refs.read_pins(pair)) == [f'{pair}/run.bfrec']
        assert m3refs.read_pins(pair)[f'{pair}/run.bfrec']['sha256'] == own['sha256']
        assert not os.path.exists(m3refs.ref_dir(pair, 'run.bfrec'))
        m3refs.set_run(None)
    src = inspect.getsource(m3refs.theme_refs)
    assert "place_recording(folder, f'{config_slug(config)}.bfrec', data, spec['recordings_in_cache']" in src
    src = inspect.getsource(m3refs.episode_refs)
    assert "place_recording(folder, 'run.bfrec', data, spec['recording_in_cache']" in src and 'cached_from=theme' in src
    return 'with recordings_in_cache a theme\'s recording moves from its folder to the pinned cache (entry, pin, ' \
           'committed copy gone); a pair takes its theme\'s cached copy, or pins its own with recording_in_cache'

def the_key_variant_sets_its_loop():
    """ruling M3-R45: the key variant's appended text sets LOOP to the other config's loop (prog_duel's #45 sets its
    own LOOP, which would otherwise stay over the new chords), the same loop bed.json's `ff` gives the fight file"""
    common.kit()
    roster = os.path.join(common.REPO, 'projects', 'ballfight')      # the configs import the roster, as in an export
    if roster not in sys.path:
        sys.path.append(roster)
    for config in ('ballfight_piano_vs_organ', 'examples/flute_vs_trombone'):
        facts = m3refs.read_config(config)
        c, text, ff = m3refs.key_variant(facts, None)
        with open(m3refs.config_source(config), encoding='utf-8') as fh:
            src = fh.read()
        ns = {}
        exec(compile(src + '\n' + text, config, 'exec'), ns)     # noqa: S102 (the kit config, as the kit loads it)
        assert ns['LOOP'] == ff['chord_loop'], (config, ns['LOOP'], ff['chord_loop'])
        assert ns['CHORDS'][2:2 + len(ns['LOOP'])] == ns['LOOP'], (config, ns['CHORDS'])
        assert (ns['KEY'], ns['SCALE']) == (ff['key'], ff['scale']), config
    return 'the key variant sets KEY, SCALE, LOOP and CHORDS together, LOOP being the fight file\'s chord_loop'

def a_configs_lines_and_roles_reach_theme_json():
    """read_config records the config's ROLES as `roles` (absent when it sets none) beside its LINES (`lines`), so the
    theme gates' fight file plays as the config does (dark_tango's bed plays the melody fighter's line)"""
    common.kit()
    c = m3refs.read_config('ballfight_cello_vs_contrabass')
    assert c['roles'] == {'CELLO': 'melody', 'CONTRABASS': 'bass'}, c.get('roles')
    assert set(c['lines']) == {'CELLO', 'CONTRABASS'} and all(len(v) == 32 for v in c['lines'].values()), c['lines']
    src = open(m3refs.config_source('ballfight_cello_vs_contrabass'), encoding='utf-8').read()
    assert "\nROLES = " in src
    saved = m3refs.config_source
    try:                                                # the same config without its ROLES line: no `roles`
        tmp = os.path.join(tempfile.mkdtemp(prefix='m3refs_roles_'), 'episode.py')
        with open(tmp, 'w', encoding='utf-8') as fh:
            fh.write("\n".join(ln for ln in src.split("\n") if not ln.startswith("ROLES = ")))
        m3refs.config_source = lambda config: tmp
        assert 'roles' not in m3refs.read_config('ballfight_cello_vs_contrabass')
    finally:
        m3refs.config_source = saved
    return 'ROLES and LINES reach theme.json (cello_vs_contrabass: melody and bass, 32 notes each); a config ' \
           'without ROLES records none'

def every_config_name_the_soundtrack_reads_is_recorded():
    """every config name kit/music.py, the moods and the fighters' parts read from T that kit/episode.py does not
    derive itself is recorded by read_config (the music, LINES, MUSIC_EXTRAS, THEME_OPTIONS), asserted fixed
    (LOOP_START), or a fighter part's own (OBBLIGATO: SOUSAPHONE's opt-in, which no kit config sets and no fight
    file holds); MIX, SOLO (a chord as a list of notes) and TITLE_HITS reach theme.json under the fight file's names"""
    import re
    kit = os.path.join(common.REPO, 'projects', 'ballfight', 'kit')
    files = ['music.py'] + [os.path.join(d, f) for d in ('moods', 'fighters')
                            for f in sorted(os.listdir(os.path.join(kit, d))) if f.endswith('.py')]
    src = ''.join(open(os.path.join(kit, f), encoding='utf-8').read() for f in files)
    read = set(re.findall(r"\bT\.([A-Z][A-Z_0-9]*)\b", src))
    read |= set(re.findall(r"getattr\((?:self\.)?(?:M\.)?T, '([A-Z_]+)'", src))
    with open(os.path.join(kit, 'episode.py'), encoding='utf-8') as fh:
        derived = set(re.findall(r"\bT\.([A-Z][A-Z_0-9]*)\s*=", fh.read())) - {'SOLO'}   # T.SOLO = dict(T.SOLO)
    recorded = {'BPM', 'KEY', 'SCALE', 'CHORDS', 'LOOP', 'FIGHTERS', 'MOOD', 'LINES', 'ROLES', 'MIX', 'SOLO',
                'TITLE_HITS'} | set(m3refs.THEME_OPTIONS)
    fixed, fighters_own = {'LOOP_START'}, {'OBBLIGATO'}
    left = read - derived - recorded - fixed - fighters_own
    assert not left, f'the soundtrack reads config names nothing records: {sorted(left)}'
    common.kit()
    roster = os.path.join(common.REPO, 'projects', 'ballfight')
    if roster not in sys.path:
        sys.path.append(roster)
    assert not [c for c in m3refs.kit_configs() if hasattr(m3refs.config_module(c), 'OBBLIGATO')]
    c = m3refs.read_config('ballfight_flute_vs_clarinet')
    assert c['mix']['clarinet'] == [2.4, 0.35] and set(c['title_hits']) == {'FLUTE', 'CLARINET'}, c.get('mix')
    assert all(len(n) == 3 and isinstance(n[1], int) for v in c['solos'].values() for n in v), c['solos']
    c = m3refs.read_config('ballfight_electric_vs_acoustic')          # ACOUSTIC's solo strums chords: held as lists
    assert set(c['solos']) == {'ELECTRIC', 'ACOUSTIC'}, sorted(c['solos'])
    acoustic = c['solos']['ACOUSTIC']
    assert acoustic[6] == [3.0, [40, 47, 52, 55, 59, 64], 1.0] and acoustic[0] == [0.0, 40, 0.5], acoustic[:7]
    assert [e[0] for e in acoustic if isinstance(e[1], list)] == [3.0, 6.0, 10.0, 12.0], acoustic
    return f"the soundtrack reads {', '.join(sorted(read - derived))} from a config: each is recorded, fixed " \
           f"(LOOP_START) or a fighter part's own (OBBLIGATO)"


def every_folder_reproduces_its_inputs():
    """the cheap half of `cache all` (m3refs.drift): every committed fighter, theme and pair folder's JSON holds what
    the current framework writes from its own inputs (the module's SPEC, the kit config's values, the manifests and
    the pins), so a cache run cannot stop on `... would change` for any of those reasons. The kit's own numbers are
    still proven only by the cache run itself"""
    folders = m3refs.pinned_folders()
    problems = [p for f in folders for p in m3refs.drift(f)]
    assert not problems, '\n  ' + '\n  '.join(problems)
    areas = {a: sum(f.startswith(a + '/') for f in folders) for a in m3refs.AREAS}
    return ', '.join(f'{n} {a}' for a, n in areas.items()) + ': each folder\'s JSON follows from its inputs'


def drift_finds_what_a_cache_run_would_stop_on():
    """drift names the two faults found after storage plan B and the framework round: a fight.json `same_as` entry
    pointing at a left.bfrec that moved to the cache, and a theme config without a value read_config now records;
    and the rest of what it compares"""
    import copy
    import json

    def doc(folder, name):
        with open(os.path.join(common.REFERENCE, *folder.split('/'), name), encoding='utf-8') as f:
            return json.load(f)
    fight = doc('fighters/tuba', 'fight.json')
    stale = copy.deepcopy(fight)
    stale['same_as']['fighters/tuba/scenarios/plain_s9.bfrec'] = 'studio/reference/fighters/tuba/left.bfrec'
    got = m3refs.fighter_drift('fighters/tuba', fight=stale)
    assert len(got) == 1 and 'same_as' in got[0] and 'plain_s9' in got[0], got
    moved = copy.deepcopy(fight)
    moved['left'] = {k: v for k, v in moved['left'].items() if k not in ('cache', 'sha256')}
    moved['left']['file'] = 'fighters/tuba/left.bfrec'
    got = m3refs.fighter_drift('fighters/tuba', fight=moved)
    assert any('fight.json: left' in g for g in got) and any('look.json: left' in g for g in got), got
    respec = copy.deepcopy(fight)
    respec['spec']['fights']['stats'] = 1999
    got = m3refs.fighter_drift('fighters/tuba', fight=respec)
    assert len(got) == 1 and 'spec differs' in got[0] and 'fights' in got[0], got
    theme = doc('themes/heist', 'theme.json')
    old = copy.deepcopy(theme)
    del old['configs'][0]['roles']
    got = m3refs.theme_drift('themes/heist', theme=old)
    assert len(got) == 1 and 'missing: roles' in got[0], got
    old = copy.deepcopy(theme)
    old['configs'][0]['recording'] = 'themes/heist/gone.bfrec'
    got = m3refs.theme_drift('themes/heist', theme=old)
    assert len(got) == 1 and 'not a file a manifest lists' in got[0], got
    ep = doc('episodes/flute_vs_trombone', 'episode.json')
    old = copy.deepcopy(ep)
    old['solos'] = {'FLUTE': [[0.0, 76, 1.0]]}
    got = m3refs.episode_drift('episodes/flute_vs_trombone', episode=old)
    assert len(got) == 1 and 'solos' in got[0], got
    return 'a stale same_as, a moved left recording, a changed spec, a missing config value, a recording no ' \
           'manifest lists and a changed episode value are each named'


CHECKS = (shard_takes_a_fighter_file, verify_finds_a_changed_byte_in_a_shard, check_pins_names_the_command,
          a_pin_never_moves_on_its_own, remake_makes_a_folder_again_whole, a_cache_run_writes_no_committed_file,
          a_port_never_points_at_another_port, budget_reports_an_overfull_fighter,
          budget_refuses_a_committed_recording, budget_limits_origin_json, the_tripwire_warns,
          recordings_go_to_the_cache_by_default,
          the_command_line_parses, the_framework_records_milestone_1s_fights, hull_rows_go_to_the_cache,
          the_left_file_is_on_disk_for_the_parts, the_replay_guard_compares_values, the_checks_fight_ends_after_44_s,
          a_port_declares_its_sprite_size, windows_has_a_fork_stand_in,
          cfg_keys_follow_each_function, a_padded_sprite_is_declared_too,
          a_failed_part_ends_the_export_promptly, a_rolls_fight_goes_through_the_framework_alone,
          a_neutral_handicap_is_the_same_fight_either_way, a_big_left_recording_goes_to_the_cache,
          a_left_file_finish_wrote_is_kept, a_nan_rectangular_key_is_an_array,
          every_option_the_moods_read_is_listed, theme_options_are_recorded_per_config,
          every_hook_call_reaches_its_method, a_stem_window_past_the_end_is_cut, a_theme_recording_goes_to_the_cache,
          the_key_variant_sets_its_loop, a_configs_lines_and_roles_reach_theme_json,
          every_config_name_the_soundtrack_reads_is_recorded, every_folder_reproduces_its_inputs,
          drift_finds_what_a_cache_run_would_stop_on)


def main():
    entry.check_environment()
    for check in CHECKS:
        try:
            print(f'ok    {check.__name__}: {check()}')
        except (AssertionError, SystemExit, Exception) as ex:            # noqa: B014 (say which check, then stop)
            print(f'FAIL  {check.__name__}: {type(ex).__name__}: {ex}')
            return 1
    print(f'm3refs_checks: {len(CHECKS)} checks passed')
    return 0


if __name__ == '__main__':
    sys.exit(main())

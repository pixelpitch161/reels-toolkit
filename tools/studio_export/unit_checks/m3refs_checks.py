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
    return 'existing_copy looks in the top-level manifest and the folder\'s own shard, never another port\'s'


def budget_reports_an_overfull_fighter():
    files = [('fighters/flute/left.bfrec', 290_000), ('fighters/flute/fight.json', 10_000),
             ('fighters/tuba/left.bfrec', 291_000), ('fighters/tuba/fight.json', 10_000),
             ('themes/heist/a.bfrec', 150_000), ('themes/heist/theme.json', 130_000),
             ('episodes/trombone_vs_tuba/run.bfrec', 255_000), ('episodes/oboe_vs_bassoon/checks.json', 100_001),
             ('fights/flute_vs_trombone/ep392.bfrec', 146_000_000)]
    rows = {r['folder']: r for r in m3refs.budget_rows(files)}
    assert rows['fighters/flute']['bytes'] == 300_000 and rows['fighters/flute']['limit'] == 300_000
    assert rows['fighters/tuba']['bytes'] == 301_000 > rows['fighters/tuba']['limit'], rows['fighters/tuba']
    assert rows['themes/heist']['limit'] == 280_000 >= rows['themes/heist']['bytes']
    assert rows['episodes/trombone_vs_tuba']['limit'] == 260_000
    assert rows['episodes/oboe_vs_bassoon']['bytes'] > rows['episodes/oboe_vs_bassoon']['limit'] == 100_000
    total = rows['studio/reference']
    assert total['limit'] == 147_000_000 and total['bytes'] == sum(b for _, b in files) > total['limit']
    over = sorted(r['folder'] for r in rows.values() if r['bytes'] > r['limit'])
    assert over == ['episodes/oboe_vs_bassoon', 'fighters/tuba', 'studio/reference'], over
    return 'budget: a 301 KB fighter folder is over, 300 KB is not; theme, episode and total limits as section 4'


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


CHECKS = (shard_takes_a_fighter_file, verify_finds_a_changed_byte_in_a_shard, check_pins_names_the_command,
          a_pin_never_moves_on_its_own, remake_makes_a_folder_again_whole, a_cache_run_writes_no_committed_file,
          a_port_never_points_at_another_port, budget_reports_an_overfull_fighter,
          the_command_line_parses, the_framework_records_milestone_1s_fights)


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

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


def check_pins_names_the_command():
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        path = m3refs.cache_bytes('fighters/flute', 'right.bfrec', b'abc', 'fighter_flute')
        assert m3refs.check_pins('fighters/flute') == 1
        pins = m3refs.read_pins('fighters/flute')
        assert list(pins) == ['fighters/flute/right.bfrec'] and pins['fighters/flute/right.bfrec']['bytes'] == 3
        assert 'studio/reference/fighters/flute/pins.json' in entries_of(
            'studio/reference/fighters/flute/manifest.json'), 'pins.json is a committed file of the shard'
        os.remove(path)
        text = exits(m3refs.check_pins, 'fighters/flute')
        assert 'missing  studio/reference/.cache/m3/fighters/flute/right.bfrec' in text, text
        assert 'run: python -m tools.studio_export cache fighters/flute' in text, text
        common.write_bytes(path, b'abd')
        text = exits(m3refs.check_pins, 'fighters/flute')
        assert 'changed  studio/reference/.cache/m3/fighters/flute/right.bfrec' in text, text
        assert 'run: python -m tools.studio_export cache fighters/flute' in text, text
    return 'check_pins stops naming the file and `cache fighters/flute` on a missing and on a changed cache file'


def a_pin_never_moves_on_its_own():
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        m3refs.cache_bytes('fighters/tuba', 'stats_rows.json', b'1\n', 'fighter_tuba')
        text = exits(m3refs.cache_bytes, 'fighters/tuba', 'stats_rows.json', b'2\n', 'fighter_tuba')
        assert 'differs from its pin' in text and 'cache fighters/tuba --repin' in text, text
        assert m3refs.read_pins('fighters/tuba')['fighters/tuba/stats_rows.json']['bytes'] == 2
        m3refs.set_run(m3refs.Run('cache'))              # a cache run: an unpinned file is refused too
        text = exits(m3refs.cache_bytes, 'fighters/tuba', 'new.json', b'3\n', 'fighter_tuba')
        assert 'not pinned' in text, text
        m3refs.set_run(m3refs.Run('cache', repin=True))
        m3refs.cache_bytes('fighters/tuba', 'stats_rows.json', b'22\n', 'fighter_tuba')
        assert m3refs.read_pins('fighters/tuba')['fighters/tuba/stats_rows.json']['bytes'] == 3, 'repin'
        m3refs.set_run(m3refs.Run('export'))              # a pin the run no longer makes stops finish()
        m3refs.cache_bytes('fighters/tuba', 'stats_rows.json', b'22\n', 'fighter_tuba')
        m3refs.read_pins('fighters/tuba')
        pins = m3refs.read_pins('fighters/tuba')
        pins['fighters/tuba/old.bfrec'] = dict(path='fighters/tuba/old.bfrec', sha256='0' * 64, bytes=1,
                                               export='fighter_tuba')
        m3refs.write_pins('fighters/tuba', pins)
        text = exits(m3refs.finish, 'fighters/tuba')
        assert 'pinned but no longer made: fighters/tuba/old.bfrec' in text, text
    return 'a changed regenerated file stops the run (only --repin moves a pin); a stale pin stops finish()'


def a_cache_run_writes_no_committed_file():
    with scratch():
        m3refs.set_run(m3refs.Run('export'))
        p = m3refs.commit_bytes('fighters/oboe', 'fight.json', b'{}\n', 'tools.studio_export.fighter_oboe')
        m3refs.set_run(m3refs.Run('cache'))
        m3refs.commit_bytes('fighters/oboe', 'fight.json', b'{}\n', 'tools.studio_export.fighter_oboe')
        text = exits(m3refs.commit_bytes, 'fighters/oboe', 'fight.json', b'{"a": 1}\n',
                     'tools.studio_export.fighter_oboe')
        assert 'would change' in text and 'fighter_oboe' in text, text
        assert open(p, 'rb').read() == b'{}\n'
    return 'a cache run reproduces the committed files and stops (writing nothing) when one would change'


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
        assert 'all stands alone' in exits(entry.main, ['cache', 'all', 'fighters/flute'])
        assert 'not a reference folder' in exits(entry.main, ['cache', 'flute'])
    found = entry.exports()
    takes = getattr(sys.modules[found['cache'][2]], 'TAKES_ARGS', ())
    assert 'cache' in takes and 'budget' in takes, 'all must leave them out'
    return '`cache all`, `cache <folder> --repin` parse; a bad option or folder is refused; all leaves both out'


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
          a_pin_never_moves_on_its_own, a_cache_run_writes_no_committed_file, budget_reports_an_overfull_fighter,
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

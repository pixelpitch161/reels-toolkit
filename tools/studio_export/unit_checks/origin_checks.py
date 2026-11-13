"""Checks of Gate O and the older reference kit modules (tools/studio_export/older/: plan m4 Task 2).

Run from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
    export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
    .studio-tools/py313/Scripts/python.exe -m tools.studio_export.unit_checks.origin_checks

The scratch copy holds the episode's delivered sources only and is the only place an episode runs; an episode run
leaves projects/, reelkit/ and series/ as they were; install() is idempotent and installs only modules that set
PORTED = True (any other fighter is the bare ball); and Gate O sees a one-pixel change in a layer and a one-sample
change in a bus of the ORGAN module, made at run time, naming the frame and pixel or the bus and sample. The two
optional SPEC keys: a `silence` patch matching twice or never stops the gate naming it; with `silence`, a shared bus
(fx) is bit-identical (the episode's stems copy silenced to the ORGAN's own placements, the kit side to its Sound's
own) and a one-sample change inside the silenced copy fails naming the bus and sample; a two-fight spec (the ORGAN
winning, seed 237, and losing, seed 11) writes both sections, and a one-pixel change in the second fight fails naming
that fight and frame. Uses the real kit and #35's episode (in the git-ignored cache only); 12-30 minutes. Prints one
line per check; exits 1 on the first failure."""
import os
import shutil
import sys
import tempfile

from tools.studio_export import __main__ as entry
from tools.studio_export import common, m3parts, older
from tools.studio_export.older import origin as O

EPISODE = 'ballfight_organ_vs_harp'


def the_scratch_copy_holds_the_delivered_sources_only():
    folder = O.scratch(EPISODE)
    src = os.path.join(common.REPO, 'projects', EPISODE)
    files = sorted(f for f in os.listdir(folder) if os.path.isfile(os.path.join(folder, f)))
    assert files == O.delivered(EPISODE), (files, O.delivered(EPISODE))
    assert not any('backup' in f for f in files), files
    assert 'seed.txt' in files and 'render.py' in files and 'audio.py' in files, files
    for f in files:
        assert open(os.path.join(folder, f), 'rb').read() == open(os.path.join(src, f), 'rb').read(), f
    dirs = sorted(f for f in os.listdir(folder) if os.path.isdir(os.path.join(folder, f)))
    assert set(dirs) <= {'check', '__pycache__'}, dirs
    assert O._under(folder, O.ROOT) and not O._under(folder, O.PROJECTS)
    return f'{len(files)} files, byte for byte, under {common.rel(folder)}'


def it_refuses_to_run_anywhere_else():
    refused = []
    for where in (os.path.join(common.REPO, 'projects', EPISODE), os.path.join(common.REPO, 'projects'),
                  O.ROOT, common.CACHE, tempfile.gettempdir()):
        try:
            O.guard(where)
        except SystemExit as ex:
            assert 'refuses' in str(ex), ex
            refused.append(where)
            continue
        raise AssertionError(f'guard accepted {where}')
    for bad in ('../ballfight_organ_vs_harp', 'ballfight_organ_vs_harp/v2', 'organ_vs_harp', 'ballfight_nothing'):
        try:
            O.episode_dir(bad)
        except SystemExit:
            continue
        raise AssertionError(f'episode_dir accepted {bad!r}')
    try:
        O.scratch(EPISODE, [('render.py', 'import math', 'import math')])
    except SystemExit:
        pass
    else:
        raise AssertionError('a patched copy without a variant name was accepted')
    try:
        O.scratch(EPISODE, [('render.py', 'no such text in render.py', '')], 'bad')
    except SystemExit as ex:
        assert 'matches 0 times' in str(ex), ex
    else:
        raise AssertionError('a patch that matches nothing was accepted')
    return f'{len(refused)} folders refused, bad names and patches refused'


def an_episode_run_leaves_the_kit_folders_clean():
    assert O.kit_status() == '', O.kit_status()
    seed = int(open(os.path.join(common.REPO, 'projects', EPISODE, 'seed.txt')).read().split()[0])
    got = O._run_parts([(O.episode_facts_child, (O.scratch(EPISODE), seed))])[0]
    frames = O.episode_frames(EPISODE, [600])
    assert O.kit_status() == '', O.kit_status()
    assert got['line'].startswith(f'{seed} '), got['line']
    assert frames[600].shape == (640, 360, 3), frames[600].shape
    assert os.path.isdir(os.path.join(O.scratch(EPISODE), 'check')), 'the episode wrote its caches elsewhere'
    return f"replay line {got['line']!r} and frame 600 drawn; git status --short projects reelkit series is empty"


def _tmp_modules(files):
    root = tempfile.mkdtemp(prefix='origin_checks_')
    for rel, text in files.items():
        p = os.path.join(root, *rel.split('/'))
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w', encoding='utf-8', newline='\n') as fh:
            fh.write(text)
    return root


BARE = '"""a fighter module that is not ported yet"""\nPORTED = False\nfrom . import base\nLook, Sound = base.Look, base.Sound\n'
DONE = ('"""a ported stand-in"""\nPORTED = True\nfrom . import base\n\n\nclass Look(base.Look):\n    glass = (1, 2, 3)\n\n\n'
        'class Sound(base.Sound):\n    pass\n')


def install_child(root):
    """in a fresh process: what the kit's own loader finds for a ported and an unported older module"""
    a = older.install(root)
    b = older.install(root)
    common.kit()
    from ballfight.kit import fighters as FM
    harp, drums = FM.module('HARP'), FM.module('DRUMS')
    return dict(first=a, second=b, harp=harp is None, drums=getattr(drums, '__name__', None),
                drums_file=getattr(drums, '__file__', None), glass=list(getattr(drums.Look, 'glass', ())),
                finders=sum(1 for f in sys.meta_path if type(f).__name__ == '_Finder'),
                bare_look=FM.look.__module__)


def install_is_idempotent_and_installs_only_ported_modules():
    root = _tmp_modules({'fighters/harp.py': BARE, 'fighters/drums.py': DONE, 'moods/__init__.py': ''})
    try:
        got = m3parts.fresh(install_child, root)
        assert got['first'] == got['second'], got
        assert list(got['first']) == ['ballfight.kit.fighters.drums'], got['first']
        assert got['harp'] is True, 'a module with PORTED = False was installed'
        assert got['drums'] == 'ballfight.kit.fighters.drums' and got['glass'] == [1, 2, 3], got
        assert os.path.normcase(got['drums_file']) == os.path.normcase(os.path.join(root, 'fighters', 'drums.py')), got
        assert got['finders'] == 1, got
        clash = _tmp_modules({'fighters/organ.py': DONE})
        try:
            older.install(clash)
        except SystemExit as ex:
            assert 'already exist in the kit' in str(ex), ex
        else:
            raise AssertionError('an older module shadowing the kit ORGAN was installed')
        finally:
            shutil.rmtree(clash, ignore_errors=True)
            older.install()
    finally:
        shutil.rmtree(root, ignore_errors=True)
    real = older.install()
    assert older.install() == real
    return (f"install twice: the same table {sorted(got['first'])}; HARP (PORTED = False) is the bare ball; a clash "
            f"is refused")


def _gate_fails(mutation, want):
    from tools.studio_export import origin_selfproof as SP
    try:
        O.fighter_origin(SP.ORGAN, out=False, mutation=mutation)
    except SystemExit as ex:
        line = str(ex)
        for w in want:
            assert w in line, (w, line)
        return line
    raise AssertionError(f'Gate O passed the ORGAN module changed by {mutation}')


def a_one_pixel_change_fails_gate_o_naming_the_frame():
    line = _gate_fails(('pixel', 435, 180, 300), ('frame 435 ', 'pixel (x=180, y=300)'))
    return line[:160]


def a_one_sample_change_fails_gate_o_naming_the_sample():
    i = 30 * O.SR
    line = _gate_fails(('sample', 'organ', i, 1e-3), ("bus 'organ'", f'sample {i} '))
    return line[:160]


# ---------------------------------------------------------------- `silence` (the stems' copy) and several `fights`
# #35's audio.py with every fx placement dropped but the ORGAN's own (by line: 400 a pipe row's thud, 455 TOCCATA's
# stomp, 468 the pipes clattering down when it loses); one line, so the line numbers stay as delivered. The kit side
# keeps only the ORGAN Sound's own fx placements (silence_kit, implied by `silence`).
ORGAN_FX_LINES = (400, 455, 468)
ORGAN_FX_SILENCE = [
    ('audio.py', "    mix = A.Mix(T.DUR + 4.0)\n",
     "    mix = A.Mix(T.DUR + 4.0); _place = mix.place; mix.place = lambda bus, *a, **kw: mix.bus(bus) if bus == 'fx' "
     f"and sys._getframe(1).f_lineno not in {ORGAN_FX_LINES!r} else _place(bus, *a, **kw)\n")]
HARP_WINS = 11                                  # a seed of #35's config the HARP wins (the ORGAN's shards, fall-apart)
FX_SAMPLE = 30 * O.SR


def _organ(**origin):
    from tools.studio_export import origin_selfproof as SP
    return dict(SP.ORGAN, origin=dict(SP.ORGAN['origin'], **origin))


def _fails(fn, want):
    try:
        fn()
    except SystemExit as ex:
        line = str(ex)
        for w in want:
            assert w in line, (w, line)
        return line
    raise AssertionError(f'Gate O passed; it had to fail naming {want}')


def a_silence_patch_matching_twice_or_never_fails_naming_it():
    twice = ('audio.py', "mix.place('fx', A.whoosh(", "mix.place('fx', A.whoosh(")
    never = ('audio.py', 'no such text in audio.py', '')
    a = _fails(lambda: O.fighter_origin(_organ(silence=ORGAN_FX_SILENCE + [twice], buses=['fx']), out=False),
               ('silence patch 1 (audio.py) matches ', 'times, not once', 'A.whoosh('))
    assert 'matches 1 times' not in a and 'matches 0 times' not in a, a
    b = _fails(lambda: O.fighter_origin(_organ(silence=[never], buses=['fx']), out=False),
               ('silence patch 0 (audio.py) matches 0 times', 'no such text'))
    return f'{a[:90]} | {b[:70]}'


def two_fights_write_both_sections_with_a_silenced_shared_bus():
    spec = _organ(silence=ORGAN_FX_SILENCE, fights=[dict(seed=237, buses=['organ', 'solo', 'fx']),
                                                    dict(seed=HARP_WINS, frames=12, buses=['organ', 'fx'])])
    doc = O.fighter_origin(spec, out=False)
    size = len(common.dumps(doc).encode())        # origin.json's bytes (written by common.write_json in this layout)
    f1, f2 = doc['fights']
    assert [f1['seed'], f2['seed']] == [237, HARP_WINS], (f1['seed'], f2['seed'])
    assert f1['wins'] and not f2['wins'], (f1['wins'], f2['wins'])
    assert len(f1['frame_rows']) >= 24 and len(f2['frame_rows']) >= 12, (len(f1['frame_rows']), len(f2['frame_rows']))
    assert all(len(r) == len(O.FRAME_COLUMNS) and len(r[2]) == 64 for f in (f1, f2) for r in f['frame_rows'])
    assert f1['line'].startswith('237 ') and f2['line'].startswith(f'{HARP_WINS} '), (f1['line'], f2['line'])
    for f in (f1, f2):
        fx = next(b for b in f['buses'] if b['bus'] == 'fx')
        assert fx['peak'] == 'bit-identical', fx
    assert doc['silence']['kit_own_placements_only'] is True and len(doc['silence']['patches']) == 1, doc['silence']
    assert size <= 32_000, f'origin.json is {size} B, over the 32 KB budget'
    return (f"fight 1 seed 237 (the ORGAN wins) {len(f1['frame_rows'])} frames, fight 2 seed {HARP_WINS} (it loses) "
            f"{len(f2['frame_rows'])} frames; fx bit-identical in both; {size} B")


def a_one_sample_change_in_the_silenced_copy_fails_naming_the_bus():
    bump = ('audio.py', "    out, stems = master(mix)\n",
            f"    mix.bus('fx')[{FX_SAMPLE}, 0] += 1e-3\n    out, stems = master(mix)\n")
    line = _fails(lambda: O.fighter_origin(_organ(silence=ORGAN_FX_SILENCE + [bump], buses=['fx']), out=False),
                  ("bus 'fx'", f'sample {FX_SAMPLE} '))
    return line[:160]


def a_one_pixel_change_in_the_second_fight_fails_naming_that_fight():
    frames = [1400 + 10 * j for j in range(12)]   # inside both windows ([3.8 s, the reassembly)); 1440 = 48.0 s
    spec = _organ(fights=[dict(seed=237), dict(seed=HARP_WINS, frames=frames, buses=['organ'])])
    line = _fails(lambda: O.fighter_origin(spec, out=False, mutation=('pixel', 1440, 180, 300), mutation_fight=2),
                  (f'ORGAN fight 2 (seed {HARP_WINS}) frame 1440 ', '; 1 pixels differ'))
    return line[:160]


CHECKS = (the_scratch_copy_holds_the_delivered_sources_only, it_refuses_to_run_anywhere_else,
          an_episode_run_leaves_the_kit_folders_clean, install_is_idempotent_and_installs_only_ported_modules,
          a_one_pixel_change_fails_gate_o_naming_the_frame, a_one_sample_change_fails_gate_o_naming_the_sample,
          a_silence_patch_matching_twice_or_never_fails_naming_it,
          two_fights_write_both_sections_with_a_silenced_shared_bus,
          a_one_sample_change_in_the_silenced_copy_fails_naming_the_bus,
          a_one_pixel_change_in_the_second_fight_fails_naming_that_fight)


def main():
    entry.check_environment()
    for check in CHECKS:
        try:
            print(f'ok    {check.__name__}: {check()}', flush=True)
        except (AssertionError, SystemExit, Exception) as ex:            # noqa: B014 (say which check, then stop)
            print(f'FAIL  {check.__name__}: {type(ex).__name__}: {ex}')
            return 1
    assert O.kit_status() == ''
    print(f'origin_checks: {len(CHECKS)} checks passed')
    return 0


if __name__ == '__main__':
    sys.exit(main())

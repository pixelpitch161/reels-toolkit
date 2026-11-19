"""Checks of the older episodes as kit configs (tools/studio_export/older/configs/: plan m4 Task 5).

Run from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
    export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
    .studio-tools/py313/Scripts/python.exe -m tools.studio_export.unit_checks.older_configs_checks

The fourteen configs are there (the ten of Table O3 and the four stand-ins of Q2); each loads with the kit's
`episode.load(<path>)` from outside projects/ (a copy in the git-ignored cache, where the kit's own caches land) with
the series-standard beat sheet (FIGHT! on bar 2, no episode tag) and its theme; every value is one a format 2 fight
file holds, and CHORDS is `chords_for` of the intro, the loop and the tail, except the values GAPS names (no form in
format 2, reported for a ruling: a new gap or a gap that went away both fail); each fight runs to a K.O. at its seed;
the configs whose episode already used the 2-bar start and today's roster give the episode's replay-guard line, and
the two that cannot (#28: FIGHT! on bar 1; #40: its sim pins `steer` off) are checked to differ (plan question Q-E:
#40's own seed reaches no K.O. under today's BARI SAX, so its SEED, like each stand-in's, is the lowest seed from 1
that fits). Runs each episode only from its scratch copy; about 5
minutes. Prints one line per check; exits 1 on the first failure."""
import ast
import os
import sys

from tools.studio_export import __main__ as entry
from tools.studio_export import common, m3refs
from tools.studio_export.older import origin as O

HERE = os.path.join(common.REPO, 'tools', 'studio_export', 'older', 'configs')

# name: (the episode folder it is read from, its theme, whether the kit's run at its seed is the episode's fight)
CONFIGS = {
    'drums_vs_violin': ('ballfight_drums_vs_violin', 'balkan_fire', False),
    'bagpipes_vs_accordion': ('ballfight_bagpipes_vs_accordion', 'sea_shanty', True),
    'turntables_vs_synth': ('ballfight_turntables_vs_synth', 'dark_breakbeat', True),
    'tuba_vs_flute': ('ballfight_tuba_vs_flute', 'villain_march', True),
    'ukulele_vs_acoustic': ('ballfight_ukulele_vs_acoustic', 'spaghetti_western', True),
    'organ_vs_harp': ('ballfight_organ_vs_harp', 'gothic_horror', True),
    'trombone_vs_tuba_brass_house': ('ballfight_trombone_vs_tuba', 'brass_house', True),
    'bassoon_vs_barisax': ('ballfight_bassoon_vs_barisax', 'ska_punk', True),
    'barisax_vs_bassclarinet': ('ballfight_barisax_vs_bassclarinet', 'industrial', False),
    'viola_vs_violin': ('ballfight_viola_vs_violin', 'orchestral_drill', True),
    # the stand-ins (Q2): no episode of their own; their themes' episodes fought a retired fighter
    'drift_phonk_a': (None, 'drift_phonk', None),
    'drift_phonk_b': (None, 'drift_phonk', None),
    'noir_jazz_a': (None, 'noir_jazz', None),
    'noir_jazz_b': (None, 'noir_jazz', None),
}
# why a config's fight is not its episode's (plan question Q-E; the kit's run at the seed is the reference recording)
NOT_REPLAYED = {
    'drums_vs_violin': '#28 fought from bar 1 (timeline.py 41, no VS card); the 2-bar start moves the launch a bar',
    'barisax_vs_bassclarinet': "#40's sim.py 22-24 pins the BARI SAX's `steer` off; today's BARI SAX keeps it (Q3), "
                               "and at #40's seed 128 its fight reaches no K.O., so SEED is the kit's first fit",
}
# configs whose SEED is the lowest seed from 1 that fits (kit/search.py's `one`), not an episode's: the stand-ins, and
# #40, whose own seed reaches no K.O. under today's BARI SAX (EPISODE_SEED: the episode's seed, checked to be one)
EPISODE_SEED = {'barisax_vs_bassclarinet': 128}
# values with no form in a format 2 fight file (task-05-report.md: reported for a ruling, not forced)
GAPS = {
    ('organ_vs_harp', 'CHORDS'): "#35's three sections (timeline.py 76-79) are no intro + one 4/8-chord loop + tail",
    ('organ_vs_harp', 'SOLO.ORGAN'): "('dim', r) and (chord, r) entries (timeline.py 118-122)",
    ('organ_vs_harp', 'SOLO.HARP'): "a ('gliss', top) entry (timeline.py 123-127)",
    ('drums_vs_violin', 'SOLO.DRUMS'): '(beat, piece, velocity) entries, piece a drum name (timeline.py 119-125)',
    ('turntables_vs_synth', 'SOLO.TURNTABLES'): '(beat, scratch kind, beats) entries (timeline.py 116-123)',
    ('ukulele_vs_acoustic', 'SOLO.UKULELE'): 'strums (chords) in a solo, which format 2 allows ACOUSTIC only',
}
STANDINS = [n for n, (ep, _, _) in CONFIGS.items() if ep is None]
SEARCHED = STANDINS + sorted(EPISODE_SEED)

# the kit config names a format 2 fight file holds (bf_core::fightfile, its module notes) and the ones it refuses
FIGHT_FILE_NAMES = {'FIGHTERS', 'SEED', 'DMG_SCALE', 'DMG_SIDE', 'RULES', 'BPM', 'MOOD', 'KEY', 'SCALE', 'LOOP',
                    'CHORDS', 'LINES', 'SOLO', 'TITLE_HITS', 'ROLES', 'MIX', 'SHARD_SEED', 'FX_SEED', 'CAPTION', 'TAGS',
                    'TITLE_LINES', 'TITLE_BALLS', 'MIDDLE'}
OLD_TIMELINE = {'SERIES': 'an episode tag (new fights have none)', 'FIGHT_BAR': 'FIGHT! on another bar than 2',
                'LOOP_START': 'a loop that starts on another bar than FIGHT!'}
# bf_core::fightfile constants
LOOP_LENGTHS, TAIL_MAX, INTRO_BARS, TEMPO_RANGE = (4, 8), 8, 2, (30.0, 300.0)
SOLO_CHORDS = ('ACOUSTIC',)
RETIRED = ('GUITAR', 'SAX')


def source(name):
    return os.path.join(HERE, name, 'episode.py')


def config_module(name):
    """the config's values (episode.py executed on its own, without the kit)"""
    import importlib.util
    spec = importlib.util.spec_from_file_location(f'older_config_{name}', source(name))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    return cfg


def folder(name):
    """the config copied into the git-ignored cache (studio/reference/.cache/episodes/older_configs/<name>/): the
    kit's caches (check/) land there, never beside the config or under projects/"""
    with open(source(name), encoding='utf-8', newline='') as fh:
        return m3refs.config_folder('older_configs', name, fh.read())


def n_bars(bpm):
    from reelkit.config import loop_bpm
    return int(round(61.0 / (4 * (60.0 / loop_bpm(bpm, 61.0)))))


# ---------------------------------------------------------------- children (fresh processes)
def load_child(cfg_folder):
    """the kit's episode.load(<path>) of the copy, and the timeline values the checks read"""
    ep, _ = m3refs.load_config(cfg_folder)
    T = ep.T
    return dict(dir=ep.dir, fighters=list(T.FIGHTERS), t_fight=float(T.T_FIGHT), bar=float(T.BAR),
                series=T.SERIES, mood=T.MOOD, n_bars=int(T.N_BARS), chords=list(T.CHORDS), loop=list(T.LOOP),
                count=[(float(t), s) for t, s in T.COUNT], title=[(float(t), s) for t, s in T.TITLE],
                seed=ep.seed, timeline=sys.modules['timeline'] is T)


def fight_child(cfg_folder, seed=None):
    """the kit's replay-guard line of the config's SEED, or of `seed` (story.replay_hash: seed, winner, K.O., events,
    hash)"""
    ep, story = m3refs.load_config(cfg_folder)
    return story.replay_hash(ep.seed if seed is None else seed)


def fits_child(cfg_folder, seeds):
    """kit/search.py's `one` for each seed: [(seed, fits)]"""
    _, story = m3refs.load_config(cfg_folder)
    out = []
    for s in seeds:
        st = story.story(story.run_seed(s, record=False))
        out.append((s, bool(story.score(st)[0])))
    return out


# ---------------------------------------------------------------- the checks
def the_fourteen_configs_are_there():
    names = sorted(n for n in os.listdir(HERE) if os.path.isdir(os.path.join(HERE, n)) and n != '__pycache__')
    assert names == sorted(CONFIGS), (names, sorted(CONFIGS))
    for n in names:
        assert os.path.isfile(source(n)), n
    return f'{len(names)} configs: {len(names) - len(STANDINS)} of Table O3, {len(STANDINS)} stand-ins'


def each_config_loads_from_outside_projects():
    got = O._run_parts([(load_child, (folder(n),)) for n in CONFIGS])
    projects = os.path.join(common.REPO, 'projects')
    for n, g in zip(CONFIGS, got):
        assert g['timeline'], n
        assert not O._under(g['dir'], projects), (n, g['dir'])
        assert abs(g['t_fight'] - 2 * g['bar']) < 1e-12, (n, 'FIGHT! is not on bar 2', g['t_fight'], g['bar'])
        assert g['count'][-1] == (2 * g['bar'], 'FIGHT!') and abs(g['count'][0][0] - g['bar']) < 1e-12, (n, g['count'])
        assert [w for _, w in g['title']] == [g['fighters'][0], 'VS', g['fighters'][1]], (n, g['title'])
        assert g['series'] == '', (n, 'an episode tag', g['series'])
        assert g['mood'] == CONFIGS[n][1], (n, g['mood'])
        assert len(g['chords']) == g['n_bars'], n
        assert not set(g['fighters']) & set(RETIRED), (n, g['fighters'])
        assert isinstance(g['seed'], int) and g['seed'] > 0, (n, g['seed'])
    return (f'{len(got)} loaded by episode.load(<path>) from studio/reference/.cache/episodes/older_configs/: '
            f'FIGHT! on bar 2, the VS card on bar 0, no tag, the theme of Table O2')


def _gaps(n, cfg):
    """the config's values a format 2 fight file cannot hold: {(name, field): why}; AssertionError for anything a
    fight file refuses outright"""
    from ballfight.kit import episode as E                # noqa: N812  (the harmony helpers only)
    gaps = {}
    names = {k for k in dir(cfg) if k.isupper()}
    for k in names:
        assert k not in OLD_TIMELINE, f'{n}: {k} ({OLD_TIMELINE[k]})'
        assert k in FIGHT_FILE_NAMES, f'{n}: {k} is no field of a format 2 fight file'
    with open(source(n), encoding='utf-8') as fh:
        tree = ast.parse(fh.read())
    for node in tree.body:                                # values only: no import, no roster pin, no code
        ok = isinstance(node, ast.Assign) or (isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant))
        assert ok, f'{n}: line {node.lineno} is not a plain value (a per-fight roster pin or code)'
    fighters = list(cfg.FIGHTERS)
    assert len(fighters) == 2 and len(set(fighters)) == 2 and not set(fighters) & set(RETIRED), (n, fighters)
    assert isinstance(cfg.SEED, int) and cfg.SEED > 0, (n, cfg.SEED)
    assert TEMPO_RANGE[0] <= cfg.BPM <= TEMPO_RANGE[1], (n, cfg.BPM)
    assert cfg.DMG_SCALE > 0, n
    side = getattr(cfg, 'DMG_SIDE', None)
    assert side is None or (len(side) == 2 and all(x > 0 for x in side)), (n, side)
    E.pitch_class(cfg.KEY)
    assert isinstance(cfg.SCALE, str) and cfg.SCALE in E.SCALES, (n, cfg.SCALE)
    loop, chords = list(cfg.LOOP), list(cfg.CHORDS)
    assert len(loop) in LOOP_LENGTHS, (n, loop)
    for c in set(chords) | set(loop):
        E.parse_chord(c)
    assert len(chords) == n_bars(cfg.BPM), (n, len(chords), n_bars(cfg.BPM))
    intro = chords[:INTRO_BARS]
    whole = (len(chords) - INTRO_BARS) // len(loop)
    while whole and chords[INTRO_BARS:INTRO_BARS + whole * len(loop)] != loop * whole:
        whole -= 1
    tail = chords[INTRO_BARS + whole * len(loop):]
    if len(tail) > TAIL_MAX or m3refs.chords_at(intro, loop, tail, len(chords)) != chords:
        gaps[(n, 'CHORDS')] = f'{len(tail)} bars after {whole} whole loops'
    for k, line in getattr(cfg, 'LINES', {}).items():
        assert k in fighters, (n, k)
        assert len(line) >= len(loop) and len(line) % len(loop) == 0, (n, k, len(line))
        assert all(m is None or (isinstance(m, int) and 0 <= m <= 127) for m in line), (n, k)
    for k, solo in getattr(cfg, 'SOLO', {}).items():
        assert k in fighters, (n, k)
        for b, m, d in solo:
            plain = isinstance(m, int) and 0 <= m <= 127
            chord = isinstance(m, tuple) and m and all(isinstance(x, int) and 0 <= x <= 127 for x in m)
            if not (b >= 0 and d > 0 and (plain or (chord and k in SOLO_CHORDS))):
                gaps[(n, f'SOLO.{k}')] = repr((b, m, d))
                break
    for k, hits in getattr(cfg, 'TITLE_HITS', {}).items():
        assert k in fighters and all(isinstance(x, int) and 0 <= x <= 127 for x in hits), (n, k)
    for k in getattr(cfg, 'ROLES', {}):
        assert k in fighters, (n, k)
    for k in getattr(cfg, 'TITLE_LINES', {}):
        assert k in fighters, (n, k)
    return gaps


def the_values_read_as_a_format_2_fight_file():
    common.kit()[0].activate_paths()
    found = {}
    for n in CONFIGS:
        found.update(_gaps(n, config_module(n)))
    new = sorted(set(found) - set(GAPS))
    gone = sorted(set(GAPS) - set(found))
    assert not new, f'values with no form in format 2 that GAPS does not name: {[(k, found[k]) for k in new]}'
    assert not gone, f'GAPS names values that now read as format 2: {gone}'
    clean = len(CONFIGS) - len({n for n, _ in GAPS})
    chords = len(CONFIGS) - sum(1 for _, f in GAPS if f == 'CHORDS')
    return (f'every name a format 2 field, no SERIES / FIGHT_BAR / LOOP_START, no roster pin; CHORDS = chords_for of '
            f'the intro, loop and tail in {chords} of {len(CONFIGS)}; {clean} configs read whole, '
            f'{len(GAPS)} values listed in GAPS: {sorted(GAPS)}')


def each_fight_runs_to_a_ko_and_the_replay_lines():
    names = list(CONFIGS)
    seeds = {n: config_module(n).SEED for n in names}
    at = {n: EPISODE_SEED.get(n, seeds[n]) for n in names}       # the seed the episode's own line is compared at
    jobs = [(fight_child, (folder(n),)) for n in names]
    jobs += [(fight_child, (folder(n), s)) for n, s in EPISODE_SEED.items()]
    got = O._run_parts(jobs)
    kit, kit_at = got[:len(names)], dict(zip(EPISODE_SEED, got[len(names):]))
    eps = [(n, CONFIGS[n][0]) for n in names if CONFIGS[n][0]]
    epl = O._run_parts([(O.episode_facts_child, (O.scratch(ep), at[n])) for n, ep in eps])
    O.check_kit_clean('an episode run (older_configs_checks)')
    episode_line = {n: g['line'] for (n, _), g in zip(eps, epl)}
    out = []
    for n, s in EPISODE_SEED.items():                    # the episode's seed: a K.O. in #40, none for today's roster
        assert episode_line[n].split()[1] != 'None', (n, episode_line[n])
        assert kit_at[n].split()[1:3] == ['None', 'None'], (n, kit_at[n])
        out.append(f'{n} at the episode seed {s}: no K.O. ({kit_at[n]}; the episode {episode_line[n]})')
    for n, line in zip(names, kit):
        seed, winner, t_ko = line.split()[:3]
        assert int(seed) == seeds[n], (n, line)
        assert winner != 'None' and t_ko != 'None', f'{n}: no K.O. at seed {seed}: {line}'
        if n in EPISODE_SEED:
            out.append(f'{n} new fight at its first fit ({line})')
        elif n in episode_line:
            same = line == episode_line[n]
            assert same == CONFIGS[n][2], (f'{n}: the kit {line!r}, projects/{CONFIGS[n][0]} {episode_line[n]!r}: '
                                           f'expected {"the same line" if CONFIGS[n][2] else "a different fight"}')
            assert same or n in NOT_REPLAYED, n
            out.append(f"{n} {'replays' if same else 'new fight'} ({line})")
        else:
            out.append(f'{n} K.O. ({line})')
    return '; '.join(out)


def each_searched_seed_is_the_first_fit():
    jobs, spans = [], []
    for n in SEARCHED:
        seed = config_module(n).SEED
        cfg = folder(n)
        chunk = max(1, -(-seed // O.WORKERS))
        for a in range(1, seed + 1, chunk):
            jobs.append((fits_child, (cfg, list(range(a, min(seed, a + chunk - 1) + 1)))))
            spans.append(n)
    got = O._run_parts(jobs)
    out = []
    for n in SEARCHED:
        seed = config_module(n).SEED
        fits = dict(x for s, g in zip(spans, got) if s == n for x in g)
        assert sorted(fits) == list(range(1, seed + 1)), n
        assert fits[seed], f'{n}: SEED {seed} does not fit the beat sheet'
        lower = [s for s in range(1, seed) if fits[s]]
        assert not lower, f'{n}: seeds {lower} fit before SEED {seed}'
        out.append(f'{n} {seed}')
    return 'the lowest fit from seed 1: ' + ', '.join(out)


CHECKS = (the_fourteen_configs_are_there, the_values_read_as_a_format_2_fight_file,
          each_config_loads_from_outside_projects, each_fight_runs_to_a_ko_and_the_replay_lines,
          each_searched_seed_is_the_first_fit)


def main():
    entry.check_environment()
    for check in CHECKS:
        try:
            print(f'ok    {check.__name__}: {check()}', flush=True)
        except (AssertionError, SystemExit, Exception) as ex:            # noqa: B014 (say which check, then stop)
            print(f'FAIL  {check.__name__}: {type(ex).__name__}: {ex}')
            return 1
    assert O.kit_status() == ''
    print(f'older_configs_checks: {len(CHECKS)} checks passed')
    return 0


if __name__ == '__main__':
    sys.exit(main())

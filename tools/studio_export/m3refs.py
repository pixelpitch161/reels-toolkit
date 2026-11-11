"""The milestone 3 reference framework: where a fighter's, a theme's and a pair gate's reference data lives, the
pinned cache, the per-fighter export, and the size budgets.

    REF = 'studio/reference'; CACHE = 'studio/reference/.cache/m3'
    fighter_refs(spec)   a fighter_<slug>.py's SPEC: writes fighters/<slug>/ (committed) and its cache files
    theme_refs(spec)     themes/<name>/ + cache (filled by Task 2 and the theme ports)
    episode_refs(spec)   episodes/<pair>/ + cache (filled by Task 6 and the pair gates)
    pin(path)            record a cache file in its folder's pins.json (path, sha256, bytes, export)
    check_pins(folder)   every pinned file present and equal, else SystemExit naming the command to run

    python -m tools.studio_export cache <area>/<name>... | all [--repin]   regenerate cache files and check the pins
    python -m tools.studio_export budget                                  sizes per folder against the budgets

Storage (plan section 4). Every milestone 3 folder, `studio/reference/fighters/<slug>/`, `themes/<name>/` and
`episodes/<pair>/`, holds two kinds of data:

- committed files (in git, listed in the folder's own manifest shard `manifest.json`): what the always-run gates
  read, small enough for the budgets below;
- cache files, under `studio/reference/.cache/m3/<area>/<name>/` (git-ignored): the bulky data the `#[ignore]`d
  gates read (scenario recordings, statistics rows, stems, ...). The export writes them, and the folder's committed
  `pins.json` records each one's SHA-256, size and the export that makes it. Because the kit is deterministic on
  the reference PC with the pinned libraries, a regenerated cache file is the pinned file, and its pin proves it.

The pins are never changed silently. When a regenerated cache file differs from its pin, or a pinned file is no
longer made, the run stops naming the file; `cache <folder> --repin` is the only way to change a pin, and on a
folder whose pins are on the build branch that is the controller's decision (ruling M3-R1). A folder's first run
(no pins.json yet) records its pins.

Modes. An export run (`python -m tools.studio_export fighter_flute`) writes the committed files and the cache
files and records new pins. A cache run (`cache fighters/flute`) does the same work, but must reproduce every
committed file byte for byte (it stops naming a file that would change, and writes none) and every pinned cache
file, so regenerating the cache never touches the working tree.

A fighter's SPEC (fighter_<slug>.py; every key but `name` and `opponent` has the default shown):

    SPEC = dict(
        name='TUBA',                 # the roster name (fighters.py), never a retired one
        opponent='FLUTE',            # the other fighter of every fight (TROMBONE for FLUTE itself)
        fights=dict(
            base='flute_vs_trombone',    # the kit example whose config the fights are built on: the big top at
                                         # 132 BPM in E harmonic minor with milestone 1's loop and lines
            handicap=(1.0, 1.0),         # DMG_SIDE, left corner first (the same for every fight of the folder)
            dmg_scale=1.0,               # DMG_SCALE
            rules={},                    # opt-in rules on top of the pair's own (PAIR_RULES); normally empty
            stats=2000,                  # statistics seeds 1..stats, X left against the opponent
            scan=500,                    # the highest seed tried when choosing `left` and `right`
            left=None, right=None,       # a fixed seed for the two look fights instead of the scan
            must=(),                     # coverage items required besides the automatic ones ("hit/<what>", ...)
        ),
        scenarios=None,              # None: default_scenarios(spec); else a list of
                                     #   dict(name=, seeds=(..), overrides={}, swapped=False, opponent=None)
        frames=dict(left=64, right=32, times=()),          # the look gates' frame counts and extra times (Task 6)
        roles=(),                    # the roles its sound is exported in; () = its kit role (Task 6)
        voice_notes=dict(pitches=None, velocities=(0.6, 0.9), durations=(0.12, 0.25, 1.0)),   # its voice (Task 6)
        hooks=(),                    # the look and title hooks its picture gates log (Task 6)
    )

What fighter_refs writes for it today (the fight half of Template F; Task 6 adds the picture, sound, voice and
check parts, which read `frames`, `roles`, `voice_notes` and `hooks`):

- `fighters/<slug>/fight.json` (committed): the normalised spec, the fight config (fighters, handicap, damage
  scale, rules), the two look fights, every scenario with its full rules, the statistics' pooled figures and
  `fights_with` counts, and the coverage table: for every required item its first occurrence over the scenario
  fights (scenario, seed, time, event index, the event itself).
- `fighters/<slug>/left.bfrec` (committed): X in the left corner and winning, all three of its abilities fired:
  the lowest such seed among the statistics fights (or `fights.left`). When the bytes equal a recording the set
  already holds, fight.json points at that file instead of copying it.
- cache: `right.bfrec` (the opponent in the left corner and winning: X's shards fly; the lowest such seed),
  `scenarios/<name>_s<seed>.bfrec`, `stats_rows.json` (one row per seed, the columns of `stats.py`). fight.json's
  `same_as` names every cache recording whose bytes equal a committed reference recording (for FLUTE and
  TROMBONE: milestone 1's fights, which proves the framework records the fights milestone 1 proved).

Coverage: required = every event kind and `hit` sub-kind seen in at least REQUIRE_FIGHTS of the statistics fights,
X's three abilities ("ability/<name>") and `fights.must`; each must occur within WINDOW s of FIGHT! in some scenario
fight. If one does not, every scenario group gets EXTRA_SEEDS more seeds, up to EXTRA_ROUNDS times, and the export
stops naming any item it still cannot reach (it never lowers a threshold of its own accord: a port that needs a
scenario with other overrides adds it to `scenarios`).

Budgets (plan section 4; KB and MB are 1,000 and 1,000,000 bytes, as m1-measurements section 10 counts):
fighters/<slug>/ 300 KB; themes/<name>/ 120 KB + 160 KB per .bfrec in it; episodes/<pair>/ 100 KB (260 KB for
trombone_vs_tuba and bassclarinet_vs_euphonium); everything under studio/reference/ 147 MB. Counted over the files
git tracks or would track (`git ls-files --cached --others --exclude-standard`), so a folder an export has just
written is measured before it is committed. bf-testdata's `budget` test applies the same numbers."""
import hashlib
import json
import multiprocessing as mp
import os
import sys
import time

from . import common, manifest

REF = 'studio/reference'
CACHE = 'studio/reference/.cache/m3'
AREAS = manifest.AREAS
PREFIX = {'fighters': 'fighter_', 'themes': 'theme_', 'episodes': 'episode_'}   # the export that owns a folder
PINS = 'pins.json'
PIN_FORMAT = 1
WORKERS = 3                                   # every process pool: six lanes share the PC's 16 threads (plan, 2)
WINDOW = 20.0                                 # s after FIGHT! a required coverage item must occur by
REQUIRE_FIGHTS = 20                           # ... when it occurs in at least this many statistics fights
EXTRA_SEEDS, EXTRA_ROUNDS = 3, 3              # seeds added to every scenario group per round, and how many rounds
# the series' pair rules (bf_content::pair_rules, Task 3): switched on whichever side each fighter stands on
PAIR_RULES = (('FLUTE', 'TROMBONE', 'swat_cobras'), ('EUPHONIUM', 'TROMBONE', 'swat_shells'),
              ('BASS DRUM', 'TUBA', 'swallow_rollers'))

KB, MB = 1000, 1000 * 1000
FIGHTER_BUDGET = 300 * KB
THEME_BUDGET, THEME_PER_RECORDING = 120 * KB, 160 * KB
EPISODE_BUDGET, EPISODE_BUDGET_LARGE = 100 * KB, 260 * KB
LARGE_EPISODES = ('trombone_vs_tuba', 'bassclarinet_vs_euphonium')   # Tasks 45 and 46: their own kit recording
TOTAL_BUDGET = 147 * MB


# ---------------------------------------------------------------- paths
def cache_root():
    """<repo>/studio/reference/.cache/m3 (from common.CACHE when called, so the unit checks can redirect it)"""
    return os.path.join(common.CACHE, 'm3')


def check_folder(folder):
    """'<area>/<name>' with a known area, else SystemExit"""
    parts = folder.strip('/').split('/')
    if len(parts) != 2 or parts[0] not in AREAS or not parts[1] or parts[1].startswith('.'):
        raise SystemExit(f"{folder!r} is not a reference folder: give <area>/<name> with area one of "
                         f"{', '.join(AREAS)} (for example fighters/flute)")
    return '/'.join(parts)


def ref_dir(folder, *parts):
    """studio/reference/<folder>/<parts>, absolute"""
    return os.path.join(common.REFERENCE, *check_folder(folder).split('/'), *parts)


def cache_dir(folder, *parts):
    """studio/reference/.cache/m3/<folder>/<parts>, absolute"""
    return os.path.join(cache_root(), *check_folder(folder).split('/'), *parts)


def cache_rel(path):
    """a cache file's path below studio/reference/.cache/m3/, with forward slashes ('fighters/flute/right.bfrec')"""
    r = os.path.relpath(os.path.abspath(path), cache_root()).replace(os.sep, '/')
    if r.startswith('..') or r.count('/') < 2:
        raise ValueError(f'{path} is not a file of a folder under {common.rel(cache_root())}')
    return r


def folder_of(cache_path):
    return '/'.join(cache_rel(cache_path).split('/')[:2])


def command(folder, repin=False):
    return f"python -m tools.studio_export cache {folder}" + (' --repin' if repin else '')


# ---------------------------------------------------------------- the run: export or cache mode
class Run:
    """what one run of a folder's export has done so far: its mode, the cache files it made, the committed files it
    wrote or checked"""

    def __init__(self, mode='export', repin=False):
        assert mode in ('export', 'cache')
        self.mode, self.repin = mode, repin
        self.made = {}                         # folder -> {cache rel}
        self.committed = {}                    # folder -> {repo-relative path}


_RUN = None


def run_state():
    global _RUN
    if _RUN is None:
        _RUN = Run()
    return _RUN


def set_run(run):
    """make `run` the current run (None: a fresh export run next time); returns the one it replaces"""
    global _RUN
    old, _RUN = _RUN, run
    return old


# ---------------------------------------------------------------- pins
def pins_path(folder):
    return ref_dir(folder, PINS)


def read_pins(folder):
    """{cache rel: pin} from the folder's pins.json ({} when it has none)"""
    p = pins_path(folder)
    if not os.path.exists(p):
        return {}
    with open(p, encoding='utf-8') as f:
        doc = json.load(f)
    if doc.get('format') != PIN_FORMAT:
        raise SystemExit(f"{common.rel(p)}: format {doc.get('format')!r}, this tool reads {PIN_FORMAT}")
    return {e['path']: e for e in doc['pins']}


def write_pins(folder, pins):
    p = pins_path(folder)
    doc = dict(
        about=f'the cache files of {folder}: each is regenerated by its export on the reference PC and must have '
              f'exactly this SHA-256 and size; `path` is below {CACHE}/',
        format=PIN_FORMAT, folder=folder, cache=CACHE,
        command=command(folder),
        pins=[dict(path=r, sha256=e['sha256'], bytes=e['bytes'], export=e['export']) for r, e in sorted(pins.items())])
    common.write_json(p, doc, sort_keys=False, register=False)
    manifest.add(p, tool=__name__)


def pin(path, export=None):
    """record the cache file at `path` in its folder's pins.json: a new file is pinned; a file equal to its pin is
    left as it is; a file that differs from its pin stops the run (unless this is a `--repin` run). Returns the pin"""
    run = run_state()
    r, folder = cache_rel(path), folder_of(path)
    export = export or PREFIX[folder.split('/')[0]] + folder.split('/')[1]
    entry = dict(path=r, sha256=manifest.sha256(path), bytes=os.path.getsize(path), export=export)
    pins = read_pins(folder)
    old = pins.get(r)
    run.made.setdefault(folder, set()).add(r)
    if old is not None and (old['sha256'], old['bytes']) == (entry['sha256'], entry['bytes']):
        return old
    if old is not None and not run.repin:
        raise SystemExit(f"{CACHE}/{r}: the regenerated file differs from its pin in {common.rel(pins_path(folder))} "
                         f"(sha256 {entry['sha256'][:16]}..., {entry['bytes']} bytes; pinned "
                         f"{old['sha256'][:16]}..., {old['bytes']} bytes). Find out why before anything else: a "
                         f"re-pin ({command(folder, repin=True)}) is the controller's decision")
    if old is None and run.mode == 'cache' and not run.repin:
        raise SystemExit(f'{CACHE}/{r}: made by {export} but not pinned in {common.rel(pins_path(folder))}: run the '
                         f'export ({export}) first, or `{command(folder, repin=True)}`')
    pins[r] = entry
    write_pins(folder, pins)
    return entry


def finish(folder):
    """end a folder's run: every pin must be a file this run made (a pin it no longer makes stops the run, or is
    dropped by --repin), then check_pins"""
    run = run_state()
    made = run.made.get(folder, set())
    pins = read_pins(folder)
    stale = sorted(set(pins) - made)
    if stale and not run.repin:
        raise SystemExit(f"{folder}: pinned but no longer made: {', '.join(stale)}. If that is intended, "
                         f"`{command(folder, repin=True)}` (the controller's decision on a merged folder)")
    if stale:
        for r in stale:
            del pins[r]
        write_pins(folder, pins)
    check_pins(folder)


def check_pins(folder):
    """every pinned file of the folder is present and equal to its pin; else SystemExit naming each file and the
    command that regenerates it. Returns the number of pins"""
    folder = check_folder(folder)
    problems = []
    pins = read_pins(folder)
    for r, e in sorted(pins.items()):
        p = os.path.join(cache_root(), *r.split('/'))
        if not os.path.isfile(p):
            problems.append(f'missing  {CACHE}/{r}')
        elif os.path.getsize(p) != e['bytes'] or manifest.sha256(p) != e['sha256']:
            problems.append(f'changed  {CACHE}/{r}  (differs from its pin)')
    if problems:
        raise SystemExit(f"{folder}: the cache does not match {common.rel(pins_path(folder))}:\n  "
                         + '\n  '.join(problems) + f'\nrun: {command(folder)}')
    return len(pins)


# ---------------------------------------------------------------- writing a folder's files
def commit_bytes(folder, name, data, tool):
    """a committed file of the folder: written and listed in its shard (export mode), or compared byte for byte
    with the file already there (cache mode: nothing is written; a difference stops the run). Returns the path"""
    run = run_state()
    path = ref_dir(folder, *name.split('/'))
    run.committed.setdefault(folder, set()).add(common.rel(path))
    if run.mode == 'cache':
        have = open(path, 'rb').read() if os.path.isfile(path) else None
        if have != data:
            how = 'is missing' if have is None else 'would change'
            raise SystemExit(f'{common.rel(path)} {how}: the cache run must reproduce the committed files. Run the '
                             f'export ({tool.rsplit(".", 1)[-1]}) and review the difference')
        return path
    common.write_bytes(path, data)
    manifest.add(path, tool=tool)
    return path


def commit_json(folder, name, obj, tool, sort_keys=True):
    return commit_bytes(folder, name, common.dumps(obj, sort_keys).encode('utf-8'), tool)


def cache_bytes(folder, name, data, export):
    """a cache file of the folder: written, then pinned"""
    path = cache_dir(folder, *name.split('/'))
    common.write_bytes(path, data)
    pin(path, export)
    return path


def cache_json(folder, name, obj, export, sort_keys=True):
    return cache_bytes(folder, name, common.dumps(obj, sort_keys).encode('utf-8'), export)


# ---------------------------------------------------------------- the spec
SPEC_KEYS = ('name', 'opponent', 'fights', 'scenarios', 'frames', 'roles', 'voice_notes', 'hooks')
FIGHT_DEFAULTS = dict(base='flute_vs_trombone', handicap=(1.0, 1.0), dmg_scale=1.0, rules={}, stats=2000, scan=500,
                      left=None, right=None, must=())
FRAME_DEFAULTS = dict(left=64, right=32, times=())
VOICE_DEFAULTS = dict(pitches=None, velocities=(0.6, 0.9), durations=(0.12, 0.25, 1.0))
SCENARIO_KEYS = ('name', 'seeds', 'overrides', 'swapped', 'opponent')
LV = (2.0, 5.0)


def slug(name):
    """the kit's folder name of a fighter: name.lower().replace(' ', '_') ('BASS CLARINET' -> 'bass_clarinet')"""
    return name.lower().replace(' ', '_')


def _merge(where, given, defaults):
    if given is None:
        return dict(defaults)
    if not isinstance(given, dict):
        raise SystemExit(f'{where}: a dict, not {type(given).__name__}')
    unknown = sorted(set(given) - set(defaults))
    if unknown:
        raise SystemExit(f"{where}: unknown key(s) {', '.join(unknown)} (known: {', '.join(defaults)})")
    return {**defaults, **given}


def default_scenarios(name, opponent):
    """Template F's scenarios: early_levels, early_low, early_ko and plain (three seeds each) against the opponent,
    swapped (three), and two plain fights against TROMBONE (a lance against its kinds) when neither fighter is it"""
    out = [dict(name='early_levels', seeds=(1, 2, 3), overrides=dict(LV_DMG=LV)),
           dict(name='early_low', seeds=(1, 2, 3), overrides=dict(LV_DMG=LV, LOW_HP=92.0)),
           dict(name='early_ko', seeds=(1, 2, 3), overrides=dict(dmg_scale=3.0)),
           dict(name='plain', seeds=(7, 8, 9)),
           dict(name='swapped', seeds=(1, 2, 3), swapped=True)]
    if 'TROMBONE' not in (name, opponent):
        out.append(dict(name='vs_trombone', seeds=(1, 2), opponent='TROMBONE'))
    return out


def normalize(spec):
    """the spec with every default filled in, checked: unknown keys, wrong types, unknown or retired fighters"""
    from . import roster as R
    if not isinstance(spec, dict):
        raise SystemExit('SPEC: a dict')
    unknown = sorted(set(spec) - set(SPEC_KEYS))
    if unknown:
        raise SystemExit(f"SPEC: unknown key(s) {', '.join(unknown)} (known: {', '.join(SPEC_KEYS)})")
    table = common.roster()
    for key in ('name', 'opponent'):
        n = spec.get(key)
        if not isinstance(n, str) or n not in table or n in R.RETIRED:
            raise SystemExit(f'SPEC.{key}: {n!r} is not a fighter of the roster')
    if spec['name'] == spec['opponent']:
        raise SystemExit('SPEC.opponent: the fighter itself')
    out = dict(name=spec['name'], opponent=spec['opponent'])
    f = _merge('SPEC.fights', spec.get('fights'), FIGHT_DEFAULTS)
    f['handicap'] = tuple(float(x) for x in f['handicap'])
    assert len(f['handicap']) == 2, 'SPEC.fights.handicap: two numbers, left corner first'
    f['dmg_scale'] = float(f['dmg_scale'])
    for k in ('stats', 'scan'):
        if not (isinstance(f[k], int) and f[k] >= 1):
            raise SystemExit(f'SPEC.fights.{k}: a positive int')
    for k in ('left', 'right'):
        if f[k] is not None and not (isinstance(f[k], int) and f[k] >= 0):
            raise SystemExit(f'SPEC.fights.{k}: a seed (int) or None')
    if not isinstance(f['rules'], dict):
        raise SystemExit('SPEC.fights.rules: a dict')
    f['must'] = tuple(f['must'])
    out['fights'] = f
    scen = spec.get('scenarios')
    scen = default_scenarios(out['name'], out['opponent']) if scen is None else scen
    groups, seen = [], set()
    for i, s in enumerate(scen):
        s = _merge(f'SPEC.scenarios[{i}]', s, dict(name=None, seeds=None, overrides={}, swapped=False, opponent=None))
        if not isinstance(s['name'], str) or not s['name'] or s['name'] in seen or '/' in s['name']:
            raise SystemExit(f'SPEC.scenarios[{i}].name: a new name')
        seen.add(s['name'])
        s['seeds'] = tuple(s['seeds'] or ())
        if not s['seeds'] or not all(isinstance(x, int) and x >= 0 for x in s['seeds']):
            raise SystemExit(f'SPEC.scenarios[{i}].seeds: one or more seeds')
        s['opponent'] = s['opponent'] or out['opponent']
        if s['opponent'] not in table or s['opponent'] in R.RETIRED or s['opponent'] == out['name']:
            raise SystemExit(f"SPEC.scenarios[{i}].opponent: {s['opponent']!r}")
        groups.append(s)
    out['scenarios'] = groups
    out['frames'] = _merge('SPEC.frames', spec.get('frames'), FRAME_DEFAULTS)
    out['frames']['times'] = tuple(out['frames']['times'])
    out['roles'] = tuple(spec.get('roles') or ())
    out['voice_notes'] = _merge('SPEC.voice_notes', spec.get('voice_notes'), VOICE_DEFAULTS)
    out['hooks'] = tuple(spec.get('hooks') or ())
    return out


# ---------------------------------------------------------------- fight configs
def pair_rules(a, b):
    """the series' opt-in rules for this pair, whichever side each stands on"""
    return {rule: True for x, y, rule in PAIR_RULES if {a, b} == {x, y}}


def config_text(base, fighters, handicap, dmg_scale, rules):
    """the kit example `base`'s episode.py with the fight's own settings appended (later assignments win)"""
    src = os.path.join(common.REPO, 'projects', 'ballfight', 'examples', base, 'episode.py')
    with open(src, encoding='utf-8', newline='') as fh:
        text = fh.read()
    if not text.endswith('\n'):
        text += '\n'
    return (text + '\n# ---------------------------------------------------------------- tools/studio_export m3refs\n'
            f'FIGHTERS = {tuple(fighters)!r}\nDMG_SCALE = {float(dmg_scale)!r}\n'
            f'DMG_SIDE = {tuple(float(x) for x in handicap)!r}\nRULES = {dict(sorted(rules.items()))!r}\n')


def config_folder(tag, base, text):
    """write the config under studio/reference/.cache/episodes/<tag>/<base>/ (if it changed); returns the folder"""
    folder = os.path.join(common.CACHE, 'episodes', tag, base)
    dst = os.path.join(folder, 'episode.py')
    data = text.encode('utf-8')
    if not (os.path.exists(dst) and open(dst, 'rb').read() == data):
        common.write_bytes(dst, data)
    return folder


def load_config(folder):
    """make the config in `folder` the kit's current episode; returns (episode, story)"""
    E, story = common.kit()
    ep = E.load(folder)
    return ep, story


class Config:
    """one fight setup of a folder: who stands where, the handicap, the damage scale, the rules"""

    def __init__(self, tag, base, fighters, handicap, dmg_scale, rules):
        self.tag, self.base, self.fighters = tag, base, tuple(fighters)
        self.handicap, self.dmg_scale, self.rules = tuple(handicap), dmg_scale, dict(rules)
        self.folder = config_folder(tag, base, config_text(base, fighters, handicap, dmg_scale, rules))

    def load(self):
        ep, story = load_config(self.folder)
        assert tuple(ep.T.FIGHTERS) == self.fighters, (ep.T.FIGHTERS, self.fighters)
        return ep, story

    def facts(self, story, over=None):
        ep, _ = self.load()
        T = ep.T
        return dict(base=self.base, fighters=list(self.fighters), handicap=list(self.handicap),
                    dmg_scale=self.dmg_scale, rules_on=dict(self.rules), overrides=dict(over or {}),
                    bpm=T.BPM, key=T.KEY if hasattr(T, 'KEY') else None, t_fight=T.T_FIGHT,
                    rules=dict(vars(story.rules(**(over or {})))))


def fighter_configs(spec):
    """{(opponent, swapped): Config} for every fight the spec has"""
    s, f = spec['name'], spec['fights']
    out = {}
    wanted = {(spec['opponent'], False), (spec['opponent'], True)}
    wanted |= {(g['opponent'], g['swapped']) for g in spec['scenarios']}
    for opp, swapped in sorted(wanted):
        fighters = (opp, s) if swapped else (s, opp)
        handicap = tuple(reversed(f['handicap'])) if swapped else f['handicap']
        rules = {**pair_rules(s, opp), **f['rules']}
        tag = f"m3_{slug(s)}_{slug(opp)}{'_swapped' if swapped else ''}"
        out[(opp, swapped)] = Config(tag, f['base'], fighters, handicap, f['dmg_scale'], rules)
    return out


# ---------------------------------------------------------------- recordings
def record(cfg, seed, over, path):
    """story.run_seed(seed, **over) under cfg, written to `path` (not listed anywhere), read back and checked as the
    reference fights are: value for value, and the kit's replay-guard hash again from the file alone. Returns the
    recording read back"""
    from . import fights, recording
    _, story = cfg.load()
    rules = story.rules(**over)
    r = story.run_seed(seed, **over)
    recording.write_bfrec(r, rules, path, rect_keys=story.REC_KEYS, register=False)
    rec = recording.read_bfrec(path)
    fights.check(r, rules, rec, story.REC_KEYS)
    assert fights.replay_hash(fights.as_kit_run(rec)) == fights.replay_hash(r), f'{seed}: the replay guard'
    return rec


def rec_note(rec, **more):
    from . import recording
    return dict(seed=rec['seed'], names=list(rec['names']), winner=rec['winner'], t_ko=rec['t_ko'],
                events=len(rec['events']), frames=len(rec['arrays']['HP']), hash=recording.rec_hash(rec), **more)


def existing_copy(sha):
    """the repo-relative path of a committed reference recording with these bytes, or None"""
    for shard, entries in manifest.load_all().items():
        for e in entries:
            if e['sha256'] == sha and e['path'].endswith('.bfrec'):
                return e['path']
    return None


# ---------------------------------------------------------------- statistics: a spawn pool, three workers
_STORY = None


def _init(folder):
    global _STORY
    _, _STORY = load_config(folder)


def _row(seed):
    from . import stats
    return stats.row(_STORY, seed)


def statistics(cfg, n):
    """[(row, items)] for seeds 1..n under cfg, as stats.py computes them, in seed order; seeds 1 to 3 are checked
    against direct calls in this process"""
    from . import stats
    seeds = list(range(1, n + 1))
    with mp.get_context('spawn').Pool(WORKERS, initializer=_init, initargs=(cfg.folder,)) as pool:
        got = pool.map(_row, seeds, chunksize=8)
    assert [g[0]['seed'] for g in got] == seeds
    for r, _ in got:
        stats.check_row(r)
    _, story = cfg.load()
    for seed in seeds[:3]:
        direct, items = stats.row(story, seed)
        common.same(direct, got[seed - 1][0], f'stats row {seed}', ordered=True)
        assert items == got[seed - 1][1], seed
    return got


# ---------------------------------------------------------------- the fighter's references
def ability_names(name):
    ab = common.roster()[name]['abilities']
    return [ab[s]['name'] for s in ('unlock1', 'unlock2', 'low')]


def fighter_refs(spec):
    """write fighters/<slug>/ (committed) and its cache files for a fighter_<slug>.py SPEC (module docstring)"""
    from . import recording, scenarios as SC, stats
    module = sys._getframe(1).f_globals.get('__name__', __name__)
    spec = normalize(spec)
    name, opp, f = spec['name'], spec['opponent'], spec['fights']
    folder = f'fighters/{slug(name)}'
    export = f'fighter_{slug(name)}'
    run = run_state()
    run.made.pop(folder, None)
    t_all = time.time()
    cfgs = fighter_configs(spec)
    main = cfgs[(opp, False)]
    abilities = ability_names(name)

    # ---- statistics: X left against the opponent, seeds 1..stats
    t0 = time.time()
    got = statistics(main, f['stats'])
    rows = [g[0] for g in got]
    counts = {}
    for _, items in got:
        for k in items:
            counts[k] = counts.get(k, 0) + 1
    _, story = main.load()
    pooled = stats.pooled(rows, main.fighters)
    cache_json(folder, 'stats_rows.json', dict(
        about=f'{name} (left) against {opp}: one row per seed 1..{f["stats"]}, the columns of stats.py',
        columns=list(stats.ROW_KEYS), rows=rows), export)
    print(f"{export}: statistics {f['stats']} fights in {time.time() - t0:.1f} s on {WORKERS} workers: "
          f"{name} wins {100 * pooled['win_share']:.1f} % of {pooled['decided']}, median K.O. "
          f"{pooled['ko_median']:.2f} s")

    # ---- the two look fights
    want = {f'ability/{a}' for a in abilities}
    if f['left'] is None:
        left_seed = next((r['seed'] for r, items in got if r['winner'] == 0 and want <= set(items)), None)
        if left_seed is None:
            raise SystemExit(f'{export}: no statistics fight has {name} winning from the left with all three '
                             f'abilities ({", ".join(abilities)}); set fights.left')
    else:
        left_seed = f['left']
    tmp = cache_dir(folder, 'left.bfrec.tmp')
    rec_left = record(main, left_seed, {}, tmp)
    data = open(tmp, 'rb').read()
    os.remove(tmp)
    assert rec_left['winner'] == 0, f'{export}: left seed {left_seed}: {name} does not win'
    copy = existing_copy(hashlib.sha256(data).hexdigest())
    if copy and not copy.startswith(f'{REF}/{folder}/'):
        left_file = copy[len(REF) + 1:]
        print(f'{export}: left seed {left_seed} is {copy} byte for byte: pointing at it')
        own = ref_dir(folder, 'left.bfrec')
        if run.mode == 'export' and os.path.exists(own):          # a copy an earlier run wrote
            os.remove(own)
            manifest.remove([own])
    else:
        commit_bytes(folder, 'left.bfrec', data, module)
        left_file = f'{folder}/left.bfrec'
    swapped = cfgs[(opp, True)]
    _, story = swapped.load()
    if f['right'] is None:
        right_seed = None
        for seed in range(1, f['scan'] + 1):
            r = story.run_seed(seed, record=False)
            if r['winner'] == 0:
                right_seed = seed
                break
        if right_seed is None:
            raise SystemExit(f'{export}: {opp} never wins from the left in seeds 1..{f["scan"]}; set fights.right')
    else:
        right_seed = f['right']
    rpath = cache_dir(folder, 'right.bfrec')
    rec_right = record(swapped, right_seed, {}, rpath)
    assert rec_right['winner'] == 0, f'{export}: right seed {right_seed}: {name} does not lose'
    pin(rpath, export)
    same_as = {}                                  # cache file -> the committed reference recording it equals
    copy = existing_copy(manifest.sha256(rpath))
    if copy:
        same_as[f'{folder}/right.bfrec'] = copy

    # ---- scenarios, extended until every required item is covered
    by_data = sorted(k for k, n in counts.items() if n >= REQUIRE_FIGHTS and not k.startswith('ability/'))
    required = sorted(set(by_data) | want | set(f['must']))
    groups = [dict(g, seeds=list(g['seeds'])) for g in spec['scenarios']]
    recs, done = [], set()
    t_fight = None
    for rnd in range(EXTRA_ROUNDS + 1):
        for g in groups:
            cfg = cfgs[(g['opponent'], g['swapped'])]
            for seed in g['seeds']:
                if (g['name'], seed) in done:
                    continue
                done.add((g['name'], seed))
                rel_name = f"scenarios/{g['name']}_s{seed}.bfrec"
                path = cache_dir(folder, *rel_name.split('/'))
                rec = record(cfg, seed, g['overrides'], path)
                pin(path, export)
                copy = existing_copy(manifest.sha256(path))
                if copy:
                    same_as[f'{folder}/{rel_name}'] = copy
                recs.append(((g['name'], seed, f'{folder}/{rel_name}'), rec))
                ep, _ = cfg.load()
                t_fight = ep.T.T_FIGHT if t_fight is None else t_fight
                assert ep.T.T_FIGHT == t_fight, 'one FIGHT! time for the folder'
        best = SC.first_uses(recs, t_fight)
        missing = [k for k in required if k not in best or best[k]['after_fight'] > WINDOW]
        if not missing or rnd == EXTRA_ROUNDS:
            break
        print(f"{export}: not covered yet: {', '.join(missing)}; {EXTRA_SEEDS} more seeds per scenario")
        for g in groups:
            top = max(g['seeds'])
            g['seeds'] += list(range(top + 1, top + 1 + EXTRA_SEEDS))
    if missing:
        raise SystemExit(f"{export}: not covered within {WINDOW} s of FIGHT!: "
                         + ', '.join(f"{k} ({best[k]['after_fight']:.2f} s)" if k in best else f'{k} (never)'
                                     for k in missing))
    rows_cov = []
    for k in sorted(best):
        b = best[k]
        rows_cov.append(dict(item=k, required=k in required, stats_fights=counts.get(k, 0), scenario=b['scenario'],
                             seed=b['seed'], t=b['t'], after_fight=b['after_fight'], index=b['index'],
                             event=b['event']))
    scen_doc = []
    for g in groups:
        cfg = cfgs[(g['opponent'], g['swapped'])]
        _, story = cfg.load()
        files = [f"{folder}/scenarios/{g['name']}_s{x}.bfrec" for x in g['seeds']]
        hashes = {x: recording.rec_hash(rec) for (n, x, _), rec in recs if n == g['name']}
        scen_doc.append(dict(name=g['name'], opponent=g['opponent'], swapped=g['swapped'],
                             fighters=list(cfg.fighters), overrides=dict(g['overrides']), seeds=list(g['seeds']),
                             cache=files, hashes=[hashes[x] for x in g['seeds']],
                             rules=dict(vars(story.rules(**g['overrides'])))))

    # ---- fight.json
    _, story = main.load()
    doc = dict(
        about=f"{name}'s fight references (milestone 3, Template F): the config, the two look fights, the scenario "
              f"fights, the statistics against {opp} and the coverage table",
        tool=module, format=1, fighter=name, slug=slug(name), opponent=opp, abilities=abilities,
        spec=spec,
        config=main.facts(story),
        swapped_config=dict(fighters=list(swapped.fighters), handicap=list(swapped.handicap)),
        left=rec_note(rec_left, file=left_file,
                      why=f'the lowest statistics seed with {name} in the left corner winning and all three of its '
                          f'abilities firing' if f['left'] is None else 'fixed by the spec'),
        right=rec_note(rec_right, cache=f'{folder}/right.bfrec',
                       why=f'the lowest seed with {opp} in the left corner winning (so {name} is knocked out)'
                           if f['right'] is None else 'fixed by the spec'),
        scenarios=scen_doc,
        same_as=dict(sorted(same_as.items())),
        stats=dict(seeds=[1, f['stats']], cache=f'{folder}/stats_rows.json', columns=list(stats.ROW_KEYS),
                   pooled=pooled, definitions={**stats.DEFINITIONS, 'n': f"fights: seeds 1 to {f['stats']}"},
                   fights_with=dict(sorted(counts.items()))),
        coverage=dict(window=WINDOW, require_fights=REQUIRE_FIGHTS, t_fight=t_fight, required=required,
                      rule='required = every event kind and hit sub-kind ("hit/<what>") in at least require_fights '
                           'of the statistics fights, the fighter\'s three abilities and spec.fights.must; a row is '
                           'an item\'s first occurrence over the scenario fights (the earliest t; a tie goes to the '
                           'scenario listed first, then the lower seed): its scenario and seed (rules in '
                           '`scenarios`), t, after_fight = t - t_fight, its index in the events and the event',
                      rows=rows_cov),
        cache_files='cache files are under ' + CACHE + '/ and pinned in pins.json')
    commit_json(folder, 'fight.json', doc, module)
    finish(folder)
    print(f"{export}: {folder}: left seed {left_seed} ({len(rec_left['events'])} events, "
          f"K.O. {rec_left['t_ko']}), right seed {right_seed}, {len(recs)} scenario fights, {len(rows_cov)} coverage "
          f"rows ({len(required)} required), {len(read_pins(folder))} pins, {len(same_as)} cache recordings equal to "
          f"committed ones, {time.time() - t_all:.1f} s")
    for k, v in sorted(same_as.items()):
        print(f'{export}:   {CACHE}/{k} = {v}')


def theme_refs(spec):
    """themes/<name>/ + cache: Task 2 writes the big top's part (voicings, the bed alone), Template T the rest"""
    raise SystemExit('theme_refs: not written yet (Task 2 and the theme ports fill it)')


def episode_refs(spec):
    """episodes/<pair>/ + cache: Task 6 and Template P fill it"""
    raise SystemExit('episode_refs: not written yet (Task 6 and the pair gates fill it)')


# ---------------------------------------------------------------- the budget
def budget_rows(files):
    """[{folder, bytes, limit, files}] for every milestone 3 folder that has files, then the whole set
    ('studio/reference'), from `files`: (path below studio/reference/, bytes) pairs"""
    per, total = {}, 0
    for rel_path, size in files:
        total += size
        parts = rel_path.split('/')
        if len(parts) >= 3 and parts[0] in AREAS:
            row = per.setdefault(f'{parts[0]}/{parts[1]}', dict(bytes=0, files=0, recordings=0))
            row['bytes'] += size
            row['files'] += 1
            row['recordings'] += rel_path.endswith('.bfrec')
    out = []
    for folder in sorted(per):
        area, name = folder.split('/')
        row = per[folder]
        if area == 'fighters':
            limit = FIGHTER_BUDGET
        elif area == 'themes':
            limit = THEME_BUDGET + THEME_PER_RECORDING * row['recordings']
        else:
            limit = EPISODE_BUDGET_LARGE if name in LARGE_EPISODES else EPISODE_BUDGET
        out.append(dict(folder=folder, bytes=row['bytes'], limit=limit, files=row['files']))
    out.append(dict(folder=REF, bytes=total, limit=TOTAL_BUDGET, files=len(files)))
    return out


def tracked_files():
    """(path below studio/reference/, bytes) of every file git tracks there or would track (untracked, not
    ignored); a tracked file deleted from the working tree is left out"""
    text = common.git('ls-files', '--cached', '--others', '--exclude-standard', '-z', '--', REF)
    out = {}
    for r in text.split('\0'):
        if not r:
            continue
        p = os.path.join(common.REPO, *r.split('/'))
        if os.path.isfile(p):
            out[r[len(REF) + 1:]] = os.path.getsize(p)
    return sorted(out.items())


def budget_cmd(args):
    if args:
        raise SystemExit('budget takes no arguments')
    rows = budget_rows(tracked_files())
    over = [r for r in rows if r['bytes'] > r['limit']]
    for r in rows:
        mark = 'OVER' if r['bytes'] > r['limit'] else 'ok'
        print(f"budget: {r['folder']:40s} {r['bytes']:>12,d} of {r['limit']:>12,d} bytes  {r['files']:4d} files  {mark}")
    if over:
        raise SystemExit('budget: over: ' + ', '.join(f"{r['folder']} ({r['bytes']:,d} > {r['limit']:,d})"
                                                      for r in over))


# ---------------------------------------------------------------- the cache command
def export_for(folder):
    """the export that makes a folder: the one its pins name, else <prefix><name> by the folder's area"""
    names = {e['export'] for e in read_pins(folder).values()}
    if len(names) > 1:
        raise SystemExit(f"{folder}: pins.json names several exports: {', '.join(sorted(names))}")
    area, name = folder.split('/')
    return names.pop() if names else PREFIX[area] + name


def pinned_folders():
    out = []
    for area in AREAS:
        d = os.path.join(common.REFERENCE, area)
        if os.path.isdir(d):
            out += [f'{area}/{n}' for n in sorted(os.listdir(d)) if os.path.isfile(os.path.join(d, n, PINS))]
    return out


def cache_cmd(args):
    repin = '--repin' in args
    names = [a for a in args if a != '--repin']
    flags = [a for a in names if a.startswith('-')]
    if flags:
        raise SystemExit(f"cache: unknown option {', '.join(flags)} (only --repin)")
    if not names:
        raise SystemExit('cache: name the folders (<area>/<name>, for example fighters/flute) or all')
    if 'all' in names:
        if names != ['all']:
            raise SystemExit('cache: all stands alone')
        folders = pinned_folders()
    else:
        folders = [check_folder(n) for n in names]
    from . import __main__ as entry
    found = entry.exports()
    for folder in folders:
        name = export_for(folder)
        if name not in found:
            raise SystemExit(f'cache: {folder}: no export named {name}')
        t0 = time.time()
        old = set_run(Run('cache', repin))
        try:
            found[name][1]()
            n = check_pins(folder)
        finally:
            set_run(old)
        print(f'[cache {folder}: {name}, {n} pinned files match, {time.time() - t0:.1f} s]')


EXPORTS = {'cache': cache_cmd, 'budget': budget_cmd}
TAKES_ARGS = ('cache', 'budget')

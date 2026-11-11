"""The milestone 3 reference framework: where a fighter's, a theme's and a pair gate's reference data lives, the
pinned cache, the per-fighter export, and the size budgets.

    REF = 'studio/reference'; CACHE = 'studio/reference/.cache/m3'
    fighter_refs(spec)   a fighter_<slug>.py's SPEC: writes fighters/<slug>/ (committed) and its cache files
    theme_refs(spec)     a theme_<name>.py SPEC: themes/<name>/theme.json (each reference config's recording and its
                         bed alone under two base Sounds), voicings.json (12 roots x 13 qualities); Task 2 wrote
                         this part, the theme ports add the variants and their cache files (Template T)
    episode_refs(spec)   episodes/<pair>/ + cache (filled by Task 6 and the pair gates)
    pin(path)            record a cache file in its folder's staged pins (path, sha256, bytes, export)
    finish(folder)       end a folder's run: check every pin, then apply the committed writes and pins.json
    check_pins(folder)   every pinned file present and equal, else SystemExit naming the command to run

    python -m tools.studio_export cache <area>/<name>... | all   regenerate cache files and check the pins
    python -m tools.studio_export remake <area>/<name>...        make a folder again whole (a port's own folder)
    python -m tools.studio_export budget                         sizes per folder against the budgets

Storage (plan section 4). Every milestone 3 folder, `studio/reference/fighters/<slug>/`, `themes/<name>/` and
`episodes/<pair>/`, holds two kinds of data:

- committed files (in git, listed in the folder's own manifest shard `manifest.json`): what the always-run gates
  read, small enough for the budgets below. A fighter's model, `studio/crates/bf-content/data/models/<NAME>.json`,
  belongs to its folder too: its entry is in the fighter's shard, so a port never touches a shared manifest;
- cache files, under `studio/reference/.cache/m3/<area>/<name>/` (git-ignored): the bulky data the `#[ignore]`d
  gates read (scenario recordings, statistics rows, sprites, stems, ...). The export writes them, and the folder's
  committed `pins.json` records each one's SHA-256, size and the export that makes it. Because the kit is
  deterministic on the reference PC with the pinned libraries, a regenerated cache file is the pinned file, and its
  pin proves it.

Modes (Run). Every mode does the same work; what differs is what it may change:

- export (`python -m tools.studio_export fighter_flute`): writes the committed files and records the pins of new
  cache files. A cache file that differs from its pin, or a pin no longer made, stops it.
- cache (`cache fighters/flute`, `cache all`): must reproduce every committed file byte for byte and every pinned
  cache file; it writes no committed file, so regenerating the cache never touches the working tree.
- remake (`remake fighters/flute`): the one command that makes a folder again whole after its spec changed:
  committed files, the shard and the pins (changed pins replaced, stale ones dropped). It is typed by a port for its
  own folder before it is merged; on a merged folder it is the controller's decision (ruling M3-R1). Nothing runs
  it implicitly.

In every mode the committed writes, deletions and pins.json wait in the run's stage until finish(folder), which
checks every pin against its file first: a run that stops leaves the committed state exactly as it was.

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
check parts, which read `frames`, `roles`, `voice_notes` and `hooks`; see the hand-over at the end):

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
written is measured before it is committed. bf-testdata's `budget` test applies the same numbers.
Hand-over to Task 6 (the proving harness), which adds the picture, sound, voice and check parts to fighter_refs:

- The model: fighter_refs calls fighter_model(folder, NAME, P, tool) (models.model through commit_path) in the
  process that built the fighter's picture, so the model's entry goes to the fighter's shard and a cache run only
  compares it.
- The picture: kit.picture builds itself once per process from the current episode, so each picture part runs in a
  fresh process (a spawn worker): fighter_picture(cfg.with_seed(seed)) with the `left` (or `right`) seed, which
  appends SEED to the config.
- Sprites (ruling M3-R18): fighter_sprites(folder, NAME, P, left recording, export) writes
  .cache/m3/fighters/<slug>/sprites.npz and sprites.json (pinned) and returns {case id: SHA-256}; only those hashes
  are committed (in look.json). It works for any roster name (proven on TUBA against FLUTE: 282 sprites, model 1,054
  faces). Milestone 1's studio/reference/sprites/{FLUTE,TROMBONE}.{npz,json} stay where they are.
- The committed files Template F implies, all in fighters/<slug>/: look.json (left's frame, hook-log and draw-log
  SHA-256, the hull log, title-card and title-hook hashes, the sprite hashes), sound.json (the 16-bit master's
  SHA-256, per-bus RMS, the note order, for left and right), voice.json (the fingerprints), checks.json (the kit's
  check numbers for left) and exceptions.toml (frames the Studio may miss, with reasons; written empty once, then
  adopted by hand, as frames/exceptions.toml). `right`'s frame hashes are cached (look_right.json, pinned) while
  left's are committed; stems and whole voice notes are cached too.
- Pair folders (episode_refs): episodes/<pair>/ is named after the kit config without its `ballfight_` prefix and
  its examples/ folder (`examples/trombone_vs_tuba` -> trombone_vs_tuba, `ballfight_horn_vs_trombone_rematch` ->
  horn_vs_trombone_rematch); LARGE_EPISODES relies on that rule. Theme folders are the mood's name (themes/big_top).
- Every part stages its committed files with commit_bytes / commit_json / commit_path and its cache files with
  cache_bytes / cache_json / pin, and fighter_refs calls finish(folder) once, at the end."""
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


def command(folder):
    """the command that regenerates a folder's cache and checks its pins"""
    return f'python -m tools.studio_export cache {folder}'


def remake_command(folder):
    """the one command that re-makes a folder whole: committed files, shard and pins"""
    return f'python -m tools.studio_export remake {folder}'


# ---------------------------------------------------------------- the run: export, cache or remake mode
MODES = ('export', 'cache', 'remake')


class Stage:
    """one folder's part of a run: the pins as they were and as they will be, the cache files made, and the
    committed writes and deletions waiting for finish()"""

    def __init__(self, folder):
        self.folder = folder
        self.old_pins = read_pins(folder)
        self.pins = dict(self.old_pins)
        self.made = set()
        self.commits = {}                      # absolute path -> (bytes, tool)
        self.deletes = set()                   # absolute paths


class Run:
    """one run of exports: its mode and each folder's stage. `export` (python -m tools.studio_export fighter_<slug>):
    committed files are written, new cache files pinned, a changed or stale pin stops it. `cache`: nothing
    committed may change, every cache file must equal its pin. `remake` (python -m tools.studio_export remake
    <folder>): committed files, shard and pins are all made again, changed pins replaced and stale ones dropped. In
    every mode the committed writes and pins.json are applied only in finish(), after the last check: a run that
    stops leaves the committed state as it was"""

    def __init__(self, mode='export'):
        assert mode in MODES, mode
        self.mode = mode
        self.stages = {}

    def stage(self, folder):
        if folder not in self.stages:
            self.stages[folder] = Stage(folder)
        return self.stages[folder]


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
    """{cache rel: pin} from the folder's pins.json on disk ({} when it has none)"""
    p = pins_path(folder)
    if not os.path.exists(p):
        return {}
    with open(p, encoding='utf-8') as f:
        doc = json.load(f)
    if doc.get('format') != PIN_FORMAT:
        raise SystemExit(f"{common.rel(p)}: format {doc.get('format')!r}, this tool reads {PIN_FORMAT}")
    return {e['path']: e for e in doc['pins']}


def pins_bytes(folder, pins):
    doc = dict(
        about=f'the cache files of {folder}: each is regenerated by its export on the reference PC and must have '
              f'exactly this SHA-256 and size; `path` is below {CACHE}/',
        format=PIN_FORMAT, folder=folder, cache=CACHE,
        command=command(folder),
        pins=[dict(path=r, sha256=e['sha256'], bytes=e['bytes'], export=e['export']) for r, e in sorted(pins.items())])
    return common.dumps(doc, sort_keys=False).encode('utf-8')


def write_pins(folder, pins):
    """write pins.json now and list it in the shard (finish() does this; the unit checks call it directly)"""
    p = pins_path(folder)
    common.write_bytes(p, pins_bytes(folder, pins))
    manifest.add(p, tool=__name__)


def pin(path, export=None):
    """record the cache file at `path` in its folder's staged pins: a new file is pinned (not in a cache run); a file
    equal to its pin is left as it is; a file that differs from its pin stops the run unless it is a remake.
    pins.json itself is written by finish(). Returns the pin"""
    run = run_state()
    r, folder = cache_rel(path), folder_of(path)
    st = run.stage(folder)
    export = export or PREFIX[folder.split('/')[0]] + folder.split('/')[1]
    entry = dict(path=r, sha256=manifest.sha256(path), bytes=os.path.getsize(path), export=export)
    old = st.old_pins.get(r)
    st.made.add(r)
    if old is not None and (old['sha256'], old['bytes']) == (entry['sha256'], entry['bytes']):
        st.pins[r] = old
        return old
    if old is not None and run.mode != 'remake':
        raise SystemExit(f"{CACHE}/{r}: the regenerated file differs from its pin in {common.rel(pins_path(folder))} "
                         f"(sha256 {entry['sha256'][:16]}..., {entry['bytes']} bytes; pinned "
                         f"{old['sha256'][:16]}..., {old['bytes']} bytes). Find out why before anything else. A port "
                         f"re-makes its own unmerged folder with `{remake_command(folder)}`; on a merged folder that "
                         f"is the controller's decision (ruling M3-R1)")
    if old is None and run.mode == 'cache':
        raise SystemExit(f'{CACHE}/{r}: made by {export} but not pinned in {common.rel(pins_path(folder))}: the '
                         f'folder was made by another spec; `{remake_command(folder)}` re-makes it')
    st.pins[r] = entry
    return entry


def finish(folder):
    """end a folder's run, all or nothing: every pin must be a file this run made (a stale pin stops an export or
    cache run; a remake drops it); every staged pin is checked against its cache file; only then are the committed
    writes, deletions and pins.json applied (export and remake). Then check_pins re-reads them from disk; returns
    the number of pins"""
    run = run_state()
    st = run.stages.pop(folder, None) or Stage(folder)
    stale = sorted(set(st.old_pins) - st.made)
    if stale and run.mode != 'remake':
        raise SystemExit(f"{folder}: pinned but no longer made: {', '.join(stale)}. A port re-makes its own unmerged "
                         f"folder with `{remake_command(folder)}` (the controller's decision on a merged folder)")
    for r in stale:
        del st.pins[r]
    problems = []
    for r, e in sorted(st.pins.items()):
        p = os.path.join(cache_root(), *r.split('/'))
        if not os.path.isfile(p) or os.path.getsize(p) != e['bytes'] or manifest.sha256(p) != e['sha256']:
            problems.append(r)
    if problems:
        raise SystemExit(f"{folder}: cache files changed during the run: {', '.join(problems)}; nothing committed "
                         f"was written")
    if run.mode != 'cache':
        for path, (data, tool) in sorted(st.commits.items()):
            have = open(path, 'rb').read() if os.path.isfile(path) else None
            if have != data:
                common.write_bytes(path, data)
            manifest.add(path, tool=tool)
        gone = [p for p in sorted(st.deletes) if os.path.exists(p)]
        for p in gone:
            os.remove(p)
        if gone:
            manifest.remove(gone)
        if st.pins != st.old_pins or (st.pins and not os.path.exists(pins_path(folder))):
            write_pins(folder, st.pins)
    return check_pins(folder)


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
def commit_path(folder, path, data, tool):
    """a committed file that belongs to `folder` (inside it, or the fighter's model under
    studio/crates/bf-content/data/models/, whose entry goes to the folder's shard): staged for finish() in an
    export or remake run, compared byte for byte with the file there in a cache run (a difference stops the run).
    Returns the path"""
    run = run_state()
    path = os.path.abspath(path)
    if manifest.shard_of(path) != ref_dir(folder, manifest.NAME):
        raise ValueError(f'{common.rel(path)} does not belong to {folder}')
    if run.mode == 'cache':
        have = open(path, 'rb').read() if os.path.isfile(path) else None
        if have != data:
            how = 'is missing' if have is None else 'would change'
            raise SystemExit(f'{common.rel(path)} {how}: a cache run must reproduce the committed files. If the spec '
                             f'changed, `{remake_command(folder)}` re-makes the folder; else find out why')
        return path
    st = run.stage(folder)
    st.commits[path] = (data, tool)
    st.deletes.discard(path)
    return path


def commit_bytes(folder, name, data, tool):
    """a committed file of the folder (`name` below it): see commit_path"""
    return commit_path(folder, ref_dir(folder, *name.split('/')), data, tool)


def commit_json(folder, name, obj, tool, sort_keys=True):
    return commit_bytes(folder, name, common.dumps(obj, sort_keys).encode('utf-8'), tool)


def delete_committed(folder, name):
    """a committed file of the folder that this run no longer makes: removed (with its entry) in finish()"""
    run = run_state()
    if run.mode != 'cache':
        path = ref_dir(folder, *name.split('/'))
        run.stage(folder).commits.pop(path, None)
        run.stage(folder).deletes.add(path)


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


def config_text(base, fighters, handicap, dmg_scale, rules, seed=None):
    """the kit example `base`'s episode.py with the fight's own settings appended (later assignments win); `seed`
    sets SEED too (the fight the kit's picture and soundtrack draw)"""
    src = os.path.join(common.REPO, 'projects', 'ballfight', 'examples', base, 'episode.py')
    with open(src, encoding='utf-8', newline='') as fh:
        text = fh.read()
    if not text.endswith('\n'):
        text += '\n'
    return (text + '\n# ---------------------------------------------------------------- tools/studio_export m3refs\n'
            f'FIGHTERS = {tuple(fighters)!r}\nDMG_SCALE = {float(dmg_scale)!r}\n'
            f'DMG_SIDE = {tuple(float(x) for x in handicap)!r}\nRULES = {dict(sorted(rules.items()))!r}\n'
            + ('' if seed is None else f'SEED = {int(seed)!r}\n'))


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

    def __init__(self, tag, base, fighters, handicap, dmg_scale, rules, seed=None):
        self.tag, self.base, self.fighters = tag, base, tuple(fighters)
        self.handicap, self.dmg_scale, self.rules, self.seed = tuple(handicap), dmg_scale, dict(rules), seed
        self.folder = config_folder(tag, base, config_text(base, fighters, handicap, dmg_scale, rules, seed))

    def with_seed(self, seed):
        """the same setup with SEED = seed, in a folder of its own (for the picture and the soundtrack)"""
        return Config(f'{self.tag}_s{seed}', self.base, self.fighters, self.handicap, self.dmg_scale, self.rules,
                      seed)

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


# ---------------------------------------------------------------- the kit's picture, sprites and model of a fighter
def fighter_picture(cfg):
    """the kit's renderer (ballfight.kit.picture) for cfg, which must carry a seed (Config.with_seed). The module
    builds everything at import from the current episode, so this works once per process, in a process that has
    not imported it yet (a spawn worker or a fresh interpreter)"""
    assert cfg.seed is not None, 'a picture needs its fight: Config.with_seed(seed)'
    if 'ballfight.kit.picture' in sys.modules:
        raise SystemExit('fighter_picture: kit.picture is already built in this process (for another episode); '
                         'draw each fighter\'s picture in a fresh process')
    ep, _ = cfg.load()
    from ballfight.kit import picture as P                # noqa: N812  (the kit's own short name for it)
    assert ep.seed == cfg.seed and tuple(P.NAMES) == cfg.fighters, (ep.seed, P.NAMES)
    return P


def fighter_sprites(folder, name, P, recording_path, export):
    """the fighter's sprite cache (ruling M3-R18): sprites.run_one for `name` in the picture P (fighter_picture of
    the fight in `recording_path`), written to .cache/m3/<folder>/sprites.npz and sprites.json and pinned. Returns
    {case id: SHA-256 of its RGBA bytes} for the folder's committed hashes (Task 6: in look.json). Milestone 1's
    committed sprite files (studio/reference/sprites/) stay where they are"""
    from . import sprites
    ready = sprites.prepare(P, recording_path)
    got = sprites.run_one(name, ready, out=(cache_dir(folder, 'sprites.npz'), cache_dir(folder, 'sprites.json')))
    pin(got['npz'], export)
    pin(got['json'], export)
    return got['sha256']


def fighter_model(folder, name, P, tool):
    """the fighter's model (models.model) in the process that built its picture P, staged through commit_path:
    its entry goes to the folder's shard, a cache run only compares it, a remake stages it. Returns the path"""
    from . import models
    return models.model(name, P=P, write=lambda path, data: commit_path(folder, path, data, tool))


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


def existing_copy(sha, folder):
    """the repo-relative path of a committed reference recording with these bytes, or None. Only the top-level
    manifest (milestones 0 to 2) and the folder's own shard are searched, never another port's folder, so one port's
    files never depend on another's"""
    for shard in (manifest.top(), ref_dir(folder, manifest.NAME)):
        for e in manifest.load(shard):
            if e['sha256'] == sha and e['path'].endswith('.bfrec'):
                return e['path']
    return None


# ---------------------------------------------------------------- statistics: a spawn pool, three workers
_STORY = None


THREAD_VARS = ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS')


def _init(folder):
    """pool worker start: one BLAS thread, the reference string hash, then the episode (spawn starts from nothing)"""
    global _STORY
    from . import __main__ as entry
    for k in THREAD_VARS:
        os.environ[k] = '1'
    assert 'numpy' not in sys.modules, 'numpy was imported before the worker set its thread count'
    assert hash(entry.HASH_PROBE[0]) == entry.HASH_PROBE[1], 'the worker does not hash strings as the reference does'
    _, _STORY = load_config(folder)


def _row(seed):
    from . import stats
    return stats.row(_STORY, seed)


def statistics(cfg, n):
    """[(row, items)] for seeds 1..n under cfg, as stats.py computes them, in seed order; seeds 1 to 3 are checked
    against direct calls in this process"""
    from . import stats
    seeds = list(range(1, n + 1))
    for k in THREAD_VARS:                         # the workers inherit it (this process has numpy loaded already)
        os.environ[k] = '1'
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
    run.stages.pop(folder, None)
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
    copy = existing_copy(hashlib.sha256(data).hexdigest(), folder)
    if copy and not copy.startswith(f'{REF}/{folder}/'):
        left_file = copy[len(REF) + 1:]
        print(f'{export}: left seed {left_seed} is {copy} byte for byte: pointing at it')
        delete_committed(folder, 'left.bfrec')                  # a copy an earlier run wrote, if any
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
    copy = existing_copy(manifest.sha256(rpath), folder)
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
                copy = existing_copy(manifest.sha256(path), folder)
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


# ---------------------------------------------------------------- a theme's references (Task 2; Template T adds)
THEME_SPEC_KEYS = ('name', 'voicings', 'configs', 'extra')
# the 12 roots (the kit's spellings: a sharp or a flat, as its configs write them) and the 13 qualities of QUALITY
ROOTS = ('C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B')


def theme_spec(spec):
    """a theme_<name>.py SPEC with its defaults: name (the mood), voicings (the Bed methods that voice a chord, each
    called for the 12 roots x 13 qualities), configs (the reference kit configs, `examples/<name>` or
    `ballfight_<pair>`: the bed alone under each one's own recording), extra (a function(folder, export, module)
    the theme's own module runs before finish, for files only that theme writes)"""
    unknown = sorted(set(spec) - set(THEME_SPEC_KEYS))
    if unknown:
        raise SystemExit(f"theme spec: unknown keys {', '.join(unknown)} (it has {', '.join(THEME_SPEC_KEYS)})")
    out = dict(voicings=(), configs=(), extra=None)
    out.update(spec)
    if not out.get('name'):
        raise SystemExit('theme spec: a name (the kit mood) is needed')
    return out


def config_source(config):
    """projects/<config>/episode.py for a kit config name ('examples/x' is under projects/ballfight/)"""
    parts = config.split('/')
    if parts[0] == 'examples':
        return os.path.join(common.REPO, 'projects', 'ballfight', *parts, 'episode.py')
    return os.path.join(common.REPO, 'projects', *parts, 'episode.py')


def config_slug(config):
    """the pair folder name of a kit config: without `examples/` and `ballfight_` (Task 1's rule)"""
    name = config.split('/')[-1]
    return name[len('ballfight_'):] if name.startswith('ballfight_') else name


def theme_config_folder(theme, config):
    """a private copy of a kit config under studio/reference/.cache/episodes/theme_<theme>/<slug>/ (the kit's own
    caches land there, never under projects/); returns the folder"""
    with open(config_source(config), encoding='utf-8', newline='') as fh:
        text = fh.read()
    return config_folder(f'theme_{theme}', config_slug(config), text)


def read_config(config):
    """the kit config's values (episode.py executed on its own, without the kit), with the bar count the kit's
    timeline derives from its BPM (kit/episode.py build_timeline: loop_bpm, then round(DUR / BAR))"""
    import importlib.util
    from reelkit.config import loop_bpm
    spec = importlib.util.spec_from_file_location('m3refs_config', config_source(config))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    bpm = loop_bpm(cfg.BPM, 61.0)
    n_bars = int(round(61.0 / (4 * (60.0 / bpm))))
    chords = list(cfg.CHORDS)
    start = getattr(cfg, 'LOOP_START', 2)
    loop = list(getattr(cfg, 'LOOP', chords[start:start + 8]))
    whole = (len(chords) - start) // len(loop)
    assert start == 2 and chords[start:start + whole * len(loop)] == loop * whole, config
    assert len(chords) == n_bars, (config, len(chords), n_bars)
    return dict(config=config, fighters=list(cfg.FIGHTERS), mood=cfg.MOOD, bpm=cfg.BPM, loop_bpm=common.hexf(bpm),
                n_bars=n_bars, key=cfg.KEY, scale=getattr(cfg, 'SCALE', 'minor'), intro=chords[:start], loop=loop,
                tail=chords[start + whole * len(loop):], chords=chords)


def chord_names():
    """every chord the harmony parses: the 12 roots x the 13 qualities, in the kit's QUALITY order"""
    E, _ = common.kit()
    return [r + q for r in ROOTS for q in E.QUALITY]


def _theme_child(folder, voicings, chords):
    """in a fresh interpreter (a spawn worker): kit.music for the config in `folder` with both fighters replaced by
    the kit's base Sound (silent), its score and master; then each voicing method of the bed for every chord.
    Returns the 16-bit master's SHA-256, the buses and the voicings"""
    import numpy as np
    for k in THREAD_VARS:
        os.environ.setdefault(k, '1')
    assert 'ballfight.kit.music' not in sys.modules
    E, _ = common.kit()
    E.load(folder)
    from ballfight.kit import music as M                  # noqa: N812  (the kit's own short name for it)
    M.SOUNDS = [M.FIGHTER_MODULES.base.Sound(M, k) for k in (0, 1)]   # read at call time by score() and master()
    mix = M.score()
    m, stems = M.master(mix)
    pcm = (np.clip(m, -1, 1) * 32767).astype('<i2')     # A.write's samples
    assert pcm.shape == (M.NA, 2)
    out = dict(seed=M.EP.seed, names=list(M.NAMES), mood=M.T.MOOD, buses=list(mix.buses),
               stems=list(stems), master_sha256=hashlib.sha256(pcm.tobytes()).hexdigest(),
               bed=dict(buses={k: list(v) for k, v in M.bed.buses.items()}, reverb=list(M.bed.reverb),
                        comp=list(M.bed.comp)))
    T = M.T
    voiced = {}
    for method in voicings:
        fn = getattr(M.bed, method)
        row = {}
        for ch in chords:
            if ch not in T.CHORD_PCS:                   # the chord's tones are read from the episode's chords
                T.CHORD_PCS[ch] = E.parse_chord(ch)[1]
            row[ch] = [int(x) for x in fn(ch)]
        voiced[method] = row
    out['voicings'] = voiced
    return out


def theme_refs(spec):
    """themes/<name>/ (committed) for a theme_<name>.py SPEC (theme_spec): theme.json (each reference config: its
    recording, the bed alone under two base Sounds: the 16-bit master's SHA-256 and the buses) and voicings.json
    (each voicing method for the 12 roots x 13 qualities, as integers). The theme ports add the variants, per-bus
    RMS, hook vectors and their cache files (Template T)"""
    module = sys._getframe(1).f_globals.get('__name__', __name__)
    spec = theme_spec(spec)
    name = spec['name']
    folder, export = f'themes/{name}', f'theme_{name}'
    run = run_state()
    run.stages.pop(folder, None)
    t_all = time.time()
    chords = chord_names()
    configs = []
    voicings = None
    for config in spec['configs']:
        t0 = time.time()
        facts = read_config(config)
        if facts['mood'] != name:
            raise SystemExit(f'{export}: {config} plays {facts["mood"]}, not {name}')
        cfg_folder = theme_config_folder(name, config)
        ep, story = load_config(cfg_folder)
        seed = ep.seed
        # the config's own fight, recorded as the reference fights are (a committed file of milestones 0 to 2, or
        # this folder's own copy)
        from . import fights, recording
        r = story.run_seed(seed)
        rules = story.rules()
        tmp = cache_dir(folder, f'{config_slug(config)}.bfrec.tmp')
        recording.write_bfrec(r, rules, tmp, rect_keys=story.REC_KEYS, register=False)
        rec = recording.read_bfrec(tmp)
        fights.check(r, rules, rec, story.REC_KEYS)
        data = open(tmp, 'rb').read()
        os.remove(tmp)
        copy = existing_copy(hashlib.sha256(data).hexdigest(), folder)
        rec_name = f'{config_slug(config)}.bfrec'
        if copy and not copy.startswith(f'{REF}/{folder}/'):
            rec_file = copy[len(REF) + 1:]
            delete_committed(folder, rec_name)
        else:
            commit_bytes(folder, rec_name, data, module)
            rec_file = f'{folder}/{rec_name}'
        with mp.get_context('spawn').Pool(1) as pool:
            got = pool.apply(_theme_child, (cfg_folder, list(spec['voicings']), chords if voicings is None else []))
        if voicings is None:
            voicings = got.pop('voicings')
        else:
            got.pop('voicings')
        assert got['seed'] == seed and got['names'] == facts['fighters'], (got['seed'], got['names'])
        configs.append(dict(facts, seed=seed, recording=rec_file, winner=rec['winner'], t_ko=rec['t_ko'],
                            bed_alone=dict(master_sha256=got['master_sha256'], buses=got['buses'],
                                           stems=got['stems'], bed=got['bed'])))
        print(f"{export}: {config}: seed {seed} ({rec_file}), the bed alone: buses {', '.join(got['buses'])}, "
              f"16-bit master {got['master_sha256'][:16]}..., {time.time() - t0:.1f} s")
    if spec['voicings']:
        commit_json(folder, 'voicings.json', dict(
            about=f'kit/moods/{name}.py Bed: each voicing method for every chord the harmony parses (12 roots x 13 '
                  f'qualities: the kit\'s QUALITY order), the chord added to T.CHORD_PCS first as the episode\'s chords '
                  f'are; the notes as MIDI numbers, in the order the method gives them',
            theme=name, roots=list(ROOTS), qualities=list(common.kit()[0].QUALITY), voicings=voicings), module)
    if spec['extra'] is not None:
        spec['extra'](folder, export, module)
    commit_json(folder, 'theme.json', dict(
        about=f'{name}: its reference configs, each with the kit\'s own recording of it and the bed alone (both '
              f'fighters replaced at run time by kit/fighters/base.py Sound, which is silent): the SHA-256 of the '
              f'16-bit master\'s samples (little-endian int16, interleaved: what A.write writes after its header), '
              f'the buses in the order the score made them, and the bed\'s class levels',
        theme=name, voicing_methods=list(spec['voicings']), configs=configs,
        cache_files='cache files are under ' + CACHE + '/ and pinned in pins.json'), module)
    finish(folder)
    print(f"{export}: {folder}: {len(configs)} reference configs, {len(chords)} chords x {len(spec['voicings'])} "
          f"voicings, {len(read_pins(folder))} pins, {time.time() - t_all:.1f} s")


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
        print(f"budget: {r['folder']:40s} {r['bytes']:>12,d} of {r['limit']:>12,d} bytes  {r['files']:4d} files  "
              f"{mark}")
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


def run_folder(folder, mode, found=None):
    """run the export that makes `folder` in `mode` ('cache' or 'remake'); returns the number of pins. `found` is
    the export registry ({name: (order, function, module)}; the entry point's when not given)"""
    if found is None:
        from . import __main__ as entry
        found = entry.exports()
    name = export_for(folder)
    if name not in found:
        raise SystemExit(f'{mode}: {folder}: no export named {name}')
    old = set_run(Run(mode))
    try:
        found[name][1]()
        leftover = sorted(run_state().stages)
        if leftover:                                  # an export must finish() every folder it staged
            raise SystemExit(f"{mode}: {name} left {', '.join(leftover)} unfinished")
        return check_pins(folder)
    finally:
        set_run(old)


def _folders(cmd, args, allow_all):
    flags = [a for a in args if a.startswith('-')]
    if flags:
        hint = ' (a pin is re-made with `remake <folder>`)' if '--repin' in flags else ''
        raise SystemExit(f"{cmd}: unknown option {', '.join(flags)}{hint}")
    if not args:
        raise SystemExit(f'{cmd}: name the folders (<area>/<name>, for example fighters/flute)'
                         + (' or all' if allow_all else ''))
    if 'all' in args:
        if not allow_all:
            raise SystemExit(f'{cmd}: all is not allowed: name your own folder')
        if args != ['all']:
            raise SystemExit(f'{cmd}: all stands alone')
        return pinned_folders()
    return [check_folder(n) for n in args]


def cache_cmd(args, found=None):
    """cache <area>/<name>... | all: regenerate the cache files and check the pins; nothing committed changes"""
    for folder in _folders('cache', args, True):
        t0 = time.time()
        n = run_folder(folder, 'cache', found)
        print(f'[cache {folder}: {export_for(folder)}, {n} pinned files match, {time.time() - t0:.1f} s]')


def remake_cmd(args, found=None):
    """remake <area>/<name>...: make the folders again whole, all or nothing: committed files, the shard and the
    pins (changed pins replaced, stale ones dropped). Typed by a port for its own folder after a spec change; on a
    folder merged into the build branch it is the controller's decision (ruling M3-R1). Never run implicitly"""
    for folder in _folders('remake', args, False):
        t0 = time.time()
        n = run_folder(folder, 'remake', found)
        print(f'[remake {folder}: {export_for(folder)}, {n} pins, {time.time() - t0:.1f} s]')


EXPORTS = {'cache': cache_cmd, 'remake': remake_cmd, 'budget': budget_cmd}
TAKES_ARGS = ('cache', 'remake', 'budget')

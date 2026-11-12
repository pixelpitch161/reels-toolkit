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
        sprite_size=None,            # the size of the sprites its look draws itself: None = 2 r + 5 (FLUTE's);
                                     #   dict(add=7, clip=(12, 19)) = 2 * clip(r, 12, 19) + 7 (TRUMPET's); left out
                                     #   of fight.json's spec when not given (sprites.own_size); a padded sprite
                                     #   adds pad=<float>: 2 * int(round(r * pad)) + add (SOUSAPHONE: add=7, pad=1.22)
        left_in_cache=False,         # ruling M3-R36: True when the left recording alone would take the folder over
                                     #   its budget (PIANO's roll, 487 KB): it goes to the pinned cache, its SHA-256
                                     #   committed, and the gates read it from there; left out of fight.json when False
    )

A neutral handicap is written DMG_SIDE = (1.0, 1.0) (config_text), so the kit records the rule dmg_side; the Studio
records none for a neutral fight file. Both are accepted as the same rules (bf_harness::fight::rules_text), and the
kit's fights are bit-identical either way (m3refs_checks `a_neutral_handicap_is_the_same_fight_either_way`), so a
port may also write DMG_SIDE = None in its own module; config_text stays as it is so no merged pin changes.

fighter_refs(spec, extra=None): `extra(spec, folder, export, module, main, left_seed, left_file)`, when given, makes
a port's own additional committed and cache files after the template's parts (TRUMPET's sound_line.json), in the
same run and stage, with left.bfrec on disk.

What fighter_refs writes for it today (the fight half of Template F; Task 6 adds the picture, sound, voice and
check parts, which read `frames`, `roles`, `voice_notes` and `hooks`; see the hand-over at the end):

- `fighters/<slug>/fight.json` (committed): the normalised spec, the fight config (fighters, handicap, damage
  scale, rules), the two look fights, every scenario with its full rules, the statistics' pooled figures and
  `fights_with` counts, and the coverage table: for every required item its first occurrence over the scenario
  fights (scenario, seed, time, event index, the event itself).
- `fighters/<slug>/left.bfrec` (committed): X in the left corner and winning, all three of its abilities fired, and
  the K.O. after 44 s (CHECKS_FROM: the checks part runs the kit's music.main(), which needs it): the lowest such
  seed among the statistics fights (or `fights.left`, which must meet the K.O. rule too; the run stops at once,
  saying so, when it does not or no seed does). When the bytes equal a recording the set already holds, fight.json
  points at that file instead of copying it. The parts read it by its committed path while the run is still
  staged (staged_on_disk), so no port needs a milestone 1 recording to point at.
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
The picture, sound, voice and check parts (Task 6, m3parts.py; fighter_parts below): look.json, sound.json,
voice.json, checks.json and exceptions.toml committed; look_right.json, the hull logs, the sprites (ruling M3-R18),
the stem windows and the voice notes cached. The hull logs (ruling M3-R29): every ConvexHull call of each look frame
is in the pinned hull_left.json / hull_right.json, and look.json (format 2) commits only each fight's counts and the
SHA-256 of their exact part (look_docs; FLUTE and TROMBONE, merged before the ruling, keep format 1). episode_refs
writes a pair gate's folder (Template P) and theme_parts a theme's bed.json (Template T). The Rust gates are
studio/crates/bf-harness; the manual studio/docs/porting-a-fighter.md."""
import contextlib
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
        self.applied = False                   # finish() has started writing the committed state


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
        st.applied = True                     # from here the folder's committed state is the new one (staged_on_disk)
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
SPEC_KEYS = ('name', 'opponent', 'fights', 'scenarios', 'frames', 'roles', 'voice_notes', 'hooks', 'sprite_size',
             'left_in_cache')
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
    size = spec.get('sprite_size')
    if size is not None:                          # (absent, not None, when not given: fight.json's spec is unchanged)
        ok = isinstance(size, dict) and set(size) <= {'add', 'clip', 'pad'} and isinstance(size.get('add'), int)
        clip = size.get('clip') if ok else None
        pad = size.get('pad') if ok else None
        ok = ok and (clip is None or (len(clip) == 2 and all(isinstance(v, int) for v in clip) and clip[0] <= clip[1]))
        ok = ok and (pad is None or (isinstance(pad, float) and pad > 0))
        if not ok:
            raise SystemExit("SPEC.sprite_size: dict(add=<int>, clip=(<lo>, <hi>) or None, pad=<float> or None): the "
                             "size of the look's own sprites is 2 * int(round(clip(radius, lo, hi) * pad)) + add "
                             "(sprites.own_size)")
        out['sprite_size'] = dict(add=size['add'], clip=None if clip is None else tuple(clip))
        if pad is not None:                       # (left out when not given)
            out['sprite_size']['pad'] = pad
    if spec.get('left_in_cache') is not None:     # (ruling M3-R36; left out when not given)
        if not isinstance(spec['left_in_cache'], bool):
            raise SystemExit('SPEC.left_in_cache: True or False')
        if spec['left_in_cache']:
            out['left_in_cache'] = True
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
        import shutil
        shutil.rmtree(os.path.join(folder, 'check'), ignore_errors=True)   # its caches belong to the old config
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

    def part(self, name):
        """the same setup in a folder of its own for one part (m3parts): the kit keeps file caches in its episode
        folder (check/sprites_title.npz, the run, the stems), so a part drawn with one real look must never leave
        them to a part drawn with both, whichever ran first"""
        return Config(f'{self.tag}_{name}', self.base, self.fighters, self.handicap, self.dmg_scale, self.rules,
                      self.seed)

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
    reference fights are: value for value, and the kit's replay-guard hash again from the file alone, over the run's
    values (fights.replay_hash_of_values: a numpy float64 the kit leaves in an event, ORGAN's rope length, hashes as
    the Python float of the same bits it is in the file). Returns the recording read back"""
    from . import fights, recording
    _, story = cfg.load()
    rules = story.rules(**over)
    r = story.run_seed(seed, **over)
    recording.write_bfrec(r, rules, path, rect_keys=recording.rect_keys(story), register=False)
    rec = recording.read_bfrec(path)
    fights.check(r, rules, rec, recording.rect_keys(story))
    assert fights.replay_hash(fights.as_kit_run(rec)) == fights.replay_hash_of_values(r), f'{seed}: the replay guard'
    return rec


def rec_note(rec, **more):
    from . import recording
    return dict(seed=rec['seed'], names=list(rec['names']), winner=rec['winner'], t_ko=rec['t_ko'],
                events=len(rec['events']), frames=len(rec['arrays']['HP']), hash=recording.rec_hash(rec), **more)


def existing_copy(sha, folder, also=()):
    """the repo-relative path of a committed reference recording with these bytes, or None. Only the top-level
    manifest (milestones 0 to 2), the folder's own shard and the folders named in `also` (a pair gate's theme
    folder, which holds the kit recording of its config) are searched, never another port's folder, so one port's
    files never depend on another's"""
    for shard in (manifest.top(), ref_dir(folder, manifest.NAME), *(ref_dir(a, manifest.NAME) for a in also)):
        for e in manifest.load(shard):
            if e['sha256'] == sha and e['path'].endswith('.bfrec'):
                return e['path']
    return None


def place_recording(folder, name, data, in_cache, export, module, also=(), cached_from=None):
    """where a theme's or pair's kit recording (`data`, to be the folder's `name`) lives, as its reference file names
    it: a committed copy elsewhere (existing_copy: milestone 1's, or the `also` folders') as its path; else a pinned
    cache copy of `cached_from` (a pair's theme folder) as dict(cache=, sha256=); else, with `in_cache` (ruling
    M3-R43), the folder's own pinned cache file as dict(cache=, sha256=); else the committed file `<folder>/<name>`.
    Every branch but the last removes a committed copy an earlier run made"""
    sha = hashlib.sha256(data).hexdigest()
    copy = existing_copy(sha, folder, also=also)
    cached = cached_copy(sha, cached_from) if cached_from else None
    if copy and not copy.startswith(f'{REF}/{folder}/'):
        delete_committed(folder, name)
        return copy[len(REF) + 1:]
    if cached or in_cache:
        delete_committed(folder, name)
        if cached:
            return cached
        path = cache_bytes(folder, name, data, export)
        return dict(cache=cache_rel(path), sha256=manifest.sha256(path))
    commit_bytes(folder, name, data, module)
    return f'{folder}/{name}'


def cached_copy(sha, folder):
    """dict(cache=, sha256=) of a recording the folder (a pair gate's theme folder) pins in the cache with these
    bytes (ruling M3-R43), or None"""
    for rel, p in sorted(read_pins(folder).items()):
        if p['sha256'] == sha and rel.endswith('.bfrec'):
            return dict(cache=rel, sha256=sha)
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


# The checks part runs the kit's music.main() on the `left` fight, which prints the level of the span from 44 s to
# the K.O. (kit/music.py, `spans`) and raises when that span holds no sample (reelkit/audio.py section_levels: the
# peak of an empty slice). So the left fight's K.O. must come after CHECKS_FROM s, by at least one sample.
CHECKS_FROM = 44.0


def checks_can_run(t_ko):
    """the kit's checks can run on a fight with this K.O. time: the span CHECKS_FROM s to the K.O. holds a sample at
    48 kHz (int(t_ko * SR) > int(CHECKS_FROM * SR), as section_levels slices it)"""
    from . import m3parts as MP
    return t_ko is not None and int(t_ko * MP.SR) > int(CHECKS_FROM * MP.SR)


def checks_too_early(export, seed, t_ko):
    return (f'{export}: fights.left = {seed}: its K.O. is at {t_ko} s, before {CHECKS_FROM:g} s. The checks part '
            f'runs the kit\'s music.main(), which prints the level from {CHECKS_FROM:g} s to the K.O. and stops on a '
            f'fight that ends before it; name a left seed whose K.O. is after {CHECKS_FROM:g} s (fight.json\'s '
            f'stats rows: winner 0, all three abilities, t_ko > {CHECKS_FROM:g})')


def choose_left(export, name, abilities, got, fixed):
    """(the left seed, the lower seeds passed over for an early K.O.): `fixed` (spec fights.left) when given, else
    the lowest statistics seed with the fighter left and winning, all three abilities fired and a K.O. late enough
    for the checks (checks_can_run). Stops, saying which condition no seed meets, when there is none"""
    if fixed is not None:
        return fixed, []
    want = {f'ability/{a}' for a in abilities}
    early = []
    for r, items in got:
        if r['winner'] == 0 and want <= set(items):
            if checks_can_run(r['t_ko']):
                return r['seed'], early
            early.append(r['seed'])
    if early:
        raise SystemExit(f'{export}: no statistics fight has {name} winning from the left with all three abilities '
                         f'({", ".join(abilities)}) and a K.O. after {CHECKS_FROM:g} s: {len(early)} fights have the '
                         f'rest but end too early for the kit\'s music.main(), which the checks part runs (it prints '
                         f'the level from {CHECKS_FROM:g} s to the K.O.). Set fights.left to a seed whose fight ends '
                         f'after {CHECKS_FROM:g} s (a higher fights.stats, or another handicap, finds more)')
    raise SystemExit(f'{export}: no statistics fight has {name} winning from the left with all three abilities '
                     f'({", ".join(abilities)}); set fights.left')


def early_note(early):
    """fight.json's `left.why` addition when choose_left passed seeds over (none for FLUTE and TROMBONE)"""
    if not early:
        return ''
    return (f', and a K.O. after {CHECKS_FROM:g} s, which the kit\'s checks need (passed over for an earlier K.O.: '
            f'seed{"s" if len(early) > 1 else ""} {", ".join(str(s) for s in early)})')


# Ruling M3-R36: a fighter whose own left recording would take its folder over the budget declares
# SPEC.left_in_cache = True; the recording is then the pinned cache file .cache/m3/<folder>/left.bfrec (its SHA-256
# committed in pins.json and fight.json), and every gate reads it from the cache (bf_harness::refs::recording_of),
# failing with the cache command when it is missing. `left_file` (below studio/reference/) is then
# LEFT_IN_CACHE + its cache path, which is where the file is, so a part joins it to REF as for a committed file.
LEFT_IN_CACHE = '.cache/m3/'


def left_ref(left_file):
    """what fight.json and look.json say of the left recording: dict(file=<below studio/reference/>) for a committed
    one, dict(cache=<below .cache/m3/>, sha256=) for one in the cache (ruling M3-R36)"""
    if left_file.startswith(LEFT_IN_CACHE):
        rel = left_file[len(LEFT_IN_CACHE):]
        return dict(cache=rel, sha256=manifest.sha256(os.path.join(cache_root(), *rel.split('/'))))
    return dict(file=left_file)


def left_path_of(entry):
    """the absolute path of the recording a fight.json / look.json entry names (`file` or `cache`)"""
    if 'cache' in entry:
        return os.path.join(cache_root(), *entry['cache'].split('/'))
    return os.path.join(common.REFERENCE, *entry['file'].split('/'))


@contextlib.contextmanager
def staged_on_disk(folder, path):
    """a committed file this run has staged (commit_path) on disk while the block runs, for the run's own child
    processes, which read it by its committed path before finish() applies the stage (the parts read left.bfrec).
    Nothing happens when the run staged nothing there (a pointer at another committed file, or a cache run, which
    compared the file already). If the block stops, the file is put back as it was (its old bytes, or no file), so a
    run that stops still leaves the committed state as it found it. Once finish() has applied the stage, the file is
    part of the folder's new committed state and is left as it is whatever happens after (check_pins)"""
    run = run_state()
    key = os.path.normcase(os.path.abspath(path))
    staged, st = None, None
    if run.mode != 'cache':
        st = run.stage(folder)
        staged = next((v for p, v in st.commits.items() if os.path.normcase(p) == key), None)
    if staged is None:
        yield path
        return
    old = open(path, 'rb').read() if os.path.isfile(path) else None
    if old != staged[0]:
        common.write_bytes(path, staged[0])
    try:
        yield path
    except BaseException:
        if st.applied:                            # finish() wrote the new committed state: this file belongs to it
            raise
        if old is None:
            if os.path.exists(path):
                os.remove(path)
        elif old != staged[0]:
            common.write_bytes(path, old)
        raise


def fighter_refs(spec, extra=None):
    """write fighters/<slug>/ (committed) and its cache files for a fighter_<slug>.py SPEC (module docstring);
    `extra`: a port's own additional part, called after the template's parts in the same run"""
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
    left_seed, early = choose_left(export, name, abilities, got, f['left'])
    tmp = cache_dir(folder, 'left.bfrec.tmp')
    rec_left = record(main, left_seed, {}, tmp)
    data = open(tmp, 'rb').read()
    os.remove(tmp)
    assert rec_left['winner'] == 0, f'{export}: left seed {left_seed}: {name} does not win'
    if not checks_can_run(rec_left['t_ko']):
        raise SystemExit(checks_too_early(export, left_seed, rec_left['t_ko']))
    copy = existing_copy(hashlib.sha256(data).hexdigest(), folder)
    if spec.get('left_in_cache'):                 # ruling M3-R36: too big for the folder; pinned, its SHA committed
        left_file = cache_bytes(folder, 'left.bfrec', data, export)
        left_file = LEFT_IN_CACHE + cache_rel(left_file)
        delete_committed(folder, 'left.bfrec')                  # a copy an earlier run wrote, if any
        print(f'{export}: left seed {left_seed}: {len(data):,} bytes, in the pinned cache (SPEC.left_in_cache)')
    elif copy and not copy.startswith(f'{REF}/{folder}/'):
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
        left=rec_note(rec_left, **left_ref(left_file),
                      why=f'the lowest statistics seed with {name} in the left corner winning and all three of its '
                          f'abilities firing' + early_note(early) if f['left'] is None else 'fixed by the spec'),
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
    # the parts' child processes read left.bfrec by its committed path, so a copy this run staged is on disk while
    # they run (and put back as it was if the run stops)
    with staged_on_disk(folder, os.path.join(common.REPO, *f'{REF}/{left_file}'.split('/'))):
        fighter_parts(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath)
        if extra is not None:
            extra(spec, folder, export, module, main, left_seed, left_file)
        finish(folder)
    print(f"{export}: {folder}: left seed {left_seed} ({len(rec_left['events'])} events, "
          f"K.O. {rec_left['t_ko']}), right seed {right_seed}, {len(recs)} scenario fights, {len(rows_cov)} coverage "
          f"rows ({len(required)} required), {len(read_pins(folder))} pins, {len(same_as)} cache recordings equal to "
          f"committed ones, {time.time() - t_all:.1f} s")
    for k, v in sorted(same_as.items()):
        print(f'{export}:   {CACHE}/{k} = {v}')


# ---------------------------------------------------------------- the picture, sound, voice and check parts (Task 6)
EXCEPTIONS = 'exceptions.toml'
EXCEPTIONS_TEXT = """\
# What this folder's gates may miss (plan section 2, "The bars, never loosened"). Empty unless the controller has
# ruled on a difference that survived real investigation; the proving harness FAILS on any entry without a ruling.
# One table per difference, replacing the empty list below:
#
#   [[exceptions]]
#   gate = "look"              # look, title, sprites, sound, stems, voice, checks, frames
#   what = "frame 1234"        # the frame, the bus and window, the note or the check
#   ruling = "M3-R<n>"         # the controller's ruling, as studio/reference/rulings.toml lists it
#   reason = "why it cannot be made identical, with the stage and the numbers"
#
# Written once by the folder's export and never overwritten; edited only by hand after a ruling.
exceptions = []
"""


OLD_TEMPLATES = (EXCEPTIONS_TEXT.replace('"M3-R<n>"         # the controller\'s ruling, as studio/reference/'
                                         'rulings.toml lists it',
                                         '"M3-R99"          # the controller\'s ruling that allows it'),)


def keep_or_write(folder, name, text, tool):
    """a committed file written once and then edited only by hand: its bytes on disk when it exists (an untouched
    copy of an older template is replaced by the current one), else `text`"""
    path = ref_dir(folder, name)
    data = open(path, 'rb').read() if os.path.isfile(path) else text.encode('utf-8')
    if data.decode('utf-8') in OLD_TEMPLATES:
        data = text.encode('utf-8')
    return commit_bytes(folder, name, data, tool)


def _rms_of(summary, buses):
    return {b: summary['rms'][b] for b in buses if b in summary['rms']}


# Ruling M3-R29: a look's per-frame hull rows go to the pinned cache (hull_left.json / hull_right.json, every call's
# points and vertices) and look.json commits their counts and one SHA-256 (format 2), as the sprites do under M3-R18.
# The two folders merged before the ruling keep their committed look.json as it is (format 1: the compact hull
# calls in every frame row), because a merged folder's files are never re-made by a worker (M3-R1); every other
# folder is written in format 2. No folder joins this list.
INLINE_HULL_ROWS = ('fighters/flute', 'fighters/trombone')
LOOK_COLUMNS = ['frame', 'sha256 of the frame', 'hook calls', 'sha256 of the hook calls', 'boxes drawn',
                'sha256 of the boxes', 'hull calls: [points, vertices, sum of x, sum of y]', 'why']


def look_format(folder):
    """look.json's format for a folder: 1 (hull calls in the frame rows) for INLINE_HULL_ROWS, else 2 (M3-R29)"""
    return 1 if folder in INLINE_HULL_ROWS else 2


def look_docs(folder, name, opp, module, left_file, look_l, look_r):
    """(look.json, look_right.json) of a fighter folder from the two look_child results. Format 2 (ruling M3-R29):
    the frame rows lose their hull column and each fight gets `hull` (m3parts.hull_summary: the counts and the
    SHA-256 of the exact part of its hull log, the pinned file that holds every call). Format 1 is the layout of the
    folders in INLINE_HULL_ROWS, unchanged byte for byte"""
    from . import m3parts as MP
    from . import look_flute as LF
    fmt = look_format(folder)
    left, right = dict(look_l['doc'], **left_ref(left_file)), dict(look_r['doc'])
    columns = list(LOOK_COLUMNS)
    if fmt == 2:
        left['hull'] = MP.hull_summary(left['frames'], f'{folder}/hull_left.json')
        right['hull'] = MP.hull_summary(right['frames'], f'{folder}/hull_right.json')
        left['frames'], right['frames'] = MP.without_hull(left['frames']), MP.without_hull(right['frames'])
        del columns[MP.HULL_COLUMN]
    look_right = dict(
        about=f"{name}'s look in the `right` fight ({opp} left and winning, so {name}'s shards fly), drawn with its "
              f"real look and kit/fighters/base.py's Look for {opp}: the rows of look.json's `left`",
        **right)
    look = dict(
        about=f"{name}'s look against the Python kit: the `left` fight drawn with its real look and "
              f"kit/fighters/base.py's Look for {opp} (every hook of both spied, as look_flute does); the VS card "
              f"alone on milestone 1's noise image with {name} on either side; its title hooks; its sprites",
        tool=module, format=fmt, fighter=name, opponent=opp, other_look='kit/fighters/base.py Look',
        cfg_keys=look_l['cfg_keys'],
        cfg_keys_note="the roster keys the kit's Look class reads from self.cfg (m3parts.cfg_keys): the look takes "
                      "them from setup_as's cfg and names exactly these in cfg_summary (EUPHONIUM's copies draw it "
                      "with another fighter's numbers)",
        hook_names=list(LF.HOOKS), hook_note="a hook call is '<side>.<hook>'; state adds '(ball,phase)', ball, "
                                             "ball_sprite and hides_hp add '(ball)'; hooks_build leaves out rope",
        frame_columns=columns,
        left=left,
        right=dict(cache=f'{folder}/look_right.json', hull_cache=f'{folder}/hull_right.json',
                   frames=len(look_r['doc']['frames']), seed=look_r['doc']['seed'],
                   recording=look_r['doc']['recording']),
        cards=dict(left=look_l['doc']['cards'], right=look_r['doc']['cards'],
                   call='hashlib.sha256(title.draw(noise, frame / 30).tobytes()), noise = '
                        'studio/reference/frames/flute_vs_trombone/title/noise.png; left: the fighter on side 0, '
                        'right: on side 1'),
        hull_cache=f'{folder}/hull_left.json', title=look_l['title'],
        sprites=dict(look_l['sprites'], cache=f'{folder}/sprites.npz'))
    if fmt == 2:
        look['hull_note'] = ("ruling M3-R29: each fight's hull calls are in the pinned cache (left: hull_cache, "
                             "right: hull_cache under `right`; every call's points and vertices); `left.hull` and "
                             "look_right.json's `hull` hold their counts and the SHA-256 of their exact part (`line`)")
    return look, look_right


def fighter_parts(spec, folder, export, module, main, swapped, left_seed, right_seed, left_file, rpath):
    """look.json, sound.json, voice.json, checks.json and exceptions.toml of fighters/<slug>/, and their cache files
    (look_right.json, hull_left.json, hull_right.json, sprites.npz / .json, stems_*.npz, voice_notes.npz). Each
    part runs in a fresh process (m3parts); at most three run at once, the checks (which start their own pool of
    three) alone"""
    from . import m3parts as MP
    from . import look_flute as LF
    t0 = time.time()
    name, opp = spec['name'], spec['opponent']
    fr, vn = spec['frames'], spec['voice_notes']
    lcfg, rcfg = main.with_seed(left_seed), swapped.with_seed(right_seed)
    left_path = os.path.join(common.REPO, *f'{REF}/{left_file}'.split('/'))

    def cd(n):
        return cache_dir(folder, n)
    os.makedirs(cache_dir(folder), exist_ok=True)
    h = [MP.start(MP.look_child, lcfg.part('look'), name, 'left', left_path, fr['left'], fr['times'],
                  (cd('sprites.npz'), cd('sprites.json')), cd('hull_left.json'), spec.get('sprite_size')),
         MP.start(MP.look_child, rcfg.part('look'), name, 'right', rpath, fr['right'], fr['times'], None,
                  cd('hull_right.json')),
         MP.start(MP.sound_child, lcfg.part('sound'), name, None, cd('stems_left.npz'), left_path)]
    look_l, look_r, sound_l = [MP.wait(x) for x in h]
    roles = list(spec['roles'])
    h = [MP.start(MP.sound_child, rcfg.part('sound'), name, None, cd('stems_right.npz'), rpath),
         MP.start(MP.voice_child, lcfg.part('voice'), name, vn['pitches'], tuple(vn['velocities']),
                  tuple(vn['durations']), cd('voice_notes.npz'))]
    h += [MP.start(MP.sound_child, lcfg.part(f'role_{r}'), name, r, cd(f'stems_role_{r}.npz'), left_path)
          for r in roles[:1]]
    sound_r, voice = MP.wait(h[0]), MP.wait(h[1])
    role_docs = {roles[0]: MP.wait(h[2])} if roles else {}
    for r in roles[1:]:
        role_docs[r] = MP.fresh(MP.sound_child, lcfg.part(f'role_{r}'), name, r, cd(f'stems_role_{r}.npz'),
                                left_path)
    checks, checks_rows = MP.split_checks(MP.fresh(MP.checks_child, lcfg.part('checks'), left_path))
    print(f'{export}: the picture, sound, voice and check parts in {time.time() - t0:.1f} s')
    # ---- the look
    for which, got in (('left', look_l), ('right', look_r)):
        need = fr[which]
        assert len(got['doc']['frames']) >= need, f"{export}: {which}: {len(got['doc']['frames'])} frames, {need} asked"
    assert look_l['doc']['k'] == 0 and look_r['doc']['k'] == 1
    for p in (cd('hull_left.json'), cd('hull_right.json'), cd('sprites.npz'), cd('sprites.json')):
        pin(p, export)
    look, look_right = look_docs(folder, name, opp, module, left_file, look_l, look_r)
    cache_json(folder, 'look_right.json', look_right, export)
    if look_l['model']:
        commit_path(folder, look_l['model']['path'], look_l['model']['data'], 'tools.studio_export.models')
    commit_json(folder, 'look.json', look, module)
    # ---- the sound
    for p in [cd('stems_left.npz'), cd('stems_right.npz')] + [cd(f'stems_role_{r}.npz') for r in roles]:
        pin(p, export)
    k_bus = name.lower()
    commit_json(folder, 'sound.json', dict(
        about=f"{name}'s part against the kit: the soundtrack of the `left` and `right` fights with its real part "
              f"and kit/fighters/base.py's Sound for {opp} (silent), over the base config's theme",
        tool=module, format=1, fighter=name, opponent=opp, bed_calls=sound_l['bed_calls'],
        rms_note="every stem's and the master's RMS per 100 ms block, in dB to 3 decimals: 20 log10(sqrt(mean(x "
                 "** 2)) + 1e-12) over both channels",
        left=dict(sound_l, cache=f'{folder}/stems_left.npz'),
        right=dict({k: v for k, v in sound_r.items() if k != 'rms'}, cache=f'{folder}/stems_right.npz',
                   rms_cache=f'{folder}/sound_right_rms.json'),
        roles={r: dict({k: v for k, v in d.items() if k != 'rms'}, rms=_rms_of(d, (k_bus, 'master')),
                       cache=f'{folder}/stems_role_{r}.npz') for r, d in role_docs.items()}), module)
    cache_json(folder, 'sound_right_rms.json', dict(about="the right soundtrack's RMS per 100 ms of every stem and "
                                                        "the master (dB, 3 decimals)", rms=sound_r['rms']), export)
    # ---- the voice
    pin(cd('voice_notes.npz'), export)
    commit_json(folder, 'voice.json', dict(
        about=f"{name}'s instrument voice (pp_bridge.voice) against the kit: each note from the shared noise state "
              f"the one before left (the first from a fresh default_rng(77)); per note its length, its peak (float.hex()) "
              f"and the RMS of every 10 ms block (rounded to 1e-9; the bar is 1e-6), the noise state after it and the SHA-256 of its float64 samples; "
              f"the notes themselves are in the cache",
        tool=module, format=1, block=MP.VBLOCK, velocities=list(vn['velocities']), durations=list(vn['durations']),
        cache=f'{folder}/voice_notes.npz', **voice), module)
    # ---- the checks, and the exceptions list
    commit_json(folder, 'checks.json', dict(
        about=f"what each check of kit/checks.py gives for the `left` fight ({name} against {opp}, both real looks "
              f"and parts): the verdicts, the printed lines and the numbers (tools/studio_export/checks.py's "
              f"functions); the per-frame rows (the safe-zone boxes, the flash areas, the loop steps, the levels "
              f"windows) are in the cache",
        tool=module, format=1, rows_cache=f'{folder}/checks_rows.json', **checks), module)
    cache_json(folder, 'checks_rows.json', dict(about='the per-frame rows of checks.json', **checks_rows), export)
    keep_or_write(folder, EXCEPTIONS, EXCEPTIONS_TEXT, module)
    print(f"{export}: look {len(look_l['doc']['frames'])} + {len(look_r['doc']['frames'])} frames, "
          f"{look_l['sprites']['count']} sprites; sound {sound_l['notes']['count']} + {sound_r['notes']['count']} "
          f"first-time notes; voice {len(voice['notes'])} notes; checks "
          + ', '.join(f"{c} {d.get('pass')}" for c, d in checks['checks'].items()))


# ---------------------------------------------------------------- a pair gate's references (Template P)
EPISODE_SPEC_KEYS = ('config', 'stats', 'links', 'scenarios', 'weapon_hit', 'frames', 'recording_in_cache')


class Loaded:
    """a config folder already written (a kit config's private copy): what the parts and statistics() need"""

    def __init__(self, folder, fighters, seed):
        self.folder, self.fighters, self.seed = folder, tuple(fighters), seed

    def load(self):
        return load_config(self.folder)


def config_module(config):
    import importlib.util
    spec = importlib.util.spec_from_file_location('m3refs_config_values', config_source(config))
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    return cfg


def episode_facts(config):
    """read_config's values and the rest of what a pair's fight file holds"""
    facts = read_config(config)
    c = config_module(config)

    def g(k, d=None):
        return getattr(c, k, d)
    side = g('DMG_SIDE')
    facts.update(seed=g('SEED'), dmg_scale=float(g('DMG_SCALE', 1.0)),
                 handicap=[1.0, 1.0] if side is None else [float(x) for x in side], rules=dict(g('RULES', {})),
                 roles=dict(g('ROLES', {})), title_hits={k: list(v) for k, v in g('TITLE_HITS', {}).items()},
                 # the kit's name is SOLO (kit/episode.py); read_config's music_extras already read it, leaving out
                 # a fighter whose solo holds a chord (a tuple of notes: a fight file's music.solos cannot hold it)
                 solos=facts.get('solos', {}),
                 mix={k: list(v) for k, v in g('MIX', {}).items()}, series=g('SERIES', None),
                 shard_seed=g('SHARD_SEED', 271), fx_seed=g('FX_SEED', None), music_seed=g('MUSIC_SEED', None))
    return facts


def episode_refs(spec):
    """episodes/<pair>/ (committed) + cache for an episode_<pair>.py SPEC: dict(config= the kit config
    (`examples/<x>` or `ballfight_<pair>`), stats=4000, links=() the items of the pair's own events to cover (event
    kinds, "hit/<what>", "ability/<name>"), scenarios=None (default: plain and swapped, seeds 1 to 3), weapon_hit={}
    (fighter -> the `how` of its weapon's hits, for the golden frames), frames=40 (at least)). Writes episode.json
    (the config's values), frames.json (the golden frames, both real looks, and 16 card frames), sound.json (the
    kit's soundtrack: its 16-bit master's SHA-256 = audio.wav's samples, the buses' RMS, the notes), checks.json,
    stats.json (pooled; rows cached), pairs.json (the links' coverage), exceptions.toml; the kit's recording of the
    config is pointed at when a committed file (milestone 1's, or the theme folder's) has its bytes, or a pinned
    cache file of the theme folder (ruling M3-R43: dict(cache=, sha256=)); else copied as run.bfrec, or with
    recording_in_cache=True (M3-R43: it would take the folder over its budget) pinned as the cache's run.bfrec"""
    from . import fights, m3parts as MP, recording, scenarios as SC, stats as ST
    module = sys._getframe(1).f_globals.get('__name__', __name__)
    unknown = sorted(set(spec) - set(EPISODE_SPEC_KEYS))
    if unknown:
        raise SystemExit(f"episode spec: unknown keys {', '.join(unknown)} (it has {', '.join(EPISODE_SPEC_KEYS)})")
    spec = dict(dict(stats=4000, links=(), scenarios=None, weapon_hit={}, frames=40, recording_in_cache=False), **spec)
    if not isinstance(spec['recording_in_cache'], bool):
        raise SystemExit('episode spec: recording_in_cache is True or False')
    config = spec['config']
    pair = config_slug(config)
    folder, export = f'episodes/{pair}', f'episode_{pair}'
    run = run_state()
    run.stages.pop(folder, None)
    t_all = time.time()
    facts = episode_facts(config)
    with open(config_source(config), encoding='utf-8', newline='') as fh:
        text = fh.read()
    cfg_folder = config_folder(f'episode_{pair}', pair, text)
    ep, story = load_config(cfg_folder)
    seed = ep.seed
    # ---- the kit's own recording of the config
    r = story.run_seed(seed)
    rules = story.rules()
    if not checks_can_run(r['t_ko']):
        raise SystemExit(f"{export}: the config's own fight (seed {seed}) ends at {r['t_ko']} s, before "
                         f"{CHECKS_FROM:g} s: the kit's music.main(), which the checks part runs, prints the level "
                         f"from {CHECKS_FROM:g} s to the K.O. and cannot run on it")
    os.makedirs(cache_dir(folder), exist_ok=True)
    tmp = cache_dir(folder, 'run.bfrec.tmp')
    recording.write_bfrec(r, rules, tmp, rect_keys=recording.rect_keys(story), register=False)
    rec = recording.read_bfrec(tmp)
    fights.check(r, rules, rec, recording.rect_keys(story))
    data = open(tmp, 'rb').read()
    theme = f"themes/{facts['mood']}"
    rec_file = place_recording(folder, 'run.bfrec', data, spec['recording_in_cache'], export, module, also=(theme,),
                               cached_from=theme)
    rec_path = tmp                                  # the same bytes; the parts read it before finish() applies them
    cfg = Loaded(cfg_folder, facts['fighters'], seed)
    # ---- the parts in fresh processes
    def part(n):
        return Loaded(config_folder(f'episode_{pair}_{n}', pair, text), facts['fighters'], seed)
    h = [MP.start(MP.frames_child, part('frames'), rec_path, dict(spec['weapon_hit'])),
         MP.start(MP.full_sound_child, part('sound'), cache_dir(folder, 'stems.npz'), rec_path)]
    frames_doc, sound = MP.wait(h[0]), MP.wait(h[1])
    checks, checks_rows = MP.split_checks(MP.fresh(MP.checks_child, part('checks'), rec_path))
    own = ('master',) + tuple(n.lower() for n in facts['fighters'])
    bed_rms = {b: v for b, v in sound['rms'].items() if b not in own}
    sound['rms'] = {b: v for b, v in sound['rms'].items() if b in own}
    cache_json(folder, 'sound_bed_rms.json', dict(about="the bed buses' RMS per 100 ms (dB, 3 decimals)",
                                                  rms=bed_rms), export)
    cache_json(folder, 'checks_rows.json', dict(about='the per-frame rows of checks.json', **checks_rows), export)
    os.remove(tmp)
    pin(cache_dir(folder, 'stems.npz'), export)
    assert len(frames_doc['frames']) >= spec['frames'], f"{export}: {len(frames_doc['frames'])} golden frames"
    assert sound['master_sha256'] and checks['episode_facts']['audio_wav_sha256']
    # ---- statistics at the config's own handicap
    got = statistics(cfg, spec['stats'])
    rows = [g[0] for g in got]
    counts = {}
    for _, items in got:
        for k in items:
            counts[k] = counts.get(k, 0) + 1
    pooled = ST.pooled(rows, tuple(facts['fighters']))
    cache_json(folder, 'stats_rows.json', dict(about=f'{config}: one row per seed 1..{spec["stats"]}',
                                               columns=list(ST.ROW_KEYS), rows=rows), export)
    # ---- the pair's links in scenario fights
    groups = spec['scenarios'] or [dict(name='plain', seeds=[1, 2, 3], swapped=False),
                                   dict(name='swapped', seeds=[1, 2, 3], swapped=True)]
    groups = [dict(dict(overrides={}, swapped=False), **dict(g, seeds=list(g['seeds']))) for g in groups]
    a, b = facts['fighters']
    swapped_folder = config_folder(f'episode_{pair}_swapped', pair, text + (
        '\n# ---------------------------------------------------------------- tools/studio_export m3refs\n'
        f"FIGHTERS = {(b, a)!r}\nDMG_SIDE = {tuple(reversed(facts['handicap']))!r}\n"))
    recs, done, best = [], set(), {}
    missing = list(spec['links'])
    for rnd in range(EXTRA_ROUNDS + 1):
        for g in groups:
            fold = swapped_folder if g['swapped'] else cfg_folder
            for s_ in g['seeds']:
                if (g['name'], s_) in done:
                    continue
                done.add((g['name'], s_))
                rel_name = f"scenarios/{g['name']}_s{s_}.bfrec"
                path = cache_dir(folder, *rel_name.split('/'))
                rec_ = record(Loaded(fold, (b, a) if g['swapped'] else (a, b), None), s_, g['overrides'], path)
                pin(path, export)
                recs.append(((g['name'], s_, f'{folder}/{rel_name}'), rec_))
        best = SC.first_uses(recs, ep.T.T_FIGHT)
        missing = [k for k in spec['links'] if k not in best or best[k]['after_fight'] > WINDOW]
        if not missing or rnd == EXTRA_ROUNDS:
            break
        for g in groups:
            top = max(g['seeds'])
            g['seeds'] += list(range(top + 1, top + 1 + EXTRA_SEEDS))
    if missing:
        raise SystemExit(f"{export}: links not covered within {WINDOW} s of FIGHT!: {', '.join(missing)}")
    scen_doc = []
    for g in groups:
        _, story_ = load_config(swapped_folder if g['swapped'] else cfg_folder)
        scen_doc.append(dict(name=g['name'], swapped=g['swapped'], overrides=dict(g['overrides']),
                             fighters=[b, a] if g['swapped'] else [a, b], seeds=list(g['seeds']),
                             handicap=list(reversed(facts['handicap'])) if g['swapped'] else facts['handicap'],
                             cache=[f"{folder}/scenarios/{g['name']}_s{x}.bfrec" for x in g['seeds']],
                             hashes=[recording.rec_hash(rr) for (n_, x_, _), rr in recs if n_ == g['name']],
                             rules=dict(vars(story_.rules(**g['overrides'])))))
    links = [dict(item=k, scenario=best[k]['scenario'], seed=best[k]['seed'], t=best[k]['t'],
                  after_fight=best[k]['after_fight'], index=best[k]['index'], event=best[k]['event'])
             for k in spec['links']]
    # ---- the committed files
    commit_json(folder, 'episode.json', dict(
        about=f"{config}: the kit config's values, which studio/fights/{pair}.toml must hold, and its recording",
        tool=module, format=1, config=config, pair=pair, recording=rec_file, rec_hash=recording.rec_hash(rec),
        winner=rec['winner'], t_ko=rec['t_ko'], **{k: v for k, v in facts.items() if k != 'config'}), module)
    commit_json(folder, 'frames.json', dict(
        about=f"{config}: the golden frames (milestone 1's rules of frames.py for any pair) with both real looks: "
              f"each frame's SHA-256, its draw log and its hull calls; and the VS card alone on milestone 1's "
              f"noise image at 16 times", tool=module, format=1, **frames_doc), module)
    commit_json(folder, 'sound.json', dict(
        about=f"{config}: the kit's soundtrack (both real parts): the 16-bit master's SHA-256 (audio.wav's "
              f"samples), the buses, every stem's RMS per 100 ms (dB, 3 decimals), the first-time notes",
        tool=module, format=1, cache=f'{folder}/stems.npz', rms_cache=f'{folder}/sound_bed_rms.json', **sound),
        module)
    commit_json(folder, 'checks.json', dict(about=f'{config}: what each check of kit/checks.py gives',
                                            tool=module, format=1, rows_cache=f'{folder}/checks_rows.json',
                                            **checks), module)
    commit_json(folder, 'stats.json', dict(
        about=f"{config}: the kit's statistics at the config's own handicap, seeds 1 to {spec['stats']}",
        tool=module, format=1, seeds=[1, spec['stats']], cache=f'{folder}/stats_rows.json', pooled=pooled,
        definitions=ST.DEFINITIONS, fights_with=dict(sorted(counts.items()))), module)
    commit_json(folder, 'pairs.json', dict(
        about=f"{config}: the pair's own links (plan Table S5): each item's first occurrence over the scenario "
              f"fights (scenario, seed, time, event index, the event), within {WINDOW} s of FIGHT!",
        tool=module, format=1, window=WINDOW, t_fight=ep.T.T_FIGHT, links=links, scenarios=scen_doc), module)
    keep_or_write(folder, EXCEPTIONS, EXCEPTIONS_TEXT, module)
    finish(folder)
    print(f"{export}: {folder}: {len(frames_doc['frames'])} golden frames, {len(links)} links, "
          f"{spec['stats']} statistics fights, {len(read_pins(folder))} pins, {time.time() - t_all:.1f} s")


# ---------------------------------------------------------------- a theme's references (Task 2; Template T adds)
THEME_SPEC_KEYS = ('name', 'voicings', 'configs', 'extra', 'variants', 'key_from', 'options', 'hooks',
                   'recordings_in_cache')
# the 12 roots (the kit's spellings: a sharp or a flat, as its configs write them) and the 13 qualities of QUALITY
ROOTS = ('C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B')


def theme_spec(spec):
    """a theme_<name>.py SPEC with its defaults: name (the mood), voicings (the Bed methods that voice a chord, each
    called for the 12 roots x 13 qualities), configs (the reference kit configs, `examples/<name>` or
    `ballfight_<pair>`: the bed alone under each one's own recording), extra (a function(folder, export, module)
    the theme's own module runs before finish, for files only that theme writes); for bed.json (Task 6, Template
    T): variants (True: a second tempo and a second key), key_from (the config whose key and loop the key variant
    takes; None: the first kit config with a loop of the same length in another key), options (one variant per
    theme option value: dict(name=, text= the config lines it appends, ff= {option: value} for the fight file,
    why=)), hooks (the optional bed hooks to call directly; None: every one the bed class defines);
    recordings_in_cache (ruling M3-R43, R36 extended to themes: True when the configs' kit recordings would take the
    folder over its budget: each one not already committed elsewhere goes to the pinned cache, theme.json's
    `recording` then dict(cache=, sha256=) instead of a committed path)"""
    unknown = sorted(set(spec) - set(THEME_SPEC_KEYS))
    if unknown:
        raise SystemExit(f"theme spec: unknown keys {', '.join(unknown)} (it has {', '.join(THEME_SPEC_KEYS)})")
    out = dict(voicings=(), configs=(), extra=None, variants=True, key_from=None, options=(), hooks=None,
               recordings_in_cache=False)
    out.update(spec)
    if not isinstance(out['recordings_in_cache'], bool):
        raise SystemExit('theme spec: recordings_in_cache is True or False')
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


# the config names a mood reads besides the music (kit/moods/<mood>.py: `getattr(T, '<NAME>', ...)`), each with
# its mood and the fight file's names (`music.theme_options`, bf-content's ThemeOption.key): a dict's keys one by
# one (each a bool), or one fighter of the fight by name. m3refs_checks proves this lists every such name of the 13
# moods; read_config records a config's values as `theme_options` (absent when it sets none: the defaults)
THEME_OPTIONS = {
    'TRAILER': ('brass_trailer', dict(sequel='sequel')),              # self.seq = bool(TRAILER.get('sequel'))
    'DRUMLINE': ('dark_drumline', dict(bass_drums='bass_drums', after_ko='after_ko')),
    'PLUGGED': ('stadium_rock', 'plugged'),                         # M.NAMES.index(PLUGGED), default the left one
}


def theme_options_of(cfg, mood, fighters, config='the config'):
    """the options a kit config module `cfg` sets for its mood, under the fight file's names ({} when it sets
    none). A name another mood reads is ignored, as the kit ignores it; a key or a value the mood cannot take stops
    the export, naming it"""
    out = {}
    for name, (owner, names) in THEME_OPTIONS.items():
        if owner != mood or not hasattr(cfg, name):
            continue
        value = getattr(cfg, name)
        if isinstance(names, str):
            if not isinstance(value, str) or value not in fighters:
                raise SystemExit(f'{config}: {name} = {value!r} is not one of its fighters {fighters}')
            out[names] = value
            continue
        if not isinstance(value, dict):
            raise SystemExit(f'{config}: {name} = {value!r} is not a dict')
        for k, v in value.items():
            if k not in names:
                raise SystemExit(f"{config}: {name} sets {k!r}, which {mood} does not read (it reads "
                                 f"{', '.join(sorted(names))})")
            if not isinstance(v, bool):
                raise SystemExit(f'{config}: {name}[{k!r}] = {v!r} is not True or False')
            out[names[k]] = v
    return out


def read_config(config):
    """the kit config's values (episode.py executed on its own, without the kit), with the bar count the kit's
    timeline derives from its BPM (kit/episode.py build_timeline: loop_bpm, then round(DUR / BAR)) and its LINES
    (a rest as None, in the config's order); `theme_options` (THEME_OPTIONS: the mood's options it sets, under the
    fight file's names) only when it sets one, so a config without options reads as before; and MUSIC_EXTRAS
    (`roles`, `mix`, `solos`, `title_hits`: its ROLES, MIX, SOLO, TITLE_HITS), each only when it sets it: the theme
    gates' fight file plays as the config does (dark_tango's bed plays the melody fighter's line, second_line's
    MIX sets its bed buses' levels)"""
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
    assert getattr(cfg, 'FIGHT_BAR', 2) == 2, f'{config}: FIGHT_BAR {cfg.FIGHT_BAR} (a fight file fights from bar 2)'
    assert len(chords) == n_bars, (config, len(chords), n_bars)
    return dict(config=config, fighters=list(cfg.FIGHTERS), mood=cfg.MOOD, bpm=cfg.BPM, loop_bpm=common.hexf(bpm),
                n_bars=n_bars, key=cfg.KEY, scale=getattr(cfg, 'SCALE', 'minor'), intro=chords[:start], loop=loop,
                tail=chords[start + whole * len(loop):], chords=chords,
                lines={k: [None if n is None else int(n) for n in v] for k, v in getattr(cfg, 'LINES', {}).items()},
                **_options_key(theme_options_of(cfg, cfg.MOOD, list(cfg.FIGHTERS), config)), **music_extras(cfg))


# what else a kit config sets that the soundtrack reads (kit/episode.py's defaults; kit/music.py, the moods and the
# fighters' parts read them from T) and a fight file holds, under the fight file's names (music.roles, .mix, .solos,
# .title_hits); read_config records each only when the config sets it, so a config without one reads as before.
# The rest of what the soundtrack reads from a config is the music read_config already records (BPM, KEY, SCALE,
# CHORDS, LOOP, LINES), the theme options (THEME_OPTIONS), or fixed: LOOP_START (asserted 2) and FIGHT_BAR (asserted
# 2, only a pre-card check config sets it)
MUSIC_EXTRAS = ('roles', 'mix', 'solos', 'title_hits')


def music_extras(cfg):
    """{roles, mix, solos, title_hits} of a kit config module, each only when it sets it (MUSIC_EXTRAS)"""
    out = {}
    if getattr(cfg, 'ROLES', None):
        out['roles'] = {k: str(v) for k, v in cfg.ROLES.items()}
    if getattr(cfg, 'MIX', None):                        # bus: (gain, reverb send)
        out['mix'] = {k: [float(g), float(r)] for k, (g, r) in cfg.MIX.items()}
    # fighter: [(beat, midi, beats)], a chord's midi a tuple of notes (ACOUSTIC's strums), written as a fight file's
    # music.solos holds it: [beat, [note, ...], beats]
    solos = {k: [solo_entry(e) for e in v] for k, v in getattr(cfg, 'SOLO', {}).items()}
    if solos:
        out['solos'] = solos
    if getattr(cfg, 'TITLE_HITS', None):                 # fighter: notes as its name slams in
        out['title_hits'] = {k: [int(n) for n in v] for k, v in cfg.TITLE_HITS.items()}
    return out


def solo_entry(e):
    """one SOLO entry (beat, midi, beats) as theme.json and a fight file's music.solos hold it: [beat, note, beats], or
    [beat, [note, ...], beats] for a chord (a tuple or list of notes, which the kit's ACOUSTIC strums)"""
    b, n, d = e
    note = [int(x) for x in n] if isinstance(n, (tuple, list)) else int(n)
    return [float(b), note, float(d)]


def _options_key(options):
    return dict(theme_options=options) if options else {}


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


def kit_configs():
    """every kit config with an episode.py, as config names (`examples/<x>`, `ballfight_<pair>`), sorted"""
    out = []
    ex = os.path.join(common.REPO, 'projects', 'ballfight', 'examples')
    out += [f'examples/{n}' for n in sorted(os.listdir(ex)) if os.path.isfile(os.path.join(ex, n, 'episode.py'))]
    pr = os.path.join(common.REPO, 'projects')
    out += [n for n in sorted(os.listdir(pr)) if n.startswith('ballfight_')
            and os.path.isfile(os.path.join(pr, n, 'episode.py'))]
    return out


def chords_at(intro, loop, tail, n_bars):
    """bf_core::harmony::chords_for: the intro, the loop repeated (the last repeat may stop part-way), the tail on
    the last bars"""
    body = max(0, n_bars - len(intro) - len(tail))
    out = list(intro) + (list(loop) * (body // max(1, len(loop)) + 1))[:body] + list(tail)
    return out[:n_bars]


def key_variant(facts, key_from):
    """(the config whose key, scale and loop the key variant takes, the text it appends): `key_from`, or the first
    kit config (sorted) with a loop of the same length in another key"""
    names = [key_from] if key_from else kit_configs()
    for c in names:
        if c == facts['config']:
            continue
        try:
            o = read_config(c)
        except (AssertionError, AttributeError, ValueError):
            continue
        if len(o['loop']) == len(facts['loop']) and (o['key'], o['scale']) != (facts['key'], facts['scale']):
            chords = chords_at(o['intro'], o['loop'], [], facts['n_bars'])
            # LOOP too (ruling M3-R45): a config that sets its own LOOP would keep it over the new chords, while the
            # harness gives the fight file the other config's loop (`chord_loop` below)
            text = f"KEY = {o['key']!r}\nSCALE = {o['scale']!r}\nLOOP = {list(o['loop'])!r}\nCHORDS = {chords!r}\n"
            return c, text, dict(key=o['key'], scale=o['scale'], chords_intro=list(o['intro']),
                                 chord_loop=list(o['loop']), chords_tail=[])
    raise SystemExit(f"theme variants: no kit config has a {len(facts['loop'])}-chord loop in another key; set key_from")


def theme_parts(spec, folder, export, module, configs):
    """bed.json (committed): for each reference config the bed alone's RMS per bus and its notes; the variants (a
    second tempo, a second key, each option of spec.options): the recording (cache), the master's SHA-256, the buses,
    the RMS; the optional hooks the bed class defines, each called directly at three times with three gains (the
    buses' SHA-256 committed, the vectors cached). Each runs in a fresh process"""
    from . import fights, m3parts as MP, recording
    from reelkit.config import loop_bpm
    out_cfg, variants, hooks = [], [], None
    for c in configs:
        facts = read_config(c['config'])
        cfg_folder = theme_config_folder(spec['name'], c['config'])
        stems = cache_dir(folder, f"bed_{config_slug(c['config'])}.npz")
        got = MP.fresh(MP.bed_child, cfg_folder, stems)
        pin(stems, export)
        assert got['master_sha256'] == c['bed_alone']['master_sha256'], 'the bed alone differs between two runs'
        hooks = got['hooks'] if hooks is None else hooks
        out_cfg.append(dict(config=c['config'], stems_cache=f"{folder}/bed_{config_slug(c['config'])}.npz",
                            **{k: got[k] for k in ('master_sha256', 'buses', 'stems', 'rms', 'notes', 'noise', 'windows')}))
        if not spec['variants'] or variants:
            continue
        with open(config_source(c['config']), encoding='utf-8', newline='') as fh:
            text = fh.read()
        bpm2 = facts['bpm'] + 14
        kc, ktext, kff = key_variant(facts, spec['key_from'])
        n2 = int(round(61.0 / (4 * (60.0 / loop_bpm(bpm2, 61.0)))))
        tempo_chords = chords_at(facts['intro'], facts['loop'], facts['tail'], n2)
        todo = [('tempo', f'BPM = {bpm2!r}\nCHORDS = {tempo_chords!r}\n', dict(tempo_bpm=float(bpm2)),
                 f"the config's BPM plus 14 ({bpm2}; the grid snaps it to {loop_bpm(bpm2, 61.0)!r})"),
                ('key', ktext, kff, f"the key, scale and loop of {kc}")]
        todo += [(o['name'], o['text'], dict(theme_options=o['ff']), o.get('why', o['name'])) for o in spec['options']]
        for vname, vtext, ff, why in todo:
            vfold = config_folder(f"theme_{spec['name']}_{vname}", config_slug(c['config']), text + (
                '\n# ---------------------------------------------------------------- tools/studio_export m3refs\n'
                + vtext))
            ep, story = load_config(vfold)
            r = story.run_seed(ep.seed)
            rules = story.rules()
            rpath = cache_dir(folder, 'variants', f'{vname}.bfrec')
            recording.write_bfrec(r, rules, rpath, rect_keys=recording.rect_keys(story), register=False)
            rec = recording.read_bfrec(rpath)
            fights.check(r, rules, rec, recording.rect_keys(story))
            pin(rpath, export)
            vstems = cache_dir(folder, 'variants', f'{vname}.npz')
            vg = MP.fresh(MP.bed_child, vfold, vstems)
            pin(vstems, export)
            variants.append(dict(name=vname, config=c['config'], why=why, appended=vtext, ff=ff,
                                 recording=f'{folder}/variants/{vname}.bfrec', rec_hash=recording.rec_hash(rec),
                                 stems_cache=f'{folder}/variants/{vname}.npz', bpm=vg['bpm'], key=vg['key'],
                                 master_sha256=vg['master_sha256'], buses=vg['buses'], stems=vg['stems'],
                                 notes=vg['notes'], noise=vg['noise'], windows=vg['windows'],
                                 rms=dict(master=vg['rms']['master'])))
    hooks = list(spec['hooks']) if spec['hooks'] is not None else (hooks or [])
    hook_doc = {}
    if hooks and configs:
        hpath = cache_dir(folder, 'hooks.npz')
        hook_doc = MP.fresh(MP.hook_child, theme_config_folder(spec['name'], configs[0]['config']), hooks, hpath)
        pin(hpath, export)
    commit_json(folder, 'bed.json', dict(
        about=f"{spec['name']}'s bed alone (both fighters kit/fighters/base.py's Sound, silent) beyond theme.json: each "
              f"reference config's RMS per bus (100 ms, dB to 3 decimals) and first-time notes; the variants (a "
              f"second tempo, a second key, the theme's options), each with its own recording in the cache; the "
              f"optional hooks the bed class defines, each called directly on an empty mix",
        tool=module, format=1, theme=spec['name'], hooks=hooks, configs=out_cfg, variants=variants,
        hook_calls=hook_doc, hook_cache=f'{folder}/hooks.npz' if hook_doc else None,
        hook_note='a call\'s args as given (floats as float.hex()); `buses` the SHA-256 of each bus\'s float64 samples '
                  'after the call; a hit\'s chord of None is the chord at its time'), module)
    keep_or_write(folder, EXCEPTIONS, EXCEPTIONS_TEXT, module)
    print(f"{export}: bed.json: {len(out_cfg)} configs, {len(variants)} variants, hooks {', '.join(hooks) or 'none'}")


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
        recording.write_bfrec(r, rules, tmp, rect_keys=recording.rect_keys(story), register=False)
        rec = recording.read_bfrec(tmp)
        fights.check(r, rules, rec, recording.rect_keys(story))
        data = open(tmp, 'rb').read()
        os.remove(tmp)
        rec_file = place_recording(folder, f'{config_slug(config)}.bfrec', data, spec['recordings_in_cache'], export,
                                   module)
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
        where = f"{rec_file['cache']}, cached" if isinstance(rec_file, dict) else rec_file
        print(f"{export}: {config}: seed {seed} ({where}), the bed alone: buses {', '.join(got['buses'])}, "
              f"16-bit master {got['master_sha256'][:16]}..., {time.time() - t0:.1f} s")
    if spec['voicings']:
        commit_json(folder, 'voicings.json', dict(
            about=f'kit/moods/{name}.py Bed: each voicing method for every chord the harmony parses (12 roots x 13 '
                  f'qualities: the kit\'s QUALITY order), the chord added to T.CHORD_PCS first as the episode\'s chords '
                  f'are; the notes as MIDI numbers, in the order the method gives them',
            theme=name, roots=list(ROOTS), qualities=list(common.kit()[0].QUALITY), voicings=voicings), module)
    if spec['extra'] is not None:
        spec['extra'](folder, export, module)
    theme_parts(spec, folder, export, module, configs)
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

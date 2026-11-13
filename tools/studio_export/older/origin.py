"""Gate O (plan m4, Task 2): a reference kit module proven against the episode it was moved from.

An older fighter's look and sound, and an older theme's bed, exist only inside their episode folders (render.py,
styles.py, audio.py). A port moves them into a kit module (older/fighters/<slug>.py, older/moods/<name>.py, or the
kit's own module for the self-proof) and Gate O compares that module, run by today's kit, with the episode's own
scripts, run from a scratch copy:

    scratch(episode, patches=(), variant=None)   a copy of projects/<episode>/'s delivered sources (its top-level files:
                                   every *.py but the *_backup*.py snapshots, and seed.txt; never v1/ ... or check/)
                                   under studio/reference/.cache/m4/origin/<episode>/ (a patched variant under
                                   <episode>~<variant>/). The episode runs there and nowhere else: every runner refuses a
                                   working folder outside that cache (and anything under projects/), and the shared
                                   modules it imports (projects/ballfight's engine, roster, ring and title card, the Pixel
                                   Pitch bridge) are read where they are, never copied or written
    episode_frames(episode, frames, layers_off=(), patches=None)   the episode's own render.py: {frame: RGB uint8}
    episode_stems(episode, silence=(), patches=None)   the episode's own audio.py: {bus: float64 (n, 2)}, the buses as
                                   main() places them (before its master: the master is the series' own in the kit)
    fighter_origin(spec, out=None)  Gate O for a fighter: writes origin.json (or `out`) or stops (SystemExit) naming the
                                   frame, the layers, the first differing pixel, or the bus and the first sample
    mood_origin(spec, out=None)     Gate O for a theme's bed

A port's SPEC gains `origin` (origin_spec() fills the defaults):

    origin=dict(
        episode='ballfight_organ_vs_harp',  # the episode folder (Table O1 / O2)
        seed=None,                     # its seed (None: the first number of its seed.txt)
        config={},                     # kit config values over the ones read from the episode's timeline.py (FIGHTERS,
                                       #   SEED, DMG_SCALE, DMG_SIDE, RULES, BPM, SERIES, KEY, SCALE, CHORDS, SOLO,
                                       #   FIGHT_BAR), e.g. FX_SEED (a renderer's own particle seed) or a stand-in MOOD;
                                       #   or a kit config's name ('examples/trombone_vs_horn') to run as it is
        frames=24,                     # at least 24: choose_frames' rule; or an explicit list of frame numbers
        layers=[],                     # the episode's layers of the fighter, each dict(name=, patches=[(file, old, new)]):
                                       #   all of them applied = the episode drawn with the fighter as a bare kit ball
        other=[],                      # patches always applied: the other fighter drawn as the kit's bare ball (and any
                                       #   other part of the episode the kit's base Look does not draw)
        harness=[],                    # the kit run matched to the episode's own series rules inside the window, each
                                       #   dict(name=, beat_pulse_pre=<s>) / dict(name=, camera_sy=<px>) /
                                       #   dict(name=, call='<module>:<function>')  (fn(stage, module), stage 'timeline' or
                                       #   'picture' or 'music')
        buses=[],                      # the fighter's buses (the kit's names = the episode's)
        silence=[],                    # theme: patches removing the fighters' placements from the bed's buses
        window=('fight', 'shatter'),   # FIGHT! to the end of the shatter (the reassembly: both timelines' earlier one)
        bar_db=-120.0,                 # the sound bar (peak difference, dBFS)
        exceptions={},                 # {bus: (bar_db up to -80, reason)}: the controller's rulings only
    )

What Gate O compares (plan m4 Task 2):

- the fight: the kit's run of the config and the replay guard's line of the episode's own sim (tools/
  ballfight_replay_hash.py's child command, run in the scratch copy) are the same line;
- the layers: on every chosen frame the episode is drawn with the fighter's layers on and off (`layers` applied), the
  kit with the module and with base.Look (the other side always base.Look; the episode's other side always bare through
  `other`); the two contributions, on minus off per pixel in int16, must be identical. The module's picture state (the
  keys its state() sets or changes on its ball, against base.Look's) must equal the episode's fighter() values there;
- the sound: each of `buses` from the kit (the module's Sound, the other side base.Sound) against the episode's, sample
  for sample in float64 over the window; the bar is `bar_db` (peak difference), an exception up to -80 dBFS only by a
  ruling, written into origin.json with its reason;
- the bed (mood_origin): the mood's buses with both fighters' parts silent (two base Sounds) against the episode's bed
  buses with the fighters' placements removed (`silence`), over the whole buffer.

origin.json: the episode and its commit, the config, the fight line, every frame compared (its time, the SHA-256 of the
two contributions, which are equal, and of the four frames), every patch and harness entry with the SHA-256 of its text,
the states compared, each bus's peak difference ('bit-identical' or dBFS), the exceptions. The frames and stems go to
the cache (.cache/m4/origin/); only hashes are committed."""
import hashlib
import json
import math
import os
import shutil
import sys

from .. import common

ROOT = os.path.join(common.CACHE, 'm4', 'origin')
PROJECTS = os.path.join(common.REPO, 'projects')
SHARED = (os.path.join(PROJECTS, 'ballfight'), os.path.join(PROJECTS, 'which_instrument_wins'))
FPS = 30
SR = 48000
MIN_FRAMES = 24
BAR_DB = -120.0
EXCEPTION_FLOOR_DB = -80.0
WORKERS = 3                                      # parts at once (plan m3 section 2: six lanes share the PC)
ORIGIN_KEYS = ('episode', 'seed', 'config', 'frames', 'layers', 'other', 'harness', 'buses', 'silence', 'window',
               'bar_db', 'exceptions')
CONFIG_FROM_TIMELINE = ('FIGHTERS', 'DMG_SCALE', 'DMG_SIDE', 'RULES', 'BPM', 'SERIES', 'KEY', 'SCALE', 'CHORDS', 'SOLO',
                        'FIGHT_BAR')
PITCH_NAMES = ('C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B')


def _sha(b):
    return hashlib.sha256(b).hexdigest()


def _text_sha(obj):
    return _sha(json.dumps(obj, sort_keys=True).encode())


# ---------------------------------------------------------------- the spec
def origin_spec(origin):
    """the origin dict with its defaults, checked"""
    if not isinstance(origin, dict):
        raise SystemExit('SPEC.origin: a dict')
    unknown = sorted(set(origin) - set(ORIGIN_KEYS))
    if unknown:
        raise SystemExit(f"SPEC.origin: unknown key(s) {', '.join(unknown)} (known: {', '.join(ORIGIN_KEYS)})")
    o = dict(seed=None, config={}, frames=MIN_FRAMES, layers=[], other=[], harness=[], buses=[], silence=[],
             window=('fight', 'shatter'), bar_db=BAR_DB, exceptions={})
    o.update(origin)
    episode_dir(o['episode'])
    if isinstance(o['frames'], int):
        if o['frames'] < MIN_FRAMES:
            raise SystemExit(f'SPEC.origin.frames: at least {MIN_FRAMES}')
    elif not (isinstance(o['frames'], (list, tuple)) and len(o['frames']) >= MIN_FRAMES
              and all(isinstance(f, int) for f in o['frames'])):
        raise SystemExit(f'SPEC.origin.frames: a count (>= {MIN_FRAMES}) or a list of at least {MIN_FRAMES} frames')
    for key in ('layers',):
        for i, layer in enumerate(o[key]):
            if not (isinstance(layer, dict) and set(layer) == {'name', 'patches'} and layer['name']):
                raise SystemExit(f'SPEC.origin.{key}[{i}]: dict(name=, patches=[(file, old, new)])')
            check_patches(layer['patches'], f'SPEC.origin.{key}[{i}]')
    for key in ('other', 'silence'):
        check_patches(o[key], f'SPEC.origin.{key}')
    for i, h in enumerate(o['harness']):
        if not (isinstance(h, dict) and 'name' in h and len(set(h) - {'name'}) == 1
                and set(h) - {'name'} <= {'beat_pulse_pre', 'camera_sy', 'call'}):
            raise SystemExit(f'SPEC.origin.harness[{i}]: dict(name=, one of beat_pulse_pre= / camera_sy= / call=)')
    if tuple(o['window']) != ('fight', 'shatter'):
        raise SystemExit("SPEC.origin.window: ('fight', 'shatter') (FIGHT! to the end of the shatter)")
    o['window'] = ('fight', 'shatter')
    if float(o['bar_db']) != BAR_DB:
        raise SystemExit(f'SPEC.origin.bar_db: {BAR_DB} (a bus that needs another bar is an exception, by a ruling)')
    for bus, ex in o['exceptions'].items():
        if not (isinstance(ex, (list, tuple)) and len(ex) == 2 and BAR_DB < float(ex[0]) <= EXCEPTION_FLOOR_DB
                and isinstance(ex[1], str) and ex[1]):
            raise SystemExit(f'SPEC.origin.exceptions[{bus!r}]: (bar_db above {BAR_DB} and at most '
                             f'{EXCEPTION_FLOOR_DB}, the ruling and its reason)')
    if not isinstance(o['config'], (dict, str)):
        raise SystemExit("SPEC.origin.config: a dict of config values or a kit config's name")
    return o


def check_patches(patches, where):
    for j, p in enumerate(patches):
        if not (isinstance(p, (list, tuple)) and len(p) == 3 and all(isinstance(x, str) for x in p) and p[1]
                and p[0].endswith('.py') and '/' not in p[0] and '\\' not in p[0]):
            raise SystemExit(f'{where}[{j}]: a patch is (file, old text, new text), the file one of the episode\'s')


# ---------------------------------------------------------------- the scratch copy
def episode_dir(episode):
    """projects/<episode>: a delivered episode folder (never a snapshot like v2/)"""
    if not (isinstance(episode, str) and episode.startswith('ballfight') and '/' not in episode
            and '\\' not in episode and '..' not in episode):
        raise SystemExit(f'origin: {episode!r} is not an episode folder of projects/ (ballfight or ballfight_<a>_vs_<b>)')
    d = os.path.join(PROJECTS, episode)
    if not os.path.isfile(os.path.join(d, 'render.py')):
        raise SystemExit(f'origin: projects/{episode}/render.py does not exist')
    return d


def delivered(episode):
    """the episode's delivered sources: its top-level *.py files but the *_backup*.py snapshots, and seed.txt"""
    d = episode_dir(episode)
    return sorted(f for f in os.listdir(d) if os.path.isfile(os.path.join(d, f))
                  and ((f.endswith('.py') and 'backup' not in f) or f == 'seed.txt'))


def _under(path, root):
    path, root = os.path.normcase(os.path.realpath(path)), os.path.normcase(os.path.realpath(root))
    return path == root or path.startswith(root + os.sep)


def guard(folder):
    """refuse any working folder that is not a scratch copy under the origin cache (and anything under projects/)"""
    if _under(folder, PROJECTS) or not _under(folder, ROOT) or os.path.normcase(os.path.realpath(folder)) == \
            os.path.normcase(os.path.realpath(ROOT)):
        raise SystemExit(f'origin: refuses to run an episode in {folder}: episodes run only from their scratch copy '
                         f'under {common.rel(ROOT)}/ (nothing under projects/ is ever written)')
    return folder


def apply_patches(text, patches, file, where):
    for j, (f, old, new) in enumerate(patches):
        if f != file:
            continue
        n = text.count(old)
        if n != 1:
            raise SystemExit(f'origin: {where} patch {j} ({file}) matches {n} times, not once: {old[:80]!r}')
        text = text.replace(old, new)
    return text


def scratch(episode, patches=(), variant=None):
    """the scratch copy of the episode's delivered sources (with `patches` applied, in a folder of the variant's own);
    files are rewritten only when they change (the episode's check/ caches go with any change). Returns the folder"""
    src = episode_dir(episode)
    if variant is None and patches:
        raise SystemExit('origin.scratch: patches need a variant name (the pristine copy stays as delivered)')
    if variant is not None and not (isinstance(variant, str) and variant.replace('_', '').replace('-', '').isalnum()):
        raise SystemExit(f'origin.scratch: variant {variant!r}: letters, digits, _ and - only')
    folder = os.path.join(ROOT, episode if variant is None else f'{episode}~{variant}')
    guard(folder)
    files = delivered(episode)
    for f in {p[0] for p in patches}:
        if f not in files:
            raise SystemExit(f'origin: a patch names {f}, which is not a delivered file of {episode}')
    want = {}
    for f in files:
        with open(os.path.join(src, f), 'rb') as fh:
            data = fh.read()
        if f.endswith('.py') and any(p[0] == f for p in patches):
            data = apply_patches(data.decode('utf-8'), patches, f, variant).encode('utf-8')
        want[f] = data
    changed = False
    os.makedirs(folder, exist_ok=True)
    for f in os.listdir(folder):
        p = os.path.join(folder, f)
        if os.path.isfile(p) and f not in want:
            os.remove(p)
            changed = True
    for f, data in want.items():
        p = os.path.join(folder, f)
        if not (os.path.exists(p) and open(p, 'rb').read() == data):
            common.write_bytes(p, data)
            changed = True
    if changed:
        shutil.rmtree(os.path.join(folder, 'check'), ignore_errors=True)
        shutil.rmtree(os.path.join(folder, '__pycache__'), ignore_errors=True)
    return folder


def kit_status():
    """`git status --short projects reelkit series` (must stay empty)"""
    return common.git('status', '--short', '--', *common.KIT_DIRS)


def check_kit_clean(where):
    s = kit_status()
    if s:
        raise SystemExit(f'origin: {where} left changes under projects/, reelkit/ or series/:\n{s}')


# ---------------------------------------------------------------- the episode's own scripts (fresh processes)
def _episode_env(folder):
    """in a fresh process: run from the scratch copy, its own modules first, the shared ones where they are"""
    guard(folder)
    os.chdir(folder)
    sys.path.insert(0, folder)
    if common.REPO not in sys.path:
        sys.path.append(common.REPO)
    for d in SHARED:
        if d not in sys.path:
            sys.path.append(d)


def _plain_state(st):
    import numpy as np
    out = {}
    for k, v in st.items():
        if isinstance(v, np.ndarray):
            out[k] = [float(x) for x in v.ravel()]
        elif isinstance(v, (bool, np.bool_)):
            out[k] = bool(v)
        elif isinstance(v, (int, np.integer)):
            out[k] = int(v)
        elif isinstance(v, (float, np.floating)):
            out[k] = float(v)
        elif isinstance(v, (list, tuple)):
            out[k] = repr(v)
        else:
            out[k] = repr(v)
    return out


def episode_facts_child(folder, seed):
    """the replay guard's line of the episode's seed (its own sim) and the timeline values a kit config reads"""
    _episode_env(folder)
    import io
    import contextlib
    import importlib.util
    spec = importlib.util.spec_from_file_location('ballfight_replay_hash',
                                                  os.path.join(common.REPO, 'tools', 'ballfight_replay_hash.py'))
    guard_mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard_mod)
    out = io.StringIO()
    argv = sys.argv
    sys.argv = ['-c', str(seed)]
    try:
        with contextlib.redirect_stdout(out):
            exec(compile(guard_mod.CODE, 'replay_hash', 'exec'), {'__name__': '__main__'})
    finally:
        sys.argv = argv
    import timeline as T
    vals = {}
    for k in ('FIGHTERS', 'DMG_SCALE', 'DMG_SIDE', 'RULES', 'BPM', 'SERIES', 'CHORDS', 'SOLO', 'KEY_ROOT', 'SCALE',
              'T_FIGHT', 'BAR', 'T_REASSEMBLE', 'T_GLIDE', 'DUR'):
        if hasattr(T, k):
            v = getattr(T, k)
            vals[k] = list(v) if isinstance(v, tuple) and k != 'SOLO' else v
    return dict(line=out.getvalue().strip(), timeline=repr(vals))


def episode_frames_child(folder, frames, out_npz, state_sides):
    """the episode's render.py: the frames (to out_npz) and fighter() of the given sides at each"""
    _episode_env(folder)
    import numpy as np
    import render as RD
    arrays, states = {}, {}
    for f in frames:
        img = np.ascontiguousarray(RD.render(f))
        assert img.dtype == np.uint8 and img.ndim == 3, (img.dtype, img.shape)
        arrays[f'f{f:05d}'] = img
        states[f] = {k: _plain_state(RD.fighter(k, f / FPS)) for k in state_sides}
    common.write_npz(out_npz, arrays, register=False)
    return dict(states=states, names=list(RD.NAMES), t_ko=float(RD.T_KO), t_hs=float(RD.T_HS),
                t_fight=float(RD.T.T_FIGHT), t_reassemble=float(RD.T.T_REASSEMBLE))


def episode_stems_child(folder, out_npz, buses):
    """the episode's audio.py main(): every bus as main() placed it, captured where it hands the mix to its master"""
    _episode_env(folder)
    import numpy as np
    import audio as AU
    got = {}
    real = AU.master

    def master(mix, *a, **kw):
        got.update({k: np.array(v, np.float64, copy=True) for k, v in mix.buses.items()})
        return real(mix, *a, **kw)
    AU.master = master
    AU.main()
    if not got:
        raise SystemExit(f'origin: {folder}/audio.py main() never called master(mix)')
    buses = sorted(got) if buses is None else buses
    missing = [b for b in buses if b not in got]
    if missing:
        raise SystemExit(f"origin: the episode has no bus {', '.join(missing)} (it has {', '.join(sorted(got))})")
    common.write_npz(out_npz, {b: got[b] for b in buses}, register=False)
    return dict(buses=sorted(got), n=int(len(next(iter(got.values())))))


# ---------------------------------------------------------------- the kit (fresh processes)
def _kit_load(cfg_folder, harness):
    E, story = common.kit()
    ep = E.load(cfg_folder)
    _harness(harness, 'timeline', ep.T)
    return ep, story


def _harness(harness, stage, mod):
    """apply the harness entries of one stage: the kit run matched to the episode's own series rules"""
    import importlib
    for h in harness:
        if 'beat_pulse_pre' in h and stage == 'timeline':
            T, pre0 = mod, float(h['beat_pulse_pre'])

            def beat_pulse(t, decay=0.12, pre=pre0, T=T):
                ph = t % T.BEAT
                up = min(1.0, max(0.0, (ph - (T.BEAT - pre)) / pre))
                return max(math.exp(-ph / decay), up * up * (3 - 2 * up))
            T.beat_pulse = beat_pulse
        elif 'camera_sy' in h and stage == 'picture':
            sy = int(h['camera_sy'])
            mod.SY = sy
            mod.KEYS = [tuple(k[:6]) + (sy,) for k in mod.KEYS]
        elif 'call' in h:
            m, _, fn = h['call'].partition(':')
            getattr(importlib.import_module(m), fn)(stage, mod)


def _bare(names):
    """kit.fighters.module(name) is None for `names`: the kit's base Look and base Sound for them"""
    from ballfight.kit import fighters as FM
    real = FM.module
    FM.module = lambda name: None if name in names else real(name)
    return FM


def _mutate_look(FM, name, mutation):
    """(self-check) a one-pixel change in the module's layers at one frame: after its after_grade hook"""
    if not mutation or mutation[0] != 'pixel':
        return
    _, frame, x, y = mutation
    cls = FM.module(name).Look
    real = cls.after_grade

    def after_grade(self, img, F, drawn):
        img = real(self, img, F, drawn)
        if F.f == frame:
            img = img.copy()
            v = img[y, x, 0]
            img[y, x, 0] = v - 40 if v >= 40 else v + 40
        return img
    cls.after_grade = after_grade


def _mutate_sound(FM, name, mutation):
    """(self-check) a one-sample change in the module's bus: after its play()"""
    if not mutation or mutation[0] != 'sample':
        return
    _, bus, i, delta = mutation
    cls = FM.module(name).Sound
    real = cls.play

    def play(self, mix):
        real(self, mix)
        mix.bus(bus)[i, 0] += delta
    cls.play = play


def kit_facts_child(cfg_folder, harness, k):
    """the kit's replay-guard line of the config's seed and the run's moments the frames are chosen from"""
    ep, story = _kit_load(cfg_folder, harness)
    T = ep.T
    line = story.replay_hash(ep.seed)
    r = story.run_seed(ep.seed)
    from fighters import FIGHTERS as ROSTER
    name = T.FIGHTERS[k]
    abil = {a['name']: slot for slot, a in ROSTER[name]['abilities'].items()}
    first_use, first_hit, low = {}, None, None
    hp = [100.0, 100.0]
    for e in r['events']:
        if e[1] == 'ability' and e[2] == k and e[3] in abil:
            first_use.setdefault(e[3], float(e[0]))
        elif e[1] == 'hit':
            hp[e[3]] -= e[4]
            if e[2] == k and first_hit is None:
                first_hit = float(e[0])
            if e[3] == k and hp[k] < 25.0 and low is None:
                low = float(e[0])
    return dict(line=f'{ep.seed} ' + line.split(' ', 1)[1] if line.startswith(f'{ep.seed} ') else line,
                raw_line=line, winner=r['winner'], t_ko=float(r['t_ko']), first_use=first_use, first_hit=first_hit,
                low=low, t_fight=float(T.T_FIGHT), t_reassemble=float(T.T_REASSEMBLE), bar=float(T.BAR),
                hitstop=0.35, names=list(T.FIGHTERS))


def kit_frames_child(cfg_folder, harness, name, module, frames, out_npz, state_sides, mutation=None):
    """the kit's renderer: X drawn by its module (module=True) or by base.Look, the other side by base.Look"""
    import numpy as np
    ep, _ = _kit_load(cfg_folder, harness)
    other = [n for n in ep.T.FIGHTERS if n != name]
    FM = _bare(set(other) | (set() if module else {name}))
    if module:
        if FM.module(name) is None:
            raise SystemExit(f'origin: the kit has no module for {name} (older.install() found none with PORTED = True)')
        _mutate_look(FM, name, mutation)
    from ballfight.kit import picture as P            # noqa: N812
    _harness(harness, 'picture', P)
    arrays, states = {}, {}
    for f in frames:
        img = np.ascontiguousarray(P.render(f))
        arrays[f'f{f:05d}'] = img
        states[f] = {k: _plain_state(P.fighter(k, f / FPS)) for k in state_sides}
    common.write_npz(out_npz, arrays, register=False)
    return dict(states=states, names=list(P.NAMES), t_ko=float(P.T_KO), t_hs=float(P.T_HS),
                look=type(P.LOOKS[list(P.NAMES).index(name)]).__module__)


def kit_buses_child(cfg_folder, harness, name, out_npz, buses, mutation=None):
    """the kit's score (before the master): X's Sound (name) or none (name None: both base Sounds, the bed alone)"""
    import numpy as np
    ep, _ = _kit_load(cfg_folder, harness)
    FM = _bare({n for n in ep.T.FIGHTERS if n != name})
    if name is not None:
        if FM.module(name) is None:
            raise SystemExit(f'origin: the kit has no module for {name}')
        _mutate_sound(FM, name, mutation)
    from ballfight.kit import music as M              # noqa: N812
    _harness(harness, 'music', M)
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly  # noqa: F401  (the series' drums, as m3parts.load_music)
    M.SOUNDS = [M.SOUNDS[k] if M.NAMES[k] == name else M.FIGHTER_MODULES.base.Sound(M, k) for k in (0, 1)]
    mix = M.score()
    got = {k: np.array(v, np.float64, copy=True) for k, v in mix.buses.items()}
    missing = [b for b in buses if b not in got]
    if missing:
        raise SystemExit(f"origin: the kit's score has no bus {', '.join(missing)} (it has {', '.join(sorted(got))})")
    common.write_npz(out_npz, {b: got[b] for b in buses}, register=False)
    return dict(buses=sorted(got), n=int(len(next(iter(got.values())))))


# ---------------------------------------------------------------- the kit config of the episode
def kit_config_text(o, timeline):
    """a kit config of the episode: the values its timeline.py gives, then the spec's `config` values"""
    vals = dict(timeline)
    cfg = {}
    for k in CONFIG_FROM_TIMELINE:
        if k in vals and k not in ('FIGHT_BAR',):
            cfg[k] = vals[k]
    if 'KEY_ROOT' in vals:
        cfg['KEY'] = PITCH_NAMES[int(vals['KEY_ROOT']) % 12]
    if 'SCALE' in vals:
        cfg['SCALE'] = tuple(vals['SCALE'])
    if 'FIGHTERS' in cfg:
        cfg['FIGHTERS'] = tuple(cfg['FIGHTERS'])
    fb = vals['T_FIGHT'] / vals['BAR']
    if abs(fb - round(fb)) > 1e-9:
        raise SystemExit(f'origin: the episode fights from {vals["T_FIGHT"]} s, not on a bar line')
    if int(round(fb)) != 2:
        cfg['FIGHT_BAR'] = int(round(fb))
    if cfg.get('RULES') is None:
        cfg.pop('RULES', None)
    cfg['SEED'] = int(o['seed'])
    cfg.update(o['config'])
    lines = ['"""Gate O: the kit config of projects/%s (made by tools/studio_export/older/origin.py)."""' % o['episode']]
    for k in sorted(cfg):
        lines.append(f'{k} = {cfg[k]!r}')
    return '\n'.join(lines) + '\n'


def kit_config_folder(episode, tag, text):
    folder = os.path.join(ROOT, 'kit', episode, tag)
    guard(folder)
    dst = os.path.join(folder, 'episode.py')
    data = text.encode('utf-8')
    if not (os.path.exists(dst) and open(dst, 'rb').read() == data):
        shutil.rmtree(os.path.join(folder, 'check'), ignore_errors=True)
        common.write_bytes(dst, data)
    return folder


def example_config_text(name):
    """a kit config's own text ('examples/<x>' under projects/ballfight/, or 'ballfight_<pair>')"""
    parts = name.split('/')
    src = os.path.join(PROJECTS, 'ballfight', *parts, 'episode.py') if parts[0] == 'examples' else \
        os.path.join(PROJECTS, *parts, 'episode.py')
    if not os.path.isfile(src):
        raise SystemExit(f'origin: no kit config {name!r} ({src})')
    with open(src, encoding='utf-8', newline='') as fh:
        return fh.read()


# ---------------------------------------------------------------- running parts
def _run_parts(jobs):
    """[(fn, args)] in fresh spawn processes, WORKERS at once; returns their results in order"""
    from .. import m3parts
    out, live = [None] * len(jobs), []
    for i, (fn, args) in enumerate(jobs):
        while len(live) >= WORKERS:
            j, h = live.pop(0)
            out[j] = m3parts.wait(h)
        live.append((i, m3parts.start(fn, *args)))
    for j, h in live:
        out[j] = m3parts.wait(h)
    return out


def _load_npz(path):
    import numpy as np
    with np.load(path, allow_pickle=False) as z:
        return {k: z[k] for k in z.files}


def episode_frames(episode, frames, layers_off=(), patches=None):
    """the episode's own render.py in its scratch copy: {frame: RGB uint8}. `layers_off`: the named layers (from
    `patches`, a list of dict(name=, patches=) or a flat patch list as 'other') turned off; anything else of
    `patches` given as a flat list is always applied"""
    flat, variant = _variant(episode, layers_off, patches)
    folder = scratch(episode, flat, variant)
    out = os.path.join(ROOT, 'out', episode, f'frames_{variant or "pristine"}.npz')
    _run_parts([(episode_frames_child, (folder, list(frames), out, []))])
    got = _load_npz(out)
    check_kit_clean(f'projects/{episode}/render.py')
    return {int(k[1:]): v for k, v in got.items()}


def episode_stems(episode, silence=(), patches=None):
    """the episode's own audio.py in its scratch copy: {bus: float64 (n, 2)} as main() placed them; `silence` patches
    applied (in a variant of their own)"""
    import numpy as np
    flat = list(silence) + [p for p in (patches or ()) if not isinstance(p, dict)]
    variant = None if not flat else 'silence_' + _text_sha(flat)[:10]
    folder = scratch(episode, flat, variant)
    out = os.path.join(ROOT, 'out', episode, f'buses_{variant or "pristine"}.npz')
    _run_parts([(episode_stems_child, (folder, out, None))])
    check_kit_clean(f'projects/{episode}/audio.py')
    return {k: np.asarray(v, np.float64) for k, v in _load_npz(out).items()}


def _variant(episode, layers_off, patches):
    if patches is None:
        if layers_off:
            raise SystemExit('origin.episode_frames: layers_off needs the layers (patches=)')
        return [], None
    layers = [p for p in patches if isinstance(p, dict)]
    flat = [p for p in patches if not isinstance(p, dict)]
    names = {L['name'] for L in layers}
    unknown = sorted(set(layers_off) - names)
    if unknown:
        raise SystemExit(f"origin: no layer named {', '.join(unknown)} (there are {', '.join(sorted(names))})")
    for L in layers:
        if L['name'] in layers_off:
            flat += list(L['patches'])
    return flat, ('p_' + _text_sha(flat)[:12]) if flat else None


# ---------------------------------------------------------------- choosing the frames
def choose_frames(facts, k, n, t0, t1):
    """at least n frames inside [t0, t1): each of X's abilities 0.2 s and 0.6 s after its first use, its first hit, its
    HP below 25, the K.O. and the shatter (its shards when it loses, its solo when it wins), then an even spread"""
    want = []
    for name in sorted(facts['first_use'], key=lambda a: facts['first_use'][a]):
        te = facts['first_use'][name]
        want += [te + 0.2, te + 0.6]
    if facts['first_hit'] is not None:
        want.append(facts['first_hit'] + 1.0 / FPS)
    if facts['low'] is not None:
        want.append(facts['low'] + 0.1)
    t_ko = facts['t_ko']
    t_hs = t_ko + facts['hitstop']
    want += [t_ko - 0.5, t_ko + 0.1]
    want += [t_hs + 0.1, t_hs + 0.6, t_hs + 1.5, t_hs + 3.0] if facts['winner'] != k else \
        [t_hs + 0.5, t_hs + 2.0, t_hs + 3.5]
    frames = []
    for t in want:
        f = int(round(t * FPS))
        if t0 * FPS <= f < t1 * FPS and f not in frames:
            frames.append(f)
    f0, f1 = int(math.ceil(t0 * FPS)), int(math.floor(t1 * FPS - 1e-9))
    m = 1
    while len(frames) < n:
        m += 1
        for j in range(1, m):
            f = f0 + (f1 - f0) * j // m
            if f not in frames:
                frames.append(f)
            if len(frames) >= n:
                break
    return sorted(frames)


# ---------------------------------------------------------------- comparisons
def compare_layers(name, frame, e_on, e_off, k_on, k_off, layer_names):
    """None when the two contributions are equal, else the failure line (the first differing pixel, row by row)"""
    import numpy as np
    de = e_on.astype(np.int16) - e_off.astype(np.int16)
    dk = k_on.astype(np.int16) - k_off.astype(np.int16)
    bad = np.any(de != dk, axis=2)
    if not bad.any():
        return None, _sha(de.tobytes())
    ys, xs = np.nonzero(bad)
    y, x = int(ys[0]), int(xs[0])
    base = int(np.any(e_off != k_off, axis=2).sum())
    return (f'Gate O: {name} frame {frame} (t = {frame / FPS:.3f} s), layers {", ".join(layer_names) or "(none)"}: '
            f'the contributions differ at pixel (x={x}, y={y}): the episode {de[y, x].tolist()}, the kit '
            f'{dk[y, x].tolist()} (on: episode {e_on[y, x].tolist()}, kit {k_on[y, x].tolist()}; off: episode '
            f'{e_off[y, x].tolist()}, kit {k_off[y, x].tolist()}); {int(bad.sum())} pixels differ; the two off frames '
            f'differ in {base} pixels'), None


def compare_state(name, frame, ep_state, kit_on, kit_off):
    """the module's state (keys its state() sets or changes on its own ball) against the episode's fighter() there"""
    keys = sorted(k for k in kit_on if k not in kit_off or kit_on[k] != kit_off[k])
    for key in keys:
        if key not in ep_state:
            return keys, (f'Gate O: {name} frame {frame} (t = {frame / FPS:.3f} s): its state sets {key!r} = '
                          f'{kit_on[key]!r}, which the episode\'s fighter() does not have')
        if ep_state[key] != kit_on[key]:
            return keys, (f'Gate O: {name} frame {frame} (t = {frame / FPS:.3f} s): its state {key!r} is '
                          f'{kit_on[key]!r}, the episode\'s {ep_state[key]!r}')
    return keys, None


def db(x):
    return -math.inf if x == 0 else 20.0 * math.log10(x)


def compare_bus(name, bus, e, k, i0, i1, bar):
    """(summary, failure line or None): the bus over samples [i0, i1), float64, against the bar (dBFS)"""
    import numpy as np
    if e.shape != k.shape:
        return dict(bus=bus, shape=[list(e.shape), list(k.shape)]), \
            f'Gate O: {name} bus {bus!r}: the episode has {e.shape} samples, the kit {k.shape}'
    d = np.abs(k[i0:i1] - e[i0:i1])
    peak = float(d.max()) if d.size else 0.0
    whole = float(np.abs(k - e).max()) if e.size else 0.0
    out = dict(bus=bus, window=[i0, i1], peak='bit-identical' if peak == 0 else f'{db(peak):.1f} dBFS',
               whole='bit-identical' if whole == 0 else f'{db(whole):.1f} dBFS',
               sha256_episode=_sha(np.ascontiguousarray(e[i0:i1]).tobytes()),
               sha256_kit=_sha(np.ascontiguousarray(k[i0:i1]).tobytes()))
    if peak == 0 or db(peak) <= bar:
        return out, None
    lim = 10.0 ** (bar / 20.0)
    idx = np.argwhere(d > lim)[0]
    i, c = int(idx[0]) + i0, int(idx[1])
    return out, (f'Gate O: {name} bus {bus!r} differs from the episode\'s at sample {i} (t = {i / SR:.6f} s, channel '
                 f'{c}): the episode {float(e[i, c])!r}, the kit {float(k[i, c])!r}; the peak difference over the '
                 f'window is '
                 f'{db(peak):.1f} dBFS, the bar {bar:.0f} dBFS')


# ---------------------------------------------------------------- Gate O for a fighter
def _episode_commit(episode):
    return common.git('log', '-1', '--format=%H', '--', f'projects/{episode}')


def _patch_doc(patches):
    return [dict(file=f, sha256=_sha((old + '\0' + new).encode()), old=old.strip().splitlines()[0][:100])
            for f, old, new in patches]


def _config(o, ep_facts, tag):
    timeline = eval(ep_facts['timeline'], {'__builtins__': {}})           # (a repr of plain values: our own child's)
    if isinstance(o['config'], str):
        text = example_config_text(o['config'])
    else:
        text = kit_config_text(o, timeline)
    return kit_config_folder(o['episode'], tag, text), text, timeline


def fighter_origin(spec, out=None, mutation=None):
    """Gate O for spec['name'] (the roster name) against spec['origin']: writes origin.json (fighters/<slug>/ unless
    `out`) and returns its contents; stops naming the frame and pixel, or the bus and sample, that differ.
    `mutation` (the self-check only): ('pixel', frame, x, y) or ('sample', bus, index, delta) applied to the kit's
    module at run time, which must make the gate fail"""
    import time
    import numpy as np
    from . import install
    install()
    name = spec['name']
    o = origin_spec(spec['origin'])
    ep = o['episode']
    t_all = time.time()
    slug = name.lower().replace(' ', '_')
    other = list(o['other'])
    off = other + [p for L in o['layers'] for p in L['patches']]
    layer_names = [L['name'] for L in o['layers']]
    seed = o['seed'] if o['seed'] is not None else int(open(os.path.join(episode_dir(ep), 'seed.txt')).read().split()[0])
    e_on_dir = scratch(ep, other, 'on_' + _text_sha(other)[:10]) if other else scratch(ep)
    e_off_dir = scratch(ep, off, 'off_' + _text_sha(off)[:10]) if off else scratch(ep)
    # ---- the fight
    ep_facts = _run_parts([(episode_facts_child, (scratch(ep), seed))])[0]
    check_kit_clean(f'projects/{ep}/sim.py')
    cfg_dir, cfg_text, timeline = _config(dict(o, seed=seed), ep_facts, f'{slug}')
    names = list(timeline['FIGHTERS'])
    if name not in names:
        raise SystemExit(f'Gate O: {name} does not fight in projects/{ep} ({names})')
    k = names.index(name)
    kf = _run_parts([(kit_facts_child, (cfg_dir, o['harness'], k))])[0]
    if kf['raw_line'] != ep_facts['line']:
        raise SystemExit(f'Gate O: {name}: the kit\'s run of the config is not the episode\'s fight: the replay guard '
                         f'gives {ep_facts["line"]!r} for projects/{ep}, the kit {kf["raw_line"]!r}')
    t0 = kf['t_fight']
    t1 = min(kf['t_reassemble'], float(timeline['T_REASSEMBLE']))
    frames = list(o['frames']) if not isinstance(o['frames'], int) else choose_frames(kf, k, o['frames'], t0, t1)
    outside = [f for f in frames if not (t0 * FPS <= f < t1 * FPS)]
    if outside:
        raise SystemExit(f'Gate O: {name}: frames {outside} are outside the window [{t0:.3f}, {t1:.3f}) s')
    print(f'[origin {name}: fight {kf["raw_line"]}; {len(frames)} frames in [{t0:.2f}, {t1:.2f}) s]', flush=True)
    # ---- the four pictures and the sound, in fresh processes
    od = os.path.join(ROOT, 'out', ep, slug)
    paths = {w: os.path.join(od, f'{w}.npz') for w in ('e_on', 'e_off', 'k_on', 'k_off', 'e_bus', 'k_bus')}
    cfg_on = kit_config_folder(ep, f'{slug}_on', cfg_text)
    cfg_off = kit_config_folder(ep, f'{slug}_off', cfg_text)
    cfg_snd = kit_config_folder(ep, f'{slug}_sound', cfg_text)
    jobs = [(episode_frames_child, (e_on_dir, frames, paths['e_on'], [k])),
            (episode_frames_child, (e_off_dir, frames, paths['e_off'], [k])),
            (kit_frames_child, (cfg_on, o['harness'], name, True, frames, paths['k_on'], [k], mutation)),
            (kit_frames_child, (cfg_off, o['harness'], name, False, frames, paths['k_off'], [k]))]
    if o['buses']:
        jobs += [(episode_stems_child, (scratch(ep), paths['e_bus'], list(o['buses']))),
                 (kit_buses_child, (cfg_snd, o['harness'], name, paths['k_bus'], list(o['buses']), mutation))]
    t_parts = time.time()
    res = _run_parts(jobs)
    check_kit_clean(f'projects/{ep}')
    print(f'[origin {name}: drawn and played in {time.time() - t_parts:.1f} s]', flush=True)
    e_on, e_off, k_on, k_off = (_load_npz(paths[w]) for w in ('e_on', 'e_off', 'k_on', 'k_off'))
    if res[2]['look'] == 'ballfight.kit.fighters.base':
        raise SystemExit(f'Gate O: {name}: the kit drew it with base.Look (no module installed)')
    failures, rows, state_keys = [], [], set()
    for f in frames:
        key = f'f{f:05d}'
        bad, dsha = compare_layers(name, f, e_on[key], e_off[key], k_on[key], k_off[key], layer_names)
        keys, sbad = compare_state(name, f, res[0]['states'][f][k], res[2]['states'][f][k], res[3]['states'][f][k])
        state_keys |= set(keys)
        if bad:
            failures.append(bad)
        if sbad:
            failures.append(sbad)
        rows.append(dict(frame=f, t=round(f / FPS, 6), contribution_sha256=dsha,
                         sha256=dict((w, _sha(arr[key].tobytes())) for w, arr in
                                     (('episode_on', e_on), ('episode_off', e_off), ('kit_on', k_on), ('kit_off', k_off))),
                         whole_frame_equal=bool(np.array_equal(e_on[key], k_on[key]))))
    buses = []
    if o['buses']:
        eb, kb = _load_npz(paths['e_bus']), _load_npz(paths['k_bus'])
        i0, i1 = int(round(t0 * SR)), int(round(t1 * SR))
        for bus in o['buses']:
            bar = float(o['exceptions'][bus][0]) if bus in o['exceptions'] else o['bar_db']
            summary, bad = compare_bus(name, bus, eb[bus], kb[bus], i0, i1, bar)
            if bus in o['exceptions']:
                summary['exception'] = dict(bar_db=bar, reason=o['exceptions'][bus][1])
            buses.append(summary)
            if bad:
                failures.append(bad)
    if failures:
        more = f' (and {len(failures) - 1} more: ' + '; '.join(x.split(':')[1].strip() for x in failures[1:6]) + \
            (' ...' if len(failures) > 6 else '') + ')' if len(failures) > 1 else ''
        raise SystemExit(failures[0] + more)
    doc = dict(
        about=f'Gate O (plan m4 Task 2): {name}\'s reference kit module against projects/{ep}, its layers pixel for '
              f'pixel (the contribution, on minus off, in int16) and its buses sample for sample (float64) from FIGHT! '
              f'to the end of the shatter',
        name=name, side=k, episode=ep, episode_commit=_episode_commit(ep), seed=seed,
        fight=dict(line=kf['raw_line'], equal=True, winner=kf['winner'], t_ko=kf['t_ko']),
        window=dict(t0=t0, t1=t1, frames=[int(round(t0 * FPS)), int(math.ceil(t1 * FPS))]),
        config=dict(sha256=_sha(cfg_text.encode()), text=cfg_text.splitlines()),
        layers=[dict(name=L['name'], patches=_patch_doc(L['patches'])) for L in o['layers']],
        other=_patch_doc(other), harness=[dict(h) for h in o['harness']],
        frames=rows, state_keys=sorted(state_keys), buses=buses,
        exceptions={b: dict(bar_db=float(v[0]), reason=v[1]) for b, v in sorted(o['exceptions'].items())},
        spec_sha256=_text_sha(_plain_origin(o)),
        cache=common.rel(od))
    if out is not False:
        path = out or common.ref('fighters', slug, 'origin.json')
        common.write_json(path, doc)
    print(f'[origin {name}: PASS, {len(frames)} frames, {len(buses)} buses, {time.time() - t_all:.1f} s]', flush=True)
    return doc


def _plain_origin(o):
    return json.loads(json.dumps(o, default=list))


# ---------------------------------------------------------------- Gate O for a theme's bed
def mood_origin(spec, out=None):
    """Gate O for the mood spec['name'] against spec['origin']: the bed alone in the kit (both fighters base Sounds)
    against the episode's bed buses with the fighters' placements removed (`silence`), over the whole buffer"""
    import time
    from . import install
    install()
    name = spec['name']
    o = origin_spec(spec['origin'])
    ep = o['episode']
    if not o['buses']:
        raise SystemExit(f'Gate O: {name}: name the bed buses (origin.buses)')
    t_all = time.time()
    seed = o['seed'] if o['seed'] is not None else int(open(os.path.join(episode_dir(ep), 'seed.txt')).read().split()[0])
    ep_facts = _run_parts([(episode_facts_child, (scratch(ep), seed))])[0]
    check_kit_clean(f'projects/{ep}/sim.py')
    cfg_dir, cfg_text, timeline = _config(dict(o, seed=seed), ep_facts, f'mood_{name}')
    kf =_run_parts([(kit_facts_child, (cfg_dir, o['harness'], 0))])[0]
    if kf['raw_line'] != ep_facts['line']:
        raise SystemExit(f'Gate O: {name}: the kit\'s run of the config is not the episode\'s fight: the replay guard '
                         f'gives {ep_facts["line"]!r} for projects/{ep}, the kit {kf["raw_line"]!r}')
    silence = list(o['silence'])
    e_dir = scratch(ep, silence, 'silence_' + _text_sha(silence)[:10]) if silence else scratch(ep)
    od = os.path.join(ROOT, 'out', ep, f'mood_{name}')
    paths = dict(e=os.path.join(od, 'episode_buses.npz'), k=os.path.join(od, 'kit_buses.npz'))
    cfg_snd = kit_config_folder(ep, f'mood_{name}_sound', cfg_text)
    res = _run_parts([(episode_stems_child, (e_dir, paths['e'], list(o['buses']))),
                      (kit_buses_child, (cfg_snd, o['harness'], None, paths['k'], list(o['buses']), None))])
    check_kit_clean(f'projects/{ep}/audio.py')
    eb, kb = _load_npz(paths['e']), _load_npz(paths['k'])
    failures, buses = [], []
    for bus in o['buses']:
        bar = float(o['exceptions'][bus][0]) if bus in o['exceptions'] else o['bar_db']
        summary, bad = compare_bus(name, bus, eb[bus], kb[bus], 0, len(eb[bus]), bar)
        if bus in o['exceptions']:
            summary['exception'] = dict(bar_db=bar, reason=o['exceptions'][bus][1])
        buses.append(summary)
        if bad:
            failures.append(bad)
    if failures:
        raise SystemExit(failures[0] + (f' (and {len(failures) - 1} more)' if len(failures) > 1 else ''))
    doc = dict(
        about=f'Gate O (plan m4 Task 2): the {name} mood against projects/{ep}\'s bed: its buses with both fighters\' '
              f'parts silent, sample for sample (float64), over the whole buffer',
        name=name, episode=ep, episode_commit=_episode_commit(ep), seed=seed,
        fight=dict(line=kf['raw_line'], equal=True),
        config=dict(source=o['config'] if isinstance(o['config'], str) else 'timeline', sha256=_sha(cfg_text.encode())),
        silence=_patch_doc(silence), harness=[dict(h) for h in o['harness']], buses=buses,
        episode_buses=res[0]['buses'], kit_buses=res[1]['buses'],
        exceptions={b: dict(bar_db=float(v[0]), reason=v[1]) for b, v in sorted(o['exceptions'].items())},
        spec_sha256=_text_sha(_plain_origin(o)), cache=common.rel(od))
    if out is not False:
        path = out or common.ref('themes', name, 'origin.json')
        common.write_json(path, doc)
    print(f'[origin {name}: PASS, {len(buses)} buses, {time.time() - t_all:.1f} s]', flush=True)
    return doc


def origin_done(folder, origin):
    """None if folder/origin.json is Gate O's for this origin spec, else what to run (fighter_refs / theme_refs read
    it: an older port's export needs its Gate O first)"""
    path = common.ref(*folder.split('/'), 'origin.json')
    if not os.path.exists(path):
        return f'{folder}/origin.json is missing: run python -m tools.studio_export origin {folder}'
    with open(path, encoding='utf-8') as fh:
        doc = json.load(fh)
    if doc.get('spec_sha256') != _text_sha(_plain_origin(origin_spec(origin))):
        return f'{folder}/origin.json is for another origin spec: run python -m tools.studio_export origin {folder}'
    return None

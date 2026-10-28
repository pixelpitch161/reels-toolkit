"""An episode = one short config file (`episode.py` in the episode's folder). This module loads it and builds the
episode's timeline from it: the module every shared BALLFIGHT piece expects under the name `timeline` (title.py, the
fighters' looks and sounds, the renderer, the audio), with the tempo, the bar grid, the title card, the story targets,
the scripted tail and the harmony helpers all derived from a handful of config values.

    from ballfight.kit import episode
    EP = episode.load('ballfight_harp_vs_tuba')       # a folder under projects/ (or any path holding episode.py)
    T = EP.T                                         # = sys.modules['timeline']

Config names (UPPER_CASE in episode.py; everything else has a series default):
    FIGHTERS    ('LEFT', 'RIGHT'): first named = left corner           SEED       the chosen honest run
    BPM         the base tempo (loop_bpm makes it a whole number of bars in 61 s)
    DMG_SCALE   shared damage scale (1.0)        DMG_SIDE   per-fighter handicap (crossovers' only lever) or None
    RULES       opt-in engine rules, e.g. dict(swat_cobras=True)
    MOOD        a bed in kit/moods/ (e.g. 'big_top')                   KEY, SCALE   e.g. 'E', 'harmonic minor'
    CHORDS      one chord per bar (len = the bar count); LOOP_START (2) and LOOP (the cycle from FIGHT!) for lines
    LINES       {fighter: notes over the loop: 8 per bar = 8ths, 4 = beats}   SOLO {fighter: [(beat, midi, beats)]}
    TITLE_HITS  {fighter: notes it plays as its name slams in}         ROLES {fighter: 'bass' / 'melody' / ...}
    CAPTION     the post caption; TAGS {'instagram': ..., 'tiktok': ..., 'shorts': ...}
    MIX         bus gain / reverb send overrides {bus: (gain, send)}
    Only to reproduce an older episode exactly: SERIES (the old "BALLFIGHT #N" tag), SHARD_SEED, FX_SEED, FIGHT_BAR
    (the bar FIGHT! lands on: 2, or 1 for an episode from before the VS card, e.g. #29; the card then overlaps it)."""
import importlib.util
import math
import os
import sys
import types

KIT = os.path.dirname(os.path.abspath(__file__))
BALLFIGHT = os.path.dirname(KIT)                      # projects/ballfight: the engine, roster, ring, title card
PROJECTS = os.path.dirname(BALLFIGHT)
ROOT = os.path.dirname(PROJECTS)
_CURRENT = None

# ---------------------------------------------------------------- harmony
NOTE_PC = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}
QUALITY = {'': (0, 4, 7), 'm': (0, 3, 7), '7': (0, 4, 7, 10), 'm7': (0, 3, 7, 10), 'maj7': (0, 4, 7, 11),
           'dim': (0, 3, 6), 'dim7': (0, 3, 6, 9), 'm7b5': (0, 3, 6, 10), 'aug': (0, 4, 8), 'sus2': (0, 2, 7),
           'sus4': (0, 5, 7), '6': (0, 4, 7, 9), 'm6': (0, 3, 7, 9)}
SCALES = {'major': (0, 2, 4, 5, 7, 9, 11), 'minor': (0, 2, 3, 5, 7, 8, 10), 'harmonic minor': (0, 2, 3, 5, 7, 8, 11),
          'melodic minor': (0, 2, 3, 5, 7, 9, 11), 'dorian': (0, 2, 3, 5, 7, 9, 10), 'phrygian': (0, 1, 3, 5, 7, 8, 10),
          'mixolydian': (0, 2, 4, 5, 7, 9, 10), 'lydian': (0, 2, 4, 6, 7, 9, 11)}


def pitch_class(name):
    pc = NOTE_PC[name[0]]
    for ch in name[1:]:
        pc += 1 if ch == '#' else (-1 if ch == 'b' else 0)
    return pc % 12


def parse_chord(name):
    """'Em' -> (4, (4, 7, 11)); 'B7' -> (11, (11, 3, 6, 9)); 'Bb' -> (10, (10, 2, 5)): root pc + pcs (root, 3rd, ...)"""
    n = 2 if len(name) > 1 and name[1] in '#b' else 1
    root = pitch_class(name[:n])
    return root, tuple((root + i) % 12 for i in QUALITY[name[n:]])


# ---------------------------------------------------------------- the timeline every shared piece imports
def build_timeline(cfg, folder):
    """the episode's `timeline` module from its config (config names first, series defaults for the rest)"""
    from reelkit.config import loop_bpm
    T = types.ModuleType('timeline')
    T.__file__ = os.path.join(folder, 'episode.py')     # title.py keeps its sprite cache in the episode's check/
    for k in dir(cfg):
        if k.isupper():
            setattr(T, k, getattr(cfg, k))
    T.FPS = 30
    T.DUR = 61.0
    T.NF = int(T.FPS * T.DUR)
    T.BPM_BASE = cfg.BPM
    T.BPM = loop_bpm(cfg.BPM, T.DUR)                    # a whole number of bars in 61 s
    T.BEAT = 60.0 / T.BPM
    T.BAR = 4 * T.BEAT
    T.N_BARS = int(round(T.DUR / T.BAR))
    T.S16 = T.BEAT / 4
    fb = getattr(cfg, 'FIGHT_BAR', 2)                   # (only a pre-card episode's check config sets it: #29 fought from bar 1)
    T.T_FIGHT = fb * T.BAR                              # VS card on bar 0, count-in on bar 1, FIGHT! on the bar-2 downbeat
    c0 = (fb - 1) * T.BAR
    T.COUNT = [(c0, '3'), (c0 + T.BEAT, '2'), (c0 + 2 * T.BEAT, '1'), (fb * T.BAR, 'FIGHT!')]
    a, b = cfg.FIGHTERS
    T.TITLE = [(0.0, a), (T.BEAT, 'VS'), (2 * T.BEAT, b)]
    T.T_DOORS = (3 * T.BEAT + 0.10, T.BAR - 0.03)
    T.T_CLOSE = 0.30
    T.SERIES = getattr(cfg, 'SERIES', None) or ''      # no episode tag on new episodes (user, Oct 27)
    for k, v in dict(DMG_SCALE=1.0, DMG_SIDE=None, RULES={}, T_KO=(49.0, 53.0), MIN_LEAD_CHANGES=2,
                     MAX_WINNER_HP=30.0, CAP_GAP=4.0, MAX_GAP=4.0, SLOWMO=0.7, SLOWMO_RATE=0.25, LINES={}, SOLO={},
                     TITLE_HITS={}, ROLES={}, MIX={}, LOOP_START=2, SHARD_SEED=271, FX_SEED=None, CAPTION='',
                     TAGS={}).items():
        if not hasattr(T, k):
            setattr(T, k, v)
    if T.FX_SEED is None:
        T.FX_SEED = sum(map(ord, a + b)) % 1000
    T.T_REASSEMBLE = (T.N_BARS - 2) * T.BAR            # the loser's pieces fly back together
    T.T_GLIDE = (T.N_BARS - 1) * T.BAR                 # last bar: both glide to their corners, HP refills
    T.T_REV = T.T_GLIDE + 1.05
    # harmony
    T.KEY_PC = pitch_class(cfg.KEY)
    T.KEY_ROOT = 36 + T.KEY_PC                         # the key's root in the C2 octave (E -> 40, D -> 38)
    sc = getattr(cfg, 'SCALE', 'minor')
    T.SCALE = SCALES[sc] if isinstance(sc, str) else tuple(sc)
    assert len(T.CHORDS) == T.N_BARS, f'CHORDS has {len(T.CHORDS)} bars, the episode has {T.N_BARS}'
    T.CHORD_PCS = {c: parse_chord(c)[1] for c in set(T.CHORDS)}
    if not hasattr(T, 'LOOP'):
        T.LOOP = T.CHORDS[T.LOOP_START:T.LOOP_START + 8]
    if not hasattr(T, 'CAP_TARGETS'):
        from fighters import FIGHTERS as ROSTER
        T.CAP_TARGETS = {ROSTER[n]['abilities'][s]['name']: v for n in cfg.FIGHTERS
                         for s, v in (('unlock1', 12), ('unlock2', 24), ('low', 40))}
    _helpers(T)
    return T


def _helpers(T):
    def bar_t(b):
        return b * T.BAR

    def beat_env(t, decay=0.12):
        return math.exp(-((t % T.BEAT) / decay))

    def beat_pulse(t, decay=0.12, pre=0.12):
        """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that
        lands on frame 0 doesn't jump at the loop seam (#34's 120 ms ramp)"""
        ph = t % T.BEAT
        up = min(1.0, max(0.0, (ph - (T.BEAT - pre)) / pre))
        return max(math.exp(-ph / decay), up * up * (3 - 2 * up))

    def next_bar(t):
        return math.ceil(t / T.BAR - 1e-6) * T.BAR

    def chord_at(t):
        return T.CHORDS[int(max(0.0, t) // T.BAR) % T.N_BARS]

    def chord_tones(lo, hi, chord):
        return [m for m in range(lo, hi + 1) if m % 12 in T.CHORD_PCS[chord]]

    def scale_notes(lo, hi, chord=None):
        """the key's scale between midi lo..hi; over a chord whose tone the scale lacks, the scale step a semitone
        below it is raised (D minor over A: C -> C#, the leading tone)"""
        pcs = [(T.KEY_PC + s) % 12 for s in T.SCALE]
        if chord is not None:
            for c in T.CHORD_PCS[chord]:
                if c not in pcs and (c - 1) % 12 in pcs:
                    pcs = [c if p == (c - 1) % 12 else p for p in pcs]
        return [m for m in range(lo, hi + 1) if m % 12 in pcs]

    def root_above(chord, lo):
        """the chord's root at or above midi lo (the lowest one)"""
        r = parse_chord(chord)[0]
        return lo + (r - lo) % 12

    def loop_index(t, per_bar):
        """slot index inside the loop (bars LOOP_START.. follow it), `per_bar` slots a bar, snapped to the nearest"""
        n = per_bar * len(T.LOOP)
        return (int(round(t / (T.BAR / per_bar))) - per_bar * T.LOOP_START) % n

    def line_at(name, t):
        """the note of fighter `name`'s line (config LINES, else a default) at t: 8 notes a bar = 8ths, 4 = beats"""
        line = T.LINES[name] if name in T.LINES else default_line(name)
        per_bar = len(line) // len(T.LOOP)
        return line[loop_index(t, per_bar)]

    def default_line(name, per_bar=8):
        """a line for a fighter the config wrote none for: each loop bar's chord tones up and back down in the upper
        two thirds of its register, one per 8th (a sketch to replace with a real line)"""
        from fighters import FIGHTERS as ROSTER
        lo, hi = ROSTER[name]['register']
        out = []
        for ch in T.LOOP:
            tones = chord_tones(lo + (hi - lo) // 3, hi, ch)[:4] or chord_tones(lo, hi, ch)[:4]
            shape = tones + tones[-2:0:-1]
            out += [shape[i % len(shape)] for i in range(per_bar)]
        return out

    def default_solo(name):
        """a 2-bar victory motif on the tonic chord in the fighter's register (a sketch to replace)"""
        from fighters import FIGHTERS as ROSTER
        lo, hi = ROSTER[name]['register']
        ch = T.CHORDS[0]
        tones = chord_tones(lo + (hi - lo) // 4, hi, ch)
        up = tones[:4] if len(tones) >= 4 else (tones * 4)[:4]
        return [(0, up[0], .5), (.5, up[1], .5), (1, up[2], .5), (1.5, up[3], .5), (2, up[2], .5), (2.5, up[1], .5),
                (3, up[3], 1.0), (4, up[2], .5), (4.5, up[1], .5), (5, up[0], 1.0), (6, up[0], 2.0)]

    for f in (bar_t, beat_env, beat_pulse, next_bar, chord_at, chord_tones, scale_notes, root_above, loop_index,
              line_at, default_line, default_solo):
        setattr(T, f.__name__, f)
    T.SOLO = dict(T.SOLO)
    for n in T.FIGHTERS:
        if n not in T.SOLO:
            T.SOLO[n] = default_solo(n)


# ---------------------------------------------------------------- loading
class Episode:
    def __init__(self, folder, cfg=None):
        self.dir = os.path.abspath(folder)
        self.name = os.path.basename(self.dir.rstrip('/'))
        if cfg is None:
            path = os.path.join(self.dir, 'episode.py')
            spec = importlib.util.spec_from_file_location('ballfight_episode_config', path)
            cfg = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(cfg)
        self.cfg = cfg
        self.check = os.path.join(self.dir, 'check')
        os.makedirs(self.check, exist_ok=True)
        self.T = build_timeline(self.cfg, self.dir)
        self.seed = getattr(self.cfg, 'SEED', None)
        self.out = os.path.join(self.dir, (self.name if self.name.startswith('ballfight') else 'ballfight_' + self.name)
                                + '.mp4')

    def path(self, *p):
        return os.path.join(self.dir, *p)


def activate_paths():
    """the shared modules by name, as every episode imports them: the engine, roster, ring and title card first, then
    the Pixel Pitch bridge (read-only)"""
    for d in (PROJECTS, BALLFIGHT, os.path.join(PROJECTS, 'which_instrument_wins')):
        if d not in sys.path:
            sys.path.append(d)


def find(name):
    """an episode folder from a name: a path, a folder under projects/, ballfight_<name>, or a kit example"""
    for c in (name, os.path.join(PROJECTS, name), os.path.join(PROJECTS, 'ballfight_' + name),
              os.path.join(BALLFIGHT, 'examples', name)):
        if os.path.exists(os.path.join(c, 'episode.py')):
            return os.path.abspath(c)
    raise SystemExit(f'no episode.py for {name!r} (tried a path, projects/{name}, projects/ballfight_{name}, '
                     f'projects/ballfight/examples/{name})')


def load(name, cfg=None):
    """load an episode and make it the current one: its timeline becomes `timeline` for the shared modules. With
    `cfg` (a config object), `name` is just the folder for its caches and outputs (previews)"""
    global _CURRENT
    activate_paths()
    folder = os.path.abspath(name) if cfg is not None else find(name)
    sys.modules.pop('timeline', None)
    from fighters import FIGHTERS                       # noqa: F401  (the roster, before the timeline needs it)
    ep = Episode(folder, cfg)
    sys.modules['timeline'] = ep.T
    os.environ['BALLFIGHT_EPISODE'] = ep.dir             # render workers (reelkit.video) load the same episode
    _CURRENT = ep
    return ep


def current():
    if _CURRENT is None:
        if os.environ.get('BALLFIGHT_EPISODE'):
            return load(os.environ['BALLFIGHT_EPISODE'])
        raise RuntimeError('no episode loaded: ballfight.kit.episode.load(name) first')
    return _CURRENT

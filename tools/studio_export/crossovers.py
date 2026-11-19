"""crossovers: six crossover starters as the kit plays them (plan Task 59, Step 1).

    python -m tools.studio_export crossovers           writes studio/reference/episodes/crossovers/
    python -m tools.studio_export cache episodes/crossovers   regenerates the cache and checks the pins

A pair the series never wrote music for starts from the left fighter's home config (its newest kit config, by the
episode number on the first line of its notes; an example only when it has no other): its theme (or, if the theme
lacks a bed hook a part plays, the right fighter's home theme, else the big top), tempo, key, scale, intro, loop,
tail and that theme's options (a PLUGGED naming a fighter not in the pair becomes the left fighter). Both fighters
play the kit's sketch line and solo (no LINES, no SOLO), no title hits, no MIX; roles: the left keeps its home role,
the right takes its own unless that is the same, then 'bass' if its card can play bass and the left's role is not
'bass', else 'melody' unless the left plays the melody, else 'line'; DMG_SIDE None (neutral), DMG_SCALE 1.0, the
pair's rules (m3refs.PAIR_RULES). bf_content::starter makes the same fight file; this module is the kit's side,
written from the kit's own files only.

For each of the six pairs (one per theme family) the kit config this rule makes is written as a private episode.py
under studio/reference/.cache/episodes/crossovers_<pair>/, the kit's own seed search (kit/search.py's `one`: the
honest run, story.story, story.score) runs from seed 1 upwards until the first fit, as far as MAX_SEED (the Studio's
own search limit), and that fit is recorded, drawn at 8 frames and played with both real parts. At the rule's neutral
handicap a one-sided pair has no fit at all (five of the six: the K.O. lands far before 49 s, or one fighter wins every
fight); then the lowest seed whose fight ends in a K.O. stands in, marked `fit: false`, since the kit plays and draws
it all the same and the proof is that the Studio's soundtrack and frames are the kit's. Committed: crossovers.json
(the home config of every kit fighter, and per crossover the kit config's values, the seed, whether it is a fit and
how many seeds were searched, its recording's hash, the 16-bit master's SHA-256 and the 8 frames' SHA-256). Cached
and pinned (ruling M3-R44): each crossover's recording, <pair>.bfrec."""
import importlib
import os
import re
import sys
import time

from . import common, m3refs

FOLDER = 'episodes/crossovers'
EXPORT = 'crossovers'
ORDER = {'crossovers': 90}                    # after every theme and pair gate under `all`

# one crossover per theme family (plan Task 59): left corner first
PAIRS = (('PIANO', 'TUBA'), ('CELLO', 'FLUTE'), ('BASS DRUM', 'HORN'), ('ALTO SAX', 'ORGAN'),
         ('EUPHONIUM', 'CLARINET'), ('ELECTRIC', 'BASSOON'))
# the fighters whose Studio card can play bass (bf_content::cards, `can_play_bass`); starters.rs proves the cards
# agree, so the kit side and the Studio side use the same list
CAN_PLAY_BASS = ('BASS CLARINET', 'CONTRABASS', 'ORGAN', 'SOUSAPHONE', 'TROMBONE', 'TUBA')
FALLBACK_THEME = 'big_top'
SEARCH_CHUNK = 48                             # seeds handed to the pool at a time (the first fit is the lowest seed)
MAX_SEED = 5000                               # as far as the Studio's own search goes (native_render)
N_FRAMES = 8


# ---------------------------------------------------------------- the rule, from the kit's files
def episode_number(config):
    """the episode number on the first line of a kit config's notes ('#51 BALLFIGHT: ...' -> 51)"""
    with open(m3refs.config_source(config), encoding='utf-8') as fh:
        first = fh.readline()
    m = re.match(r'\s*[rRuUbB]*("""|\'\'\')\s*#(\d+)', first)
    if not m:
        raise SystemExit(f'{config}: its notes do not start with an episode number: {first.strip()!r}')
    return int(m.group(2))


def config_fighters(config):
    """a kit config's FIGHTERS, read from its text (some example configs import the roster, so they are not
    executed here)"""
    import ast
    with open(m3refs.config_source(config), encoding='utf-8') as fh:
        for line in fh:
            if line.startswith('FIGHTERS'):
                return tuple(ast.literal_eval(line.split('=', 1)[1].split('#', 1)[0].strip()))
    raise SystemExit(f'{config}: no FIGHTERS line')


def kit_fighters():
    """the fighters the kit has a look and sound for (kit/fighters/), the Studio's 20"""
    common.kit()[0].activate_paths()
    from ballfight.kit import fighters as FM          # noqa: N812
    return FM.ported()


def homes(fighters):
    """{fighter: its home config}: the newest config with it (the highest episode number), an example only when it
    has no other"""
    found = {}
    for c in m3refs.kit_configs():
        names = config_fighters(c)
        for n in names:
            if n in fighters:
                found.setdefault(n, []).append((not c.startswith('examples/'), episode_number(c), c))
    missing = [n for n in fighters if n not in found]
    if missing:
        raise SystemExit(f"crossovers: no kit config has {', '.join(missing)}")
    return {n: max(found[n])[2] for n in fighters}


def bed_hooks(name):
    """the bed hooks the fighter's part calls without asking (m3parts._bed_calls on kit/fighters/<name>.py)"""
    from . import m3parts as MP
    common.kit()[0].activate_paths()
    from ballfight.kit import fighters as FM          # noqa: N812
    return MP._bed_calls(FM.module(name))


def theme_has(mood, hooks):
    common.kit()[0].activate_paths()
    bed = importlib.import_module(f'ballfight.kit.moods.{mood}').Bed
    return all(callable(getattr(bed, h, None)) for h in hooks)


def rule(a, b, home):
    """the kit config values the starter rule gives the pair a (left) vs b: dict(fighters, home, theme_from, mood,
    bpm, key, scale, intro, loop, tail, chords, roles, options (the kit's names), theme_options (the fight file's),
    rules, handicap, dmg_scale)"""
    ha, hb = read(home[a]), read(home[b])
    hooks = sorted(set(bed_hooks(a)) | set(bed_hooks(b)))
    if theme_has(ha['mood'], hooks):
        mood, theme_from = ha['mood'], 'left'
    elif theme_has(hb['mood'], hooks):
        mood, theme_from = hb['mood'], 'right'
    else:
        mood, theme_from = FALLBACK_THEME, 'big_top'
    cfg = m3refs.config_module(home[a])
    options = {}
    if mood == ha['mood']:
        for name, (owner, names) in m3refs.THEME_OPTIONS.items():
            if owner != mood or not hasattr(cfg, name):
                continue
            value = getattr(cfg, name)
            if isinstance(names, str) and value not in (a, b):
                value = a
            options[name] = value
    ra, rb = ha['roles'][a], hb['roles'][b]
    if rb == ra:
        if b in CAN_PLAY_BASS and ra != 'bass':
            rb = 'bass'
        elif ra != 'melody':
            rb = 'melody'
        else:
            rb = 'line'
    return dict(fighters=[a, b], home=home[a], theme_from=theme_from, mood=mood, bpm=ha['bpm'], key=ha['key'],
                scale=ha['scale'], intro=ha['intro'], loop=ha['loop'], tail=ha['tail'], chords=ha['chords'],
                roles={a: ra, b: rb}, options=options,
                theme_options=m3refs.theme_options_of(_Options(options), mood, [a, b]),
                rules=m3refs.pair_rules(a, b), handicap=[1.0, 1.0], dmg_scale=1.0)


class _Options:
    """the kit option names of a rule as attributes (what theme_options_of reads from a config module)"""

    def __init__(self, options):
        for k, v in options.items():
            setattr(self, k, v)


_READ = {}


def read(config):
    if config not in _READ:
        _READ[config] = m3refs.read_config(config)
    return _READ[config]


def config_text(r, seed=None):
    """the kit episode.py of a rule's values (every other config name at the series default)"""
    a, b = r['fighters']
    lines = [f'"""a crossover starter (plan Task 59): {a} vs {b} on the music of {r["home"]}, the kit\'s sketch lines '
             f'and solos; written by tools/studio_export/crossovers.py"""',
             f'FIGHTERS = {(a, b)!r}']
    if seed is not None:
        lines.append(f'SEED = {int(seed)!r}')
    lines += [f"BPM = {r['bpm']!r}", f"DMG_SCALE = {float(r['dmg_scale'])!r}", 'DMG_SIDE = None',
              f"RULES = {dict(sorted(r['rules'].items()))!r}", f"MOOD = {r['mood']!r}",
              f"KEY, SCALE = {r['key']!r}, {r['scale']!r}", f"LOOP = {list(r['loop'])!r}",
              f"CHORDS = {list(r['chords'])!r}", f"ROLES = {dict(r['roles'])!r}"]
    lines += [f'{k} = {v!r}' for k, v in sorted(r['options'].items())]
    return '\n'.join(lines) + '\n'


def pair_slug(a, b):
    return f"{a.lower().replace(' ', '')}_vs_{b.lower().replace(' ', '')}"


# ---------------------------------------------------------------- the kit's search, a spawn pool
def _fit(seed):
    """kit/search.py's `one` for a seed (the pool worker's story is m3refs._STORY): (seed, fits, t_ko, winner)"""
    S = m3refs._STORY                                 # noqa: N806
    run = S.run_seed(seed, record=False)
    st = S.story(run)
    fits, _ = S.score(st)
    return seed, bool(fits), st['t_ko'], st['winner']


def first_fit(folder, label):
    """(seed, fits, tried): the lowest seed from 1 whose honest run fits the beat sheet; seeds are handed out
    SEARCH_CHUNK at a time and the first chunk with a fit ends it. When no seed up to MAX_SEED fits (the rule's
    neutral handicap on a one-sided pair: five of the six crossovers), the lowest seed whose fight ends in a K.O.,
    with fits False: the kit plays and draws it all the same, so the soundtrack and frame proof still holds"""
    import hashlib
    import json
    import multiprocessing as mp
    # a search already made for this exact config text (a local note beside the private config, never committed:
    # a fresh cache searches again, and the recording's pin proves the same fight either way)
    with open(os.path.join(folder, 'episode.py'), 'rb') as fh:
        key = hashlib.sha256(fh.read()).hexdigest()
    note = os.path.join(folder, 'first_fit.json')
    if os.path.isfile(note):
        with open(note, encoding='utf-8') as fh:
            got = json.load(fh)
        if got.get('config_sha256') == key and (got['seed'] is not None or got['tried'] >= MAX_SEED):
            seed = got['seed'] if got['seed'] is not None else got['first_ko']
            print(f"crossovers: {label}: seed {seed}, a fit: {got['seed'] is not None} (this config's search, "
                  f"{got['tried']} seeds, made before)", flush=True)
            return seed, got['seed'] is not None, min(got['tried'], MAX_SEED) if got['seed'] is None else got['tried']
    for k in m3refs.THREAD_VARS:
        os.environ[k] = '1'
    t0, done, first_ko = time.time(), 0, None
    with mp.get_context('spawn').Pool(m3refs.WORKERS, initializer=m3refs._init, initargs=(folder,)) as pool:
        for start in range(1, MAX_SEED + 1, SEARCH_CHUNK):
            got = pool.map(_fit, range(start, min(start + SEARCH_CHUNK, MAX_SEED + 1)), chunksize=4)
            done = got[-1][0]
            if first_ko is None:
                first_ko = next((g[0] for g in got if g[2] is not None), None)
            fits = [g for g in got if g[1]]
            if fits:
                seed = fits[0][0]
                common.write_bytes(note, json.dumps(dict(config_sha256=key, seed=seed, tried=done,
                                                         first_ko=first_ko)).encode())
                print(f'crossovers: {label}: first fit seed {seed} (K.O. {fits[0][2]}, winner {fits[0][3]}); '
                      f'{done} seeds tried, {time.time() - t0:.0f} s', flush=True)
                return seed, True, done
    if first_ko is None:
        raise SystemExit(f'crossovers: {label}: no fit and no K.O. in seeds 1 to {MAX_SEED}')
    common.write_bytes(note, json.dumps(dict(config_sha256=key, seed=None, tried=done, first_ko=first_ko)).encode())
    print(f'crossovers: {label}: no fit in seeds 1 to {MAX_SEED} ({time.time() - t0:.0f} s): the lowest seed with a '
          f'K.O., {first_ko}, instead', flush=True)
    return first_ko, False, done


# ---------------------------------------------------------------- the parts, each in a fresh process
def frames_child(cfg, rec_path):
    """the kit's picture of the fight at 8 frames (the title card, FIGHT!, three moments of the fight, the K.O., 0.6 s
    after it, the middle of the winner's solo): [frame, SHA-256 of the RGB bytes]"""
    import numpy as np
    from . import frames as F
    from . import look_flute as LF
    from . import m3parts as MP
    MP.fresh_check(cfg.folder)
    cfg.load()
    from ballfight.kit import picture as P            # noqa: N812
    F.check_recording(P, rec_path)
    T = P.T                                           # noqa: N806
    f_fight, f_ko = MP.first_at(T.T_FIGHT), MP.first_at(P.T_KO)
    picks = sorted({0, f_fight, *(f_fight + (f_ko - f_fight) * i // 4 for i in (1, 2, 3)), f_ko,
                    MP.first_at(P.T_KO + 0.6), MP.first_at((P.T_SOLO + T.T_REASSEMBLE) / 2)})
    assert len(picks) == N_FRAMES, picks
    rows = []
    for f in picks:
        P.ui.reset_drawn()
        img = P.render(f)
        assert img.dtype == np.uint8
        rows.append([int(f), LF.sha_array(img)])
    return rows


def crossover(a, b, home, module):
    """one crossover: the rule's config, the kit's first fit (or, with none, its lowest seed with a K.O.; recorded to
    the cache and pinned), its master and frames"""
    from . import m3parts as MP
    from . import recording
    slug = pair_slug(a, b)
    r = rule(a, b, home)
    search_folder = m3refs.config_folder(f'crossovers_{slug}', slug, config_text(r))
    ep, _ = m3refs.load_config(search_folder)
    T = ep.T                                          # noqa: N806
    # the kit read the config as the rule wrote it, and plays the sketch lines and solos
    assert (list(T.FIGHTERS), T.MOOD, T.KEY, list(T.LOOP), list(T.CHORDS), dict(T.ROLES), dict(T.LINES)) == \
        (r['fighters'], r['mood'], r['key'], r['loop'], r['chords'], r['roles'], {}), slug
    assert (T.DMG_SIDE, T.DMG_SCALE, dict(T.RULES), dict(T.TITLE_HITS), dict(T.MIX)) == \
        (None, 1.0, r['rules'], {}, {}), slug
    seed, fits, tried = first_fit(search_folder, f'{a} vs {b}')
    text = config_text(r, seed)
    cfg = m3refs.Loaded(m3refs.config_folder(f'crossovers_{slug}', f'{slug}_s{seed}', text), (a, b), seed)
    tmp = m3refs.cache_dir(FOLDER, f'{slug}.bfrec.tmp')
    os.makedirs(os.path.dirname(tmp), exist_ok=True)
    rec = m3refs.record(cfg, seed, {}, tmp)
    data = open(tmp, 'rb').read()
    os.remove(tmp)
    path = m3refs.cache_bytes(FOLDER, f'{slug}.bfrec', data, EXPORT)

    def part(n):
        return m3refs.Loaded(m3refs.config_folder(f'crossovers_{slug}_{n}', f'{slug}_s{seed}', text), (a, b), seed)
    stems = m3refs.cache_dir(FOLDER, f'{slug}_stems_tmp.npz')
    h = [MP.start(frames_child, part('frames'), path), MP.start(MP.full_sound_child, part('sound'), stems, path)]
    frames, sound = MP.wait(h[0]), MP.wait(h[1])
    if os.path.exists(stems):
        os.remove(stems)
    assert sound['fighters'] == [a, b] and sound['seed'] == seed, sound['fighters']
    out = {k: r[k] for k in ('fighters', 'home', 'theme_from', 'mood', 'bpm', 'key', 'scale', 'intro', 'loop', 'tail',
                             'roles', 'theme_options', 'rules', 'handicap', 'dmg_scale')}
    out.update(n_bars=len(r['chords']), seed=seed, fit=fits, searched=tried,
               recording=dict(cache=m3refs.cache_rel(path), sha256=m3refs.manifest.sha256(path)),
               rec_hash=recording.rec_hash(rec), winner=rec['winner'], t_ko=rec['t_ko'],
               master_sha256=sound['master_sha256'], buses=sound['buses'], frames=frames)
    return out


def export():
    module = sys._getframe(1).f_globals.get('__name__', __name__)
    run = m3refs.run_state()
    run.stages.pop(FOLDER, None)
    t0 = time.time()
    fighters = kit_fighters()
    home = homes(fighters)
    rows = [crossover(a, b, home, module) for a, b in PAIRS]
    m3refs.commit_json(FOLDER, 'crossovers.json', dict(
        about="plan Task 59's crossover starters as the kit plays them: every kit fighter's home config (its newest "
              "kit config by episode number, an example only when it has no other), and for six pairs with no kit "
              "config the kit config bf_content::starter's rule makes, the kit's first fit of it (its own search "
              f"from seed 1 to {MAX_SEED}; with no fit, `fit` false and the lowest seed with a K.O.; the recording "
              "pinned in the cache), the 16-bit master's SHA-256 and 8 frames' SHA-256",
        tool=__name__, format=1, fighters=fighters, homes=home,
        episodes={c: episode_number(c) for c in sorted(set(home.values()))},
        can_play_bass=list(CAN_PLAY_BASS), crossovers=rows), __name__)
    m3refs.finish(FOLDER)
    print(f'crossovers: {FOLDER}: {len(rows)} crossovers, {len(m3refs.read_pins(FOLDER))} pins, '
          f'{time.time() - t0:.0f} s')


# ---------------------------------------------------------------- drift (m3refs.drift)
def drift(folder=FOLDER, doc=None):
    """what a cache run would find changed in crossovers.json that follows from the kit's files alone (the homes and
    each crossover's config values), and every recording pinned"""
    doc = m3refs._read_json(folder, 'crossovers.json') if doc is None else doc
    out = []
    fighters = kit_fighters()
    home = homes(fighters)
    if doc['fighters'] != fighters or doc['homes'] != home:
        out.append(f'{folder}/crossovers.json: the fighters or their home configs differ from the kit\'s')
    if doc['can_play_bass'] != list(CAN_PLAY_BASS):
        out.append(f'{folder}/crossovers.json: can_play_bass differs from CAN_PLAY_BASS')
    if [c['fighters'] for c in doc['crossovers']] != [list(p) for p in PAIRS]:
        out.append(f'{folder}/crossovers.json: the pairs differ from PAIRS')
        return out
    pins = m3refs.read_pins(folder)
    for c in doc['crossovers']:
        r = m3refs._plain(rule(*c['fighters'], home))
        diff = sorted(k for k in r if k in c and c[k] != r[k])
        if diff:
            out.append(f"{folder}/crossovers.json: {c['fighters']}: {', '.join(diff)} differ from the rule")
        p = pins.get(c['recording']['cache'])
        if p is None or p['sha256'] != c['recording']['sha256']:
            out.append(f"{folder}/crossovers.json: {c['fighters']}: the recording is not a pinned cache file")
    return out


EXPORTS = {'crossovers': export}

"""scenarios: scenario fights of the example episode, the kind-coverage table, and a fight with no K.O.

studio/reference/fights/flute_vs_trombone/scenarios/<scenario>_s<seed>.bfrec: honest fights (story.run_seed) of
FLUTE against TROMBONE under rule overrides passed through story.rules(**over), so that every event kind, every hit
sub-kind and every ability fires early, inside the window where the Rust port must agree with Python event by
event. The overrides are part of each recording's `rules`; the seeds are three per scenario.

    early_levels   LV_DMG=(2.0, 5.0): the unlocks come after a few hits
    early_low      LV_DMG=(2.0, 5.0), LOW_HP=92.0: the desperation moves come early too
    early_ko       dmg_scale=3.0: the shared damage scale (an argument of story.rules), so the K.O. comes inside
                   the window
    plain          no overrides (seeds 7, 8, 9; seeds 1, 2, 3 are in fights/)
    swapped        no overrides, TROMBONE in the left corner: the episode config with FIGHTERS and DMG_SIDE reversed
                   (nothing else in the config is per side), written as its own copy under the cache folder

studio/reference/fights/flute_vs_trombone/coverage.json: for every event kind, every `hit` sub-kind ("hit/<what>",
element 7) and every ability name ("ability/<name>", element 3) seen in the scenario recordings, the recording,
event index, time and the event itself of its first occurrence (earliest time after FIGHT! over all scenarios; a tie
goes to the scenario listed first, then the lower seed). The required list is built from stats.json (run `stats`
first): every event kind and every hit sub-kind that occurs in at least REQUIRE_FIGHTS of the 4,000 statistics
fights, plus the fixed list the brief names. Each required item must occur within WINDOW s of FIGHT!, or the export
stops.

studio/reference/fights/flute_vs_trombone/no_ko.bfrec: the lowest seed from 1 upward with no K.O. before rules.t_max
under the episode's own rules, a real sample of the null winner / t_ko path. Its seed is `no_ko_seed` in
coverage.json, and its stats.json row must agree (null t_ko, fits false)."""
import json
import os

from . import common, fights, manifest, recording, stats

EPISODE = 'flute_vs_trombone'
TAG = 'scenarios'
LV = (2.0, 5.0)
SCENARIOS = (                                             # name, overrides, swapped, seeds
    ('early_levels', dict(LV_DMG=LV), False, (1, 2, 3)),
    ('early_low', dict(LV_DMG=LV, LOW_HP=92.0), False, (1, 2, 3)),
    ('early_ko', dict(dmg_scale=3.0), False, (1, 2, 3)),
    ('plain', dict(), False, (7, 8, 9)),
    ('swapped', dict(), True, (1, 2, 3)),
)
WINDOW = 20.0                                             # s after FIGHT! every required item must occur by
REQUIRE_FIGHTS = 20                                       # occurs in at least this many of the statistics fights
MUST = ('basket', 'strike', 'thrust', 'swat', 'rope', 'clash', 'hit/fang', 'hit/blade',
        'ability/FLUTTER TONGUE', 'ability/SNAKE DANCE', 'ability/PICCOLO',
        'ability/SAD TROMBONE', 'ability/POWER SLIDE', 'ability/PEDAL TONE',
        'hit/wave', 'hit/shockwave', 'hit/slam')       # the brief's list: the six abilities and the hits they cause
FOLDER = common.ref('fights', EPISODE)
COVERAGE = os.path.join(FOLDER, 'coverage.json')
NO_KO = os.path.join(FOLDER, 'no_ko.bfrec')
SWAP = {"FIGHTERS = ('FLUTE', 'TROMBONE')": "FIGHTERS = ('TROMBONE', 'FLUTE')",
        'DMG_SIDE = (2.0, 0.40)': 'DMG_SIDE = (0.40, 2.0)'}


def swapped_episode():
    """the episode with TROMBONE in the left corner: a copy of the example's episode.py with FIGHTERS and DMG_SIDE
    reversed, under the cache folder (studio/reference/.cache/episodes/scenarios_swapped/), loaded with kit.episode"""
    src = os.path.join(common.REPO, 'projects', 'ballfight', 'examples', EPISODE, 'episode.py')
    with open(src, encoding='utf-8', newline='') as f:
        text = f.read()
    for old, new in SWAP.items():
        assert text.count(old) == 1, f'{common.rel(src)}: expected {old!r} once'
        text = text.replace(old, new)
    folder = os.path.join(common.CACHE, 'episodes', f'{TAG}_swapped', EPISODE)
    dst = os.path.join(folder, 'episode.py')
    data = text.encode('utf-8')
    if not (os.path.exists(dst) and open(dst, 'rb').read() == data):
        common.write_bytes(dst, data)
    E, _ = common.kit()
    return E.load(folder)


def per_side_check(base_T, swap_T):
    """the swapped config differs from the episode's only in the order of the two sides"""
    assert tuple(swap_T.FIGHTERS) == tuple(reversed(base_T.FIGHTERS))
    assert tuple(swap_T.DMG_SIDE) == tuple(reversed(base_T.DMG_SIDE))
    same = [k for k in vars(base_T) if k.isupper() and k not in ('FIGHTERS', 'DMG_SIDE', 'TITLE')]
    for k in same:
        a, b = getattr(base_T, k), getattr(swap_T, k)
        assert repr(a) == repr(b) or (isinstance(a, dict) and a == b), k


def record(story, rules, seed, over, out):
    """one recorded scenario fight, written, read back and checked like the reference fights"""
    r = story.run_seed(seed, **over)
    recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)
    rec = recording.read_bfrec(out)
    fights.check(r, rules, rec, story.REC_KEYS)
    manifest.add(out)
    return rec


def items(e):
    out = [e[1]]
    if e[1] == 'hit':
        out.append(f'hit/{e[7]}')
    elif e[1] == 'ability':
        out.append(f'ability/{e[3]}')
    return out


def first_uses(recs, t_fight):
    """{item: entry} for the earliest occurrence of every item over the recordings (in SCENARIOS order)"""
    best = {}
    for (name, seed, path), rec in recs:
        for i, e in enumerate(rec['events']):
            for item in items(e):
                old = best.get(item)
                if old is None or e[0] < old['t']:
                    best[item] = dict(item=item, scenario=name, seed=seed, file=path, index=i, t=e[0],
                                      after_fight=e[0] - t_fight, event=e)
    return best


def run():
    if not os.path.exists(stats.OUT):
        raise SystemExit(f'scenarios: {common.rel(stats.OUT)} is missing: run `stats` first (its counts are the '
                         f'coverage requirement)')
    with open(stats.OUT, encoding='utf-8') as f:
        st = json.load(f)
    ep = common.episode(EPISODE, TAG)
    _, story = common.kit()
    base_T = ep.T
    assert tuple(base_T.FIGHTERS) == ('FLUTE', 'TROMBONE'), base_T.FIGHTERS
    t_fight = base_T.T_FIGHT
    recs, table = [], {}
    for name, over, swapped, seeds in SCENARIOS:
        if swapped:
            sw = swapped_episode()
            per_side_check(base_T, sw.T)
        else:
            common.episode(EPISODE, TAG)
        _, story = common.kit()
        rules = story.rules(**over)
        for k, v in over.items():                         # the overrides are in the recording's rules
            assert getattr(rules, k) == v, (name, k)
        for seed in seeds:
            out = os.path.join(FOLDER, 'scenarios', f'{name}_s{seed}.bfrec')
            rec = record(story, rules, seed, over, out)
            path = common.rel(out)[len(common.rel(common.REFERENCE)) + 1:]
            recs.append(((name, seed, path), rec))
            print(f"scenarios: {common.rel(out)}  {os.path.getsize(out)} bytes  winner {rec['winner']}  "
                  f"K.O. {rec['t_ko']}  events {len(rec['events'])}  hash {recording.rec_hash(rec)}")
            table[f'{name}_s{seed}'] = recording.rec_hash(rec)
    # ---- the fight with no K.O.: the lowest such seed under the episode's own rules
    common.episode(EPISODE, TAG)
    _, story = common.kit()
    rules = story.rules()
    no_ko_seed = next(r['seed'] for r in st['rows'] if r['t_ko'] is None)
    assert [r['seed'] for r in st['rows'][:no_ko_seed]] == list(range(1, no_ko_seed + 1))
    rec = record(story, rules, no_ko_seed, {}, NO_KO)
    row = st['rows'][no_ko_seed - 1]
    assert rec['winner'] is None and rec['t_ko'] is None, 'no_ko: the recording has a K.O.'
    assert row['seed'] == no_ko_seed and row['winner'] is None and row['t_ko'] is None and row['fits'] is False, row
    s2 = story.story(story.run_seed(no_ko_seed, record=False))
    assert story.score(s2) == (row['fits'], row['score']), (story.score(s2), row)
    table['no_ko'] = recording.rec_hash(rec)
    print(f"scenarios: {common.rel(NO_KO)}  seed {no_ko_seed}  winner None  K.O. None  events {len(rec['events'])}  "
          f"frames {len(rec['arrays']['HP'])}  hash {table['no_ko']}  (stats row: fits {row['fits']}, "
          f"score {row['score']})")
    # ---- coverage
    counts = st['fights_with']
    by_data = sorted(k for k, n in counts.items() if n >= REQUIRE_FIGHTS and not k.startswith('ability/'))
    required = sorted(set(by_data) | set(MUST))
    best = first_uses(recs, t_fight)
    missing = [k for k in required if k not in best or best[k]['after_fight'] > WINDOW]
    print(f'scenarios: coverage (first occurrence over the scenario fights; FIGHT! at {t_fight!r} s)')
    print(f"  {'item':24s} {'req':3s} {'stats fights':>12s}  {'after FIGHT!':>12s}  where")
    for k in sorted(best, key=lambda k: (best[k]['after_fight'], k)):
        b = best[k]
        print(f"  {k:24s} {'yes' if k in required else '':3s} {counts.get(k, 0):12d}  {b['after_fight']:10.3f} s  "
              f"{b['scenario']} seed {b['seed']} event {b['index']}")
    if missing:
        raise SystemExit(f"scenarios: not covered within {WINDOW} s of FIGHT!: "
                         + ', '.join(f"{k} ({best[k]['after_fight']:.2f} s)" if k in best else f'{k} (never)'
                                     for k in missing))
    entries = []
    for k in sorted(best):
        b = dict(best[k])
        b.update(required=k in required, stats_fights=counts.get(k, 0))
        entries.append(b)
    doc = dict(
        about=f'{EPISODE}: where every event kind, hit sub-kind and ability first fires in the scenario fights',
        tool='scenarios',
        format=1,
        episode=EPISODE,
        t_fight=t_fight,
        window=WINDOW,
        require_fights=REQUIRE_FIGHTS,
        stats_fights=st['pooled']['n'],
        required=required,
        rule='required = every event kind and every hit sub-kind ("hit/<what>") that occurs in at least '
             'require_fights of the stats.json fights, plus the brief\'s fixed list; each occurs within window s of '
             't_fight in the entry\'s recording. An entry: the first occurrence over all scenario recordings (the '
             'earliest t; a tie goes to the scenario listed first, then the lower seed); file is relative to '
             'studio/reference/, index is the position in its events, event is the event itself',
        scenarios=[dict(name=n, overrides=dict(o), swapped=s, seeds=list(seeds),
                        files=[f'fights/{EPISODE}/scenarios/{n}_s{x}.bfrec' for x in seeds])
                   for n, o, s, seeds in SCENARIOS],
        no_ko_seed=no_ko_seed,
        no_ko_file=f'fights/{EPISODE}/no_ko.bfrec',
        hashes=table,
        entries=entries)
    common.write_json(COVERAGE, doc)
    print(f'scenarios: {common.rel(COVERAGE)}  {len(entries)} items, {len(required)} required, all within '
          f'{WINDOW} s; no-K.O. seed {no_ko_seed}')
    text = open(fights.DOC, encoding='utf-8').read() if os.path.exists(fights.DOC) else ''
    stale = [f'{n} {h}' for n, h in table.items() if h not in text]          # the format document lists them
    if stale:
        raise SystemExit(f"scenarios: {common.rel(fights.DOC)} does not state these hashes, update it: "
                         f"{', '.join(stale)}")


EXPORTS = {'scenarios': run}
ORDER = {'scenarios': 60}                                 # after stats (55): the coverage requirement reads stats.json

"""stats: 4,000 honest fights of the example episode, one row per seed, and the pooled figures.

studio/reference/fights/flute_vs_trombone/stats.json: for seeds 1 to 4,000 under the episode's own rules
(story.run_seed(seed, record=False)), the kit's story.story() and story.score() of each fight, reduced to the row

    seed, winner, t_ko, lead_changes, winner_hp, n_caps, max_gap, lows, fits, score

with the kit's exact values (floats by their shortest round-trip repr, `winner` and `t_ko` null when there is no
K.O.; `winner_hp` is then 100.0 and `t_ko` counts as 99 s in the score, as the kit has it). Then the pooled figures,
defined the way kit/tuning.py reports them (the file says how each is defined), the rule set the fights ran under,
and `fights_with`: for every event kind, every `hit` sub-kind (element 7, "hit/<what>") and every ability name
("ability/<name>"), in how many of the fights it occurs at least once. The scenarios export builds its coverage
requirement from those counts.

The fights run in a `spawn` process pool (Windows has no fork): a module-level worker that loads the episode itself.
Results are collected and sorted by seed, so the file does not depend on the worker count or on timing."""
import multiprocessing as mp
import os
import time

from . import common

EPISODE = 'flute_vs_trombone'
TAG = 'stats'
SEEDS = (1, 4000)                                         # first and last seed, both included
OUT = common.ref('fights', EPISODE, 'stats.json')
ROW_KEYS = ('seed', 'winner', 't_ko', 'lead_changes', 'winner_hp', 'n_caps', 'max_gap', 'lows', 'fits', 'score')
NO_KO_T = 99.0                                            # what a fight with no K.O. counts as (kit: tuning.py, score)
CHECK_BY_HAND = (1, 2, 3, 392, 500)                       # rows recomputed in the parent from direct kit calls

_STORY = None


def _init():
    """pool worker start: load the episode in this process (spawn starts from nothing)"""
    global _STORY
    common.episode(EPISODE, TAG)
    _STORY = common.kit()[1]


def present(events):
    """the coverage items one fight shows: event kinds, 'hit/<what>', 'ability/<name>'"""
    out = set()
    for e in events:
        out.add(e[1])
        if e[1] == 'hit':
            out.add(f'hit/{e[7]}')
        elif e[1] == 'ability':
            out.add(f'ability/{e[3]}')
    return out


def row(story, seed):
    """(the stats row of one seed, the coverage items its fight shows), from the kit's own story and score"""
    r = story.run_seed(seed, record=False)
    st = story.story(r)
    fits, sc = story.score(st)
    out = dict(seed=st['seed'], winner=st['winner'], t_ko=st['t_ko'], lead_changes=st['lead_changes'],
               winner_hp=st['winner_hp'], n_caps=len(st['caps']), max_gap=st['max_gap'], lows=list(st['lows']),
               fits=fits, score=sc)
    return out, sorted(present(r['events']))


def _one(seed):
    return row(_STORY, seed)


def check_row(r):
    """the row schema the file promises (the types Rust will read)"""
    def is_int(x):
        return isinstance(x, int) and not isinstance(x, bool)
    assert tuple(r) == ROW_KEYS, tuple(r)
    assert is_int(r['seed']) and r['seed'] >= 1, r
    assert (r['winner'] is None) == (r['t_ko'] is None), r
    assert r['winner'] is None or r['winner'] in (0, 1), r
    assert r['t_ko'] is None or type(r['t_ko']) is float, r
    assert is_int(r['lead_changes']) and r['lead_changes'] >= 0, r
    assert type(r['winner_hp']) is float and 0.0 <= r['winner_hp'] <= 100.0, r
    assert r['winner'] is not None or r['winner_hp'] == 100.0, r
    assert is_int(r['n_caps']) and r['n_caps'] >= 0, r
    assert type(r['max_gap']) is float or r['max_gap'] == 99, r          # 99 (an int) when nobody hit at all
    assert isinstance(r['lows'], list) and all(isinstance(x, str) for x in r['lows']), r
    assert r['lows'] == sorted(set(r['lows'])), r
    assert type(r['fits']) is bool, r
    assert type(r['score']) is float, r
    assert r['fits'] is False or r['t_ko'] is not None, r                # a fight with no K.O. is never a fit


def pooled(rows, names):
    """the pooled figures, as kit/tuning.py computes them (numpy's median and percentile)"""
    import numpy as np
    n = len(rows)
    dec = [r for r in rows if r['winner'] is not None]
    left = sum(r['winner'] == 0 for r in dec)
    tko = np.array([r['t_ko'] if r['t_ko'] is not None else NO_KO_T for r in rows])
    return dict(
        n=n,
        decided=len(dec),
        left_wins=left,
        win_share=left / len(dec),
        ko_median=float(np.median(tko)),
        ko_p10=float(np.percentile(tko, 10)),
        ko_p90=float(np.percentile(tko, 90)),
        ko_median_decided=float(np.median([r['t_ko'] for r in dec])),
        no_ko=n - len(dec),
        no_ko_share=(n - len(dec)) / n,
        lead_changes_mean=float(np.mean([r['lead_changes'] for r in rows])),
        fits=sum(r['fits'] for r in rows),
        fit_rate=sum(r['fits'] for r in rows) / n,
        both_lows_share=sum(len(r['lows']) == 2 for r in rows) / n,
        winner_hp_median_decided=float(np.median([r['winner_hp'] for r in dec])),
        names=list(names))


DEFINITIONS = {
    'n': 'fights in the file: seeds 1 to 4,000',
    'decided': 'fights with a K.O. before rules.t_max (winner not null)',
    'left_wins': 'decided fights the left fighter (names[0]) won',
    'win_share': 'left_wins / decided: the left fighter\'s share of the decided fights (fights with no K.O. left '
                 'out), as kit/tuning.py',
    'ko_median': 'numpy median of t_ko over ALL n fights, a fight with no K.O. counting as 99.0 s (kit/tuning.py; '
                 'also how kit/story.py score treats it)',
    'ko_p10': 'numpy percentile 10 (linear) of the same list as ko_median',
    'ko_p90': 'numpy percentile 90 (linear) of the same list as ko_median',
    'ko_median_decided': 'numpy median of t_ko over the decided fights only',
    'no_ko': 'fights with no K.O. (n - decided)',
    'no_ko_share': 'no_ko / n',
    'lead_changes_mean': 'mean of lead_changes over all n fights (fights with no K.O. included)',
    'fits': 'fights whose score() fits is true',
    'fit_rate': 'fits / n, over all n fights (a fight with no K.O. never fits)',
    'both_lows_share': 'share of all n fights in which both desperation moves (lows) fired',
    'winner_hp_median_decided': 'numpy median of winner_hp over the decided fights',
    'names': 'the fighters, left corner first: index 0 is the left fighter in winner and win_share',
}


def run():
    ep = common.episode(EPISODE, TAG)
    _, story = common.kit()
    T = ep.T
    assert tuple(T.FIGHTERS) == ('FLUTE', 'TROMBONE'), T.FIGHTERS
    rules = story.rules()
    seeds = list(range(SEEDS[0], SEEDS[1] + 1))
    procs = max(1, min(16, os.cpu_count() or 1))
    t0 = time.time()
    with mp.get_context('spawn').Pool(procs, initializer=_init) as pool:
        got = pool.map(_one, seeds, chunksize=8)
    dt = time.time() - t0
    got.sort(key=lambda x: x[0]['seed'])                   # (map keeps the order already; said outright)
    rows = [g[0] for g in got]
    assert [r['seed'] for r in rows] == seeds
    for r in rows:
        check_row(r)
    for seed in CHECK_BY_HAND:                             # the pool's rows equal direct kit calls in this process
        direct, items = row(story, seed)
        common.same(direct, rows[seed - 1], f'row {seed}', ordered=True)
        assert items == got[seed - 1][1], seed
    counts = {}
    for _, items in got:
        for k in items:
            counts[k] = counts.get(k, 0) + 1
    pool_figs = pooled(rows, T.FIGHTERS)
    doc = dict(
        about=f'{EPISODE}: one row per seed {SEEDS[0]}..{SEEDS[1]} under the episode\'s rules (record=False), from '
              f'the kit\'s story.story() and story.score(); then the pooled figures',
        tool='stats',
        format=1,
        episode=EPISODE,
        source=dict(kind='python', commit=common.kit_commit()),
        names=list(T.FIGHTERS),
        seeds=list(SEEDS),
        rules=dict(vars(rules)),
        beat_sheet=dict(T_KO=list(T.T_KO), MIN_LEAD_CHANGES=T.MIN_LEAD_CHANGES, MAX_WINNER_HP=T.MAX_WINNER_HP,
                        CAP_GAP=T.CAP_GAP, MAX_GAP=T.MAX_GAP, CAP_TARGETS=dict(T.CAP_TARGETS), MIN_CAPS=5),
        columns=list(ROW_KEYS),
        row_notes=dict(
            winner='index into names; null when nobody was knocked out before rules.t_max',
            t_ko='seconds on the video clock; null exactly when winner is null',
            lead_changes='fight.lead_changes(events)[0] (5 HP hysteresis, from 4 s)',
            winner_hp='max(0, the winner\'s HP from the hit events); 100.0 when there is no K.O.',
            n_caps='len(story.captions(events)): first uses of abilities kept >= CAP_GAP s apart',
            max_gap='longest time between hits, from t_fight to the first hit included; the int 99 if no hit',
            lows='sorted names of the desperation moves (low abilities) that fired',
            fits='story.score() fits: the beat sheet\'s hard windows',
            score='story.score() score: the soft story score, round(s, 2)'),
        pooled=pool_figs,
        definitions=DEFINITIONS,
        fights_with=dict(sorted(counts.items())),
        rows=rows)
    common.write_json(OUT, doc, register=False)
    back = _read(OUT)
    for a, b in zip(rows, back['rows']):
        common.same(a, b, f"row {a['seed']}")
    from . import manifest
    manifest.add(OUT)
    p = pool_figs
    print(f'stats: {common.rel(OUT)}  {os.path.getsize(OUT)} bytes  {len(rows)} fights in {dt:.1f} s on {procs} '
          f'workers')
    print(f"stats: {p['names'][0]} wins {100 * p['win_share']:.1f} % of {p['decided']} decided "
          f"({p['names'][1]} {100 * (1 - p['win_share']):.1f} %)  median K.O. {p['ko_median']:.2f} s "
          f"(10th-90th {p['ko_p10']:.2f}-{p['ko_p90']:.2f}; decided only {p['ko_median_decided']:.2f})  "
          f"no K.O. {p['no_ko']} ({100 * p['no_ko_share']:.1f} %)")
    print(f"stats: mean lead changes {p['lead_changes_mean']:.3f}  fit rate {100 * p['fit_rate']:.2f} % "
          f"({p['fits']} fits)  both lows {100 * p['both_lows_share']:.1f} %")


def _read(path):
    import json
    with open(path, encoding='utf-8') as f:
        return json.load(f)


EXPORTS = {'stats': run}
ORDER = {'stats': 55}                                     # before scenarios: its coverage requirement reads this file

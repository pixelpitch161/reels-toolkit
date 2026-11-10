"""stories: the kit's full story and verdict of every committed recording of the example episode.

studio/reference/fights/flute_vs_trombone/stories.json: for each recording in that folder (the five reference fights,
no_ko.bfrec and the 15 scenario recordings), keyed by its path relative to the folder, the kit's story.story(run)
dict exactly as the kit builds it (tuples written as lists) and its story.score() verdict, computed under the
recording's own episode (the swapped config for `swapped`) from the run read back out of the file. The kit gives no
reasons a run does not fit (score() returns only (fits, score)), so the file has none. Every story is computed a
second time from a fresh run of the kit (story.run_seed with the scenario's overrides, record=False) and must be
equal, and the recording's rules must equal story.rules(**overrides). Also the beat-sheet constants the kit used.

Run after `fights` and `scenarios`: it reads their recordings and coverage.json (for the scenarios' overrides)."""
import json
import os

from . import common, fights, recording, scenarios

EPISODE = 'flute_vs_trombone'
TAG = 'stories'
FOLDER = common.ref('fights', EPISODE)
OUT = os.path.join(FOLDER, 'stories.json')
REFERENCE = ['ep392.bfrec'] + [f'seed{s}.bfrec' for s in fights.OTHER_SEEDS]
MIN_CAPS = 5                                              # story.score: len(st['caps']) >= 5 (a literal in the kit)


def beat_sheet(T):
    return dict(T_KO=list(T.T_KO), MIN_LEAD_CHANGES=T.MIN_LEAD_CHANGES, MAX_WINNER_HP=T.MAX_WINNER_HP,
                CAP_GAP=T.CAP_GAP, MAX_GAP=T.MAX_GAP, MIN_CAPS=MIN_CAPS, CAP_TARGETS=dict(T.CAP_TARGETS),
                T_FIGHT=T.T_FIGHT, LOWS=dict(zip(T.FIGHTERS, _story().lows())))


def _story():
    return common.kit()[1]


def recordings():
    """[(name relative to the folder, overrides, swapped)] in a fixed order: reference fights, no_ko, scenarios"""
    with open(scenarios.COVERAGE, encoding='utf-8') as f:
        cov = json.load(f)
    out = [(n, {}, False) for n in REFERENCE] + [('no_ko.bfrec', {}, False)]
    for sc in cov['scenarios']:
        for seed in sc['seeds']:
            out.append((f"scenarios/{sc['name']}_s{seed}.bfrec", sc['overrides'], sc['swapped']))
    return out


def run():
    base = common.episode(EPISODE, TAG)
    sheet = beat_sheet(base.T)
    stories = {}
    for name, over, swapped in recordings():
        ep = scenarios.swapped_episode() if swapped else common.episode(EPISODE, TAG)
        story = _story()
        sh = beat_sheet(ep.T)
        assert {k: v for k, v in sh.items() if k != 'LOWS'} == {k: v for k, v in sheet.items() if k != 'LOWS'}
        over = {k: tuple(v) if isinstance(v, list) else v for k, v in over.items()}
        rec = recording.read_bfrec(os.path.join(FOLDER, name))
        assert rec['names'] == list(ep.T.FIGHTERS), (name, rec['names'])
        common.same(common.plain(dict(vars(story.rules(**over)))), rec['rules'], f'{name} rules')
        st = story.story(fights.as_kit_run(rec))
        fits, sc = story.score(st)
        again = story.story(story.run_seed(rec['seed'], record=False, **over))       # the kit's own fight, no file
        common.same(common.plain(st), common.plain(again), f'{name} story')
        assert story.score(again) == (fits, sc), name
        assert type(fits) is bool and type(sc) is float, (name, fits, sc)
        stories[name] = dict(names=rec['names'], overrides=over, story=st, fits=fits, score=sc)
        print(f"stories: {name:34s} winner {st['winner']}  K.O. {st['t_ko']}  lead changes {st['lead_changes']}  "
              f"caps {len(st['caps'])}  fits {fits}  score {sc}")
    doc = dict(
        about=f'{EPISODE}: the kit\'s story.story() and story.score() of every committed recording, under its own '
              f'rules and episode',
        tool='stories',
        format=1,
        episode=EPISODE,
        source=dict(kind='python', commit=common.kit_commit()),
        beat_sheet=sheet,
        notes=dict(
            key='the recording\'s path relative to studio/reference/fights/flute_vs_trombone/',
            story='story.story(run) as the kit returns it: tuples (firsts and caps entries are (t, who, name)) as '
                  'lists; who indexes names; max_gap is the int 99 when there is no hit; winner_hp is 100.0 with no '
                  'K.O.',
            fits_score='story.score(story): (fits, score); the kit gives no reasons for a miss',
            beat_sheet='the episode timeline constants score() and captions() read; MIN_CAPS is a literal in '
                       'score(); LOWS are the fighters\' low ability names (story.lows(), base episode order)',
            overrides='the rule overrides of the recording (story.rules(**overrides)); empty for the episode\'s '
                      'own rules'),
        stories=stories)
    common.write_json(OUT, doc)
    print(f'stories: {common.rel(OUT)}  {len(stories)} recordings  {os.path.getsize(OUT)} bytes')


EXPORTS = {'stories': run}
ORDER = {'stories': 70}                                   # after fights (50) and scenarios (60): it reads their files

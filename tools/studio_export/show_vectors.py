"""show: the show timings and caption windows the kit's picture derives from a run, for bf_core::show.

studio/reference/show/flute_vs_trombone.json: for each of the five reference recordings (ep392 and seeds 1, 2, 3,
500) and for no_ko.bfrec, what `projects/ballfight/kit/picture.py` computes at import from that run: WIN, LOSE, T_KO,
HITSTOP, T_HS, T_SOLO, SOLO_T, NAMES, STORY['caps'] (= story.captions(events) with the timeline's CAP_GAP), and every
caption window of _caps() in order (CAP_ABIL[0], CAP_ABIL[1], CAP_KO, CAP_WINS, CAP_COMMENT, and CAPTIONS, the
concatenation the audio reads). Floats are float.hex() strings.

kit.picture derives these values at import, for the episode's configured SEED, so each seed is imported in its own
interpreter: the episode is loaded through common.episode(name, "show") (the run pickle and every cache picture
writes land under studio/reference/.cache/episodes/show/), its `seed` is set to the recording's seed before
kit.picture is imported (story.load reads it), and the run pickle of that seed is removed first so the kit fights
it afresh. The whole module is imported, not a copy of its expressions, so the numbers are the kit's own (about 6 s
a seed). The run picture uses is checked against the committed recording: the same winner, K.O. time, names and
events (the repr of the event list, which the replay guard hashes). For no_ko the import fails (float(None) at
T_KO); the file records the exception and the captions list story.captions gives for that run."""
import hashlib
import os
import subprocess
import sys

from . import common, fights, recording

EPISODE = 'flute_vs_trombone'
TAG = 'show'
FOLDER = common.ref('fights', EPISODE)
OUT = common.ref('show', f'{EPISODE}.json')
RECORDINGS = [('ep392.bfrec', 392)] + [(f'seed{s}.bfrec', s) for s in fights.OTHER_SEEDS]
NO_KO = 'no_ko.bfrec'


def scratch(seed):
    return os.path.join(common.CACHE, 'episodes', TAG, f'picture_{seed}.json')


def events_sha(events):
    return hashlib.sha256(repr(events).encode()).hexdigest()


def caps(windows):
    return [dict(t0=common.hexf(t0), t1=common.hexf(t1), lines=list(lines)) for t0, t1, lines in windows]


def captions(lst):
    return [[common.hexf(t), int(who), name] for t, who, name in lst]


def child():
    """in a fresh interpreter: import kit.picture for the seed in argv[1] and write what it derived"""
    seed = int(sys.argv[1])
    ep = common.episode(EPISODE, TAG)
    assert ep.seed == 392, ep.seed
    ep.seed = seed                                       # story.load() (called by picture at import) reads it
    pkl = os.path.join(ep.check, f'run_{seed}.pkl')
    if os.path.exists(pkl):
        os.remove(pkl)
    try:
        from ballfight.kit import picture as P
    except Exception as e:                                # no K.O.: picture cannot be built for this run
        from ballfight.kit import story
        run = story.load()
        out = dict(seed=seed, error=f'{type(e).__name__}: {e}', winner=run['winner'], t_ko=run['t_ko'],
                   names=list(run['names']), events_sha=events_sha(run['events']),
                   captions=captions(story.captions(run['events'])))
    else:
        T = P.T
        out = dict(seed=seed, winner=P.RUN['winner'], names=list(P.NAMES), events_sha=events_sha(P.EV),
                   win=P.WIN, lose=P.LOSE, t_ko=common.hexf(P.T_KO), hitstop=common.hexf(P.HITSTOP),
                   t_hs=common.hexf(P.T_HS), t_solo=common.hexf(P.T_SOLO),
                   solo_t=[common.hexf(t) for t in P.SOLO_T], cap_gap=common.hexf(float(T.CAP_GAP)),
                   t_reassemble=common.hexf(T.T_REASSEMBLE), captions=captions(P.STORY['caps']),
                   cap_title=caps(P.CAP_TITLE), cap_ability=[caps(P.CAP_ABIL[0]), caps(P.CAP_ABIL[1])],
                   cap_ko=caps(P.CAP_KO), cap_wins=caps(P.CAP_WINS), cap_comment=caps(P.CAP_COMMENT),
                   all=caps(P.CAPTIONS))
        assert P.STORY['caps'] == P.story.captions(P.EV)
        assert P.CAP_TITLE == []
    common.write_json(scratch(seed), out, register=False)


def in_fresh_interpreter(seed):
    code = ('import sys\n'
            'from tools.studio_export.__main__ import check_environment\n'
            'check_environment()\n'
            'from tools.studio_export import show_vectors as m\n'
            'sys.exit(m.child() or 0)\n')
    sys.stdout.flush()
    r = subprocess.run([sys.executable, '-c', code, str(seed)], cwd=common.REPO)
    if r.returncode:
        raise SystemExit(f'show: kit.picture for seed {seed} failed in its own interpreter (exit {r.returncode})')
    import json
    with open(scratch(seed), encoding='utf-8') as f:
        return json.load(f)


def run():
    import json
    common.episode(EPISODE, TAG)
    _, story = common.kit()
    with open(os.path.join(FOLDER, 'coverage.json'), encoding='utf-8') as f:
        no_ko_seed = json.load(f)['no_ko_seed']
    out = {}
    for name, seed in RECORDINGS + [(NO_KO, no_ko_seed)]:
        rec = recording.read_bfrec(os.path.join(FOLDER, name))
        assert rec['seed'] == seed, (name, rec['seed'])
        kit_run = fights.as_kit_run(rec)
        got = in_fresh_interpreter(seed)
        # the run picture uses is the committed recording
        assert got['names'] == rec['names'] and got['winner'] == rec['winner'], name
        assert got['events_sha'] == events_sha(kit_run['events']), name
        want_caps = captions(story.captions(kit_run['events']))
        assert got['captions'] == want_caps, name
        if name == NO_KO:
            assert rec['winner'] is None and rec['t_ko'] is None and got['t_ko'] is None and 'error' in got, got
        else:
            assert 'error' not in got and got['t_ko'] == common.hexf(rec['t_ko']), name
        del got['events_sha']
        out[name] = got
        print(f"show: {name:12s} seed {seed:4d}  "
              + (f"error {got['error']}" if 'error' in got else
                 f"win {got['win']}  K.O. {float.fromhex(got['t_ko'])}  solo {float.fromhex(got['t_solo'])}  "
                 f"captions {len(got['all'])}"))
    doc = dict(
        about=f'{EPISODE}: the show timings and caption windows kit/picture.py derives at import from each '
              f'reference recording\'s run',
        tool='show',
        format=1,
        episode=EPISODE,
        source=dict(kind='python', commit=common.kit_commit()),
        notes=dict(
            key='the recording\'s path relative to studio/reference/fights/flute_vs_trombone/',
            values='picture.py module values: win = WIN = int(RUN["winner"]), lose = LOSE = 1 - WIN, t_ko = T_KO = '
                   'float(RUN["t_ko"]), hitstop = HITSTOP, t_hs = T_HS = T_KO + HITSTOP, t_solo = T_SOLO = '
                   'T.next_bar(T_KO + 1.6), solo_t = SOLO_T = [T_SOLO + bt * T.BEAT for bt, m, d in '
                   'T.SOLO[NAMES[WIN]]], names = NAMES = RUN["names"]; cap_gap and t_reassemble are the timeline\'s',
            captions='STORY["caps"] = story.captions(events) with gap CAP_GAP: [t, who, ability name]',
            caps='_caps() windows {t0, t1, lines} in order: cap_title (always empty), cap_ability[0] and [1], '
                 'cap_ko, cap_wins, cap_comment; all = CAPTIONS = CAP_TITLE + CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + '
                 'CAP_WINS + CAP_COMMENT',
            no_ko='no K.O.: importing kit.picture raises (error); captions is story.captions of that run'),
        recordings=out)
    common.write_json(OUT, doc)
    print(f'show: {common.rel(OUT)}  {len(out)} recordings  {os.path.getsize(OUT)} bytes')


EXPORTS = {'show': run}
ORDER = {'show': 75}                                      # after fights and scenarios: it reads their recordings

"""Run (and cache) #39's honest fight for the chosen seed on the shared BALLFIGHT engine (projects/ballfight/fight.py,
imported, not copied), plus the story analysis shared by seed_search.py, render.py and audio.py.

    python sim.py            # the seed in seed.txt: story summary
    python sim.py 1234       # any seed

    from sim import load
    RUN = load()             # cached in check/run_<seed>.pkl (git-ignored)"""
import os
import pickle
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(1, os.path.join(HERE, '..', '..'))
sys.path.append(os.path.join(HERE, '..', 'ballfight'))     # the engine + roster; appended so this episode's own
                                                            # timeline / sim / render win
import timeline as T
from fight import Fight, Rules, lead_changes
from fighters import FIGHTERS

LOWS = tuple(FIGHTERS[n]['abilities']['low']['name'] for n in T.FIGHTERS)
REC_KEYS = ('P', 'V', 'TH', 'BL', 'FR', 'NV', 'HP', 'LV', 'SH', 'HA', 'SPIN', 'OM', 'K', 'HOP', 'HOOK', 'RATTLE', 'WRECK', 'SNAP')


def rules(dmg_scale=None):
    """the engine's rules at this episode's tempo (beat length, the fight's first beat) and the shared damage scale
    (tuned once with tune.py, then frozen; the fighters' own numbers live in fighters.py)"""
    return Rules(bpm=T.BPM, t_fight=T.T_FIGHT, fps=T.FPS, dmg_scale=T.DMG_SCALE if dmg_scale is None else dmg_scale)


def run_seed(seed, record=True):
    return Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=rules(), record=record).run()


def captions(events, gap=T.CAP_GAP):
    """first use of each ability, greedily kept when >= gap s after the previous kept caption: [(t, who, name)]"""
    out = []
    for e in events:
        if e[1] == 'ability' and e[5]:
            if not out or e[0] - out[-1][0] >= gap:
                out.append((e[0], e[2], e[3]))
    return out


def story(run):
    ev = run['events']
    hp = [100.0, 100.0]
    hits_t = []
    for e in ev:
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            hits_t.append(e[0])
    lc, lc_t = lead_changes(ev)
    firsts = [(e[0], e[2], e[3]) for e in ev if e[1] == 'ability' and e[5]]
    caps = captions(ev)
    tko = run['t_ko']
    w = run['winner']
    whp = max(0.0, hp[w]) if w is not None else 100.0
    gaps = [b - a for a, b in zip([T.T_FIGHT] + hits_t, hits_t)]
    lows = [e[3] for e in ev if e[1] == 'ability' and e[3] in LOWS]
    return dict(seed=run['seed'], winner=w, t_ko=tko, lead_changes=lc, lc_t=lc_t, winner_hp=whp, firsts=firsts,
                caps=caps, max_gap=max(gaps) if gaps else 99, lows=sorted(set(lows)))


def score(st):
    """(fits, score): #27's hard windows from the beat sheet, then a soft story score"""
    tko = st['t_ko'] or 99
    fits = (T.T_KO[0] <= tko <= T.T_KO[1] and st['lead_changes'] >= T.MIN_LEAD_CHANGES
            and st['winner_hp'] <= T.MAX_WINNER_HP and len(st['caps']) >= 5
            and st['max_gap'] <= getattr(T, 'MAX_GAP', 99))
    s = 0.0
    s += (T.MAX_WINNER_HP - st['winner_hp']) / 3.0                       # a close finish
    s += 2.0 * min(4, st['lead_changes'] - 2)
    s += 3.0 * any(t > 40 for t in st['lc_t'])                            # a late lead change
    s += 1.5 * (len(st['caps']) - 5)
    s += 2.0 * (len(st['lows']) == 2)
    for t, who, name in st['caps']:
        s -= abs(t - T.CAP_TARGETS.get(name, t)) / 4.0
    s -= 2.0 * max(0.0, st['max_gap'] - 2.5)
    s -= 2.0 * abs(tko - 51.0)
    return fits, round(s, 2)


def load(seed=None):
    if seed is None:
        seed = int(open(os.path.join(HERE, 'seed.txt')).read().split()[0])
    path = os.path.join(HERE, 'check', f'run_{seed}.pkl')
    if os.path.exists(path):
        with open(path, 'rb') as f:
            return pickle.load(f)
    run = run_seed(seed)
    import numpy as np
    for k in REC_KEYS:
        if k in run:
            run[k] = np.array(run[k])
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + f'.{os.getpid()}.tmp'
    with open(tmp, 'wb') as f:
        pickle.dump(run, f)
    os.replace(tmp, path)
    return run


if __name__ == '__main__':
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else None
    run = load(seed)
    st = story(run)
    print('seed', run['seed'], 'winner', run['names'][run['winner']] if run['winner'] is not None else None,
          'K.O. at', run['t_ko'], 'winner HP', round(st['winner_hp'], 1))
    print('lead changes', st['lead_changes'], [round(t, 1) for t in st['lc_t']], ' longest gap between hits', round(st['max_gap'], 2))
    print('first uses:', [(round(t, 1), run['names'][w], n) for t, w, n in st['firsts']])
    print('captions  :', [(round(t, 1), n) for t, w, n in st['caps']])
    print('fits / score:', score(st))

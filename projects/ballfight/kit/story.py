"""The episode's honest fight on the shared engine (fight.py, imported, never copied) and its story: who led, lead
changes, captions (first use of each ability, >= CAP_GAP s apart), the desperation moves; the beat-sheet score the seed
search ranks runs by (#27's hard windows + soft story score, as every episode since #32 had it in its sim.py).

    RUN = story.load()            # the config's SEED, cached in <episode>/check/run_<seed>.pkl
    st = story.story(RUN); fits, score = story.score(st)"""
import os
import pickle

from .episode import current

# every per-frame array the engine records (any fighter); converted to numpy when they're rectangular
REC_KEYS = ('P', 'V', 'TH', 'BL', 'FR', 'NV', 'HP', 'LV', 'SH', 'HA', 'SPIN', 'OM', 'K', 'EXT', 'SLW', 'CRV', 'DET',
            'TUNE', 'SNIP')


def _T():
    return current().T


def rules(dmg_side=None, dmg_scale=None, **over):
    """the engine's rules at this episode's tempo (beat, the fight's first beat), the shared damage scale, the per-
    fighter handicap (crossovers' only lever) and the config's opt-in rules"""
    from fight import Rules
    T = _T()
    kw = dict(T.RULES)
    side = T.DMG_SIDE if dmg_side is None else dmg_side
    if side is not None:
        kw['dmg_side'] = tuple(side)
    kw.update(over)
    return Rules(bpm=T.BPM, t_fight=T.T_FIGHT, fps=T.FPS, dmg_scale=T.DMG_SCALE if dmg_scale is None else dmg_scale,
                 **kw)


def run_seed(seed, record=True, **kw):
    from fight import Fight
    T = _T()
    return Fight(T.FIGHTERS[0], T.FIGHTERS[1], seed=seed, rules=rules(**kw), record=record).run()


def lows():
    from fighters import FIGHTERS
    return tuple(FIGHTERS[n]['abilities']['low']['name'] for n in _T().FIGHTERS)


def captions(events, gap=None):
    """first use of each ability, greedily kept when >= gap s after the previous kept caption: [(t, who, name)]"""
    gap = _T().CAP_GAP if gap is None else gap
    out = []
    for e in events:
        if e[1] == 'ability' and e[5]:
            if not out or e[0] - out[-1][0] >= gap:
                out.append((e[0], e[2], e[3]))
    return out


def story(run):
    from fight import lead_changes
    T = _T()
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
    lw = lows()
    lo = [e[3] for e in ev if e[1] == 'ability' and e[3] in lw]
    return dict(seed=run['seed'], winner=w, t_ko=tko, lead_changes=lc, lc_t=lc_t, winner_hp=whp, firsts=firsts,
                caps=caps, max_gap=max(gaps) if gaps else 99, lows=sorted(set(lo)))


def score(st):
    """(fits, score): #27's hard windows from the beat sheet, then a soft story score"""
    T = _T()
    tko = st['t_ko'] or 99
    fits = (T.T_KO[0] <= tko <= T.T_KO[1] and st['lead_changes'] >= T.MIN_LEAD_CHANGES
            and st['winner_hp'] <= T.MAX_WINNER_HP and len(st['caps']) >= 5 and st['max_gap'] <= T.MAX_GAP)
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
    """the episode's run (the config's SEED by default), cached in check/run_<seed>.pkl"""
    import numpy as np
    ep = current()
    seed = ep.seed if seed is None else seed
    path = os.path.join(ep.check, f'run_{seed}.pkl')
    if os.path.exists(path):
        with open(path, 'rb') as f:
            return pickle.load(f)
    run = run_seed(seed)
    for k in REC_KEYS:
        if k in run:
            try:
                run[k] = np.array(run[k])
            except ValueError:                            # ragged for this pair: keep the list (as its episode did)
                pass
    tmp = path + f'.{os.getpid()}.tmp'
    with open(tmp, 'wb') as f:
        pickle.dump(run, f)
    os.replace(tmp, path)
    return run


def summary(run):
    st = story(run)
    names = run['names']
    lines = [f"seed {run['seed']}  winner {names[run['winner']] if run['winner'] is not None else None}  K.O. at "
             f"{run['t_ko']}  winner HP {st['winner_hp']:.1f}",
             f"lead changes {st['lead_changes']} {[round(t, 1) for t in st['lc_t']]}  longest gap between hits "
             f"{st['max_gap']:.2f}",
             f"first uses: {[(round(t, 1), names[w], n) for t, w, n in st['firsts']]}",
             f"captions  : {[(round(t, 1), n) for t, w, n in st['caps']]}",
             f"fits / score: {score(st)}"]
    return '\n'.join(lines)


def replay_hash(seed):
    """the replay guard's line for one seed (tools/ballfight_replay_hash.py hashes runs exactly this way)"""
    import hashlib
    import numpy as np
    r = run_seed(seed)
    h = hashlib.sha256(repr(r['events']).encode())
    for k in sorted(r):
        v = r[k]
        if isinstance(v, list) and v and isinstance(v[0], (list, np.ndarray)) and k not in ('events', 'names', 'corner'):
            try:
                h.update(np.ascontiguousarray(np.array(v, float)).tobytes())
            except (ValueError, TypeError):
                h.update(repr(v).encode())
        else:
            h.update(repr(v).encode())
    return f"{seed} {r['winner']} {r['t_ko']} {len(r['events'])} {h.hexdigest()[:16]}"

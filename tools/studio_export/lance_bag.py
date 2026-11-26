"""lance_bag: the TROMBONE's lance against a bag, Python fights in which only the engine core and the lance act.

studio/reference/fights/lance_vs_bag/<kind>_<corner>_s<seed>.bfrec and index.json.

The FLUTE against TROMBONE recordings show the lance at work until the first ability fires (thrusts, hits while it
goes out and while it is held, growth, cobras swatted at rest and in mid-lunge), but no thrust there is ever blocked
(neither fighter has a shield), the lance never grows to its cap before an ability changes the fight, and no thrust
runs under haste before the TROMBONE's shockwave has come first. Here the lance fights a BAG: a fighter with no
weapon (`kind='none'`) and abilities that do nothing to the fight (`kind='none'`), so every thrust, hit, block and
fret comes from the lance and the core alone. A SHIELD BAG is the same with a `low` ability of kind `shield`, so
thrusts meet a shield (`block` events, and the lance pulls straight back). A LANCER is the TROMBONE with its
POWER SLIDE (haste) as its first unlock and nothing else, so thrusts run under haste (on the 8th notes, in half the
time) with no shockwave and no slam in the fight. The three entries are added to the kit's roster table
(fighters.FIGHTERS) for the length of this export only and taken out again; nothing under projects/ is edited, and
the fights run through fight.py's own Fight(...).run().

Rules: the episode's (story.rules: its tempo, start time, damage scale and opt-in rules) with the episode's
handicap on the lance's side (a landed thrust takes 3.5 x 0.4 = 1.4, so no bag is knocked out), then per kind of
fight

    plain     the TROMBONE against the BAG, LV_DMG = (1e9, 1e9): no level, no ability; the lance alone, grown to
              its cap and thrusting on
    shield    the TROMBONE against the SHIELD BAG, no levels, LOW_HP = 97.0: the bag's shield goes up after its
              third hit and blocks thrusts for 8 beats
    haste     the LANCER against the BAG, LV_DMG = (2.0, 1e9): POWER SLIDE after the second hit, and again 6 bars
              later

and for all of them t_max = t_fight + WINDOW: a recording holds the first WINDOW seconds of the fight and ends
without a K.O. LOW_HP is -1.0 where the bag is the plain one, so no `low` ability can fire; the lance's owner never
loses a hit point, so its own never does.

index.json: `fighters` holds the three added entries in the shape of the roster export
(bf-content/data/roster.json), so the Rust roster reader reads them; `fights` lists each recording with its seed,
the lance's corner, its kind, the rules it overrode, its content hash, its events by kind, the abilities fired
(time, fighter, name, kind) and what it covers. The export stops unless the set covers the lance in both corners,
POWER SLIDE, the bag's shield, a blocked thrust, a thrust that misses, a hit while the lance is held, a thrust on
an 8th note, the reach at its cap and a hit landed after that.

The Rust test crates/bf-sim/tests/lance_vs_bag.rs replays every one of these fights and compares every frame and
every event."""
import os

from . import common, fights, manifest, recording, striker_bag

EPISODE = 'flute_vs_trombone'
TAG = 'lance_bag'
FOLDER = common.ref('fights', 'lance_vs_bag')
INDEX = os.path.join(FOLDER, 'index.json')
WINDOW = 16.0                                             # s of fighting in each recording
NEVER = 1e9                                               # damage no fighter deals: a level nobody reaches
NONE = striker_bag.NONE
LANCER = 'LANCER'
KINDS = {                                                 # kind: (the lance's owner, the bag, overrides)
    'plain': ('TROMBONE', 'BAG', dict(LV_DMG=(NEVER, NEVER), LOW_HP=-1.0)),
    'shield': ('TROMBONE', 'SHIELD BAG', dict(LV_DMG=(NEVER, NEVER), LOW_HP=97.0)),
    'haste': (LANCER, 'BAG', dict(LV_DMG=(2.0, NEVER), LOW_HP=-1.0)),
}
FIGHTS = (                                                # kind, the lance's corner (0 left, 1 right), seed
    ('plain', 0, 3), ('plain', 0, 10), ('plain', 1, 2),
    ('shield', 0, 7), ('shield', 0, 10), ('shield', 1, 2),
    ('haste', 0, 10), ('haste', 1, 2), ('haste', 1, 10),
)
MUST_COVER = ('left', 'right', 'ability/POWER SLIDE', 'ability/BRACE', 'block/blade', 'miss', 'hit_held',
              'thrust_on_8th', 'reach_cap', 'hit_at_cap')


def name_of(kind, lance, seed):
    return f"{kind}_{'left' if lance == 0 else 'right'}_s{seed}"


def added():
    """the roster entries this export adds, as fighters.py would write them: the two bags of striker_bag, and the
    LANCER, the kit's own TROMBONE entry with POWER SLIDE as its first unlock and nothing else"""
    trombone = common.roster()['TROMBONE']
    slide = trombone['abilities']['unlock2']
    assert trombone['weapon'].get('mode') == 'thrust' and (slide['name'], slide['kind']) == ('POWER SLIDE', 'haste')
    lancer = dict(trombone, weapon=dict(trombone['weapon']),
                  abilities=dict(unlock1=dict(slide), unlock2=dict(NONE), low=dict(NONE)))
    return {**{n: dict(cfg) for n, cfg in striker_bag.BAGS.items()}, LANCER: lancer}


def with_added(fn):
    """run fn(entries) with the added entries in the kit's roster table, and take them out again whatever happens"""
    table = common.roster()
    entries = added()
    clash = [n for n in entries if n in table]
    assert not clash, f'the kit already has a fighter named {clash}'
    table.update(entries)
    try:
        return fn(entries)
    finally:
        for n in entries:
            table.pop(n, None)


def rules_of(story, T, kind, lance, window=WINDOW):
    """the rules of one fight and what was overridden: the episode's, its handicap on the lance's side, the kind's
    overrides and the window"""
    who, bag, over = KINDS[kind]
    hc = dict(zip(T.FIGHTERS, T.DMG_SIDE))['TROMBONE']    # the episode's handicap on the TROMBONE
    side = (hc, 1.0) if lance == 0 else (1.0, hc)         # (the bag deals no damage: its entry scales nothing)
    over = dict(over, t_max=T.T_FIGHT + window)
    rules = story.rules(dmg_side=side, **over)
    assert tuple(rules.dmg_side) == tuple(side) and all(getattr(rules, k) == v for k, v in over.items())
    return rules, dict(over, dmg_side=side), who, bag


def fight(story, T, kind, lance, seed, window=WINDOW):
    """one fight, recorded: (the kit's run dict, its rules, what was overridden, the lance's owner, the bag). Call
    inside with_added"""
    from fight import Fight                               # the engine, on the path the kit set up (as story.run_seed)
    rules, over, who, bag = rules_of(story, T, kind, lance, window)
    names = (who, bag) if lance == 0 else (bag, who)
    return Fight(names[0], names[1], seed=seed, rules=rules, record=True).run(), rules, over, who, bag


def cap_of(w):
    """the reach a lance ends at: `grow` added while the result stays within `max_len` (+ 1e-9), as thrust_step
    adds it, hit by hit; and the fret count then"""
    reach, frets = w.get('base', 0.0), w.get('frets0', 0)
    while reach + w['grow'] <= w['max_len'] + 1e-9:
        reach += w['grow']
        frets += 1
    return reach, frets


def covers(run, lance, w, rules):
    """what one fight shows, as names of MUST_COVER and more: the lance's corner, each ability fired, a blocked
    thrust, a thrust that neither hit nor was blocked (well before the window's end), a hit at least `t_out` after
    its thrust started (so the lance was held by then, hasted or not), a thrust less than three quarters of a beat
    after the one before (on an 8th note: haste), the reach at its cap, and a hit landed with the lance at its cap
    (its sub is the capped fret count: nothing grew)"""
    ev = run['events']
    out = {'left' if lance == 0 else 'right'}
    out.update(f'ability/{e[3]}' for e in ev if e[1] == 'ability')
    out.update(f'block/{e[6]}' for e in ev if e[1] == 'block')
    thrusts = [e[0] for e in ev if e[1] == 'thrust']
    length = w['t_out'] + w['t_hold'] + w['t_back']
    for i, t0 in enumerate(thrusts):
        t1 = thrusts[i + 1] if i + 1 < len(thrusts) else rules.t_max
        mine = [e for e in ev if e[1] in ('hit', 'block') and t0 <= e[0] < t1]
        if not mine and t0 + length + 0.02 < rules.t_max:
            out.add('miss')
        if any(e[1] == 'hit' and e[0] - t0 >= w['t_out'] + 1e-4 for e in mine):
            out.add('hit_held')
        if i and t0 - thrusts[i - 1] < 0.75 * rules.beat:
            out.add('thrust_on_8th')
    reach, frets = cap_of(w)
    if max(bl[lance] for bl in run['BL']) == reach:
        out.add('reach_cap')
    if any(e[1] == 'hit' and e[8] == frets for e in ev):
        out.add('hit_at_cap')
    return sorted(out)


def by_kind(events):
    out = {}
    for e in events:
        out[e[1]] = out.get(e[1], 0) + 1
    return out


def run():
    assert FIGHTS and len(set(FIGHTS)) == len(FIGHTS)
    ep = common.episode(EPISODE, TAG)
    _, story = common.kit()
    T = ep.T
    assert tuple(T.FIGHTERS) == ('FLUTE', 'TROMBONE'), T.FIGHTERS
    w = common.roster()['TROMBONE']['weapon']
    assert w['kind'] == 'blade' and w.get('mode') == 'thrust'

    def all_fights(entries):
        rows = []
        for kind, lance, seed in FIGHTS:
            name = name_of(kind, lance, seed)
            out = os.path.join(FOLDER, f'{name}.bfrec')
            r, rules, over, who, bag = fight(story, T, kind, lance, seed)
            assert r['winner'] is None and r['t_ko'] is None, f'{name}: a K.O. inside the window'
            assert list(r['names']) == ([who, bag] if lance == 0 else [bag, who]), r['names']
            strangers = sorted({e[1] for e in r['events']} - {'thrust', 'hit', 'block', 'level', 'ability', 'rope',
                                                                 'clash'})
            assert not strangers, f'{name}: events of kinds {strangers}: something but the core and the lance acted'
            assert all(e[2] == lance for e in r['events'] if e[1] in ('thrust', 'hit', 'level'))
            assert all(e[7] == 'blade' for e in r['events'] if e[1] == 'hit')
            assert all(e[2] == 1 - lance and e[6] == 'blade' for e in r['events'] if e[1] == 'block')
            recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)
            rec = recording.read_bfrec(out)
            fights.check(r, rules, rec, story.REC_KEYS)
            manifest.add(out)
            row = dict(
                name=name, file=common.rel(out)[len(common.rel(common.REFERENCE)) + 1:], kind=kind, lance=lance,
                seed=seed, fighter=who, bag=bag, overrides=over, frames=len(rec['arrays']['HP']),
                hash=recording.rec_hash(rec), events=by_kind(rec['events']),
                abilities=[[e[0], e[2], e[3], e[4]] for e in rec['events'] if e[1] == 'ability'],
                covers=covers(r, lance, w, rules))
            rows.append(row)
            print(f"lance_bag: {common.rel(out)}  {os.path.getsize(out)} bytes  frames {row['frames']}  "
                  f"events {len(rec['events'])}  hash {row['hash']}  covers {', '.join(row['covers'])}")
        return rows, entries

    rows, entries = with_added(all_fights)
    assert not any(n in common.roster() for n in entries), 'the added fighters are still in the kit\'s roster table'
    seen = {c for row in rows for c in row['covers']}
    missing = [c for c in MUST_COVER if c not in seen]
    if missing:
        raise SystemExit(f"lance_bag: no fight covers {', '.join(missing)}")
    doc = dict(
        about='The TROMBONE\'s lance against a bag: Python fights in which only the engine core and the lance act, '
              'made by tools/studio_export/lance_bag.py (its docstring has the rules); do not edit by hand. '
              '"fighters" are the entries added to the kit\'s roster table for these fights, in the shape of '
              'bf-content/data/roster.json: two bags, and the LANCER, the TROMBONE with POWER SLIDE as its only '
              'ability. Each fight is the first "window" seconds after "t_fight".',
        tool='lance_bag',
        format=1,
        episode=EPISODE,
        t_fight=T.T_FIGHT,
        window=WINDOW,
        must_cover=list(MUST_COVER),
        fighters=[dict(name=n, **cfg) for n, cfg in entries.items()],
        fights=rows)
    common.write_json(INDEX, doc, sort_keys=False)
    print(f'lance_bag: {common.rel(INDEX)}  {len(rows)} fights; covered: {", ".join(sorted(seen))}')


EXPORTS = {'lance_bag': run}

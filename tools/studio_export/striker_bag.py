"""striker_bag: the FLUTE against a bag, whole Python fights in which only the engine core and the striker act.

studio/reference/fights/striker_vs_bag/<kind>_<corner>_s<seed>.bfrec and index.json.

In every FLUTE against TROMBONE recording the lance lands before a cobra has lunged, so those recordings cannot show
the striker (the baskets, the lunges, the bites, the growth) apart from the lance. Here the FLUTE fights a BAG: a
fighter with no weapon (`kind='none'`) and abilities that do nothing to the fight (`kind='none'`), so every lunge,
bite, level, FLUTTER TONGUE and SNAKE DANCE comes from the striker and the core alone. A SHIELD BAG is the same
with a `low` ability of kind `shield`, so some bites meet a shield (`block` events). The two entries are added to
the kit's roster table (fighters.FIGHTERS) for the length of this export only and taken out again; nothing under
projects/ is edited, and the fights run through fight.py's own Fight(...).run().

Rules: the episode's (story.rules: its tempo, start time, damage scale and opt-in rules), then per kind of fight

    episode   the episode's handicap, the FLUTE's side dealing x 2.0 (so a bite takes 2.16)
    even      no handicap (dmg_side = (1.0, 1.0)): the longest runs of plain lunges before the first unlock
    early     no handicap, LV_DMG = (2.0, 5.0): both unlocks come after a few bites, as in the early_levels scenario
    shield    as early, against the SHIELD BAG, with LOW_HP = 96.0: the bag's shield goes up after its fourth bite

and for all of them t_max = t_fight + WINDOW: a recording holds the first WINDOW seconds of the fight and ends
without a K.O. (the Rust engine agrees with Python to 1e-9 for at least 13 s in every such fight tried; the chaos of
the fight then grows the last-bit differences of the two arithmetic libraries past any tolerance). LOW_HP is -1.0
where the bag is the plain one, so no `low` ability can fire; the FLUTE never loses a hit point, so its own never
does.

index.json: `fighters` holds the two bag entries in the shape of the roster export (bf-content/data/roster.json), so
the Rust roster reader reads them; `fights` lists each recording with its seed, corner, kind, the rules it overrode,
its content hash, its events by kind, the abilities fired (time, fighter, name, kind) and what it covers. The export
stops unless the set covers the FLUTE in both corners, both unlocks, a blocked bite, a missed lunge, a basket taken
back, the reach at its cap and the lunge at its shortest.

The Rust test crates/bf-sim/tests/striker_vs_bag.rs replays every one of these fights and compares every frame and
every event."""
import os

from . import common, fights, manifest, recording

EPISODE = 'flute_vs_trombone'
TAG = 'striker_bag'
FOLDER = common.ref('fights', 'striker_vs_bag')
INDEX = os.path.join(FOLDER, 'index.json')
WINDOW = 10.0                                             # s of fighting in each recording
NONE = dict(name='NOTHING', kind='none', every_bars=4)
BAGS = {                                                  # roster entries, as fighters.py would write them
    'BAG': dict(color=(200, 200, 200), dark=(90, 90, 90), weapon=dict(kind='none'),
                abilities=dict(unlock1=dict(NONE), unlock2=dict(NONE), low=dict(NONE)),
                pp='', register=(60, 72), idiom=''),
    'SHIELD BAG': dict(color=(200, 200, 200), dark=(90, 90, 90), weapon=dict(kind='none'),
                       abilities=dict(unlock1=dict(NONE), unlock2=dict(NONE),
                                      low=dict(name='BRACE', kind='shield', dur_beats=8)),
                       pp='', register=(60, 72), idiom=''),
}
EVEN = (1.0, 1.0)
KINDS = {                                                 # kind: (bag, dmg_side or None for the episode's, overrides)
    'episode': ('BAG', None, dict(LOW_HP=-1.0)),
    'even': ('BAG', EVEN, dict(LOW_HP=-1.0)),
    'early': ('BAG', EVEN, dict(LV_DMG=(2.0, 5.0), LOW_HP=-1.0)),
    'shield': ('SHIELD BAG', EVEN, dict(LV_DMG=(2.0, 5.0), LOW_HP=96.0)),
}
FIGHTS = (                                                # kind, the FLUTE's corner (0 left, 1 right), seed
    ('episode', 0, 5), ('episode', 0, 9), ('episode', 1, 8), ('episode', 1, 9),
    ('even', 0, 5), ('even', 1, 8),
    ('early', 0, 5), ('early', 0, 9), ('early', 1, 3), ('early', 1, 8),
    ('shield', 0, 9), ('shield', 1, 8),
)
MUST_COVER = ('left', 'right', 'ability/FLUTTER TONGUE', 'ability/SNAKE DANCE', 'ability/BRACE', 'block/fang',
              'miss', 'taken_back', 'reach_cap', 't_min')


def name_of(kind, flute, seed):
    return f"{kind}_{'left' if flute == 0 else 'right'}_s{seed}"


def with_bags(fn):
    """run fn() with the two bags in the kit's roster table, and take them out again whatever happens"""
    table = common.roster()
    clash = [n for n in BAGS if n in table]
    assert not clash, f'the kit already has a fighter named {clash}'
    table.update({n: dict(cfg) for n, cfg in BAGS.items()})
    try:
        return fn()
    finally:
        for n in BAGS:
            table.pop(n, None)


def rules_of(story, T, kind, flute):
    """the rules of one fight and what was overridden: the episode's, the handicap on the FLUTE's side (or none), the
    kind's overrides and the window"""
    bag, side, over = KINDS[kind]
    if side is None:
        side = tuple(T.DMG_SIDE) if flute == 0 else tuple(reversed(T.DMG_SIDE))
    over = dict(over, t_max=T.T_FIGHT + WINDOW)
    rules = story.rules(dmg_side=side, **over)
    assert tuple(rules.dmg_side) == tuple(side) and all(getattr(rules, k) == v for k, v in over.items())
    return rules, dict(over, dmg_side=side), bag


def fight(story, T, kind, flute, seed):
    """one fight, recorded: (the kit's run dict, its rules, what was overridden, the bag's name). Call inside
    with_bags"""
    from fight import Fight                               # the engine, on the path the kit set up (as story.run_seed)
    rules, over, bag = rules_of(story, T, kind, flute)
    names = ('FLUTE', bag) if flute == 0 else (bag, 'FLUTE')
    return Fight(names[0], names[1], seed=seed, rules=rules, record=True).run(), rules, over, bag


def covers(run, flute, w, t_max):
    """what one fight shows, as names of MUST_COVER and more: the FLUTE's corner, each ability fired, a blocked bite,
    a lunge that bit nothing (no hit or block around the time its bite was due, well before the window's end), a
    basket taken back, the reach at max_reach, a lunge as short as t_min (or half of it, under haste)"""
    ev = run['events']
    out = {'left' if flute == 0 else 'right'}
    out.update(f'ability/{e[3]}' for e in ev if e[1] == 'ability')
    out.update(f'block/{e[6]}' for e in ev if e[1] == 'block')
    bites = [e[0] for e in ev if e[1] in ('hit', 'block')]
    for e in ev:
        if e[1] == 'strike' and e[0] + e[6] + 0.02 < t_max:
            due = e[0] + e[6]                             # the bite is tested on the first step at or after this
            if not any(due - 1e-4 <= t <= due + 0.01 for t in bites):
                out.add('miss')
    if any(e[1] == 'basket' and e[6] >= 0 for e in ev):
        out.add('taken_back')
    if max(bl[flute] for bl in run['BL']) == w['max_reach']:
        out.add('reach_cap')
    if any(e[1] == 'strike' and e[6] in (w['t_min'], w['t_min'] / 2.0) for e in ev):
        out.add('t_min')
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
    w = common.roster()['FLUTE']['weapon']
    assert w['kind'] == 'striker'

    def all_fights():
        rows = []
        for kind, flute, seed in FIGHTS:
            name = name_of(kind, flute, seed)
            out = os.path.join(FOLDER, f'{name}.bfrec')
            r, rules, over, bag = fight(story, T, kind, flute, seed)
            assert r['winner'] is None and r['t_ko'] is None, f'{name}: a K.O. inside the window'
            assert list(r['names']) == (['FLUTE', bag] if flute == 0 else [bag, 'FLUTE']), r['names']
            strangers = sorted({e[1] for e in r['events']} - {'basket', 'strike', 'hit', 'block', 'level', 'ability',
                                                                 'rope', 'clash'})
            assert not strangers, f'{name}: events of kinds {strangers}: something but the core and the striker acted'
            assert all(e[2] == flute for e in r['events'] if e[1] in ('basket', 'strike', 'hit', 'level'))
            assert all(e[7] == 'fang' for e in r['events'] if e[1] == 'hit')
            recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)
            rec = recording.read_bfrec(out)
            fights.check(r, rules, rec, story.REC_KEYS)
            manifest.add(out)
            row = dict(
                name=name, file=common.rel(out)[len(common.rel(common.REFERENCE)) + 1:], kind=kind, flute=flute,
                seed=seed, bag=bag, overrides=over, frames=len(rec['arrays']['HP']), hash=recording.rec_hash(rec),
                events=by_kind(rec['events']),
                abilities=[[e[0], e[2], e[3], e[4]] for e in rec['events'] if e[1] == 'ability'],
                covers=covers(r, flute, w, rules.t_max))
            rows.append(row)
            print(f"striker_bag: {common.rel(out)}  {os.path.getsize(out)} bytes  frames {row['frames']}  "
                  f"events {len(rec['events'])}  hash {row['hash']}  covers {', '.join(row['covers'])}")
        return rows

    rows = with_bags(all_fights)
    assert not any(n in common.roster() for n in BAGS), 'the bags are still in the kit\'s roster table'
    seen = {c for row in rows for c in row['covers']}
    missing = [c for c in MUST_COVER if c not in seen]
    if missing:
        raise SystemExit(f"striker_bag: no fight covers {', '.join(missing)}")
    doc = dict(
        about='The FLUTE against a bag: Python fights in which only the engine core and the striker act, made by '
              'tools/studio_export/striker_bag.py (its docstring has the rules); do not edit by hand. "fighters" '
              'are the two bag entries added to the kit\'s roster table for these fights, in the shape of '
              'bf-content/data/roster.json. Each fight is the first "window" seconds after "t_fight".',
        tool='striker_bag',
        format=1,
        episode=EPISODE,
        t_fight=T.T_FIGHT,
        window=WINDOW,
        must_cover=list(MUST_COVER),
        fighters=[dict(name=n, **cfg) for n, cfg in BAGS.items()],
        fights=rows)
    common.write_json(INDEX, doc, sort_keys=False)
    print(f'striker_bag: {common.rel(INDEX)}  {len(rows)} fights; covered: {", ".join(sorted(seen))}')


EXPORTS = {'striker_bag': run}

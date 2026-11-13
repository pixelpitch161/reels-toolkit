"""knife_edge: a Python fight in which two balls travel side by side exactly R + R apart, so `clash` events are made
or not on the last bit of a distance.

studio/reference/fights/knife_edge/core_s<seed>.bfrec and index.json.

The Rust engine and the kit agree to about 1e-13 in such a fight and still emit different `clash` events: after a
clash the balls are R + R apart to rounding, and when they then fly on together, `dist < R + R` (fight.py, `move`)
is decided on every following step by the last bit of `dist`. That is no mistake of the port, and the parity suite
must tell it from one: `bf_sim::parity::prefix_window` calls it a knife-edge and names the event, and
`clash_margin` gives the margin of the comparison. This recording is the real sample the Rust test
crates/bf-sim/tests/prefix.rs (`a_knife_edge_clash_is_told_from_a_logic_break`) shows that on.

The fight is core only: a BAG against a SHIELD BAG (the two entries of `striker_bag`: no weapon, abilities that do
nothing), under the episode's rules with no handicap and LOW_HP = -1.0, so nothing but the balls, the ropes and the
clashes acts and no ability can fire. t_max = t_fight + WINDOW: the recording holds the first WINDOW seconds. The
two entries are added to the kit's roster table (fighters.FIGHTERS) for the length of this export only and taken
out again; nothing under projects/ is edited, and the fight runs through fight.py's own Fight(...).run().

The export stops unless the fight holds a run of at least MIN_RUN `clash` events on consecutive steps: the balls
travelling side by side. index.json lists the recording with its seed, rules, content hash, events by kind and that
run (its first and last time and its length); `fighters` holds the two bag entries in the shape of the roster
export, so the Rust roster reader reads them."""
import os

from . import common, fights, manifest, recording, striker_bag

EPISODE = 'flute_vs_trombone'
TAG = 'knife_edge'
FOLDER = common.ref('fights', 'knife_edge')
INDEX = os.path.join(FOLDER, 'index.json')
WINDOW = 13.0                                             # s of fighting in the recording
SEEDS = (4,)                                              # seed 4: side by side from 11.1 s into the fight
MIN_RUN = 10                                              # clashes on consecutive steps that count as side by side
NAMES = ('BAG', 'SHIELD BAG')
OVER = dict(dmg_side=(1.0, 1.0), LOW_HP=-1.0)


def longest_run(events, dt):
    """the longest run of `clash` events on consecutive steps: (first time, last time, how many)"""
    best, run = (None, None, 0), []
    for e in events:
        if e[1] != 'clash':
            continue
        if run and abs(e[0] - run[-1] - dt) > 2e-5:      # event times are rounded to 5 places
            run = []
        run.append(e[0])
        if len(run) > best[2]:
            best = (run[0], run[-1], len(run))
    return best


def run():
    ep = common.episode(EPISODE, TAG)
    _, story = common.kit()
    T = ep.T
    over = dict(OVER, t_max=T.T_FIGHT + WINDOW)
    rules = story.rules(**over)
    assert all(getattr(rules, k) == v for k, v in over.items())

    def all_fights():
        from fight import Fight                           # the engine, on the path the kit set up (as story.run_seed)
        rows = []
        for seed in SEEDS:
            name = f'core_s{seed}'
            out = os.path.join(FOLDER, f'{name}.bfrec')
            r = Fight(NAMES[0], NAMES[1], seed=seed, rules=rules, record=True).run()
            assert r['winner'] is None and r['t_ko'] is None and list(r['names']) == list(NAMES), name
            strangers = sorted({e[1] for e in r['events']} - {'rope', 'clash'})
            assert not strangers, f'{name}: events of kinds {strangers}: something but the core acted'
            first, last, n = longest_run(r['events'], rules.DT)
            if n < MIN_RUN:
                raise SystemExit(f'knife_edge: {name}: the longest run of clashes on consecutive steps is {n}, '
                                 f'under {MIN_RUN}: the balls do not travel side by side in this fight')
            recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)
            rec = recording.read_bfrec(out)
            fights.check(r, rules, rec, story.REC_KEYS)
            manifest.add(out)
            kinds = {}
            for e in rec['events']:
                kinds[e[1]] = kinds.get(e[1], 0) + 1
            rows.append(dict(
                name=name, file=common.rel(out)[len(common.rel(common.REFERENCE)) + 1:], seed=seed,
                fighters=list(NAMES), overrides=over, frames=len(rec['arrays']['HP']), hash=recording.rec_hash(rec),
                events=kinds, side_by_side=dict(first=first, last=last, clashes=n)))
            print(f"knife_edge: {common.rel(out)}  {os.path.getsize(out)} bytes  frames {rows[-1]['frames']}  "
                  f"events {len(rec['events'])}  hash {rows[-1]['hash']}  {n} clashes on consecutive steps from "
                  f"{first} s to {last} s")
        return rows

    rows = striker_bag.with_bags(all_fights)
    assert not any(n in common.roster() for n in striker_bag.BAGS), 'the bags are still in the kit\'s roster table'
    doc = dict(
        about='A core-only Python fight (two bags, no weapon, no ability) in which the balls travel side by side '
              'exactly R + R apart, so clash events are made or not on the last bit of a distance; made by '
              'tools/studio_export/knife_edge.py (its docstring has the rules); do not edit by hand. "fighters" are '
              'the two bag entries added to the kit\'s roster table for the fight, in the shape of '
              'bf-content/data/roster.json. The fight is the first "window" seconds after "t_fight"; '
              '"side_by_side" is its longest run of clashes on consecutive steps.',
        tool='knife_edge',
        format=1,
        episode=EPISODE,
        t_fight=T.T_FIGHT,
        window=WINDOW,
        min_run=MIN_RUN,
        fighters=[dict(name=n, **cfg) for n, cfg in striker_bag.BAGS.items()],
        fights=rows)
    common.write_json(INDEX, doc, sort_keys=False)
    print(f'knife_edge: {common.rel(INDEX)}  {len(rows)} fight(s)')


EXPORTS = {'knife_edge': run}
